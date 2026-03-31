use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind};
use pretext::{PrepareOptions, PreparedTextWithSegments, PretextEngine, WhiteSpaceMode};

use crate::assets::AssetRegistry;
use crate::demos::DemoWindow;

const CARD_PADDING_X: f32 = 16.0;
const CARD_PADDING_Y: f32 = 16.0;
const CARD_GAP: f32 = 12.0;
const LINE_HEIGHT: f32 = 22.0;
const MAX_COL_WIDTH: f32 = 400.0;
const SINGLE_COLUMN_MAX_VIEWPORT_WIDTH: f32 = 520.0;
const VIEWPORT_OVERSCAN: f32 = 200.0;

pub struct MasonryDemo {
    open: bool,
    cards_hash: u64,
    cards: Option<Arc<[MasonryPreparedCard]>>,
    layout_state: Option<MasonryLayoutState>,
}

impl Default for MasonryDemo {
    fn default() -> Self {
        Self {
            open: false,
            cards_hash: 0,
            cards: None,
            layout_state: None,
        }
    }
}

#[derive(Clone)]
struct MasonryPreparedCard {
    prepared: PreparedTextWithSegments,
}

struct MasonryLayoutState {
    memo_key: u64,
    layout: MasonryLayout,
    card_visuals: Arc<[MasonryCardVisual]>,
}

#[derive(Clone, Debug)]
struct MasonryCardVisual {
    lines: Vec<String>,
    total_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MasonryColumns {
    col_count: usize,
    col_width: f32,
    text_width: f32,
    content_width: f32,
    offset_left: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PositionedCard {
    card_index: usize,
    x: f32,
    y: f32,
    height: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct MasonryLayout {
    col_count: usize,
    col_width: f32,
    content_width: f32,
    content_height: f32,
    positioned_cards: Vec<PositionedCard>,
}

impl DemoWindow for MasonryDemo {
    fn title(&self) -> &str {
        "Masonry"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, _assets: &mut AssetRegistry) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(980.0, 640.0))
            .show(ctx, |ui| {
                let available_width = ui.available_width().max(240.0);
                let state = self.ensure_layout_state(engine, available_width);

                ui.horizontal(|ui| {
                    ui.label(format!("{} cards", state.card_visuals.len()));
                    ui.separator();
                    ui.label(format!("{} columns", state.layout.col_count));
                    ui.separator();
                    ui.label(format!("{:.0}px column width", state.layout.col_width));
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        let content_width = state.layout.content_width.max(ui.available_width());
                        ui.set_width(content_width);
                        ui.set_height(state.layout.content_height);

                        let origin = ui.max_rect().min;
                        let painter = ui.painter().clone();
                        let view_top = (viewport.min.y - VIEWPORT_OVERSCAN).max(0.0);
                        let view_bottom = viewport.max.y + VIEWPORT_OVERSCAN;

                        for card in &state.layout.positioned_cards {
                            if card.y > view_bottom || card.y + card.height < view_top {
                                continue;
                            }

                            let rect = Rect::from_min_size(
                                origin + egui::vec2(card.x, card.y),
                                egui::vec2(state.layout.col_width, card.height),
                            );
                            ui.allocate_rect(rect, Sense::hover());
                            paint_masonry_card(
                                &painter,
                                rect,
                                &state.card_visuals[card.card_index],
                            );
                        }
                    });
            });
        self.open = open;
    }
}

impl MasonryDemo {
    fn ensure_cards(&mut self, engine: &PretextEngine) -> Arc<[MasonryPreparedCard]> {
        if let Some(cards) = &self.cards {
            return cards.clone();
        }

        let style = masonry_text_style();
        let options = PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        };
        let texts = load_masonry_texts();
        self.cards_hash = hash_card_texts(&texts);

        let cards: Arc<[MasonryPreparedCard]> = texts
            .into_iter()
            .map(|text| {
                let text: Arc<str> = Arc::from(text);
                let prepared = engine.prepare_with_segments(text.as_ref(), &style, &options);
                MasonryPreparedCard { prepared }
            })
            .collect::<Vec<_>>()
            .into();
        self.cards = Some(cards.clone());
        cards
    }

    fn ensure_layout_state(
        &mut self,
        engine: &PretextEngine,
        available_width: f32,
    ) -> &MasonryLayoutState {
        let cards = self.ensure_cards(engine);
        let columns = compute_masonry_columns(available_width);
        let memo_key = masonry_memo_key(self.cards_hash, columns.col_width);
        let rebuild = self
            .layout_state
            .as_ref()
            .is_none_or(|state| state.memo_key != memo_key);

        if rebuild {
            let card_visuals = compute_card_visuals(engine, &cards, columns.text_width);
            let heights: Vec<f32> = card_visuals.iter().map(|card| card.total_height).collect();
            let layout = compute_masonry_layout_with_columns(&heights, columns);
            self.layout_state = Some(MasonryLayoutState {
                memo_key,
                layout,
                card_visuals: Arc::from(card_visuals),
            });
        }

        self.layout_state
            .as_ref()
            .expect("masonry layout state should exist")
    }
}

fn masonry_text_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 15.0,
        weight: 400,
        italic: false,
    }
}

fn load_masonry_texts() -> Vec<String> {
    serde_json::from_str(include_str!("../../assets/masonry.json"))
        .expect("masonry.json should contain a string array")
}

fn hash_card_texts(texts: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    texts.hash(&mut hasher);
    hasher.finish()
}

fn masonry_memo_key(cards_hash: u64, col_width: f32) -> u64 {
    let mut hasher = DefaultHasher::new();
    cards_hash.hash(&mut hasher);
    ((col_width * 8.0).round() as u32).hash(&mut hasher);
    hasher.finish()
}

fn compute_card_visuals(
    engine: &PretextEngine,
    cards: &[MasonryPreparedCard],
    text_width: f32,
) -> Vec<MasonryCardVisual> {
    cards
        .iter()
        .map(|card| {
            let layout = engine.layout_with_lines(&card.prepared, text_width.max(1.0), LINE_HEIGHT);
            let lines = layout
                .lines
                .into_iter()
                .map(|line| line.text)
                .collect::<Vec<_>>();
            MasonryCardVisual {
                lines,
                total_height: layout.height + CARD_PADDING_Y * 2.0,
            }
        })
        .collect()
}

fn compute_masonry_columns(viewport_width: f32) -> MasonryColumns {
    let viewport_width = viewport_width.max(180.0);
    let (col_count, col_width) = if viewport_width <= SINGLE_COLUMN_MAX_VIEWPORT_WIDTH {
        (1usize, (viewport_width - CARD_GAP * 2.0).min(MAX_COL_WIDTH))
    } else {
        let min_col_width = 100.0 + viewport_width * 0.1;
        let col_count =
            (((viewport_width + CARD_GAP) / (min_col_width + CARD_GAP)).floor() as usize).max(2);
        let col_width = ((viewport_width - (col_count as f32 + 1.0) * CARD_GAP) / col_count as f32)
            .min(MAX_COL_WIDTH);
        (col_count, col_width)
    };

    let content_width =
        col_count as f32 * col_width + (col_count.saturating_sub(1) as f32) * CARD_GAP;
    let offset_left = ((viewport_width - content_width) * 0.5).max(0.0);

    MasonryColumns {
        col_count,
        col_width,
        text_width: (col_width - CARD_PADDING_X * 2.0).max(1.0),
        content_width,
        offset_left,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn compute_masonry_layout(card_heights: &[f32], viewport_width: f32) -> MasonryLayout {
    compute_masonry_layout_with_columns(card_heights, compute_masonry_columns(viewport_width))
}

fn compute_masonry_layout_with_columns(
    card_heights: &[f32],
    columns: MasonryColumns,
) -> MasonryLayout {
    let mut col_heights = vec![CARD_GAP; columns.col_count];
    let mut positioned_cards = Vec::with_capacity(card_heights.len());

    for (card_index, &height) in card_heights.iter().enumerate() {
        let mut shortest_col = 0usize;
        for col in 1..columns.col_count {
            if col_heights[col] < col_heights[shortest_col] {
                shortest_col = col;
            }
        }

        positioned_cards.push(PositionedCard {
            card_index,
            x: columns.offset_left + shortest_col as f32 * (columns.col_width + CARD_GAP),
            y: col_heights[shortest_col],
            height,
        });
        col_heights[shortest_col] += height + CARD_GAP;
    }

    let content_height = col_heights.into_iter().fold(0.0f32, f32::max).max(CARD_GAP);

    MasonryLayout {
        col_count: columns.col_count,
        col_width: columns.col_width,
        content_width: columns.content_width,
        content_height,
        positioned_cards,
    }
}

fn paint_masonry_card(painter: &egui::Painter, rect: Rect, card: &MasonryCardVisual) {
    painter.rect_filled(
        rect,
        CornerRadius::same(16),
        Color32::from_rgb(248, 246, 241),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(16),
        Stroke::new(1.0, Color32::from_rgb(217, 211, 201)),
        StrokeKind::Inside,
    );

    let font_id = FontId::new(15.0, FontFamily::Proportional);
    let mut y = rect.top() + CARD_PADDING_Y;
    let x = rect.left() + CARD_PADDING_X;

    for line in &card.lines {
        painter.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            line,
            font_id.clone(),
            Color32::from_rgb(39, 44, 52),
        );
        y += LINE_HEIGHT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masonry_asset_respects_card_budget() {
        let texts = load_masonry_texts();
        assert!(texts.len() > 32);
        assert!(texts.len() <= 200);
    }

    #[test]
    fn masonry_layout_is_deterministic() {
        let heights = vec![120.0, 84.0, 160.0, 96.0, 132.0, 78.0, 142.0, 110.0];
        let first = compute_masonry_layout(&heights, 960.0);
        let second = compute_masonry_layout(&heights, 960.0);
        assert_eq!(first, second);
    }

    #[test]
    fn masonry_visuals_follow_engine_measurement() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let texts = vec![
            "A short card.".to_owned(),
            "A longer card that forces at least one extra line in the masonry grid.".to_owned(),
        ];
        let cards: Vec<MasonryPreparedCard> = texts
            .into_iter()
            .map(|text| {
                let text: Arc<str> = Arc::from(text);
                let prepared = engine.prepare_with_segments(
                    text.as_ref(),
                    &masonry_text_style(),
                    &PrepareOptions {
                        white_space: WhiteSpaceMode::Normal,
                        paragraph_direction: pretext::ParagraphDirection::Auto,
                    },
                );
                MasonryPreparedCard { prepared }
            })
            .collect();

        let visuals = compute_card_visuals(&engine, &cards, 220.0);
        assert_eq!(visuals.len(), 2);
        assert!(visuals[0].total_height >= CARD_PADDING_Y * 2.0);
        assert!(visuals[1].total_height > visuals[0].total_height);
    }
}

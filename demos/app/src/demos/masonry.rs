use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{Color32, CornerRadius, Rect, Sense};
use pretext::{
    LayoutLine, LayoutLineGlyphRun, PrepareOptions, PreparedTextWithSegments, PretextEngine,
    WhiteSpaceMode,
};
use pretext_egui::AssetRegistry;

use crate::demos::{text_runs::paint_glyph_runs, DemoWindow};

const CARD_PADDING_X: f32 = 16.0;
const CARD_PADDING_Y: f32 = 16.0;
const CARD_GAP: f32 = 12.0;
const LINE_HEIGHT: f32 = 22.0;
const MAX_COL_WIDTH: f32 = 400.0;
const SINGLE_COLUMN_MAX_VIEWPORT_WIDTH: f32 = 520.0;
const VIEWPORT_OVERSCAN: f32 = 200.0;
const FRAME_BUILD_CARD_BUDGET: usize = 48;
const FRAME_BUILD_TIME_BUDGET: Duration = Duration::from_millis(4);

const PAGE_FILL: Color32 = Color32::from_rgb(240, 240, 240);
const CARD_FILL: Color32 = Color32::WHITE;
const INK: Color32 = Color32::from_rgb(51, 51, 51);
const CARD_RADIUS: u8 = 8;
const CARD_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 1],
    blur: 6,
    spread: 0,
    color: Color32::from_rgba_premultiplied(0, 0, 0, 20),
};

pub struct MasonryDemo {
    open: bool,
    cards_state: Option<MasonryCardsState>,
    layout_state: Option<MasonryLayoutState>,
}

impl Default for MasonryDemo {
    fn default() -> Self {
        Self {
            open: false,
            cards_state: None,
            layout_state: None,
        }
    }
}

struct MasonryDataset {
    texts: Arc<[Arc<str>]>,
    hash: u64,
}

struct MasonryCardsState {
    engine_revision: u64,
    cards_hash: u64,
    cards: Vec<MasonryCardState>,
}

struct MasonryCardState {
    text: Arc<str>,
    prepared: Option<PreparedTextWithSegments>,
    rendered: Option<MasonryRenderedCard>,
}

struct MasonryRenderedCard {
    memo_key: u32,
    lines: Vec<LayoutLine>,
    glyph_runs: Vec<Vec<LayoutLineGlyphRun>>,
}

struct MasonryLayoutState {
    engine_revision: u64,
    memo_key: u64,
    builder: MasonryPlacementBuilder,
    total_card_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MasonryBuildProgress {
    processed_cards: usize,
    reached_viewport_end: bool,
    complete: bool,
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

#[derive(Clone, Debug, PartialEq)]
struct MasonryPlacementBuilder {
    columns: MasonryColumns,
    next_card_index: usize,
    column_heights: Vec<f32>,
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

    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, assets: &mut AssetRegistry) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(980.0, 640.0))
            .show(ctx, |ui| {
                ui.painter()
                    .rect_filled(ui.max_rect(), CornerRadius::ZERO, PAGE_FILL);

                let available_width = ui.available_width().max(240.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        let view_top = (viewport.min.y - VIEWPORT_OVERSCAN).max(0.0);
                        let view_bottom = viewport.max.y + VIEWPORT_OVERSCAN;
                        let build_progress =
                            self.build_layout_for_frame(engine, available_width, view_bottom);
                        if !build_progress.complete {
                            ctx.request_repaint();
                        }

                        let (content_width, content_height, col_width, text_width, visible_cards) = {
                            let state = self
                                .layout_state
                                .as_ref()
                                .expect("masonry layout state should exist");
                            let content_width = state.builder.columns.content_width.max(ui.available_width());
                            let visible_cards = state
                                .builder
                                .positioned_cards
                                .iter()
                                .copied()
                                .filter(|card| {
                                    card.y <= view_bottom && card.y + card.height >= view_top
                                })
                                .collect::<Vec<_>>();
                            (
                                content_width,
                                state.builder.content_height(),
                                state.builder.columns.col_width,
                                state.builder.columns.text_width,
                                visible_cards,
                            )
                        };

                        ui.set_width(content_width);
                        ui.set_height(content_height.max(viewport.height()));

                        let origin = ui.max_rect().min;
                        let painter = ui.painter().clone();
                        for card in visible_cards {
                            let rect = Rect::from_min_size(
                                origin + egui::vec2(card.x, card.y),
                                egui::vec2(col_width, card.height),
                            );
                            ui.allocate_rect(rect, Sense::hover());
                            self.paint_card(
                                &painter,
                                rect,
                                card.card_index,
                                text_width,
                                ctx,
                                engine,
                                assets,
                            );
                        }
                    });
            });
        self.open = open;
    }
}

impl MasonryDemo {
    fn invalidate_cached_state(&mut self) {
        self.cards_state = None;
        self.layout_state = None;
    }

    fn ensure_cards_state(&mut self, engine: &PretextEngine) {
        let engine_revision = engine.revision();
        if self
            .cards_state
            .as_ref()
            .is_some_and(|state| state.engine_revision != engine_revision)
        {
            self.invalidate_cached_state();
        }

        if self.cards_state.is_some() {
            return;
        }

        let dataset = masonry_dataset();
        let cards = dataset
            .texts
            .iter()
            .cloned()
            .map(|text| MasonryCardState {
                text,
                prepared: None,
                rendered: None,
            })
            .collect();
        self.cards_state = Some(MasonryCardsState {
            engine_revision,
            cards_hash: dataset.hash,
            cards,
        });
    }

    fn ensure_layout_state(&mut self, engine: &PretextEngine, available_width: f32) {
        self.ensure_cards_state(engine);

        let cards_state = self
            .cards_state
            .as_ref()
            .expect("masonry cards state should exist");
        let columns = compute_masonry_columns(available_width);
        let memo_key = masonry_memo_key(cards_state.cards_hash, columns.col_width);
        let engine_revision = engine.revision();
        let total_card_count = cards_state.cards.len();
        let should_rebuild = self.layout_state.as_ref().is_none_or(|state| {
            state.engine_revision != engine_revision || state.memo_key != memo_key
        });

        if should_rebuild {
            self.layout_state = Some(MasonryLayoutState::new(
                engine_revision,
                memo_key,
                columns,
                total_card_count,
            ));
        }
    }

    fn build_layout_for_frame(
        &mut self,
        engine: &PretextEngine,
        available_width: f32,
        view_bottom: f32,
    ) -> MasonryBuildProgress {
        self.ensure_layout_state(engine, available_width);

        let text_width = self
            .layout_state
            .as_ref()
            .expect("masonry layout state should exist")
            .builder
            .columns
            .text_width;
        let target_bottom = view_bottom.max(0.0);
        let frame_start = Instant::now();
        let mut progress = MasonryBuildProgress::default();

        loop {
            let state = self
                .layout_state
                .as_ref()
                .expect("masonry layout state should exist");
            if state.is_complete() {
                break;
            }

            let must_fill_viewport = state.builder.min_insert_y() <= target_bottom;
            if !must_fill_viewport
                && (progress.processed_cards >= FRAME_BUILD_CARD_BUDGET
                    || frame_start.elapsed() >= FRAME_BUILD_TIME_BUDGET)
            {
                break;
            }

            self.advance_one_card(engine, text_width);
            progress.processed_cards += 1;
        }

        let state = self
            .layout_state
            .as_ref()
            .expect("masonry layout state should exist");
        progress.complete = state.is_complete();
        progress.reached_viewport_end =
            progress.complete || state.builder.min_insert_y() > target_bottom;
        progress
    }

    fn advance_one_card(&mut self, engine: &PretextEngine, text_width: f32) {
        let next_card_index = self
            .layout_state
            .as_ref()
            .expect("masonry layout state should exist")
            .builder
            .next_card_index;
        let prepared = {
            let cards_state = self
                .cards_state
                .as_mut()
                .expect("masonry cards state should exist");
            let card = cards_state
                .cards
                .get_mut(next_card_index)
                .expect("next card should exist while layout is incomplete");
            if card.prepared.is_none() {
                card.prepared = Some(engine.prepare_with_segments(
                    card.text.as_ref(),
                    masonry_text_style(),
                    &normal_options(),
                ));
            }
            card.prepared
                .clone()
                .expect("prepared card should exist after preparation")
        };
        let height = compute_card_height(engine, &prepared, text_width);
        self.layout_state
            .as_mut()
            .expect("masonry layout state should exist")
            .builder
            .push_next(height);
    }

    fn paint_card(
        &mut self,
        painter: &egui::Painter,
        rect: Rect,
        card_index: usize,
        text_width: f32,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut AssetRegistry,
    ) {
        let render_key = masonry_render_memo_key(text_width);
        let cards_state = self
            .cards_state
            .as_mut()
            .expect("masonry cards state should exist");
        let card = cards_state
            .cards
            .get_mut(card_index)
            .expect("visible card index should exist");
        if card
            .rendered
            .as_ref()
            .is_none_or(|rendered| rendered.memo_key != render_key)
        {
            let prepared = card
                .prepared
                .as_ref()
                .expect("visible masonry cards should already be prepared");
            let layout = engine.layout_with_lines(prepared, text_width.max(1.0), LINE_HEIGHT);
            let glyph_runs = layout
                .lines
                .iter()
                .map(|line| engine.line_glyph_runs(prepared, line))
                .collect::<Vec<_>>();
            card.rendered = Some(MasonryRenderedCard {
                memo_key: render_key,
                lines: layout.lines,
                glyph_runs,
            });
        }

        paint_masonry_card(
            painter,
            rect,
            card.rendered
                .as_ref()
                .expect("rendered masonry card should exist"),
            ctx,
            engine,
            assets,
        );
    }
}

impl MasonryLayoutState {
    fn new(
        engine_revision: u64,
        memo_key: u64,
        columns: MasonryColumns,
        total_card_count: usize,
    ) -> Self {
        Self {
            engine_revision,
            memo_key,
            builder: MasonryPlacementBuilder::new(columns, total_card_count),
            total_card_count,
        }
    }

    fn is_complete(&self) -> bool {
        self.builder.next_card_index >= self.total_card_count
    }
}

impl MasonryPlacementBuilder {
    fn new(columns: MasonryColumns, capacity: usize) -> Self {
        Self {
            columns,
            next_card_index: 0,
            column_heights: vec![CARD_GAP; columns.col_count],
            positioned_cards: Vec::with_capacity(capacity),
        }
    }

    fn min_insert_y(&self) -> f32 {
        self.column_heights
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
    }

    fn push_next(&mut self, height: f32) {
        let mut shortest_col = 0usize;
        for col in 1..self.columns.col_count {
            if self.column_heights[col] < self.column_heights[shortest_col] {
                shortest_col = col;
            }
        }

        self.positioned_cards.push(PositionedCard {
            card_index: self.next_card_index,
            x: self.columns.offset_left + shortest_col as f32 * (self.columns.col_width + CARD_GAP),
            y: self.column_heights[shortest_col],
            height,
        });
        self.column_heights[shortest_col] += height + CARD_GAP;
        self.next_card_index += 1;
    }

    fn content_height(&self) -> f32 {
        self.column_heights
            .iter()
            .copied()
            .fold(0.0f32, f32::max)
            .max(CARD_GAP)
    }

    fn finish(&self) -> MasonryLayout {
        MasonryLayout {
            col_count: self.columns.col_count,
            col_width: self.columns.col_width,
            content_width: self.columns.content_width,
            content_height: self.content_height(),
            positioned_cards: self.positioned_cards.clone(),
        }
    }
}

fn build_text_style(
    families: &[&str],
    size_px: f32,
    weight: u16,
    italic: bool,
) -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: families.iter().map(|name| (*name).to_owned()).collect(),
        size_px,
        weight,
        italic,
    }
}

fn masonry_text_style() -> &'static pretext::TextStyleSpec {
    static STYLE: OnceLock<pretext::TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| {
        build_text_style(
            &[
                "Helvetica Neue",
                "Helvetica",
                "Arial",
                "Noto Sans",
                "Noto Sans Arabic",
                "Noto Sans CJK",
                "Noto Emoji",
                "Noto Color Emoji",
            ],
            15.0,
            400,
            false,
        )
    })
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
    }
}

fn masonry_dataset() -> &'static MasonryDataset {
    static DATASET: OnceLock<MasonryDataset> = OnceLock::new();
    DATASET.get_or_init(|| {
        let texts =
            serde_json::from_str::<Vec<String>>(include_str!("../../assets/shower-thoughts.json"))
                .expect("shower-thoughts.json should contain a string array");
        let texts = texts.into_iter().map(Arc::<str>::from).collect::<Vec<_>>();
        MasonryDataset {
            hash: hash_card_texts(&texts),
            texts: texts.into(),
        }
    })
}

fn hash_card_texts<T: AsRef<str>>(texts: &[T]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for text in texts {
        text.as_ref().hash(&mut hasher);
    }
    hasher.finish()
}

fn masonry_memo_key(cards_hash: u64, col_width: f32) -> u64 {
    let mut hasher = DefaultHasher::new();
    cards_hash.hash(&mut hasher);
    ((col_width * 8.0).round() as u32).hash(&mut hasher);
    hasher.finish()
}

fn masonry_render_memo_key(text_width: f32) -> u32 {
    (text_width.max(0.0) * 8.0).round() as u32
}

fn compute_card_height(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    text_width: f32,
) -> f32 {
    engine
        .layout(prepared.as_prepared(), text_width.max(1.0), LINE_HEIGHT)
        .height
        + CARD_PADDING_Y * 2.0
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
    let mut builder = MasonryPlacementBuilder::new(columns, card_heights.len());
    for &height in card_heights {
        builder.push_next(height);
    }
    builder.finish()
}

fn paint_masonry_card(
    painter: &egui::Painter,
    rect: Rect,
    rendered: &MasonryRenderedCard,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    painter.add(CARD_SHADOW.as_shape(rect, CornerRadius::same(CARD_RADIUS)));
    painter.rect_filled(rect, CornerRadius::same(CARD_RADIUS), CARD_FILL);

    let mut y = rect.top() + CARD_PADDING_Y;
    let x = rect.left() + CARD_PADDING_X;

    for (line, glyph_runs) in rendered.lines.iter().zip(rendered.glyph_runs.iter()) {
        paint_glyph_runs(
            painter,
            x,
            y,
            &line.text,
            glyph_runs,
            masonry_text_style(),
            LINE_HEIGHT,
            INK,
            ctx,
            engine,
            assets,
        );
        y += LINE_HEIGHT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_engine() -> PretextEngine {
        PretextEngine::with_font_data_and_system_fonts(AssetRegistry::bundled_font_data(), false)
    }

    fn demo_with_texts(engine: &PretextEngine, texts: &[&str]) -> MasonryDemo {
        let cards_hash = hash_card_texts(texts);
        let cards = texts
            .iter()
            .map(|text| MasonryCardState {
                text: Arc::<str>::from(*text),
                prepared: None,
                rendered: None,
            })
            .collect();
        MasonryDemo {
            open: false,
            cards_state: Some(MasonryCardsState {
                engine_revision: engine.revision(),
                cards_hash,
                cards,
            }),
            layout_state: None,
        }
    }

    #[test]
    fn masonry_asset_matches_checked_in_js_snapshot() {
        let dataset = masonry_dataset();
        assert_eq!(dataset.texts.len(), 1_904);
        assert_eq!(
            dataset.texts[0].as_ref(),
            "Men's public restrooms are laid out all wrong. It should be urinal, stall, urinal, stall, urinal instead of urinal, urinal, urinal, stall, stall."
        );
        assert_eq!(
            dataset.texts[1_500].as_ref(),
            "LPT: Do two things every time you go somewhere.Make every move useful.One trip = two tasks done."
        );
        assert_eq!(
            dataset.texts[1_903].as_ref(),
            "ELI5: What exactly is \"time blindness\" and how is it an actual thing?"
        );
    }

    #[test]
    fn masonry_layout_is_deterministic() {
        let heights = vec![120.0, 84.0, 160.0, 96.0, 132.0, 78.0, 142.0, 110.0];
        let first = compute_masonry_layout(&heights, 960.0);
        let second = compute_masonry_layout(&heights, 960.0);
        assert_eq!(first, second);
    }

    #[test]
    fn masonry_incremental_build_matches_batch_layout() {
        let engine = bundled_engine();
        let texts = [
            "A short card.",
            "A much longer card that needs multiple lines in the masonry grid to force extra height.",
            "Short again.",
            "Another medium-length card for the second column.",
            "A final card that is long enough to move the shortest column choice during incremental layout.",
        ];
        let mut demo = demo_with_texts(&engine, &texts);
        while !demo.build_layout_for_frame(&engine, 960.0, 160.0).complete {}

        let columns = compute_masonry_columns(960.0);
        let heights = texts
            .iter()
            .map(|text| {
                let prepared =
                    engine.prepare_with_segments(text, masonry_text_style(), &normal_options());
                compute_card_height(&engine, &prepared, columns.text_width)
            })
            .collect::<Vec<_>>();
        let incremental = demo
            .layout_state
            .as_ref()
            .expect("layout state should exist")
            .builder
            .finish();
        let batch = compute_masonry_layout_with_columns(&heights, columns);

        assert_eq!(incremental, batch);
    }

    #[test]
    fn masonry_rebuilds_cached_state_when_engine_revision_changes() {
        let engine_a = bundled_engine();
        let engine_b = bundled_engine();
        let mut demo = demo_with_texts(&engine_a, &["cache me", "and me too"]);

        let _ = demo.build_layout_for_frame(&engine_a, 720.0, 180.0);
        assert!(demo
            .cards_state
            .as_ref()
            .expect("cards state should exist")
            .cards
            .iter()
            .any(|card| card.prepared.is_some()));
        assert!(demo.layout_state.is_some());

        demo.ensure_cards_state(&engine_b);

        let cards_state = demo
            .cards_state
            .as_ref()
            .expect("cards state should exist after invalidation");
        assert_eq!(cards_state.engine_revision, engine_b.revision());
        assert!(cards_state.cards.iter().all(|card| card.prepared.is_none()));
        assert!(demo.layout_state.is_none());
    }

    #[test]
    fn masonry_first_frame_build_only_measures_visible_prefix() {
        let engine = bundled_engine();
        let mut demo = MasonryDemo::default();

        let progress = demo.build_layout_for_frame(&engine, 960.0, 280.0);
        let stats = engine.runtime_stats();

        assert!(progress.processed_cards > 0);
        assert!(!progress.complete);
        assert_eq!(stats.layout_with_lines_calls, 0);
        assert!(stats.layout_calls > 0);
        assert!(stats.layout_calls < masonry_dataset().texts.len() as u64);
    }

    #[test]
    fn masonry_show_first_frame_only_materializes_visible_cards() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = AssetRegistry::default();
        let mut demo = MasonryDemo::default();
        demo.set_open(true);

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            demo.show(ctx, &engine, &mut assets);
        });

        let stats = engine.runtime_stats();
        assert!(stats.layout_calls > 0);
        assert!(stats.layout_with_lines_calls > 0);
        assert!(stats.layout_with_lines_calls < masonry_dataset().texts.len() as u64);
    }

    #[test]
    fn masonry_second_frame_reuses_rendered_visible_cards() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = AssetRegistry::default();
        let mut demo = MasonryDemo::default();
        demo.set_open(true);

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            demo.show(ctx, &engine, &mut assets);
        });
        let after_first = engine.runtime_stats();

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            demo.show(ctx, &engine, &mut assets);
        });
        let after_second = engine.runtime_stats();

        assert_eq!(
            after_second.layout_with_lines_calls,
            after_first.layout_with_lines_calls
        );
        assert_eq!(
            after_second.line_visual_runs_calls,
            after_first.line_visual_runs_calls
        );
        assert_eq!(
            after_second.line_glyph_runs_calls,
            after_first.line_glyph_runs_calls
        );
    }
}

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind};
#[cfg(test)]
use pretext::BidiDirection;
use pretext::{
    LayoutCursor, LayoutLineVisualRun, PrepareOptions, PreparedTextWithSegments, PretextEngine,
    WhiteSpaceMode,
};

use crate::assets::AssetRegistry;
use crate::demos::text_runs::paint_visual_runs;
use crate::demos::DemoWindow;
use crate::geometry::{carve_text_line_slots, Interval, Point, Rect as GeoRect};

const HEADLINE: &str = "THE FUTURE OF TEXT LAYOUT IS NOT CSS";
const BODY_TEXT: &str = "Browsers still treat text as something you ask the DOM about after the fact. \
If a layout needs the real height of a paragraph, it has to trigger measurement, and measurement is usually coupled to reflow. \
That is fine for a single article paragraph and disastrous for interactive systems that need to measure hundreds of blocks before deciding where anything goes. \
\n\n\
The argument for a dedicated text engine is not aesthetic first. It is computational. \
Once shaping and line breaking are prepared, the remaining work should be arithmetic: advance widths, break opportunities, obstacle intervals, and cursor movement. \
The cost should scale with the text you are laying out, not with every unrelated element on the page. \
\n\n\
Editorial composition exposes the problem immediately. \
As soon as a drop cap, a pull quote, or an animated object intrudes on the column, CSS either gives up or asks the browser to do expensive layout work you cannot directly control. \
The geometry becomes opaque precisely when you need it most. \
\n\n\
This demo keeps the text stream continuous across three columns. \
The first column owns the drop cap, the final column reserves space for a pull quote, and translucent orbs carve circular exclusion bands through whichever column they overlap. \
Each column resumes from the cursor returned by the previous one, so the article remains a single logical stream even though the visual layout is fragmented. \
\n\n\
The key engineering detail is that animation and reflow are decoupled. \
The orbs drift every frame, but a full text reflow only happens when an orb's coarse vertical band signature changes or when the window geometry changes. \
That keeps the page responsive without pretending the text can ignore moving obstacles forever. \
\n\n\
The point is not to mimic a print magazine down to the last pixel. \
It is to prove that stable, explicit line geometry makes layouts possible that the browser's default text stack still treats as exotic. \
Once you can compute line positions yourself, text stops being a rigid rectangle and starts behaving like a real visual material.";
const PULL_QUOTE: &str =
    "\"Animation and reflow should be coupled by geometry, not by incidental DOM measurement.\"";
const BODY_LINE_HEIGHT: f32 = 28.0;
const QUOTE_LINE_HEIGHT: f32 = 24.0;
const DROP_CAP_LINES: usize = 3;
const PAGE_MIN_HEIGHT: f32 = 520.0;
const COLUMN_GAP: f32 = 26.0;
const MARGIN: f32 = 26.0;
const BOTTOM_GAP: f32 = 18.0;
const ORB_BAND_PADDING: f32 = 8.0;

pub struct EditorialEngineDemo {
    open: bool,
    paused: bool,
    orbs: Vec<Orb>,
    last_time: Option<f64>,
    dirty: bool,
    last_page: Option<GeoRect>,
    last_orb_bands: Vec<(i32, i32)>,
    projection: Option<EditorialProjection>,
    body_prepared: Option<PreparedTextWithSegments>,
    quote_prepared: Option<PreparedTextWithSegments>,
}

#[derive(Clone, Copy)]
struct Orb {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    color: Color32,
}

#[derive(Clone, Copy)]
struct CircleObstacle {
    center: Point,
    radius: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
}

#[derive(Clone)]
struct EditorialProjection {
    headline_lines: Vec<PositionedLine>,
    body_lines: Vec<PositionedLine>,
    pull_quote_lines: Vec<PositionedLine>,
    pull_quote_rect: GeoRect,
    drop_cap_origin: Point,
    drop_cap_char: char,
}

#[derive(Clone, Debug, PartialEq)]
struct PositionedLine {
    x: f32,
    y: f32,
    width: f32,
    text: String,
    visual_runs: Vec<LayoutLineVisualRun>,
}

#[derive(Clone, Copy)]
struct EditorialPageLayout {
    headline_origin: Point,
    headline_width: f32,
    headline_line_height: f32,
    body_columns: [GeoRect; 3],
    pull_quote_rect: GeoRect,
    drop_cap_rect: GeoRect,
}

impl Default for EditorialEngineDemo {
    fn default() -> Self {
        Self {
            open: false,
            paused: false,
            orbs: initial_orbs(),
            last_time: None,
            dirty: true,
            last_page: None,
            last_orb_bands: Vec::new(),
            projection: None,
            body_prepared: None,
            quote_prepared: None,
        }
    }
}

impl DemoWindow for EditorialEngineDemo {
    fn title(&self) -> &str {
        "Editorial Engine"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.last_time = None;
        }
    }

    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, _assets: &mut AssetRegistry) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1140.0, 780.0))
            .show(ctx, |ui| {
                let now = ctx.input(|input| input.time);
                ui.horizontal(|ui| {
                    if ui
                        .button(if self.paused { "Resume" } else { "Pause" })
                        .clicked()
                    {
                        self.paused = !self.paused;
                        self.last_time = Some(now);
                    }
                    if ui.button("Reset orbs").clicked() {
                        self.orbs = initial_orbs();
                        self.last_time = Some(now);
                        self.dirty = true;
                    }
                });
                ui.add_space(8.0);

                let page_size = ui.available_size();
                let page_width = page_size.x.max(760.0);
                let page_height = page_size.y.max(PAGE_MIN_HEIGHT);
                let (page_rect, _) =
                    ui.allocate_exact_size(egui::vec2(page_width, page_height), Sense::hover());
                let page = GeoRect {
                    x: page_rect.left(),
                    y: page_rect.top(),
                    width: page_rect.width(),
                    height: page_rect.height(),
                };

                self.update_orbs(now, page);
                let projection = self.ensure_projection(engine, page).clone();

                let painter = ui.painter().clone();
                paint_editorial_background(&painter, page_rect);
                paint_projection(&painter, &projection);
                paint_orbs(&painter, &self.orbs);

                ui.painter().text(
                    egui::pos2(page_rect.right() - 216.0, page_rect.top() + 14.0),
                    Align2::LEFT_TOP,
                    "reflow only when band signatures change",
                    FontId::new(12.0, FontFamily::Proportional),
                    Color32::from_rgb(115, 106, 93),
                );

                if !self.paused {
                    ctx.request_repaint();
                }
            });
        self.open = open;
    }
}

impl EditorialEngineDemo {
    fn ensure_body_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.body_prepared.is_none() {
            self.body_prepared = Some(engine.prepare_with_segments(
                &BODY_TEXT[1..],
                &body_style(),
                &normal_options(),
            ));
        }
        self.body_prepared
            .as_ref()
            .expect("editorial body should exist")
    }

    fn ensure_quote_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.quote_prepared.is_none() {
            self.quote_prepared =
                Some(engine.prepare_with_segments(PULL_QUOTE, &quote_style(), &normal_options()));
        }
        self.quote_prepared
            .as_ref()
            .expect("editorial quote should exist")
    }

    fn update_orbs(&mut self, now: f64, page: GeoRect) {
        let dt = match self.last_time {
            Some(last_time) => (now - last_time).clamp(1.0 / 240.0, 1.0 / 20.0) as f32,
            None => 1.0 / 60.0,
        };
        self.last_time = Some(now);

        if self.paused {
            return;
        }

        for orb in &mut self.orbs {
            orb.x += orb.vx * dt;
            orb.y += orb.vy * dt;

            if orb.x - orb.radius < page.x + 10.0 {
                orb.x = page.x + 10.0 + orb.radius;
                orb.vx = orb.vx.abs();
            }
            if orb.x + orb.radius > page.right() - 10.0 {
                orb.x = page.right() - 10.0 - orb.radius;
                orb.vx = -orb.vx.abs();
            }
            if orb.y - orb.radius < page.y + 58.0 {
                orb.y = page.y + 58.0 + orb.radius;
                orb.vy = orb.vy.abs();
            }
            if orb.y + orb.radius > page.bottom() - 14.0 {
                orb.y = page.bottom() - 14.0 - orb.radius;
                orb.vy = -orb.vy.abs();
            }
        }
    }

    fn ensure_projection(&mut self, engine: &PretextEngine, page: GeoRect) -> &EditorialProjection {
        let page_changed = self
            .last_page
            .is_none_or(|last| rect_needs_reflow(last, page));
        let orb_bands = orb_band_signature(&self.orbs, page.y, BODY_LINE_HEIGHT);
        let orb_changed = orb_bands != self.last_orb_bands;

        if self.dirty || page_changed || orb_changed || self.projection.is_none() {
            let body = self.ensure_body_prepared(engine).clone();
            let quote = self.ensure_quote_prepared(engine).clone();
            let orbs = self.orbs.clone();
            let projection = compute_editorial_projection(engine, &body, &quote, page, &orbs);
            self.projection = Some(projection);
            self.last_page = Some(page);
            self.last_orb_bands = orb_bands;
            self.dirty = false;
        }

        self.projection
            .as_ref()
            .expect("editorial projection should exist")
    }
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
    }
}

fn body_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 18.0,
        weight: 400,
        italic: false,
    }
}

fn quote_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 19.0,
        weight: 500,
        italic: true,
    }
}

fn headline_style(size_px: f32) -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px,
        weight: 700,
        italic: false,
    }
}

fn initial_orbs() -> Vec<Orb> {
    vec![
        Orb {
            x: 260.0,
            y: 190.0,
            vx: 42.0,
            vy: 31.0,
            radius: 58.0,
            color: Color32::from_rgba_premultiplied(104, 145, 255, 84),
        },
        Orb {
            x: 590.0,
            y: 320.0,
            vx: -36.0,
            vy: 28.0,
            radius: 52.0,
            color: Color32::from_rgba_premultiplied(232, 116, 142, 76),
        },
        Orb {
            x: 880.0,
            y: 230.0,
            vx: 24.0,
            vy: -34.0,
            radius: 48.0,
            color: Color32::from_rgba_premultiplied(211, 176, 92, 78),
        },
    ]
}

fn rect_needs_reflow(a: GeoRect, b: GeoRect) -> bool {
    (a.width - b.width).abs() > 1.0 || (a.height - b.height).abs() > 1.0
}

fn orb_band_signature(orbs: &[Orb], page_top: f32, line_height: f32) -> Vec<(i32, i32)> {
    orbs.iter()
        .map(|orb| {
            let top =
                ((orb.y - orb.radius - ORB_BAND_PADDING - page_top) / line_height).floor() as i32;
            let bottom =
                ((orb.y + orb.radius + ORB_BAND_PADDING - page_top) / line_height).ceil() as i32;
            (top, bottom)
        })
        .collect()
}

fn compute_editorial_projection(
    engine: &PretextEngine,
    body_prepared: &PreparedTextWithSegments,
    quote_prepared: &PreparedTextWithSegments,
    page: GeoRect,
    orbs: &[Orb],
) -> EditorialProjection {
    let layout = build_editorial_page_layout(page);
    let headline_prepared =
        engine.prepare_with_segments(HEADLINE, &headline_style(40.0), &normal_options());
    let headline = engine.layout_with_lines(
        &headline_prepared,
        layout.headline_width,
        layout.headline_line_height,
    );
    let headline_lines: Vec<PositionedLine> = headline
        .lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let visual_runs = engine.line_visual_runs(&headline_prepared, &line);
            PositionedLine {
                x: layout.headline_origin.x,
                y: layout.headline_origin.y + index as f32 * layout.headline_line_height,
                width: line.width,
                text: line.text,
                visual_runs,
            }
        })
        .collect();

    let quote_layout = engine.layout_with_lines(
        quote_prepared,
        layout.pull_quote_rect.width,
        QUOTE_LINE_HEIGHT,
    );
    let pull_quote_lines: Vec<PositionedLine> = quote_layout
        .lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let visual_runs = engine.line_visual_runs(quote_prepared, &line);
            PositionedLine {
                x: layout.pull_quote_rect.x,
                y: layout.pull_quote_rect.y + index as f32 * QUOTE_LINE_HEIGHT,
                width: line.width,
                text: line.text,
                visual_runs,
            }
        })
        .collect();

    let circle_obstacles: Vec<CircleObstacle> = orbs
        .iter()
        .map(|orb| CircleObstacle {
            center: Point { x: orb.x, y: orb.y },
            radius: orb.radius,
            horizontal_padding: BODY_LINE_HEIGHT * 0.72,
            vertical_padding: BODY_LINE_HEIGHT * 0.12,
        })
        .collect();
    let drop_cap_rect = layout.drop_cap_rect;
    let pull_quote_rect = layout.pull_quote_rect;

    let (mut col1, cursor1) = layout_column(
        engine,
        body_prepared,
        LayoutCursor::default(),
        layout.body_columns[0],
        BODY_LINE_HEIGHT,
        &circle_obstacles,
        &[drop_cap_rect],
    );
    let (col2, cursor2) = layout_column(
        engine,
        body_prepared,
        cursor1,
        layout.body_columns[1],
        BODY_LINE_HEIGHT,
        &circle_obstacles,
        &[],
    );
    let (col3, _) = layout_column(
        engine,
        body_prepared,
        cursor2,
        layout.body_columns[2],
        BODY_LINE_HEIGHT,
        &circle_obstacles,
        &[pull_quote_rect],
    );

    let mut body_lines = Vec::new();
    body_lines.append(&mut col1);
    body_lines.extend(col2);
    body_lines.extend(col3);

    EditorialProjection {
        headline_lines,
        body_lines,
        pull_quote_lines,
        pull_quote_rect,
        drop_cap_origin: Point {
            x: drop_cap_rect.x,
            y: drop_cap_rect.y - 2.0,
        },
        drop_cap_char: BODY_TEXT.chars().next().unwrap_or('T'),
    }
}

fn build_editorial_page_layout(page: GeoRect) -> EditorialPageLayout {
    let headline_origin = Point {
        x: page.x + MARGIN,
        y: page.y + MARGIN,
    };
    let headline_line_height = 38.0;
    let headline_height = headline_line_height * 2.0;
    let body_top = headline_origin.y + headline_height + 34.0;
    let col_width = (page.width - MARGIN * 2.0 - COLUMN_GAP * 2.0) / 3.0;

    let col1 = GeoRect {
        x: page.x + MARGIN,
        y: body_top,
        width: col_width,
        height: page.height - (body_top - page.y) - BOTTOM_GAP,
    };
    let col2 = GeoRect {
        x: col1.right() + COLUMN_GAP,
        ..col1
    };
    let col3 = GeoRect {
        x: col2.right() + COLUMN_GAP,
        ..col1
    };

    let pull_quote_rect = GeoRect {
        x: col3.x + col3.width * 0.08,
        y: body_top + col3.height * 0.18,
        width: col3.width * 0.82,
        height: 110.0,
    };
    let drop_cap_rect = GeoRect {
        x: col1.x,
        y: body_top,
        width: 54.0,
        height: BODY_LINE_HEIGHT * DROP_CAP_LINES as f32,
    };

    EditorialPageLayout {
        headline_origin,
        headline_width: col1.width * 2.0 + COLUMN_GAP * 0.8,
        headline_line_height,
        body_columns: [col1, col2, col3],
        pull_quote_rect,
        drop_cap_rect,
    }
}

fn layout_column(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    start: LayoutCursor,
    region: GeoRect,
    line_height: f32,
    circles: &[CircleObstacle],
    rects: &[GeoRect],
) -> (Vec<PositionedLine>, LayoutCursor) {
    let mut cursor = start;
    let mut line_top = region.y;
    let mut lines = Vec::new();

    while line_top + line_height <= region.bottom() {
        let mut blocked = Vec::new();
        for circle in circles {
            if let Some(interval) = circle_interval_for_band(
                circle.center.x,
                circle.center.y,
                circle.radius,
                line_top,
                line_top + line_height,
                circle.horizontal_padding,
                circle.vertical_padding,
            ) {
                blocked.push(interval);
            }
        }
        for rect in rects {
            if line_top + line_height <= rect.y || line_top >= rect.bottom() {
                continue;
            }
            blocked.push(Interval {
                left: rect.x,
                right: rect.right(),
            });
        }

        let slots = carve_text_line_slots(
            Interval {
                left: region.x,
                right: region.right(),
            },
            &blocked,
        );
        if slots.is_empty() {
            line_top += line_height;
            continue;
        }

        for slot in slots {
            let mut next_cursor = cursor;
            let Some(line) = engine.layout_next_line(
                prepared,
                &mut next_cursor,
                (slot.right - slot.left).max(1.0),
            ) else {
                return (lines, cursor);
            };
            if next_cursor == cursor {
                return (lines, cursor);
            }
            let visual_runs = engine.line_visual_runs(prepared, &line);
            lines.push(PositionedLine {
                x: slot.left.round(),
                y: line_top.round(),
                width: line.width,
                text: line.text,
                visual_runs,
            });
            cursor = next_cursor;
        }
        line_top += line_height;
    }

    (lines, cursor)
}

fn circle_interval_for_band(
    cx: f32,
    cy: f32,
    radius: f32,
    band_top: f32,
    band_bottom: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
) -> Option<Interval> {
    let top = band_top - vertical_padding;
    let bottom = band_bottom + vertical_padding;
    if top >= cy + radius || bottom <= cy - radius {
        return None;
    }

    let min_dy = if (top..=bottom).contains(&cy) {
        0.0
    } else if cy < top {
        top - cy
    } else {
        cy - bottom
    };
    if min_dy >= radius {
        return None;
    }

    let max_dx = (radius * radius - min_dy * min_dy).sqrt();
    Some(Interval {
        left: cx - max_dx - horizontal_padding,
        right: cx + max_dx + horizontal_padding,
    })
}

fn paint_editorial_background(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(
        rect,
        CornerRadius::same(22),
        Color32::from_rgb(247, 243, 235),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(22),
        Stroke::new(1.0, Color32::from_rgb(222, 213, 197)),
        StrokeKind::Inside,
    );
}

fn paint_projection(painter: &egui::Painter, projection: &EditorialProjection) {
    let headline_font = FontId::new(40.0, FontFamily::Proportional);
    for line in &projection.headline_lines {
        paint_visual_runs(
            painter,
            line.x,
            line.y,
            &line.text,
            &line.visual_runs,
            &headline_font,
            Color32::from_rgb(36, 33, 30),
        );
    }

    painter.text(
        egui::pos2(projection.drop_cap_origin.x, projection.drop_cap_origin.y),
        Align2::LEFT_TOP,
        projection.drop_cap_char,
        FontId::new(
            BODY_LINE_HEIGHT * DROP_CAP_LINES as f32 - 6.0,
            FontFamily::Proportional,
        ),
        Color32::from_rgb(118, 64, 34),
    );

    painter.rect_filled(
        Rect::from_min_size(
            egui::pos2(
                projection.pull_quote_rect.x - 10.0,
                projection.pull_quote_rect.y - 10.0,
            ),
            egui::vec2(
                projection.pull_quote_rect.width + 20.0,
                projection.pull_quote_rect.height + 12.0,
            ),
        ),
        CornerRadius::same(14),
        Color32::from_rgba_premultiplied(255, 255, 255, 176),
    );
    painter.line_segment(
        [
            egui::pos2(
                projection.pull_quote_rect.x - 14.0,
                projection.pull_quote_rect.y - 10.0,
            ),
            egui::pos2(
                projection.pull_quote_rect.x - 14.0,
                projection.pull_quote_rect.y + projection.pull_quote_rect.height + 8.0,
            ),
        ],
        Stroke::new(3.0, Color32::from_rgb(162, 133, 92)),
    );

    let quote_font = FontId::new(19.0, FontFamily::Proportional);
    for line in &projection.pull_quote_lines {
        paint_visual_runs(
            painter,
            line.x,
            line.y,
            &line.text,
            &line.visual_runs,
            &quote_font,
            Color32::from_rgb(88, 74, 59),
        );
    }

    let body_font = FontId::new(18.0, FontFamily::Proportional);
    for line in &projection.body_lines {
        paint_visual_runs(
            painter,
            line.x,
            line.y,
            &line.text,
            &line.visual_runs,
            &body_font,
            Color32::from_rgb(54, 51, 47),
        );
    }
}

fn paint_orbs(painter: &egui::Painter, orbs: &[Orb]) {
    for orb in orbs {
        painter.circle_filled(egui::pos2(orb.x, orb.y), orb.radius, orb.color);
        painter.circle_stroke(
            egui::pos2(orb.x, orb.y),
            orb.radius,
            Stroke::new(1.0, orb.color.gamma_multiply(1.4)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod editorial_obstacle {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests_support/editorial_obstacle.rs"
        ));
    }

    #[test]
    fn obstacles_increase_editorial_line_count() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let body = engine.prepare_with_segments(&BODY_TEXT[1..], &body_style(), &normal_options());
        let quote = engine.prepare_with_segments(PULL_QUOTE, &quote_style(), &normal_options());
        let page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 720.0,
        };
        let no_orbs = compute_editorial_projection(&engine, &body, &quote, page, &[]);
        let with_orbs = compute_editorial_projection(&engine, &body, &quote, page, &initial_orbs());
        assert!(with_orbs.body_lines.len() >= no_orbs.body_lines.len());
    }

    #[test]
    fn editorial_projection_is_deterministic() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let body = engine.prepare_with_segments(&BODY_TEXT[1..], &body_style(), &normal_options());
        let quote = engine.prepare_with_segments(PULL_QUOTE, &quote_style(), &normal_options());
        let page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 980.0,
            height: 700.0,
        };
        let orbs = initial_orbs();
        let first = compute_editorial_projection(&engine, &body, &quote, page, &orbs);
        let second = compute_editorial_projection(&engine, &body, &quote, page, &orbs);
        assert_eq!(first.body_lines, second.body_lines);
        assert_eq!(first.pull_quote_lines, second.pull_quote_lines);
    }

    #[test]
    fn editorial_layout_keeps_visual_runs_for_mixed_direction_text() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let prepared = engine.prepare_with_segments(
            "English قبل العربية and then back again",
            &body_style(),
            &normal_options(),
        );
        let (lines, _) = layout_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            GeoRect {
                x: 26.0,
                y: 136.0,
                width: 320.0,
                height: 112.0,
            },
            BODY_LINE_HEIGHT,
            &[],
            &[],
        );
        let line = lines
            .iter()
            .find(|line| {
                line.visual_runs
                    .iter()
                    .any(|run| run.direction == BidiDirection::Rtl)
            })
            .expect("editorial layout should produce a line with an RTL run");

        assert!(line.visual_runs.len() >= 2);
        assert!(line
            .visual_runs
            .iter()
            .any(|run| run.direction == BidiDirection::Ltr));
        assert!(line
            .visual_runs
            .iter()
            .any(|run| run.direction == BidiDirection::Rtl));
    }

    #[test]
    fn editorial_obstacle_projection_matches_shared_helper() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let body = engine.prepare_with_segments(&BODY_TEXT[1..], &body_style(), &normal_options());
        let quote = engine.prepare_with_segments(PULL_QUOTE, &quote_style(), &normal_options());
        let page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 980.0,
            height: 700.0,
        };
        let projection =
            compute_editorial_projection(&engine, &body, &quote, page, &initial_orbs());
        let actual = editorial_obstacle::EditorialGolden {
            body_lines: projection
                .body_lines
                .into_iter()
                .map(to_golden_line)
                .collect(),
            pull_quote_lines: projection
                .pull_quote_lines
                .into_iter()
                .map(to_golden_line)
                .collect(),
        };
        let expected = editorial_obstacle::compute_editorial_golden(&engine);

        assert_eq!(
            actual, expected,
            "editorial demo projection diverged from the shared golden helper"
        );
    }

    fn to_golden_line(line: PositionedLine) -> editorial_obstacle::GoldenLine {
        editorial_obstacle::GoldenLine {
            x: round3(line.x),
            y: round3(line.y),
            width: round3(line.width),
            text: line.text,
        }
    }

    fn round3(value: f32) -> f32 {
        (value * 1000.0).round() / 1000.0
    }
}

use std::f32::consts::FRAC_PI_2;

use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontFamily, FontId, Label, Layout, Rect, RichText, Sense,
    Shape, Stroke, TextWrapMode, UiBuilder,
};
use pretext::{
    LayoutLine, LayoutLineVisualRun, LayoutWithLinesResult, PrepareOptions,
    PreparedTextWithSegments, PretextEngine, WhiteSpaceMode,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::assets::AssetRegistry;
use crate::demos::DemoWindow;

const PAGE_MAX_WIDTH: f32 = 780.0;
const INTRO_COPY_WIDTH: f32 = 580.0;
const STACK_RADIUS: u8 = 18;
const HEADER_HEIGHT: f32 = 56.0;
const HEADER_PADDING_X: f32 = 20.0;
const HEADER_GAP: f32 = 14.0;
const HEADER_GAP_COMPACT: f32 = 10.0;
const GLYPH_BOX_SIZE: f32 = 18.0;
const GLYPH_TRIANGLE_WIDTH: f32 = 7.0;
const GLYPH_TRIANGLE_HEIGHT: f32 = 10.0;
const BODY_LINE_HEIGHT: f32 = 26.0;
const BODY_TEXT_SIZE: f32 = 16.0;
const BODY_PADDING_X: f32 = 20.0;
const BODY_PADDING_BOTTOM: f32 = 18.0;
const BODY_EMOJI_SIZE: f32 = 18.0;
const BODY_SHAPED_TEXT_PAD_X: f32 = 2.0;
const BODY_SHAPED_TEXT_PAD_Y: f32 = 2.0;
const ANIMATION_TIME: f32 = 0.18;

const PAGE_FILL: Color32 = Color32::from_rgb(245, 242, 236);
const PANEL_FILL: Color32 = Color32::from_rgb(255, 253, 249);
const INK: Color32 = Color32::from_rgb(30, 26, 24);
const MUTED: Color32 = Color32::from_rgb(110, 101, 95);
const RULE: Color32 = Color32::from_rgb(216, 206, 195);
const ACCENT: Color32 = Color32::from_rgb(149, 95, 59);
const HOVER_FILL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 5);

#[derive(Clone, Copy)]
struct AccordionSection {
    id: &'static str,
    title: &'static str,
    text: &'static str,
}

struct SectionMetrics {
    layout: LayoutWithLinesResult,
    visual_runs: Vec<Vec<LayoutLineVisualRun>>,
    meta: String,
    body_height: f32,
}

struct SectionRenderState {
    metrics: SectionMetrics,
    open_t: f32,
}

pub struct AccordionDemo {
    open: bool,
    open_item: Option<usize>,
    prepared_sections: Option<Vec<PreparedTextWithSegments>>,
}

impl Default for AccordionDemo {
    fn default() -> Self {
        Self {
            open: false,
            open_item: Some(0),
            prepared_sections: None,
        }
    }
}

impl AccordionDemo {
    fn ensure_prepared_sections(&mut self, engine: &PretextEngine) -> &[PreparedTextWithSegments] {
        if self.prepared_sections.is_none() {
            let prepared = accordion_sections()
                .iter()
                .map(|section| {
                    engine.prepare_with_segments(
                        section.text,
                        &accordion_body_style(),
                        &normal_options(),
                    )
                })
                .collect();
            self.prepared_sections = Some(prepared);
        }

        self.prepared_sections
            .as_deref()
            .expect("accordion prepared sections should exist")
    }

    fn render_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut AssetRegistry,
    ) {
        let outer_width = ui.available_width().max(320.0);
        let page_gutter = if outer_width <= 640.0 { 20.0 } else { 32.0 };
        let page_width = (outer_width - page_gutter).min(PAGE_MAX_WIDTH).max(240.0);

        ui.scope_builder(
            UiBuilder::new().layout(Layout::top_down(Align::Center)),
            |ui| {
                ui.set_min_width(page_width);
                ui.set_max_width(page_width);
                ui.set_width(page_width);

                egui::Frame::new()
                    .fill(PAGE_FILL)
                    .inner_margin(egui::Margin::symmetric(0, 24))
                    .show(ui, |ui| {
                        ui.set_min_width(page_width);
                        ui.set_max_width(page_width);
                        ui.scope_builder(
                            UiBuilder::new().layout(Layout::top_down(Align::Min)),
                            |ui| {
                                paint_intro(ui);
                                ui.add_space(20.0);
                                self.paint_stack(ui, ctx, engine, assets);
                            },
                        );
                    });
            },
        );
    }

    fn paint_stack(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut AssetRegistry,
    ) {
        let sections = accordion_sections();
        let stack_width = ui.available_width().max(240.0);
        let compact = stack_width <= 640.0;
        let header_gap = if compact {
            HEADER_GAP_COMPACT
        } else {
            HEADER_GAP
        };
        let meta_font_size = if compact { 11.0 } else { 12.0 };
        let text_width = body_text_width(stack_width);
        let prepared_sections = self.ensure_prepared_sections(engine).to_vec();

        let render_states = sections
            .iter()
            .enumerate()
            .map(|(index, section)| {
                let metrics = measure_section(engine, &prepared_sections[index], text_width);
                let id = ui.make_persistent_id(("accordion_section", section.id));
                let open_t =
                    ctx.animate_bool_with_time(id, self.open_item == Some(index), ANIMATION_TIME);
                SectionRenderState { metrics, open_t }
            })
            .collect::<Vec<_>>();

        egui::Frame::new()
            .fill(PANEL_FILL)
            .stroke(Stroke::new(1.0, RULE))
            .corner_radius(CornerRadius::same(STACK_RADIUS))
            .show(ui, |ui| {
                ui.set_min_width(stack_width);
                ui.set_max_width(stack_width);
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                let left = ui.max_rect().left();
                let right = ui.max_rect().right();
                let painter = ui.painter().clone();
                let mut any_animating = false;

                for (index, section) in sections.iter().enumerate() {
                    let state = &render_states[index];
                    let animated_body_height =
                        egui::lerp(0.0..=state.metrics.body_height, state.open_t);
                    let full_width = ui.available_width().max(1.0);

                    let (header_rect, response) = ui
                        .allocate_exact_size(egui::vec2(full_width, HEADER_HEIGHT), Sense::click());
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let is_open = self.open_item == Some(index);
                    if response.clicked() {
                        self.open_item = if is_open { None } else { Some(index) };
                    }

                    if response.hovered() {
                        painter.rect_filled(
                            header_rect,
                            header_rounding(index, sections.len(), animated_body_height),
                            HOVER_FILL,
                        );
                    }

                    let mut header_ui = ui.new_child(
                        UiBuilder::new()
                            .max_rect(header_rect)
                            .layout(Layout::left_to_right(Align::Center)),
                    );
                    header_ui.set_clip_rect(header_rect);
                    paint_section_header(
                        &mut header_ui,
                        header_rect,
                        section.title,
                        &state.metrics.meta,
                        state.open_t,
                        header_gap,
                        meta_font_size,
                    );

                    let (body_rect, _) = ui.allocate_exact_size(
                        egui::vec2(full_width, animated_body_height.max(0.0)),
                        Sense::hover(),
                    );
                    if animated_body_height > 0.5 {
                        paint_section_body(
                            &painter.with_clip_rect(body_rect),
                            body_rect,
                            &state.metrics.layout.lines,
                            &state.metrics.visual_runs,
                            BODY_LINE_HEIGHT,
                            ctx,
                            engine,
                            assets,
                        );
                    }

                    if index + 1 < sections.len() {
                        painter.line_segment(
                            [
                                egui::pos2(left, body_rect.bottom()),
                                egui::pos2(right, body_rect.bottom()),
                            ],
                            Stroke::new(1.0, RULE),
                        );
                    }

                    any_animating |= state.open_t > 0.0 && state.open_t < 1.0;
                }

                if any_animating {
                    ctx.request_repaint();
                }
            });
    }
}

impl DemoWindow for AccordionDemo {
    fn title(&self) -> &str {
        "Accordion"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.prepared_sections = None;
        }
    }

    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, _assets: &mut AssetRegistry) {
        self.show_with_assets(ctx, engine, _assets);
    }
}

impl AccordionDemo {
    fn show_with_assets(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut AssetRegistry,
    ) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .default_size(egui::vec2(860.0, 640.0))
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_page(ui, ctx, engine, assets);
                    });
            });
        self.open = open;
    }
}

fn accordion_sections() -> &'static [AccordionSection; 4] {
    &[
        AccordionSection {
            id: "shipping",
            title: "Section 1",
            text: "Mina cut the release note to three crisp lines, then realized the support caveat still needed one more sentence before it could ship without surprises.",
        },
        AccordionSection {
            id: "ops",
            title: "Section 2",
            text: "The handoff doc now reads like a proper morning checklist instead of a diary entry. Restart the worker, verify the queue drains, and only then mark the incident quiet. If the backlog grows again, page the same owner instead of opening a new thread.",
        },
        AccordionSection {
            id: "research",
            title: "Section 3",
            text: "We learned the hard way that a giant native scroll range can dominate everything else. The bug looked like DOM churn, then like pooling, then like rendering pressure, until the repros were stripped down enough to show the real limit. That changed the fix completely: simplify the DOM, keep virtualization honest, and stop hiding the worst-case path behind caches that only make the common frame look cheaper.",
        },
        AccordionSection {
            id: "mixed",
            title: "Section 4",
            text: "AGI 春天到了. بدأت الرحلة 🚀 and the long URL is https://example.com/reports/q3?lang=ar&mode=full. Nora wrote “please keep 10\u{202F}000 rows visible,” Mina replied “trans\u{00AD}atlantic labels are still weird.”",
        },
    ]
}

fn paint_intro(ui: &mut egui::Ui) {
    ui.label(
        RichText::new("Demo")
            .monospace()
            .size(12.0)
            .color(ACCENT)
            .strong(),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Finally sane accordion")
            .size(30.0)
            .color(INK)
            .strong(),
    );
    ui.add_space(10.0);
    ui.scope(|ui| {
        ui.set_max_width(INTRO_COPY_WIDTH);
        ui.label(
            RichText::new(
                "The section heights have been calculated without measuring the DOM and without CSS hacks.",
            )
            .size(16.0)
            .color(MUTED),
        );
    });
}

fn measure_section(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    text_width: f32,
) -> SectionMetrics {
    let layout = engine.layout_with_lines(prepared, text_width, BODY_LINE_HEIGHT);
    let visual_runs = layout
        .lines
        .iter()
        .map(|line| engine.line_visual_runs(prepared, line))
        .collect();
    let meta = section_meta(&layout);
    let body_height = expanded_body_height(layout.height);
    SectionMetrics {
        layout,
        visual_runs,
        meta,
        body_height,
    }
}

fn section_meta(layout: &LayoutWithLinesResult) -> String {
    format!(
        "Measurement: {} lines · {}px",
        layout.line_count,
        layout.height.round() as i32
    )
}

fn accordion_body_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Noto Sans Arabic".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: BODY_TEXT_SIZE,
        weight: 400,
        italic: false,
    }
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
    }
}

fn body_text_width(section_width: f32) -> f32 {
    (section_width - BODY_PADDING_X * 2.0).max(1.0)
}

fn expanded_body_height(layout_height: f32) -> f32 {
    (layout_height + BODY_PADDING_BOTTOM).ceil()
}

fn header_rounding(index: usize, section_count: usize, animated_body_height: f32) -> CornerRadius {
    let is_first = index == 0;
    let is_last_collapsed = index + 1 == section_count && animated_body_height <= 0.5;
    CornerRadius {
        nw: if is_first { STACK_RADIUS } else { 0 },
        ne: if is_first { STACK_RADIUS } else { 0 },
        sw: if is_last_collapsed { STACK_RADIUS } else { 0 },
        se: if is_last_collapsed { STACK_RADIUS } else { 0 },
    }
}

fn paint_section_header(
    ui: &mut egui::Ui,
    rect: Rect,
    title: &str,
    meta: &str,
    open_t: f32,
    header_gap: f32,
    meta_font_size: f32,
) {
    let content_rect = Rect::from_min_max(
        egui::pos2(rect.left() + HEADER_PADDING_X, rect.top()),
        egui::pos2(rect.right() - HEADER_PADDING_X, rect.bottom()),
    );

    ui.scope_builder(
        UiBuilder::new()
            .max_rect(content_rect)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            egui::containers::Sides::new()
                .height(content_rect.height())
                .shrink_left()
                .spacing(header_gap)
                .truncate()
                .show(
                    ui,
                    |ui| {
                        ui.add(
                            Label::new(
                                RichText::new(title)
                                    .size(17.0)
                                    .color(INK)
                                    .family(FontFamily::Proportional)
                                    .strong(),
                            )
                            .truncate()
                            .wrap_mode(TextWrapMode::Truncate),
                        );
                    },
                    |ui| {
                        ui.spacing_mut().item_spacing.x = header_gap;
                        let (glyph_rect, _) = ui.allocate_exact_size(
                            egui::vec2(GLYPH_BOX_SIZE, GLYPH_BOX_SIZE),
                            Sense::hover(),
                        );
                        paint_glyph(ui.painter(), glyph_rect.center(), open_t);
                        ui.add(
                            Label::new(
                                RichText::new(meta)
                                    .monospace()
                                    .size(meta_font_size)
                                    .color(MUTED),
                            )
                            .truncate()
                            .halign(Align::RIGHT)
                            .wrap_mode(TextWrapMode::Truncate),
                        );
                    },
                );
        },
    );
}

fn paint_glyph(painter: &egui::Painter, center: egui::Pos2, open_t: f32) {
    let angle = FRAC_PI_2 * open_t;
    let cos = angle.cos();
    let sin = angle.sin();
    let half_w = GLYPH_TRIANGLE_WIDTH * 0.5;
    let half_h = GLYPH_TRIANGLE_HEIGHT * 0.5;
    let base_points = [
        egui::vec2(-half_w, -half_h),
        egui::vec2(-half_w, half_h),
        egui::vec2(half_w, 0.0),
    ];
    let points = base_points
        .into_iter()
        .map(|offset| {
            egui::pos2(
                center.x + offset.x * cos - offset.y * sin,
                center.y + offset.x * sin + offset.y * cos,
            )
        })
        .collect();

    painter.add(Shape::convex_polygon(points, ACCENT, Stroke::NONE));
}

fn paint_section_body(
    painter: &egui::Painter,
    body_rect: Rect,
    lines: &[LayoutLine],
    visual_runs: &[Vec<LayoutLineVisualRun>],
    line_height: f32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    let mut y = body_rect.top();
    for (index, line) in lines.iter().enumerate() {
        let runs = visual_runs.get(index).map(Vec::as_slice).unwrap_or(&[]);
        if runs.is_empty() {
            painter.text(
                egui::pos2(body_rect.left() + BODY_PADDING_X, y),
                Align2::LEFT_TOP,
                &line.text,
                FontId::new(BODY_TEXT_SIZE, FontFamily::Proportional),
                INK,
            );
            y += line_height;
            continue;
        }

        let mut offset = 0.0f32;
        for run in runs {
            paint_run_with_svg_emoji_fallback(
                painter,
                body_rect.left() + BODY_PADDING_X + offset,
                y,
                run,
                &FontId::new(BODY_TEXT_SIZE, FontFamily::Proportional),
                INK,
                ctx,
                engine,
                assets,
                line_height,
            );
            offset += run.width;
        }
        y += line_height;
    }
}

fn paint_run_with_svg_emoji_fallback(
    painter: &egui::Painter,
    run_left: f32,
    y: f32,
    run: &LayoutLineVisualRun,
    font: &FontId,
    color: Color32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
    line_height: f32,
) {
    if !run.text.contains('🚀') {
        paint_text_fragment_with_shaping_fallback(
            painter,
            run_left,
            y,
            run,
            0.0,
            run.width,
            run.text.as_str(),
            font,
            color,
            ctx,
            engine,
            assets,
        );
        return;
    }

    let graphemes = run.text.grapheme_indices(true).collect::<Vec<_>>();
    let prefix_widths = engine.prefix_widths(&run.text, &accordion_body_style());
    let mut fragment_start = 0usize;

    for (index, (byte_start, grapheme)) in graphemes.iter().enumerate() {
        let Some((emoji_key, emoji_svg)) = emoji_svg_for_grapheme(grapheme) else {
            continue;
        };

        if fragment_start < index {
            let text_start = graphemes[fragment_start].0;
            let text = &run.text[text_start..*byte_start];
            let fragment_offset = prefix_widths[fragment_start];
            let fragment_width = emoji_start_width(&prefix_widths, fragment_start, index);
            paint_text_fragment_with_shaping_fallback(
                painter,
                run_left,
                y,
                run,
                fragment_offset,
                fragment_width,
                text,
                font,
                color,
                ctx,
                engine,
                assets,
            );
        }

        let emoji_start = prefix_widths[index];
        let emoji_end = prefix_widths[index + 1];
        paint_emoji_fragment(
            painter,
            run_left,
            y,
            run,
            emoji_start,
            emoji_end,
            emoji_key,
            emoji_svg,
            ctx,
            assets,
            line_height,
        );

        fragment_start = index + 1;
    }

    if fragment_start < graphemes.len() {
        let text_start = graphemes[fragment_start].0;
        let text = &run.text[text_start..];
        let fragment_offset = prefix_widths[fragment_start];
        let fragment_width = prefix_widths.last().copied().unwrap_or(run.width) - fragment_offset;
        paint_text_fragment_with_shaping_fallback(
            painter,
            run_left,
            y,
            run,
            fragment_offset,
            fragment_width,
            text,
            font,
            color,
            ctx,
            engine,
            assets,
        );
    }
}

fn emoji_start_width(prefix_widths: &[f32], start: usize, end: usize) -> f32 {
    prefix_widths
        .get(end)
        .copied()
        .unwrap_or_else(|| prefix_widths.last().copied().unwrap_or(0.0))
        - prefix_widths.get(start).copied().unwrap_or(0.0)
}

fn paint_text_fragment_with_shaping_fallback(
    painter: &egui::Painter,
    run_left: f32,
    y: f32,
    run: &LayoutLineVisualRun,
    fragment_offset: f32,
    fragment_width: f32,
    text: &str,
    font: &FontId,
    color: Color32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    if text.is_empty() {
        return;
    }

    if contains_arabic_shaping(text) {
        paint_shaped_text_fragment(
            painter,
            run_left,
            y,
            run,
            fragment_offset,
            fragment_width,
            text,
            color,
            ctx,
            engine,
            assets,
        );
        return;
    }

    paint_text_fragment(
        painter,
        run_left,
        y,
        run,
        fragment_offset,
        text,
        font,
        color,
    );
}

fn paint_text_fragment(
    painter: &egui::Painter,
    run_left: f32,
    y: f32,
    run: &LayoutLineVisualRun,
    fragment_offset: f32,
    text: &str,
    font: &FontId,
    color: Color32,
) {
    if text.is_empty() {
        return;
    }

    let (anchor, pos_x) = match run.direction {
        pretext::BidiDirection::Ltr => (Align2::LEFT_TOP, run_left + fragment_offset),
        pretext::BidiDirection::Rtl => (Align2::RIGHT_TOP, run_left + run.width - fragment_offset),
    };
    painter.text(egui::pos2(pos_x, y), anchor, text, font.clone(), color);
}

fn paint_shaped_text_fragment(
    painter: &egui::Painter,
    run_left: f32,
    y: f32,
    run: &LayoutLineVisualRun,
    fragment_offset: f32,
    fragment_width: f32,
    text: &str,
    color: Color32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    let logical_size = accordion_shaped_text_logical_size(fragment_width);
    let raster_scale = ctx.pixels_per_point().max(1.0);
    let texture_size = accordion_shaped_text_texture_size(logical_size, raster_scale);
    let key = accordion_shaped_text_texture_key(text, run.direction, texture_size, color);
    let texture = assets
        .get_or_load_generated_image(
            &key,
            texture_size,
            egui::TextureOptions::NEAREST,
            ctx,
            || {
                rasterize_accordion_shaped_text_fragment(
                    text,
                    run.direction,
                    color,
                    engine,
                    raster_scale,
                    texture_size,
                )
            },
        )
        .clone();
    let slot_left = fragment_slot_left(run_left, run, fragment_offset, fragment_width);
    let rect_min = snap_pos_to_pixels(
        egui::pos2(
            slot_left - BODY_SHAPED_TEXT_PAD_X,
            y - BODY_SHAPED_TEXT_PAD_Y,
        ),
        raster_scale,
    );
    let rect = Rect::from_min_size(rect_min, logical_size);
    painter.image(
        texture.id(),
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

fn fragment_slot_left(
    run_left: f32,
    run: &LayoutLineVisualRun,
    fragment_offset: f32,
    fragment_width: f32,
) -> f32 {
    match run.direction {
        pretext::BidiDirection::Ltr => run_left + fragment_offset,
        pretext::BidiDirection::Rtl => run_left + run.width - fragment_offset - fragment_width,
    }
}

fn accordion_shaped_text_logical_size(text_width: f32) -> egui::Vec2 {
    egui::vec2(
        text_width.ceil().max(1.0) + BODY_SHAPED_TEXT_PAD_X * 2.0 + 2.0,
        BODY_LINE_HEIGHT.ceil() + BODY_SHAPED_TEXT_PAD_Y * 2.0 + 2.0,
    )
}

fn accordion_shaped_text_texture_size(logical_size: egui::Vec2, raster_scale: f32) -> [usize; 2] {
    [
        (logical_size.x * raster_scale).ceil().max(1.0) as usize,
        (logical_size.y * raster_scale).ceil().max(1.0) as usize,
    ]
}

fn accordion_shaped_text_texture_key(
    text: &str,
    direction: pretext::BidiDirection,
    size: [usize; 2],
    color: Color32,
) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    direction.hash(&mut hasher);
    size.hash(&mut hasher);
    color.to_array().hash(&mut hasher);
    format!("accordion/shaped-text/{:016x}", hasher.finish())
}

fn snap_pos_to_pixels(pos: egui::Pos2, pixels_per_point: f32) -> egui::Pos2 {
    egui::pos2(
        (pos.x * pixels_per_point).round() / pixels_per_point,
        (pos.y * pixels_per_point).round() / pixels_per_point,
    )
}

fn rasterize_accordion_shaped_text_fragment(
    text: &str,
    direction: pretext::BidiDirection,
    color: Color32,
    engine: &PretextEngine,
    raster_scale: f32,
    size: [usize; 2],
) -> Option<egui::ColorImage> {
    let spans = engine.shape_text_spans(text, &accordion_body_style(), direction);
    let mut pixmap = tiny_skia::Pixmap::new(size[0] as u32, size[1] as u32)?;
    let mut paint = tiny_skia::Paint::default();
    let [r, g, b, a] = color.to_array();
    paint.set_color_rgba8(r, g, b, a);
    paint.anti_alias = true;

    let mut span_left = BODY_SHAPED_TEXT_PAD_X * raster_scale;
    let baseline = accordion_shaped_text_baseline(&spans, raster_scale);

    for span in spans {
        rasterize_accordion_shaped_text_span(
            &mut pixmap,
            &paint,
            &span,
            span_left,
            baseline,
            raster_scale,
        );
        span_left += span.width * raster_scale;
    }

    let pixels = pixmap
        .pixels()
        .iter()
        .map(|pixel| {
            egui::Color32::from_rgba_premultiplied(
                pixel.red(),
                pixel.green(),
                pixel.blue(),
                pixel.alpha(),
            )
        })
        .collect();
    Some(egui::ColorImage::new(size, pixels))
}

fn accordion_shaped_text_baseline(spans: &[pretext::ShapedTextSpan], raster_scale: f32) -> f32 {
    let mut ascent = BODY_TEXT_SIZE * raster_scale * 0.8;
    let mut descent = -(BODY_TEXT_SIZE * raster_scale * 0.2);

    for span in spans {
        let Ok(face) = ttf_parser::Face::parse(span.face.data(), span.face.face_index()) else {
            continue;
        };
        let units_per_em = span.face.units_per_em().max(1) as f32;
        let scale = BODY_TEXT_SIZE * raster_scale / units_per_em;
        ascent = ascent.max(face.ascender() as f32 * scale);
        descent = descent.min(face.descender() as f32 * scale);
    }

    let content_height = (ascent - descent).max(1.0);
    let line_height = BODY_LINE_HEIGHT * raster_scale;
    let top_inset = ((line_height - content_height).max(0.0)) * 0.5;
    BODY_SHAPED_TEXT_PAD_Y * raster_scale + top_inset + ascent
}

fn rasterize_accordion_shaped_text_span(
    pixmap: &mut tiny_skia::Pixmap,
    paint: &tiny_skia::Paint<'_>,
    span: &pretext::ShapedTextSpan,
    span_left: f32,
    baseline: f32,
    raster_scale: f32,
) {
    let Ok(face) = ttf_parser::Face::parse(span.face.data(), span.face.face_index()) else {
        return;
    };
    let units_per_em = span.face.units_per_em().max(1) as f32;
    let glyph_scale = BODY_TEXT_SIZE * raster_scale / units_per_em;
    // `shape_text_spans` already returns spans/glyphs in visual order.
    let mut pen_x = span_left;

    for glyph in span.glyphs.iter() {
        let advance = glyph.advance * raster_scale;
        let glyph_x = pen_x + glyph.x_offset * raster_scale;
        pen_x += advance;
        let Some(path) = accordion_glyph_path(&face, glyph.glyph_id) else {
            continue;
        };
        let transform = tiny_skia::Transform::from_row(
            glyph_scale,
            0.0,
            0.0,
            -glyph_scale,
            glyph_x,
            baseline - glyph.y_offset * raster_scale,
        );
        pixmap.fill_path(&path, paint, tiny_skia::FillRule::Winding, transform, None);
    }
}

fn accordion_glyph_path(face: &ttf_parser::Face<'_>, glyph_id: u16) -> Option<tiny_skia::Path> {
    let mut builder = AccordionGlyphPathBuilder::default();
    face.outline_glyph(ttf_parser::GlyphId(glyph_id), &mut builder)?;
    builder.finish()
}

#[derive(Default)]
struct AccordionGlyphPathBuilder {
    inner: tiny_skia::PathBuilder,
}

impl AccordionGlyphPathBuilder {
    fn finish(self) -> Option<tiny_skia::Path> {
        self.inner.finish()
    }
}

impl ttf_parser::OutlineBuilder for AccordionGlyphPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.inner.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.inner.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.inner.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.inner.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.inner.close();
    }
}

fn paint_emoji_fragment(
    painter: &egui::Painter,
    run_left: f32,
    y: f32,
    run: &LayoutLineVisualRun,
    fragment_start: f32,
    fragment_end: f32,
    texture_key: &str,
    svg_bytes: &[u8],
    ctx: &egui::Context,
    assets: &mut AssetRegistry,
    line_height: f32,
) {
    let slot_left = match run.direction {
        pretext::BidiDirection::Ltr => run_left + fragment_start,
        pretext::BidiDirection::Rtl => run_left + run.width - fragment_end,
    };
    let slot_width = (fragment_end - fragment_start).max(1.0);
    let size = BODY_EMOJI_SIZE
        .min(line_height - 2.0)
        .min(slot_width)
        .max(1.0);
    let rect = Rect::from_min_size(
        egui::pos2(
            slot_left + (slot_width - size).max(0.0) * 0.5,
            y + (line_height - size) * 0.5,
        ),
        egui::vec2(size, size),
    );
    let texture = assets
        .get_or_load_svg(texture_key, svg_bytes, [96, 96], ctx)
        .clone();
    painter.image(
        texture.id(),
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

fn contains_arabic_shaping(text: &str) -> bool {
    text.chars().any(|ch| {
        matches!(
            ch as u32,
            0x0600..=0x06FF
                | 0x0750..=0x077F
                | 0x08A0..=0x08FF
                | 0xFB50..=0xFDFF
                | 0xFE70..=0xFEFF
        )
    })
}

fn emoji_svg_for_grapheme(grapheme: &str) -> Option<(&'static str, &'static [u8])> {
    match grapheme {
        "🚀" => Some(("accordion/emoji/rocket", AssetRegistry::rocket_emoji_svg())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{RawInput, TextureId};

    fn bundled_engine() -> PretextEngine {
        PretextEngine::with_font_data_and_system_fonts(AssetRegistry::bundled_font_data(), false)
    }

    fn shape_uses_user_texture(shape: &egui::Shape) -> bool {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().any(shape_uses_user_texture),
            _ => shape.texture_id() != TextureId::default(),
        }
    }

    #[test]
    fn accordion_sections_match_js_shape() {
        let sections = accordion_sections();
        assert_eq!(sections.len(), 4);
        assert_eq!(sections[0].id, "shipping");
        assert_eq!(sections[3].title, "Section 4");
    }

    #[test]
    fn expanded_height_matches_engine_layout_plus_bottom_padding() {
        let engine = bundled_engine();
        let width = 320.0;
        let body = accordion_sections()[2].text;
        let expected = engine.layout(
            &engine.prepare(body, &accordion_body_style(), &normal_options()),
            body_text_width(width),
            BODY_LINE_HEIGHT,
        );
        let actual = engine.layout_with_lines(
            &engine.prepare_with_segments(body, &accordion_body_style(), &normal_options()),
            body_text_width(width),
            BODY_LINE_HEIGHT,
        );

        assert!((actual.height - expected.height).abs() < 0.001);
        assert_eq!(
            expanded_body_height(actual.height),
            (expected.height + BODY_PADDING_BOTTOM).ceil()
        );
    }

    #[test]
    fn measurement_meta_uses_engine_line_count_and_rounded_height() {
        let engine = bundled_engine();
        let layout = engine.layout_with_lines(
            &engine.prepare_with_segments(
                accordion_sections()[1].text,
                &accordion_body_style(),
                &normal_options(),
            ),
            body_text_width(360.0),
            BODY_LINE_HEIGHT,
        );

        assert_eq!(
            section_meta(&layout),
            format!(
                "Measurement: {} lines · {}px",
                layout.line_count,
                layout.height.round() as i32
            )
        );
    }

    #[test]
    fn mixed_section_layout_keeps_arabic_and_emoji_text() {
        let engine = bundled_engine();
        let prepared = engine.prepare_with_segments(
            accordion_sections()[3].text,
            &accordion_body_style(),
            &normal_options(),
        );
        let layout = engine.layout_with_lines(&prepared, body_text_width(360.0), BODY_LINE_HEIGHT);
        let joined = layout
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<String>();

        assert!(joined.contains("بدأت الرحلة"));
        assert!(joined.contains("🚀"));
    }

    #[test]
    fn mixed_section_emits_svg_emoji_shape_when_open() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = AssetRegistry::default();
        assets.install_fonts(&ctx);

        let mut demo = AccordionDemo {
            open: true,
            open_item: Some(3),
            prepared_sections: None,
        };

        let raw_input = |time: f64| RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 960.0),
            )),
            time: Some(time),
            ..Default::default()
        };
        let _ = ctx.run(raw_input(0.0), |ctx| {
            demo.show_with_assets(ctx, &engine, &mut assets);
        });
        let output = ctx.run(raw_input(1.0), |ctx| {
            demo.show_with_assets(ctx, &engine, &mut assets);
        });

        assert!(output
            .shapes
            .iter()
            .any(|clipped| shape_uses_user_texture(&clipped.shape)));
    }

    #[test]
    fn mixed_section_arabic_run_bbox_tracks_pretext_run_width() {
        let engine = bundled_engine();
        let prepared = engine.prepare_with_segments(
            accordion_sections()[3].text,
            &accordion_body_style(),
            &normal_options(),
        );
        let layout = engine.layout_with_lines(&prepared, body_text_width(360.0), BODY_LINE_HEIGHT);
        let visual_runs = layout
            .lines
            .iter()
            .map(|line| engine.line_visual_runs(&prepared, line))
            .collect::<Vec<_>>();

        let run = visual_runs
            .iter()
            .flatten()
            .find(|run| contains_arabic_shaping(&run.text))
            .cloned()
            .expect("expected arabic visual run");
        let raster_scale = 2.0;
        let size = accordion_shaped_text_texture_size(
            accordion_shaped_text_logical_size(run.width),
            raster_scale,
        );
        let image = rasterize_accordion_shaped_text_fragment(
            &run.text,
            run.direction,
            INK,
            &engine,
            raster_scale,
            size,
        )
        .expect("image exists");

        let mut min_x = size[0];
        let mut max_x = 0usize;
        for y in 0..image.size[1] {
            for x in 0..image.size[0] {
                if image.pixels[y * image.size[0] + x].a() > 0 {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
            }
        }

        assert!(min_x <= max_x);
        assert!(
            min_x as f32 >= BODY_SHAPED_TEXT_PAD_X * raster_scale - 1.0,
            "ink started too far left: min_x={min_x}"
        );
        assert!(
            max_x as f32 <= (BODY_SHAPED_TEXT_PAD_X + run.width) * raster_scale + 2.0,
            "ink overflowed run slot: max_x={max_x} run_width={}",
            run.width
        );
    }
}

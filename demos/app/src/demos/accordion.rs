use std::f32::consts::FRAC_PI_2;

use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontFamily, FontId, Label, Layout, Rect, RichText, Sense,
    Shape, Stroke, TextWrapMode, UiBuilder,
};
use pretext::{PrepareOptions, PreparedTextWithSegments, PretextEngine, WhiteSpaceMode};
use pretext_egui::{
    paint_pretext_paragraph, AssetRegistry, BaselineMode, EmojiOverlayOptions,
    PretextParagraphLayout, PretextParagraphPaintOptions, ShapedTextRasterRequest,
};

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
    paragraph: PretextParagraphLayout,
    meta: String,
    body_height: f32,
}

struct SectionRenderState {
    open_t: f32,
}

struct AccordionMetricsCache {
    text_width_q: u32,
    metrics: Vec<SectionMetrics>,
}

pub struct AccordionDemo {
    open: bool,
    open_item: Option<usize>,
    prepared_sections: Option<Vec<PreparedTextWithSegments>>,
    metrics_cache: Option<AccordionMetricsCache>,
}

impl Default for AccordionDemo {
    fn default() -> Self {
        Self {
            open: false,
            open_item: Some(0),
            prepared_sections: None,
            metrics_cache: None,
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

    fn ensure_section_metrics(
        &mut self,
        engine: &PretextEngine,
        text_width: f32,
    ) -> &[SectionMetrics] {
        let text_width_q = quantize_width(text_width);
        let should_rebuild = self
            .metrics_cache
            .as_ref()
            .map(|cache| cache.text_width_q != text_width_q)
            .unwrap_or(true);

        if should_rebuild {
            let metrics = self
                .ensure_prepared_sections(engine)
                .iter()
                .map(|prepared| measure_section(engine, prepared, text_width))
                .collect();
            self.metrics_cache = Some(AccordionMetricsCache {
                text_width_q,
                metrics,
            });
        }

        self.metrics_cache
            .as_ref()
            .map(|cache| cache.metrics.as_slice())
            .expect("accordion metrics should exist")
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
        let current_open_item = self.open_item;
        let mut next_open_item = current_open_item;
        let metrics = self.ensure_section_metrics(engine, text_width);

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
                    let metrics = &metrics[index];
                    let id = ui.make_persistent_id(("accordion_section", section.id));
                    let state = SectionRenderState {
                        open_t: ctx.animate_bool_with_time(
                            id,
                            current_open_item == Some(index),
                            ANIMATION_TIME,
                        ),
                    };
                    let animated_body_height = egui::lerp(0.0..=metrics.body_height, state.open_t);
                    let full_width = ui.available_width().max(1.0);

                    let (header_rect, response) = ui
                        .allocate_exact_size(egui::vec2(full_width, HEADER_HEIGHT), Sense::click());
                    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                    let is_open = current_open_item == Some(index);
                    if response.clicked() {
                        next_open_item = if is_open { None } else { Some(index) };
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
                        &metrics.meta,
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
                            &metrics.paragraph,
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
        self.open_item = next_open_item;
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
            self.metrics_cache = None;
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
            .default_size(egui::vec2(1720.0, 1280.0))
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
    let style = accordion_body_style();
    let overlay_options = EmojiOverlayOptions {
        style: &style,
        slot_height: BODY_LINE_HEIGHT,
        padding_x: BODY_SHAPED_TEXT_PAD_X,
        padding_y: BODY_SHAPED_TEXT_PAD_Y,
        slack_x: 2.0,
        slack_y: 2.0,
        baseline_mode: BaselineMode::AutoFontMetrics,
    };
    let paragraph = PretextParagraphLayout::from_prepared(
        engine,
        prepared,
        text_width,
        BODY_LINE_HEIGHT,
        Some(overlay_options),
    );
    let meta = section_meta(paragraph.line_count, paragraph.height);
    let body_height = expanded_body_height(paragraph.height);
    SectionMetrics {
        paragraph,
        meta,
        body_height,
    }
}

fn section_meta(line_count: usize, height: f32) -> String {
    format!(
        "Measurement: {} lines · {}px",
        line_count,
        height.round() as i32
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

fn quantize_width(width: f32) -> u32 {
    (width.max(1.0) * 4.0).round() as u32
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
    paragraph: &PretextParagraphLayout,
    line_height: f32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    let style = accordion_body_style();
    let options = PretextParagraphPaintOptions::new(&style, line_height)
        .color(INK)
        .fallback_font(FontId::new(BODY_TEXT_SIZE, FontFamily::Proportional))
        .fallback_align(Align2::LEFT_TOP)
        .emoji_size(BODY_EMOJI_SIZE)
        .emoji_slot_height(line_height - 2.0);
    paint_pretext_paragraph(
        painter,
        egui::pos2(body_rect.left() + BODY_PADDING_X, body_rect.top()),
        paragraph,
        &options,
        ctx,
        engine,
        assets,
    );
}

#[cfg_attr(not(test), allow(dead_code))]
fn accordion_shaped_text_request<'a>(
    text: &'a str,
    style: &'a pretext::TextStyleSpec,
    direction: pretext::BidiDirection,
    color: Color32,
    fragment_width: f32,
    baseline_mode: BaselineMode,
) -> ShapedTextRasterRequest<'a> {
    ShapedTextRasterRequest {
        text,
        style,
        direction,
        color,
        fragment_width,
        slot_height: BODY_LINE_HEIGHT,
        padding_x: BODY_SHAPED_TEXT_PAD_X,
        padding_y: BODY_SHAPED_TEXT_PAD_Y,
        slack_x: 2.0,
        slack_y: 2.0,
        baseline_mode,
        texture_options: egui::TextureOptions::NEAREST,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
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

    fn shape_uses_user_texture_tint(shape: &egui::Shape, tint: Color32) -> bool {
        match shape {
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .any(|shape| shape_uses_user_texture_tint(shape, tint)),
            egui::Shape::Mesh(mesh) => {
                mesh.texture_id != TextureId::default()
                    && mesh.vertices.iter().any(|vertex| vertex.color == tint)
            }
            _ => false,
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
            section_meta(layout.line_count, layout.height),
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
            metrics_cache: None,
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
    fn mixed_section_arabic_texture_uses_ink_tint_on_light_panel() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = AssetRegistry::default();
        assets.install_fonts(&ctx);

        let mut demo = AccordionDemo {
            open: true,
            open_item: Some(3),
            prepared_sections: None,
            metrics_cache: None,
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
            .any(|clipped| shape_uses_user_texture_tint(&clipped.shape, INK)));
    }

    #[test]
    fn mixed_section_arabic_run_shaped_texture_reuses_cached_handle() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = AssetRegistry::default();
        let prepared = engine.prepare_with_segments(
            accordion_sections()[3].text,
            &accordion_body_style(),
            &normal_options(),
        );
        let layout = engine.layout_with_runs(&prepared, body_text_width(360.0), BODY_LINE_HEIGHT);

        let run = layout
            .lines
            .iter()
            .flat_map(|line| line.runs.visual_runs.iter())
            .find(|run| contains_arabic_shaping(&run.text))
            .cloned()
            .expect("expected arabic visual run");
        let style = accordion_body_style();
        let request = accordion_shaped_text_request(
            &run.text,
            &style,
            run.direction,
            INK,
            run.width,
            BaselineMode::AutoFontMetrics,
        );
        let first = assets
            .shaped_text_texture(&engine, request, &ctx)
            .expect("texture exists");
        let second = assets
            .shaped_text_texture(&engine, request, &ctx)
            .expect("cached texture exists");
        let stats = assets.stats_snapshot();

        assert_eq!(first.handle.id(), second.handle.id());
        assert_eq!(first.logical_size, second.logical_size);
        assert!(first.logical_size.x >= run.width + BODY_SHAPED_TEXT_PAD_X * 2.0);
        assert_eq!(stats.texture_uploads, 1);
        assert_eq!(stats.texture_cache_hits, 1);
        assert_eq!(stats.texture_cache_misses, 1);
        assert_eq!(stats.render.rasterizations, 1);
    }
}

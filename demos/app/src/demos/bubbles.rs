use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontFamily, FontId, Layout, Rect, RichText, Sense,
    Stroke, UiBuilder,
};
#[cfg(test)]
use pretext::BidiDirection;
use pretext::{
    ParagraphDirection, PretextEngine, PretextParagraphLayout,
    PretextParagraphOptions as PrepareOptions,
    PretextPreparedParagraph as PreparedTextWithSegments, PretextStyle as TextStyleSpec,
    WhiteSpaceMode,
};
use pretext_egui::{EguiPretextPaintOptions, EguiPretextRenderer};

use crate::demos::DemoWindow;

const DESKTOP_PAGE_MARGIN: f32 = 32.0;
const MOBILE_PAGE_MARGIN: f32 = 20.0;
const GRID_GAP: f32 = 16.0;
const PANEL_PADDING_X: f32 = 36.0;
const PANEL_RADIUS: u8 = 20;
const CONTROL_RADIUS: u8 = 18;
const CHAT_RADIUS: u8 = 14;
const CHAT_PADDING: i8 = 16;
const CHAT_GAP: f32 = 8.0;
const BUBBLE_MAX_RATIO: f32 = 0.8;
const MIN_CHAT_WIDTH: f32 = 220.0;
const MAX_CHAT_WIDTH: f32 = 1500.0;
const PAGE_MAX_WIDTH: f32 = 1600.0;
const DEFAULT_CHAT_WIDTH: f32 = 340.0;
const LINE_HEIGHT: f32 = 20.0;
const TEXT_SIZE: f32 = 15.0;
const BUBBLE_PADDING_H: f32 = 12.0;
const BUBBLE_PADDING_V: f32 = 8.0;
const BUBBLE_CORNER: u8 = 16;
const BUBBLE_TAIL: u8 = 4;
const EMOJI_SIZE: f32 = 17.0;

const PAGE_FILL: Color32 = Color32::from_rgb(244, 241, 234);
const PAGE_GLOW: Color32 = Color32::from_rgba_premultiplied(255, 248, 241, 220);
const PAGE_GLOW_SOFT: Color32 = Color32::from_rgba_premultiplied(239, 232, 222, 190);
const PANEL_FILL: Color32 = Color32::from_rgb(255, 253, 248);
const INK: Color32 = Color32::from_rgb(32, 27, 24);
const MUTED: Color32 = Color32::from_rgb(109, 100, 93);
const RULE: Color32 = Color32::from_rgb(216, 206, 195);
const ACCENT: Color32 = Color32::from_rgb(149, 95, 59);
const ACCENT_SOFT: Color32 = Color32::from_rgb(240, 228, 218);
const CHAT_FILL: Color32 = Color32::from_rgb(28, 28, 30);
const SENT_FILL: Color32 = Color32::from_rgb(11, 132, 254);
const RECV_FILL: Color32 = Color32::from_rgb(44, 44, 46);
const CHAT_TEXT: Color32 = Color32::WHITE;
const PANEL_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 12],
    blur: 28,
    spread: 0,
    color: Color32::from_rgba_premultiplied(54, 40, 23, 20),
};

#[derive(Clone, Copy)]
enum BubbleSide {
    Sent,
    Recv,
}

#[derive(Clone, Copy)]
struct BubbleMessage {
    side: BubbleSide,
    text: &'static str,
}

struct PreparedBubble {
    side: BubbleSide,
    prepared: PreparedTextWithSegments,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WrapMetrics {
    line_count: usize,
    height: f32,
    max_line_width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WrapSearchResult {
    wrap_width: f32,
    metrics: WrapMetrics,
}

struct BubbleVisual {
    side: BubbleSide,
    bubble_width: f32,
    bubble_height: f32,
    paragraph: PretextParagraphLayout,
}

struct BubbleRenderState {
    chat_width: f32,
    total_wasted_pixels: f32,
    css_bubbles: Vec<BubbleVisual>,
    tight_bubbles: Vec<BubbleVisual>,
}

struct BubbleRenderCache {
    chat_width_q: u32,
    render_state: BubbleRenderState,
}

pub struct BubblesDemo {
    open: bool,
    requested_chat_width: f32,
    prepared_bubbles: Option<Vec<PreparedBubble>>,
    render_cache: Option<BubbleRenderCache>,
}

impl Default for BubblesDemo {
    fn default() -> Self {
        Self {
            open: false,
            requested_chat_width: DEFAULT_CHAT_WIDTH,
            prepared_bubbles: None,
            render_cache: None,
        }
    }
}

impl BubblesDemo {
    fn ensure_prepared_bubbles(&mut self, engine: &PretextEngine) -> &[PreparedBubble] {
        if self.prepared_bubbles.is_none() {
            let prepared = bubble_messages()
                .iter()
                .map(|message| PreparedBubble {
                    side: message.side,
                    prepared: engine.prepare_paragraph(
                        message.text,
                        &bubble_text_style(),
                        &bubble_prepare_options(),
                    ),
                })
                .collect();
            self.prepared_bubbles = Some(prepared);
        }

        self.prepared_bubbles
            .as_deref()
            .expect("prepared bubbles should exist")
    }

    fn ensure_render_state(
        &mut self,
        engine: &PretextEngine,
        chat_width: f32,
    ) -> &BubbleRenderState {
        let chat_width_q = quantize_width(chat_width);
        let should_rebuild = self
            .render_cache
            .as_ref()
            .map(|cache| cache.chat_width_q != chat_width_q)
            .unwrap_or(true);

        if should_rebuild {
            let render_state =
                compute_bubble_render(self.ensure_prepared_bubbles(engine), engine, chat_width);
            self.render_cache = Some(BubbleRenderCache {
                chat_width_q,
                render_state,
            });
        }

        &self
            .render_cache
            .as_ref()
            .expect("bubble render cache should exist")
            .render_state
    }

    fn show_window(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .default_size(egui::vec2(1040.0, 1720.0))
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

    fn render_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let outer_width = ui.available_width().max(320.0);
        let page_width = page_width_for_viewport(outer_width);
        let max_chat_width = get_max_chat_width(MIN_CHAT_WIDTH, outer_width);
        self.requested_chat_width = self
            .requested_chat_width
            .clamp(MIN_CHAT_WIDTH, max_chat_width.max(MIN_CHAT_WIDTH));

        ui.scope_builder(
            UiBuilder::new().layout(Layout::top_down(Align::Center)),
            |ui| {
                ui.set_min_width(page_width);
                ui.set_max_width(page_width);
                ui.set_width(page_width);

                egui::Frame::new()
                    .fill(PAGE_FILL)
                    .inner_margin(egui::Margin::symmetric(0, 28))
                    .show(ui, |ui| {
                        let page_rect = ui.max_rect();
                        paint_page_backdrop(ui.painter(), page_rect);

                        ui.scope_builder(
                            UiBuilder::new().layout(Layout::top_down(Align::Min)),
                            |ui| {
                                paint_header(ui);
                                ui.add_space(24.0);
                                paint_controls(ui, &mut self.requested_chat_width, max_chat_width);
                                ui.add_space(16.0);
                                let render_state =
                                    self.ensure_render_state(engine, self.requested_chat_width);
                                paint_panels(ui, ctx, engine, assets, render_state, page_width);
                                ui.add_space(16.0);
                                paint_why_section(ui);
                            },
                        );
                    });
            },
        );
    }
}

impl DemoWindow for BubblesDemo {
    fn id(&self) -> &'static str {
        "bubbles"
    }

    fn title(&self) -> &str {
        "Bubbles"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.prepared_bubbles = None;
            self.render_cache = None;
        }
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        self.show_window(ctx, engine, assets);
    }
}

fn bubble_messages() -> &'static [BubbleMessage; 7] {
    &[
        BubbleMessage {
            side: BubbleSide::Recv,
            text: "Yo did you see the new Pretext library?",
        },
        BubbleMessage {
            side: BubbleSide::Sent,
            text: "yeah! It measures text without the DOM. Pure JavaScript arithmetic",
        },
        BubbleMessage {
            side: BubbleSide::Recv,
            text: "That shrinkwrap demo is wild it finds the exact minimum width for multiline text. CSS can't do that.",
        },
        BubbleMessage {
            side: BubbleSide::Sent,
            text: "성능 최적화가 정말 많이 되었더라고요 🎉",
        },
        BubbleMessage {
            side: BubbleSide::Recv,
            text: "Oh wow it handles CJK and emoji too??",
        },
        BubbleMessage {
            side: BubbleSide::Sent,
            text: "كل شيء! Mixed bidi, grapheme clusters, whatever you want. Try resizing",
        },
        BubbleMessage {
            side: BubbleSide::Sent,
            text: "the best part: zero layout reflow. You could shrinkwrap 10,000 bubbles and the browser wouldn't even blink",
        },
    ]
}

fn bubble_text_style() -> TextStyleSpec {
    TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Noto Sans Arabic".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: TEXT_SIZE,
        weight: 400,
        italic: false,
    }
}

fn bubble_prepare_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: pretext::WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Ltr,
    }
}

fn page_width_for_viewport(viewport_width: f32) -> f32 {
    let gutter = if viewport_width <= 760.0 {
        MOBILE_PAGE_MARGIN
    } else {
        DESKTOP_PAGE_MARGIN
    };
    (viewport_width - gutter).min(PAGE_MAX_WIDTH).max(280.0)
}

fn quantize_width(width: f32) -> u32 {
    (width.max(1.0) * 4.0).round() as u32
}

fn get_max_chat_width(min_width: f32, viewport_width: f32) -> f32 {
    let page_width = page_width_for_viewport(viewport_width);
    let panel_content_width = (page_width - PANEL_PADDING_X).max(1.0);
    panel_content_width.floor().clamp(min_width, MAX_CHAT_WIDTH)
}

fn collect_wrap_metrics(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    max_width: f32,
) -> WrapMetrics {
    let mut max_line_width = 0.0f32;
    let mut line_count = 0usize;
    engine.walk_line_ranges(prepared, max_width.max(1.0), |line| {
        line_count += 1;
        if line.width > max_line_width {
            max_line_width = line.width;
        }
    });

    WrapMetrics {
        line_count,
        height: line_count as f32 * LINE_HEIGHT,
        max_line_width,
    }
}

fn find_tight_wrap_metrics(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    max_width: f32,
) -> WrapSearchResult {
    let initial = collect_wrap_metrics(engine, prepared, max_width);
    let mut lo = 1i32;
    let mut hi = max_width.max(1.0).ceil() as i32;

    while lo < hi {
        let mid = (lo + hi) / 2;
        let mid_line_count = count_lines_for_width(engine, prepared, mid as f32);
        if mid_line_count <= initial.line_count {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }

    let wrap_width = lo as f32;
    WrapSearchResult {
        wrap_width,
        metrics: collect_wrap_metrics(engine, prepared, wrap_width),
    }
}

fn count_lines_for_width(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    width: f32,
) -> usize {
    let mut line_count = 0usize;
    engine.walk_line_ranges(prepared, width.max(1.0), |_| {
        line_count += 1;
    });
    line_count
}

fn compute_bubble_render(
    prepared_bubbles: &[PreparedBubble],
    engine: &PretextEngine,
    chat_width: f32,
) -> BubbleRenderState {
    let bubble_max_width = (chat_width * BUBBLE_MAX_RATIO).floor();
    let content_max_width = (bubble_max_width - BUBBLE_PADDING_H * 2.0).max(1.0);
    let mut total_wasted_pixels = 0.0f32;
    let mut css_bubbles = Vec::with_capacity(prepared_bubbles.len());
    let mut tight_bubbles = Vec::with_capacity(prepared_bubbles.len());

    for bubble in prepared_bubbles {
        let css = build_bubble_visual(engine, bubble, content_max_width);
        let tight_search = find_tight_wrap_metrics(engine, &bubble.prepared, content_max_width);
        let tight = build_bubble_visual(engine, bubble, tight_search.wrap_width);

        total_wasted_pixels += (css.bubble_width - tight.bubble_width).max(0.0) * css.bubble_height;
        css_bubbles.push(css);
        tight_bubbles.push(tight);
    }

    BubbleRenderState {
        chat_width,
        total_wasted_pixels,
        css_bubbles,
        tight_bubbles,
    }
}

fn build_bubble_visual(
    engine: &PretextEngine,
    bubble: &PreparedBubble,
    wrap_width: f32,
) -> BubbleVisual {
    let paragraph = bubble
        .prepared
        .layout(engine, wrap_width.max(1.0), LINE_HEIGHT);

    BubbleVisual {
        side: bubble.side,
        bubble_width: paragraph
            .lines
            .iter()
            .fold(0.0f32, |max_width, line| max_width.max(line.line.width))
            .ceil()
            + BUBBLE_PADDING_H * 2.0,
        bubble_height: paragraph.height + BUBBLE_PADDING_V * 2.0,
        paragraph,
    }
}

fn paint_page_backdrop(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::ZERO, PAGE_FILL);
    painter.circle_filled(
        egui::pos2(rect.center().x, rect.top() - rect.height() * 0.15),
        rect.width() * 0.42,
        PAGE_GLOW,
    );
    painter.circle_filled(
        egui::pos2(rect.center().x, rect.bottom() + rect.height() * 0.2),
        rect.width() * 0.55,
        PAGE_GLOW_SOFT,
    );
}

fn paint_header(ui: &mut egui::Ui) {
    ui.label(
        RichText::new("Demo")
            .monospace()
            .size(12.0)
            .color(ACCENT)
            .strong(),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new("Shrinkwrap showdown")
            .size(32.0)
            .color(INK)
            .strong(),
    );
    ui.add_space(10.0);
    ui.scope(|ui| {
        ui.set_max_width(640.0);
        ui.label(
            RichText::new(
                "CSS width: fit-content sizes a bubble to its widest wrapped line, which leaves dead space when the last line is short. Pretext finds the tightest width that still wraps to the exact same number of lines.",
            )
            .size(16.0)
            .color(MUTED),
        );
    });
}

fn panel_frame(radius: u8) -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL_FILL)
        .stroke(Stroke::new(1.0, RULE))
        .corner_radius(CornerRadius::same(radius))
        .shadow(PANEL_SHADOW)
}

fn paint_controls(ui: &mut egui::Ui, requested_chat_width: &mut f32, max_chat_width: f32) {
    panel_frame(CONTROL_RADIUS)
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let value_width = 64.0;
            let value_label = RichText::new(format!("{:.0}px", requested_chat_width.round()))
                .monospace()
                .size(12.0)
                .color(INK);
            let compact = ui.available_width() <= 560.0;
            if compact {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Container width:")
                            .monospace()
                            .size(12.0)
                            .color(MUTED),
                    );
                    ui.add_space(8.0);
                    let slider_width = ui.available_width().max(1.0);
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = slider_width;
                        ui.add(
                            egui::Slider::new(
                                requested_chat_width,
                                MIN_CHAT_WIDTH..=max_chat_width,
                            )
                            .show_value(false),
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(value_label);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Container width:")
                            .monospace()
                            .size(12.0)
                            .color(MUTED),
                    );
                    let slider_width =
                        (ui.available_width() - value_width - ui.spacing().item_spacing.x).max(1.0);
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = slider_width;
                        ui.add(
                            egui::Slider::new(
                                requested_chat_width,
                                MIN_CHAT_WIDTH..=max_chat_width,
                            )
                            .show_value(false),
                        );
                    });
                    ui.add_sized(egui::vec2(value_width, 20.0), egui::Label::new(value_label));
                });
            }
        });
}

fn paint_panels(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    render_state: &BubbleRenderState,
    page_width: f32,
) {
    let single_column =
        page_width <= 760.0 || !can_fit_two_bubble_panels(page_width, render_state.chat_width);
    let old_spacing = ui.spacing().item_spacing;
    ui.spacing_mut().item_spacing = egui::vec2(GRID_GAP, GRID_GAP);

    if single_column {
        paint_bubbles_panel(
            ui,
            ctx,
            engine,
            assets,
            "CSS fit-content",
            "Uses width: fit-content; max-width: 80%. The browser wraps the text, then sizes the bubble to the longest wrapped line. Shorter lines leave empty bubble area behind.",
            &format_pixel_count(render_state.total_wasted_pixels),
            render_state.chat_width,
            &render_state.css_bubbles,
        );
        paint_bubbles_panel(
            ui,
            ctx,
            engine,
            assets,
            "Pretext shrinkwrap",
            "Uses walkLineRanges() to binary-search the tightest width that produces the same line count. Zero wasted pixels. No DOM text measurement in the resize path.",
            "0",
            render_state.chat_width,
            &render_state.tight_bubbles,
        );
    } else {
        ui.columns(2, |columns| {
            paint_bubbles_panel(
                &mut columns[0],
                ctx,
                engine,
                assets,
                "CSS fit-content",
                "Uses width: fit-content; max-width: 80%. The browser wraps the text, then sizes the bubble to the longest wrapped line. Shorter lines leave empty bubble area behind.",
                &format_pixel_count(render_state.total_wasted_pixels),
                render_state.chat_width,
                &render_state.css_bubbles,
            );
            paint_bubbles_panel(
                &mut columns[1],
                ctx,
                engine,
                assets,
                "Pretext shrinkwrap",
                "Uses walkLineRanges() to binary-search the tightest width that produces the same line count. Zero wasted pixels. No DOM text measurement in the resize path.",
                "0",
                render_state.chat_width,
                &render_state.tight_bubbles,
            );
        });
    }

    ui.spacing_mut().item_spacing = old_spacing;
}

fn can_fit_two_bubble_panels(page_width: f32, chat_width: f32) -> bool {
    let column_width = (page_width - GRID_GAP) * 0.5;
    (column_width - PANEL_PADDING_X).max(1.0) >= chat_width
}

fn paint_bubbles_panel(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    title: &str,
    body: &str,
    wasted_pixels: &str,
    chat_width: f32,
    bubbles: &[BubbleVisual],
) {
    panel_frame(PANEL_RADIUS)
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(17.0).color(INK).strong());
            ui.add_space(10.0);
            ui.label(RichText::new(body).size(15.0).color(MUTED));
            ui.add_space(14.0);
            paint_metric_pill(ui, wasted_pixels);
            ui.add_space(16.0);
            paint_chat(ui, ctx, engine, assets, chat_width, bubbles);
        });
}

fn paint_metric_pill(ui: &mut egui::Ui, wasted_pixels: &str) {
    egui::Frame::new()
        .fill(ACCENT_SOFT)
        .corner_radius(CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Wasted pixels:")
                        .monospace()
                        .size(12.0)
                        .color(INK),
                );
                ui.label(
                    RichText::new(wasted_pixels)
                        .monospace()
                        .size(12.0)
                        .color(INK)
                        .strong(),
                );
            });
        });
}

fn paint_chat(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    chat_width: f32,
    bubbles: &[BubbleVisual],
) {
    egui::Frame::new()
        .fill(CHAT_FILL)
        .corner_radius(CornerRadius::same(CHAT_RADIUS))
        .inner_margin(egui::Margin::same(CHAT_PADDING))
        .show(ui, |ui| {
            let content_width = (chat_width - (CHAT_PADDING as f32 * 2.0)).max(1.0);
            ui.set_min_width(content_width);
            ui.set_max_width(content_width);
            ui.set_width(content_width);
            ui.spacing_mut().item_spacing.y = CHAT_GAP;

            let row_width = ui.available_width().max(1.0);
            for bubble in bubbles {
                let (row_rect, _) = ui.allocate_exact_size(
                    egui::vec2(row_width, bubble.bubble_height),
                    Sense::hover(),
                );
                let bubble_left = match bubble.side {
                    BubbleSide::Recv => row_rect.left(),
                    BubbleSide::Sent => row_rect.right() - bubble.bubble_width,
                };
                let bubble_rect = Rect::from_min_size(
                    egui::pos2(bubble_left, row_rect.top()),
                    egui::vec2(bubble.bubble_width, bubble.bubble_height),
                );
                paint_message_bubble(ui.painter(), bubble_rect, bubble.side);
                paint_message_text(
                    &ui.painter().with_clip_rect(bubble_rect),
                    bubble_rect,
                    bubble,
                    ctx,
                    engine,
                    assets,
                );
            }
        });
}

fn paint_message_bubble(painter: &egui::Painter, rect: Rect, side: BubbleSide) {
    let fill = match side {
        BubbleSide::Sent => SENT_FILL,
        BubbleSide::Recv => RECV_FILL,
    };
    let rounding = match side {
        BubbleSide::Sent => CornerRadius {
            nw: BUBBLE_CORNER,
            ne: BUBBLE_CORNER,
            sw: BUBBLE_CORNER,
            se: BUBBLE_TAIL,
        },
        BubbleSide::Recv => CornerRadius {
            nw: BUBBLE_CORNER,
            ne: BUBBLE_CORNER,
            sw: BUBBLE_TAIL,
            se: BUBBLE_CORNER,
        },
    };
    painter.rect_filled(rect, rounding, fill);
}

fn paint_message_text(
    painter: &egui::Painter,
    bubble_rect: Rect,
    bubble: &BubbleVisual,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) {
    let style = bubble_text_style();
    let options = EguiPretextPaintOptions::new(&style, LINE_HEIGHT)
        .color(CHAT_TEXT)
        .fallback_font(FontId::new(TEXT_SIZE, FontFamily::Proportional))
        .fallback_align(Align2::LEFT_TOP)
        .emoji_size(EMOJI_SIZE)
        .emoji_slot_height(LINE_HEIGHT);
    assets.paint_paragraph(
        painter,
        egui::pos2(
            bubble_rect.left() + BUBBLE_PADDING_H,
            bubble_rect.top() + BUBBLE_PADDING_V,
        ),
        &bubble.paragraph,
        &options,
        ctx,
        engine,
    );
}

fn paint_why_section(ui: &mut egui::Ui) {
    panel_frame(PANEL_RADIUS)
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.label(
                RichText::new("Why can't CSS do this?")
                    .size(20.0)
                    .color(INK)
                    .strong(),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(
                    "CSS only knows fit-content, which is the width of the widest line after wrapping. If a paragraph wraps to 3 lines and the last line is short, CSS still sizes the container to the longest line. There's no CSS property to say “find the narrowest width that still produces exactly 3 lines.” That requires measuring the text at multiple widths and comparing line counts, which is exactly what Pretext's walkLineRanges() does, without DOM text measurement in the resize path. Pure arithmetic, no reflows, instant results.",
                )
                .size(15.0)
                .color(MUTED),
            );
        });
}

fn format_pixel_count(value: f32) -> String {
    let rounded = value.round().max(0.0) as i64;
    let digits = rounded.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{RawInput, TextureId};

    fn bundled_engine() -> PretextEngine {
        PretextEngine::builder()
            .with_font_data(pretext_egui::experimental::demo_assets::bundled_font_data())
            .include_system_fonts(false)
            .build()
    }

    fn shape_uses_user_texture(shape: &egui::Shape) -> bool {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().any(shape_uses_user_texture),
            _ => shape.texture_id() != TextureId::default(),
        }
    }

    #[test]
    fn shrinkwrap_preserves_line_count_and_stays_within_fit_content_width() {
        let engine = bundled_engine();
        let prepared = engine.prepare_paragraph(
            bubble_messages()[2].text,
            &bubble_text_style(),
            &bubble_prepare_options(),
        );
        let max_width = 248.0;
        let css = collect_wrap_metrics(&engine, &prepared, max_width);
        let tight = find_tight_wrap_metrics(&engine, &prepared, max_width);

        assert_eq!(tight.metrics.line_count, css.line_count);
        assert!(tight.metrics.max_line_width <= css.max_line_width + 0.01);
    }

    #[test]
    fn render_state_tracks_all_messages_and_wasted_pixels() {
        let engine = bundled_engine();
        let prepared = bubble_messages()
            .iter()
            .map(|message| PreparedBubble {
                side: message.side,
                prepared: engine.prepare_paragraph(
                    message.text,
                    &bubble_text_style(),
                    &bubble_prepare_options(),
                ),
            })
            .collect::<Vec<_>>();

        let render = compute_bubble_render(&prepared, &engine, 340.0);

        assert_eq!(render.css_bubbles.len(), bubble_messages().len());
        assert_eq!(render.tight_bubbles.len(), bubble_messages().len());
        assert!(render.total_wasted_pixels > 0.0);
        assert!(render
            .css_bubbles
            .iter()
            .zip(render.tight_bubbles.iter())
            .all(|(css, tight)| tight.bubble_width <= css.bubble_width + 0.01));
    }

    #[test]
    fn max_chat_width_supports_1500px_when_viewport_allows() {
        let viewport_width = PAGE_MAX_WIDTH + DESKTOP_PAGE_MARGIN;

        assert_eq!(page_width_for_viewport(viewport_width), PAGE_MAX_WIDTH);
        assert_eq!(
            get_max_chat_width(MIN_CHAT_WIDTH, viewport_width),
            MAX_CHAT_WIDTH
        );
        assert!(!can_fit_two_bubble_panels(PAGE_MAX_WIDTH, MAX_CHAT_WIDTH));
    }

    #[test]
    fn party_popper_message_emits_svg_emoji_shape() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = EguiPretextRenderer::default();
        pretext_egui::experimental::demo_assets::install_demo_fonts(&ctx);
        let mut demo = BubblesDemo {
            open: true,
            requested_chat_width: DEFAULT_CHAT_WIDTH,
            prepared_bubbles: None,
            render_cache: None,
        };

        let raw_input = || RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 960.0),
            )),
            ..Default::default()
        };
        let _ = ctx.run(raw_input(), |ctx| {
            demo.show_window(ctx, &engine, &mut assets);
        });
        let output = ctx.run(raw_input(), |ctx| {
            demo.show_window(ctx, &engine, &mut assets);
        });
        let stats = assets.stats();

        assert!(output
            .shapes
            .iter()
            .any(|clipped| shape_uses_user_texture(&clipped.shape)));
        assert!(stats.atlas_entries > 0);
        assert!(stats.static_svg_textures > 0);
        assert_eq!(stats.shaped_text_textures, 0);
    }

    #[test]
    fn mixed_bidi_bubble_keeps_rtl_prefix_on_visual_first_slot() {
        let engine = bundled_engine();
        let prepared = bubble_messages()
            .iter()
            .map(|message| PreparedBubble {
                side: message.side,
                prepared: engine.prepare_paragraph(
                    message.text,
                    &bubble_text_style(),
                    &bubble_prepare_options(),
                ),
            })
            .collect::<Vec<_>>();
        let render = compute_bubble_render(&prepared, &engine, DEFAULT_CHAT_WIDTH);
        let bubble = &render.tight_bubbles[5];
        let first_line_runs = &bubble.paragraph.lines[0].runs.visual_runs;

        assert_eq!(bubble.paragraph.lines[0].line.text, "كل شيء! Mixed bidi,");
        assert_eq!(first_line_runs.len(), 2);
        assert_eq!(first_line_runs[0].direction, BidiDirection::Rtl);
        assert!(first_line_runs[0].text.contains("كل شيء"));
        assert_eq!(first_line_runs[1].direction, BidiDirection::Ltr);
        assert!(first_line_runs[1].text.contains("Mixed bidi"));
    }

    #[test]
    fn mixed_bidi_glyph_atlas_reuses_cached_entries() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = EguiPretextRenderer::default();
        let prepared = bubble_messages()
            .iter()
            .map(|message| PreparedBubble {
                side: message.side,
                prepared: engine.prepare_paragraph(
                    message.text,
                    &bubble_text_style(),
                    &bubble_prepare_options(),
                ),
            })
            .collect::<Vec<_>>();
        let render = compute_bubble_render(&prepared, &engine, DEFAULT_CHAT_WIDTH);
        let glyph_runs = render.tight_bubbles[5].paragraph.lines[0]
            .runs
            .glyph_runs
            .clone();

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("bubbles-glyph-atlas-first"),
            ));
            assert!(pretext_egui::advanced::paint_line_glyph_runs(
                &mut assets,
                &painter,
                12.0,
                12.0,
                &glyph_runs,
                &bubble_text_style(),
                LINE_HEIGHT,
                CHAT_TEXT,
                ctx,
                &engine,
            ));
        });
        let after_first = assets.stats();

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("bubbles-glyph-atlas-second"),
            ));
            assert!(pretext_egui::advanced::paint_line_glyph_runs(
                &mut assets,
                &painter,
                12.0,
                12.0,
                &glyph_runs,
                &bubble_text_style(),
                LINE_HEIGHT,
                CHAT_TEXT,
                ctx,
                &engine,
            ));
        });
        let after_second = assets.stats();

        assert!(after_first.atlas_entries > 0);
        assert_eq!(after_first.atlas_entries, after_second.atlas_entries);
        assert_eq!(after_first.texture_uploads, after_second.texture_uploads);
        assert!(after_second.atlas_hits > after_first.atlas_hits);
    }
}

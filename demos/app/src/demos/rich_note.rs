use eframe::egui;
use egui::{
    Align, Align2, Color32, CornerRadius, FontFamily, FontId, Layout, Rect, RichText, Sense,
    Stroke, StrokeKind, UiBuilder,
};
use pretext::advanced::LayoutCursor;
use pretext::{
    rich_inline::{
        layout_rich_inline, prepare_rich_inline, PreparedRichInline,
        RichInlineBreakMode as FlowBreakMode, RichInlineFragment as FlowFragment,
        RichInlineItemSpec as FlowItemSpec,
    },
    BidiDirection, ParagraphDirection, PretextEngine, PretextGlyphRun as LayoutLineGlyphRun,
    PretextParagraphOptions as PrepareOptions,
    PretextPreparedParagraph as PreparedTextWithSegments, PretextStyle as TextStyleSpec,
    PretextVisualRun as LayoutLineVisualRun, WhiteSpaceMode, WordBreakMode,
};
use pretext_egui::{
    advanced::{
        paint_styled_positioned_text_runs, split_builtin_emoji_glyphs, EmojiOverlayOptions,
        EmojiOverlayRun, StyledPositionedTextRunRef,
    },
    BaselineMode, EguiPretextPaintOptions, EguiPretextRenderer, PretextTextureRasterRequest,
};
use std::sync::OnceLock;

use crate::demos::DemoWindow;

const LINE_START_CURSOR: LayoutCursor = LayoutCursor {
    segment_index: 0,
    grapheme_index: 0,
};

const LINE_HEIGHT: f32 = 34.0;
const LAST_LINE_BLOCK_HEIGHT: f32 = 24.0;
const NOTE_SHELL_CHROME_X: f32 = 40.0;
const PAGE_MAX_WIDTH: f32 = 940.0;
const INTRO_MAX_WIDTH: f32 = 720.0;
const NOTE_SHELL_PADDING: i8 = 20;
const BODY_MIN_WIDTH: f32 = 260.0;
const BODY_DEFAULT_WIDTH: f32 = 516.0;
const BODY_MAX_WIDTH: f32 = 760.0;
const PAGE_MARGIN: f32 = 28.0;
const DESKTOP_PAGE_GUTTER: f32 = 32.0;
const MOBILE_PAGE_GUTTER: f32 = 20.0;
const MOBILE_BREAKPOINT: f32 = 640.0;
const CHIP_CHROME_WIDTH: f32 = 22.0;
const CODE_CHROME_WIDTH: f32 = 14.0;
const UNBOUNDED_WIDTH: f32 = 100_000.0;
const BODY_TEXT_SIZE: f32 = 17.0;
const LINK_TEXT_SIZE: f32 = 17.0;
const CODE_TEXT_SIZE: f32 = 14.0;
const CHIP_TEXT_SIZE: f32 = 12.0;
const LINK_UNDERLINE_Y: f32 = 18.0;
const CODE_BOX_Y: f32 = 2.0;
const CODE_BOX_HEIGHT: f32 = 19.0;
const CHIP_BOX_Y: f32 = 1.0;
const CHIP_BOX_HEIGHT: f32 = 24.0;
const SHAPED_TEXT_PAD_X: f32 = 2.0;
const SHAPED_TEXT_PAD_Y: f32 = 2.0;

const PAGE_TOP_FILL: Color32 = Color32::from_rgb(251, 247, 240);
const PAGE_FILL: Color32 = Color32::from_rgb(245, 241, 234);
const TOOLBAR_FILL: Color32 = Color32::from_rgb(255, 253, 249);
const NOTE_FILL: Color32 = Color32::from_rgb(255, 253, 248);
const INK: Color32 = Color32::from_rgb(32, 27, 24);
const MUTED: Color32 = Color32::from_rgb(109, 100, 93);
const RULE: Color32 = Color32::from_rgb(216, 206, 195);
const ACCENT: Color32 = Color32::from_rgb(149, 95, 59);
const CODE_FILL: Color32 = Color32::from_rgba_premultiplied(17, 31, 43, 20);
const TOOLBAR_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 18],
    blur: 40,
    spread: 0,
    color: Color32::from_rgba_premultiplied(54, 40, 23, 20),
};
const NOTE_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 18],
    blur: 40,
    spread: 0,
    color: Color32::from_rgba_premultiplied(54, 40, 23, 20),
};

pub struct RichNoteDemo {
    open: bool,
    requested_width: f32,
    prepared: Option<PreparedRichNote>,
    layout_cache: Option<CachedRichNoteLayout>,
}

impl Default for RichNoteDemo {
    fn default() -> Self {
        Self {
            open: false,
            requested_width: BODY_DEFAULT_WIDTH,
            prepared: None,
            layout_cache: None,
        }
    }
}

#[derive(Clone, Copy)]
enum RichInlineSpec {
    Text {
        text: &'static str,
        style: TextStyleName,
    },
    Chip {
        label: &'static str,
        tone: ChipTone,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TextStyleName {
    Body,
    Link,
    Code,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChipTone {
    Mention,
    Status,
    Priority,
    Time,
    Count,
}

#[derive(Clone, Copy)]
struct FragmentPaintMetrics {
    slot_top: f32,
    slot_height: f32,
}

struct TextStyleModel {
    style_name: TextStyleName,
    chrome_width: f32,
    spec: &'static TextStyleSpec,
}

#[derive(Clone)]
struct PreparedRichNote {
    flow: PreparedRichInline,
    items: Vec<InlineItem>,
}

#[derive(Clone)]
enum InlineItem {
    Text(TextInlineItem),
    Chip(ChipInlineItem),
}

#[derive(Clone)]
struct TextInlineItem {
    style_name: TextStyleName,
    chrome_width: f32,
    #[cfg_attr(not(test), allow(dead_code))]
    full_text: String,
    #[cfg_attr(not(test), allow(dead_code))]
    full_visual_runs: Vec<LayoutLineVisualRun>,
    prepared: PreparedTextWithSegments,
}

#[derive(Clone)]
struct ChipInlineItem {
    tone: ChipTone,
    text: String,
    glyph_runs: Vec<LayoutLineGlyphRun>,
    emoji_overlays: Vec<EmojiOverlayRun>,
    text_width: f32,
    chrome_width: f32,
    #[cfg_attr(not(test), allow(dead_code))]
    prepared: PreparedTextWithSegments,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FragmentKind {
    Body,
    Link,
    Code,
    Chip(ChipTone),
}

#[derive(Clone, Debug, PartialEq)]
struct LineFragment {
    kind: FragmentKind,
    leading_gap: f32,
    text: String,
    glyph_runs: Vec<LayoutLineGlyphRun>,
    emoji_overlays: Vec<EmojiOverlayRun>,
    text_width: f32,
    chrome_width: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct RichLine {
    fragments: Vec<LineFragment>,
}

struct CachedRichNoteLayout {
    body_width_px: u32,
    note_body_height: f32,
    lines: Vec<RichLine>,
}

const INLINE_SPECS: &[RichInlineSpec] = &[
    RichInlineSpec::Text {
        text: "Ship ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Chip {
        label: "@maya",
        tone: ChipTone::Mention,
    },
    RichInlineSpec::Text {
        text: "'s ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Text {
        text: "rich-note",
        style: TextStyleName::Code,
    },
    RichInlineSpec::Text {
        text: " card once ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Text {
        text: "pre-wrap",
        style: TextStyleName::Code,
    },
    RichInlineSpec::Text {
        text: " lands. Status ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Chip {
        label: "blocked",
        tone: ChipTone::Status,
    },
    RichInlineSpec::Text {
        text: " by ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Text {
        text: "vertical text",
        style: TextStyleName::Link,
    },
    RichInlineSpec::Text {
        text: " research, but 北京 copy and Arabic QA are both green ✅. Keep ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Chip {
        label: "جاهز",
        tone: ChipTone::Status,
    },
    RichInlineSpec::Text {
        text: " for ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Text {
        text: "Cmd+K",
        style: TextStyleName::Code,
    },
    RichInlineSpec::Text {
        text: " docs; the review bundle now includes 中文 labels, عربي fallback, and one more launch pass 🚀 for ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Chip {
        label: "Fri 2:30 PM",
        tone: ChipTone::Time,
    },
    RichInlineSpec::Text {
        text: ". Keep ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Text {
        text: "layoutNextLine()",
        style: TextStyleName::Code,
    },
    RichInlineSpec::Text {
        text: " public, tag this ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Chip {
        label: "P1",
        tone: ChipTone::Priority,
    },
    RichInlineSpec::Text {
        text: ", keep ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Chip {
        label: "3 reviewers",
        tone: ChipTone::Count,
    },
    RichInlineSpec::Text {
        text: ", and route feedback to ",
        style: TextStyleName::Body,
    },
    RichInlineSpec::Text {
        text: "design sync",
        style: TextStyleName::Link,
    },
    RichInlineSpec::Text {
        text: ".",
        style: TextStyleName::Body,
    },
];

impl DemoWindow for RichNoteDemo {
    fn id(&self) -> &'static str {
        "rich_note"
    }

    fn title(&self) -> &str {
        "Rich Text"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1020.0, 1480.0))
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

impl RichNoteDemo {
    fn render_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let outer_width = ui.available_width().max(320.0);
        let page_gutter = if outer_width <= MOBILE_BREAKPOINT {
            MOBILE_PAGE_GUTTER
        } else {
            DESKTOP_PAGE_GUTTER
        };
        let page_width = (outer_width - page_gutter).min(PAGE_MAX_WIDTH).max(280.0);
        let raw_max_body = outer_width - PAGE_MARGIN * 2.0 - NOTE_SHELL_CHROME_X;
        let max_body_width = raw_max_body.max(BODY_MIN_WIDTH).min(BODY_MAX_WIDTH).floor();
        let mut requested_width = self
            .requested_width
            .clamp(BODY_MIN_WIDTH, max_body_width)
            .round();

        ui.scope_builder(
            UiBuilder::new().layout(Layout::top_down(Align::Center)),
            |ui| {
                ui.set_min_width(page_width);
                ui.set_max_width(page_width);
                ui.set_width(page_width);

                let page_rect = ui.max_rect();
                paint_page_backdrop(ui.painter(), page_rect);

                ui.scope_builder(
                    UiBuilder::new().layout(Layout::top_down(Align::Min)),
                    |ui| {
                        paint_intro(ui);
                        ui.add_space(20.0);
                        paint_toolbar(ui, outer_width, &mut requested_width, max_body_width);
                        ui.add_space(24.0);
                        let body_width = requested_width
                            .clamp(BODY_MIN_WIDTH, max_body_width)
                            .round();
                        self.ensure_prepared(engine);
                        let layout = self.ensure_layout(engine, body_width);
                        paint_preview(
                            ui,
                            ctx,
                            engine,
                            assets,
                            body_width,
                            layout.note_body_height,
                            &layout.lines,
                        );
                    },
                );
            },
        );

        self.requested_width = requested_width.clamp(BODY_MIN_WIDTH, max_body_width);
    }

    fn ensure_prepared(&mut self, engine: &PretextEngine) -> &PreparedRichNote {
        if self.prepared.is_none() {
            self.prepared = Some(prepare_rich_note(engine, INLINE_SPECS));
        }

        self.prepared
            .as_ref()
            .expect("rich note items should be prepared")
    }

    fn ensure_layout(&mut self, engine: &PretextEngine, body_width: f32) -> &CachedRichNoteLayout {
        let body_width_px = body_width.max(1.0).round() as u32;
        let should_rebuild = self
            .layout_cache
            .as_ref()
            .is_none_or(|cache| cache.body_width_px != body_width_px);

        if should_rebuild {
            let lines = {
                let prepared = self
                    .prepared
                    .as_ref()
                    .expect("rich note items should be prepared before layout");
                layout_inline_items(engine, prepared, body_width)
            };
            let line_count = lines.len();
            let note_body_height = if line_count == 0 {
                LAST_LINE_BLOCK_HEIGHT
            } else {
                (line_count as f32 - 1.0) * LINE_HEIGHT + LAST_LINE_BLOCK_HEIGHT
            };
            self.layout_cache = Some(CachedRichNoteLayout {
                body_width_px,
                note_body_height,
                lines,
            });
        }

        self.layout_cache
            .as_ref()
            .expect("rich note layout cache should be populated")
    }
}

fn paint_page_backdrop(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::ZERO, PAGE_FILL);
    painter.circle_filled(
        egui::pos2(rect.center().x, rect.top() - rect.height() * 0.18),
        rect.width() * 0.52,
        PAGE_TOP_FILL,
    );
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
    ui.label(RichText::new("Rich Text").size(34.0).color(INK).strong());
    ui.add_space(10.0);
    ui.scope(|ui| {
        ui.set_max_width(INTRO_MAX_WIDTH);
        ui.label(
            RichText::new(
                "Rich text rendered as text runs, links, code spans, and atomic chips. Adjust the text width to watch the chips stay whole while the surrounding text and code keep wrapping naturally when space gets tight.",
            )
            .size(15.0)
            .color(MUTED),
        );
    });
}

fn paint_toolbar(
    ui: &mut egui::Ui,
    outer_width: f32,
    requested_width: &mut f32,
    max_body_width: f32,
) {
    egui::Frame::new()
        .fill(TOOLBAR_FILL)
        .stroke(Stroke::new(1.0, RULE))
        .corner_radius(CornerRadius::same(18))
        .shadow(TOOLBAR_SHADOW)
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let value_width = 64.0;
            let value_label = RichText::new(format!("{:.0}px", requested_width.round()))
                .size(13.0)
                .color(MUTED)
                .strong();
            if outer_width <= MOBILE_BREAKPOINT {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Text Width")
                            .monospace()
                            .size(11.0)
                            .color(MUTED)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    let slider_width = ui.available_width().max(1.0);
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = slider_width;
                        ui.add(
                            egui::Slider::new(requested_width, BODY_MIN_WIDTH..=max_body_width)
                                .show_value(false)
                                .step_by(1.0),
                        );
                    });
                    ui.add_space(8.0);
                    ui.label(value_label);
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Text Width")
                            .monospace()
                            .size(11.0)
                            .color(MUTED)
                            .strong(),
                    );
                    ui.add_space(14.0);
                    let slider_width =
                        (ui.available_width() - value_width - ui.spacing().item_spacing.x).max(1.0);
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = slider_width;
                        ui.add(
                            egui::Slider::new(requested_width, BODY_MIN_WIDTH..=max_body_width)
                                .show_value(false)
                                .step_by(1.0),
                        );
                    });
                    ui.add_sized(egui::vec2(value_width, 18.0), egui::Label::new(value_label));
                });
            }
        });
}

fn paint_preview(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    body_width: f32,
    note_body_height: f32,
    lines: &[RichLine],
) {
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        let note_shell_width = note_shell_width(body_width);
        ui.scope(|ui| {
            ui.set_min_width(note_shell_width);
            ui.set_max_width(note_shell_width);
            ui.set_width(note_shell_width);
            egui::Frame::new()
                .fill(NOTE_FILL)
                .stroke(Stroke::new(1.0, RULE))
                .corner_radius(CornerRadius::same(20))
                .shadow(NOTE_SHADOW)
                .inner_margin(egui::Margin::same(NOTE_SHELL_PADDING))
                .show(ui, |ui| {
                    ui.set_min_width(body_width);
                    ui.set_max_width(body_width);
                    ui.set_width(body_width);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(body_width, note_body_height.max(LAST_LINE_BLOCK_HEIGHT)),
                        Sense::hover(),
                    );
                    paint_rich_note_body(ui.painter(), rect, lines, ctx, engine, assets);
                });
        });
    });
}

fn note_shell_width(body_width: f32) -> f32 {
    body_width + NOTE_SHELL_PADDING as f32 * 2.0
}

impl LineFragment {
    fn total_width(&self) -> f32 {
        self.text_width + self.chrome_width
    }
}

fn text_style_model(style_name: TextStyleName) -> TextStyleModel {
    match style_name {
        TextStyleName::Body => TextStyleModel {
            style_name,
            chrome_width: 0.0,
            spec: body_text_style(),
        },
        TextStyleName::Link => TextStyleModel {
            style_name,
            chrome_width: 0.0,
            spec: link_text_style(),
        },
        TextStyleName::Code => TextStyleModel {
            style_name,
            chrome_width: CODE_CHROME_WIDTH,
            spec: code_text_style(),
        },
    }
}

fn build_text_style(families: &[&str], size_px: f32, weight: u16, italic: bool) -> TextStyleSpec {
    TextStyleSpec {
        families: families.iter().map(|name| (*name).to_owned()).collect(),
        size_px,
        weight,
        italic,
    }
}

fn body_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
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
            BODY_TEXT_SIZE,
            500,
            false,
        )
    })
}

fn link_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
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
            LINK_TEXT_SIZE,
            600,
            false,
        )
    })
}

fn code_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| {
        build_text_style(
            &[
                "SF Mono",
                "Menlo",
                "Monaco",
                "Noto Sans Mono",
                "Noto Sans Arabic",
                "Noto Sans CJK",
            ],
            CODE_TEXT_SIZE,
            600,
            false,
        )
    })
}

fn chip_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
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
            CHIP_TEXT_SIZE,
            700,
            false,
        )
    })
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
        letter_spacing: 0.0,
    }
}

fn is_collapsible_boundary_char(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000C}')
}

fn fragment_overlay_options(
    style: &TextStyleSpec,
    metrics: FragmentPaintMetrics,
) -> EmojiOverlayOptions<'_> {
    EmojiOverlayOptions {
        style,
        slot_height: metrics.slot_height,
        padding_x: SHAPED_TEXT_PAD_X,
        padding_y: SHAPED_TEXT_PAD_Y,
        slack_x: 2.0,
        slack_y: 4.0,
        baseline_mode: BaselineMode::AutoFontMetrics,
    }
}

fn prepare_rich_note(engine: &PretextEngine, specs: &[RichInlineSpec]) -> PreparedRichNote {
    let mut flow_items = Vec::with_capacity(specs.len());
    let mut items = Vec::with_capacity(specs.len());

    for spec in specs {
        match *spec {
            RichInlineSpec::Chip { label, tone } => {
                let label_prepared =
                    engine.prepare_paragraph(label, chip_text_style(), &normal_options());
                let mut label_cursor = LINE_START_CURSOR;
                let label_line = engine
                    .layout_next_line_with_runs(&label_prepared, &mut label_cursor, UNBOUNDED_WIDTH)
                    .expect("chip label should fit in an unbounded line");
                let (glyph_runs, emoji_overlays) = split_builtin_emoji_glyphs(
                    &label_line.runs.visual_runs,
                    &label_line.runs.glyph_runs,
                    fragment_overlay_options(chip_text_style(), chip_fragment_metrics(0.0)),
                    engine,
                );
                let text_width = label_line.line.width.ceil();
                let layout_extra_width =
                    CHIP_CHROME_WIDTH + (text_width - label_line.line.width).max(0.0);
                let prepared = engine
                    .prepare_atomic_placeholder(text_width + CHIP_CHROME_WIDTH, &normal_options());

                flow_items.push(FlowItemSpec {
                    text: label,
                    style: chip_text_style(),
                    break_mode: FlowBreakMode::Never,
                    extra_width: layout_extra_width,
                    letter_spacing: 0.0,
                });
                items.push(InlineItem::Chip(ChipInlineItem {
                    tone,
                    text: label.to_owned(),
                    glyph_runs,
                    emoji_overlays,
                    text_width,
                    chrome_width: CHIP_CHROME_WIDTH,
                    prepared,
                }));
            }
            RichInlineSpec::Text { text, style } => {
                let style_model = text_style_model(style);
                flow_items.push(FlowItemSpec {
                    text,
                    style: style_model.spec,
                    break_mode: FlowBreakMode::Normal,
                    extra_width: style_model.chrome_width,
                    letter_spacing: 0.0,
                });

                let trimmed_text = text.trim_matches(is_collapsible_boundary_char);
                assert!(
                    !trimmed_text.is_empty(),
                    "rich note text specs should keep visible content after boundary whitespace collapse",
                );

                let prepared =
                    engine.prepare_paragraph(trimmed_text, style_model.spec, &normal_options());
                let mut cursor = LINE_START_CURSOR;
                let whole_line = engine
                    .layout_next_line_with_runs(&prepared, &mut cursor, UNBOUNDED_WIDTH)
                    .expect("rich note text item should fit in an unbounded line");

                items.push(InlineItem::Text(TextInlineItem {
                    style_name: style_model.style_name,
                    chrome_width: style_model.chrome_width,
                    full_text: whole_line.line.text,
                    full_visual_runs: whole_line.runs.visual_runs,
                    prepared,
                }));
            }
        }
    }

    PreparedRichNote {
        flow: prepare_rich_inline(engine, &flow_items),
        items,
    }
}

fn layout_inline_items(
    engine: &PretextEngine,
    prepared: &PreparedRichNote,
    max_width: f32,
) -> Vec<RichLine> {
    layout_rich_inline(engine, &prepared.flow, max_width)
        .into_iter()
        .map(|line| RichLine {
            fragments: line
                .fragments
                .into_iter()
                .map(|fragment| {
                    build_line_fragment(engine, &prepared.items[fragment.item_index], fragment)
                })
                .collect(),
        })
        .collect()
}

fn build_line_fragment(
    engine: &PretextEngine,
    item: &InlineItem,
    fragment: FlowFragment,
) -> LineFragment {
    match item {
        InlineItem::Text(item) => {
            let line = fragment.line;
            let runs = engine.line_runs(&item.prepared, &line);
            let (glyph_runs, emoji_overlays) = split_builtin_emoji_glyphs(
                &runs.visual_runs,
                &runs.glyph_runs,
                fragment_overlay_options(
                    text_style_model(item.style_name).spec,
                    fragment_metrics_for_style(item.style_name, 0.0),
                ),
                engine,
            );

            LineFragment {
                kind: fragment_kind_for_style(item.style_name),
                leading_gap: fragment.leading_gap,
                text: line.text,
                glyph_runs,
                emoji_overlays,
                text_width: line.width,
                chrome_width: item.chrome_width,
            }
        }
        InlineItem::Chip(item) => LineFragment {
            kind: FragmentKind::Chip(item.tone),
            leading_gap: fragment.leading_gap,
            text: item.text.clone(),
            glyph_runs: item.glyph_runs.clone(),
            emoji_overlays: item.emoji_overlays.clone(),
            text_width: item.text_width,
            chrome_width: item.chrome_width,
        },
    }
}

fn fragment_kind_for_style(style_name: TextStyleName) -> FragmentKind {
    match style_name {
        TextStyleName::Body => FragmentKind::Body,
        TextStyleName::Link => FragmentKind::Link,
        TextStyleName::Code => FragmentKind::Code,
    }
}

fn fragment_metrics_for_style(style_name: TextStyleName, line_top: f32) -> FragmentPaintMetrics {
    match style_name {
        TextStyleName::Body | TextStyleName::Link => body_fragment_metrics(line_top),
        TextStyleName::Code => code_fragment_metrics(line_top),
    }
}

fn paint_rich_note_body(
    painter: &egui::Painter,
    rect: Rect,
    lines: &[RichLine],
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) {
    let body_left = rect.left();
    let body_top = rect.top();
    let mut text_runs = Vec::new();

    for (line_index, line) in lines.iter().enumerate() {
        let mut x = body_left;
        let y = body_top + line_index as f32 * LINE_HEIGHT;

        for fragment in &line.fragments {
            x += fragment.leading_gap;

            match fragment.kind {
                FragmentKind::Body => {
                    let metrics = body_fragment_metrics(y);
                    text_runs.push(rich_note_text_run(
                        x,
                        fragment,
                        metrics,
                        fragment_font_family(fragment.kind),
                        text_style_model(TextStyleName::Body).spec,
                        INK,
                        BODY_TEXT_SIZE,
                    ));
                }
                FragmentKind::Link => {
                    let metrics = body_fragment_metrics(y);
                    text_runs.push(rich_note_text_run(
                        x,
                        fragment,
                        metrics,
                        fragment_font_family(fragment.kind),
                        text_style_model(TextStyleName::Link).spec,
                        ACCENT,
                        LINK_TEXT_SIZE,
                    ));
                    painter.line_segment(
                        [
                            egui::pos2(x, y + LINK_UNDERLINE_Y),
                            egui::pos2(x + fragment.text_width, y + LINK_UNDERLINE_Y),
                        ],
                        Stroke::new(1.0, ACCENT),
                    );
                }
                FragmentKind::Code => {
                    let box_rect = Rect::from_min_size(
                        egui::pos2(x, y + CODE_BOX_Y),
                        egui::vec2(fragment.total_width(), CODE_BOX_HEIGHT),
                    );
                    painter.rect_filled(box_rect, CornerRadius::same(9), CODE_FILL);
                    let metrics = code_fragment_metrics(y);
                    text_runs.push(rich_note_text_run(
                        x + fragment.chrome_width * 0.5,
                        fragment,
                        metrics,
                        fragment_font_family(fragment.kind),
                        text_style_model(TextStyleName::Code).spec,
                        INK,
                        CODE_TEXT_SIZE,
                    ));
                }
                FragmentKind::Chip(tone) => {
                    let (fill, stroke, text_color) = chip_palette(tone);
                    let box_rect = Rect::from_min_size(
                        egui::pos2(x, y + CHIP_BOX_Y),
                        egui::vec2(fragment.total_width(), CHIP_BOX_HEIGHT),
                    );
                    painter.rect_filled(box_rect, CornerRadius::same(11), fill);
                    painter.rect_stroke(
                        box_rect,
                        CornerRadius::same(11),
                        Stroke::new(1.0, stroke),
                        StrokeKind::Inside,
                    );
                    let metrics = chip_fragment_metrics(y);
                    text_runs.push(rich_note_text_run(
                        x + fragment.chrome_width * 0.5,
                        fragment,
                        metrics,
                        fragment_font_family(fragment.kind),
                        chip_text_style(),
                        text_color,
                        CHIP_TEXT_SIZE,
                    ));
                }
            }

            x += fragment.total_width();
        }
    }

    let _ = paint_styled_positioned_text_runs(painter, text_runs, ctx, engine, assets);
}

fn body_fragment_metrics(line_top: f32) -> FragmentPaintMetrics {
    FragmentPaintMetrics {
        slot_top: line_top,
        slot_height: BODY_TEXT_SIZE,
    }
}

fn code_fragment_metrics(line_top: f32) -> FragmentPaintMetrics {
    let slot_top = line_top + CODE_BOX_Y;
    FragmentPaintMetrics {
        slot_top,
        slot_height: CODE_BOX_HEIGHT,
    }
}

fn chip_fragment_metrics(line_top: f32) -> FragmentPaintMetrics {
    let slot_top = line_top + CHIP_BOX_Y;
    FragmentPaintMetrics {
        slot_top,
        slot_height: CHIP_BOX_HEIGHT,
    }
}

fn chip_palette(tone: ChipTone) -> (Color32, Color32, Color32) {
    match tone {
        ChipTone::Mention => (
            Color32::from_rgba_premultiplied(21, 90, 136, 31),
            Color32::from_rgba_premultiplied(21, 90, 136, 31),
            Color32::from_rgb(21, 90, 136),
        ),
        ChipTone::Status => (
            Color32::from_rgba_premultiplied(196, 129, 20, 31),
            Color32::from_rgba_premultiplied(196, 129, 20, 36),
            Color32::from_rgb(145, 98, 7),
        ),
        ChipTone::Priority => (
            Color32::from_rgba_premultiplied(176, 44, 44, 26),
            Color32::from_rgba_premultiplied(176, 44, 44, 36),
            Color32::from_rgb(142, 35, 35),
        ),
        ChipTone::Time => (
            Color32::from_rgba_premultiplied(70, 118, 77, 28),
            Color32::from_rgba_premultiplied(70, 118, 77, 36),
            Color32::from_rgb(53, 95, 56),
        ),
        ChipTone::Count => (
            Color32::from_rgba_premultiplied(67, 57, 122, 26),
            Color32::from_rgba_premultiplied(67, 57, 122, 33),
            Color32::from_rgb(72, 62, 131),
        ),
    }
}

fn fragment_font_family(kind: FragmentKind) -> FontFamily {
    match kind {
        FragmentKind::Code => FontFamily::Monospace,
        FragmentKind::Body | FragmentKind::Link | FragmentKind::Chip(_) => FontFamily::Proportional,
    }
}

fn rich_note_text_run<'a>(
    x: f32,
    fragment: &'a LineFragment,
    metrics: FragmentPaintMetrics,
    fallback_family: FontFamily,
    style: &'static TextStyleSpec,
    color: Color32,
    emoji_size: f32,
) -> StyledPositionedTextRunRef<'a, 'static> {
    StyledPositionedTextRunRef {
        x,
        y: metrics.slot_top,
        text: &fragment.text,
        glyph_runs: &fragment.glyph_runs,
        emoji_overlays: &fragment.emoji_overlays,
        options: EguiPretextPaintOptions::new(style, metrics.slot_height)
            .color(color)
            .fallback_font(FontId::new(style.size_px, fallback_family))
            .fallback_align(Align2::LEFT_TOP)
            .emoji_size(emoji_size)
            .emoji_slot_height(metrics.slot_height),
    }
}

#[cfg_attr(not(test), allow(dead_code))]
fn rich_shaped_text_request<'a>(
    text: &'a str,
    style: &'a TextStyleSpec,
    direction: BidiDirection,
    _color: Color32,
    _fragment_width: f32,
    metrics: FragmentPaintMetrics,
    baseline_mode: BaselineMode,
) -> PretextTextureRasterRequest<'a> {
    PretextTextureRasterRequest {
        text,
        style,
        direction,
        slot_height: metrics.slot_height,
        padding_x: SHAPED_TEXT_PAD_X,
        padding_y: SHAPED_TEXT_PAD_Y,
        slack_x: 2.0,
        slack_y: 4.0,
        baseline_mode,
        texture_options: egui::TextureOptions::NEAREST,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{RawInput, TextureId};

    fn bundled_engine() -> PretextEngine {
        PretextEngine::builder()
            .with_font_data(crate::demo_assets::bundled_font_data())
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
    fn chips_stay_atomic_across_lines() {
        let engine = bundled_engine();
        let prepared = prepare_rich_note(&engine, INLINE_SPECS);
        let lines = layout_inline_items(&engine, &prepared, 310.0);

        let chip_labels: Vec<&'static str> = INLINE_SPECS
            .iter()
            .filter_map(|spec| match spec {
                RichInlineSpec::Chip { label, .. } => Some(*label),
                RichInlineSpec::Text { .. } => None,
            })
            .collect();

        for label in chip_labels {
            let count = lines
                .iter()
                .flat_map(|line| line.fragments.iter())
                .filter(|fragment| {
                    matches!(fragment.kind, FragmentKind::Chip(_)) && fragment.text == label
                })
                .count();
            assert_eq!(count, 1, "chip `{label}` should appear exactly once");
        }
    }

    #[test]
    fn rich_note_layout_is_deterministic() {
        let engine = bundled_engine();
        let prepared = prepare_rich_note(&engine, INLINE_SPECS);
        let first = layout_inline_items(&engine, &prepared, 420.0);
        let second = layout_inline_items(&engine, &prepared, 420.0);
        assert_eq!(first, second);
    }

    #[test]
    fn note_shell_width_tracks_body_width() {
        assert_eq!(
            note_shell_width(BODY_DEFAULT_WIDTH),
            BODY_DEFAULT_WIDTH + NOTE_SHELL_PADDING as f32 * 2.0
        );
        assert!(note_shell_width(640.0) > note_shell_width(420.0));
    }

    #[test]
    fn rich_note_keeps_visual_runs_for_mixed_direction_text() {
        let engine = bundled_engine();
        let prepared = prepare_rich_note(&engine, INLINE_SPECS);
        let mixed = prepared
            .items
            .iter()
            .find_map(|item| match item {
                InlineItem::Text(item) if item.full_text.contains("عربي") => Some(item),
                _ => None,
            })
            .expect("mixed-direction rich-note item should exist");

        assert!(mixed.full_visual_runs.len() >= 2);
        let rtl_index = mixed
            .full_visual_runs
            .iter()
            .position(|run| run.direction == BidiDirection::Rtl)
            .expect("mixed-direction item should contain an RTL visual run");
        assert!(rtl_index > 0);
        assert!(mixed.full_visual_runs[rtl_index + 1..]
            .iter()
            .any(|run| run.direction == BidiDirection::Ltr));
    }

    #[test]
    fn chips_use_engine_atomic_placeholders() {
        let engine = bundled_engine();
        let prepared = prepare_rich_note(&engine, INLINE_SPECS);

        let chip = prepared
            .items
            .iter()
            .find_map(|item| match item {
                InlineItem::Chip(chip) => Some(chip),
                InlineItem::Text(_) => None,
            })
            .expect("at least one chip item should exist");

        let mut cursor = LINE_START_CURSOR;
        let line = engine
            .layout_next_line(&chip.prepared, &mut cursor, 8.0)
            .expect("atomic placeholder should still lay out on an empty line");

        assert_eq!(line.text, "");
        assert_eq!(line.width, chip.text_width + chip.chrome_width);
        assert_eq!(
            cursor,
            LayoutCursor {
                segment_index: 1,
                grapheme_index: 0,
            }
        );
    }

    #[test]
    fn rich_note_render_emits_texture_shapes_for_mixed_emoji_and_arabic() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = EguiPretextRenderer::default();
        crate::demo_assets::install_demo_fonts(&ctx);

        let mut demo = RichNoteDemo {
            open: true,
            requested_width: BODY_DEFAULT_WIDTH,
            prepared: None,
            layout_cache: None,
        };

        let raw_input = |time: f64| RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 960.0),
            )),
            time: Some(time),
            ..Default::default()
        };

        let _ = ctx.run_ui(raw_input(0.0), |ctx| {
            demo.show(ctx, &engine, &mut assets);
        });
        let output = ctx.run_ui(raw_input(1.0), |ctx| {
            demo.show(ctx, &engine, &mut assets);
        });
        let stats = assets.stats();

        assert!(output
            .shapes
            .iter()
            .any(|clipped| { shape_uses_user_texture(&clipped.shape) }));
        assert!(stats.atlas_entries > 0);
        assert_eq!(stats.shaped_text_textures, 0);
    }

    #[test]
    fn rich_note_arabic_textures_materialize_for_body_and_chip_slots() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = EguiPretextRenderer::default();

        let body_request = rich_shaped_text_request(
            "عربي",
            text_style_model(TextStyleName::Body).spec,
            BidiDirection::Rtl,
            INK,
            36.0,
            FragmentPaintMetrics {
                slot_top: 0.0,
                slot_height: BODY_TEXT_SIZE,
            },
            BaselineMode::AutoFontMetrics,
        );
        let chip_request = rich_shaped_text_request(
            "جاهز",
            chip_text_style(),
            BidiDirection::Rtl,
            chip_palette(ChipTone::Status).2,
            24.0,
            FragmentPaintMetrics {
                slot_top: 0.0,
                slot_height: CHIP_BOX_HEIGHT,
            },
            BaselineMode::AutoFontMetrics,
        );

        let body = assets
            .rasterize_text_texture(&engine, body_request, &ctx)
            .expect("body arabic texture should exist");
        let body_cached = assets
            .rasterize_text_texture(&engine, body_request, &ctx)
            .expect("cached body arabic texture should exist");
        let chip = assets
            .rasterize_text_texture(&engine, chip_request, &ctx)
            .expect("chip arabic texture should exist");
        let stats = assets.stats();

        assert_eq!(body.handle.id(), body_cached.handle.id());
        assert_eq!(body.logical_size, body_cached.logical_size);
        assert!(body.logical_size.x > 0.0);
        assert!(chip.logical_size.x > 0.0);
        assert_eq!(stats.texture_uploads, 2);
        assert_eq!(stats.texture_cache_hits, 1);
        assert_eq!(stats.texture_cache_misses, 2);
        assert_eq!(stats.render.rasterizations, 2);
    }
}

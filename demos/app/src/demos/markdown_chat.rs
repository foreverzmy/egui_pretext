use std::iter::Peekable;
use std::sync::OnceLock;

use eframe::egui;
use egui::{
    epaint::{Mesh, Vertex},
    Align, Align2, Button, Color32, CornerRadius, CursorIcon, FontFamily, FontId, Layout, Rect,
    RichText, Sense, Shape, Stroke, StrokeKind, UiBuilder,
};
use pretext::{
    rich_inline::{
        materialize_rich_inline_line_range, measure_rich_inline_stats, prepare_rich_inline,
        walk_rich_inline_line_ranges, PreparedRichInline, RichInlineBreakMode, RichInlineItemSpec,
    },
    ParagraphDirection, PretextEngine, PretextGlyphRun, PretextParagraphLayout,
    PretextParagraphOptions as PrepareOptions,
    PretextPreparedParagraph as PreparedTextWithSegments, PretextStyle as TextStyleSpec,
    WhiteSpaceMode, WordBreakMode,
};
use pretext_egui::{
    advanced::{
        paint_styled_positioned_text_runs, split_builtin_emoji_glyphs, EmojiOverlayOptions,
        EmojiOverlayRun, StyledPositionedTextRunRef,
    },
    BaselineMode, EguiPretextPaintOptions, EguiPretextRenderer,
};
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::demos::DemoWindow;

const PAGE_FILL: Color32 = Color32::from_rgb(51, 55, 64);
const PANEL_FILL: Color32 = Color32::from_rgb(51, 55, 64);
const PANEL_RULE: Color32 = Color32::from_rgb(69, 75, 85);
const INK: Color32 = Color32::from_rgb(213, 217, 225);
const MUTED: Color32 = Color32::from_rgb(158, 166, 178);
const ACCENT: Color32 = Color32::from_rgb(183, 192, 207);
const USER_BUBBLE_FILL: Color32 = Color32::from_rgb(57, 64, 72);
const USER_BUBBLE_STROKE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 12);
const CODE_BLOCK_FILL: Color32 = Color32::from_rgb(49, 56, 64);
const INLINE_CODE_FILL: Color32 = Color32::from_rgb(49, 56, 64);
const IMAGE_CHIP_FILL: Color32 = Color32::from_rgb(61, 69, 79);
const IMAGE_CHIP_STROKE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20);
const QUOTE_RAIL: Color32 = Color32::from_rgba_premultiplied(158, 166, 178, 46);
const OCCLUSION_TINT: Color32 = Color32::from_rgba_premultiplied(51, 55, 64, 236);
const OCCLUSION_TINT_ACTIVE: Color32 = Color32::from_rgba_premultiplied(51, 55, 64, 188);
const OCCLUSION_EDGE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 10);
const OCCLUSION_EDGE_ACTIVE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 18);
const VIRTUALIZATION_BUTTON_FILL: Color32 = Color32::from_rgba_premultiplied(57, 64, 72, 234);
const VIRTUALIZATION_BUTTON_FILL_ACTIVE: Color32 =
    Color32::from_rgba_premultiplied(57, 64, 72, 250);
const VIRTUALIZATION_BUTTON_STROKE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 16);

const WINDOW_DEFAULT_WIDTH: f32 = 1120.0;
const WINDOW_DEFAULT_HEIGHT: f32 = 960.0;
const PAGE_MAX_WIDTH: f32 = 1160.0;
const PAGE_GUTTER: f32 = 28.0;
const PAGE_TOP_GAP: f32 = 18.0;
const HEADER_GAP: f32 = 18.0;
const CONTROL_RADIUS: u8 = 18;
const CHAT_RADIUS: u8 = 24;
const CHAT_PANEL_PADDING: i8 = 16;
const CHAT_BODY_MIN_HEIGHT: f32 = 360.0;
const CHAT_BODY_DEFAULT_HEIGHT: f32 = 640.0;
const VIEWPORT_OVERSCAN: f32 = 180.0;
const OCCLUSION_BANNER_HEIGHT: f32 = 61.0;
const COMPACT_OCCLUSION_BANNER_HEIGHT: f32 = 43.0;
const COMPACT_OCCLUSION_VIEWPORT_HEIGHT: f32 = 460.0;
const CHAT_TOP_PADDING_OFFSET: f32 = 14.0;
const CHAT_BOTTOM_PADDING_OFFSET: f32 = 10.0;
const VIRTUALIZATION_BUTTON_HEIGHT: f32 = 30.0;
const COMPACT_VIRTUALIZATION_BUTTON_HEIGHT: f32 = 26.0;
const VIRTUALIZATION_BUTTON_PAD_X: f32 = 14.0;
const COMPACT_VIRTUALIZATION_BUTTON_PAD_X: f32 = 12.0;
const VIRTUALIZATION_BUTTON_TEXT_SIZE: f32 = 12.0;
const COMPACT_VIRTUALIZATION_BUTTON_TEXT_SIZE: f32 = 11.0;

const MIN_CHAT_WIDTH: f32 = 360.0;
const DEFAULT_CHAT_WIDTH: f32 = 640.0;
const MAX_CHAT_WIDTH: f32 = 860.0;
const TOTAL_MESSAGE_COUNT: usize = 10_000;
const MESSAGE_GAP: f32 = 12.0;
const MESSAGE_SIDE_PADDING: f32 = 22.0;
const BUBBLE_MAX_RATIO: f32 = 0.78;
const BUBBLE_PADDING_X: f32 = 16.0;
const BUBBLE_PADDING_Y: f32 = 10.0;
const BODY_LINE_HEIGHT: f32 = 22.0;
const HEADING_ONE_LINE_HEIGHT: f32 = 28.0;
const HEADING_TWO_LINE_HEIGHT: f32 = 25.0;
const HARD_BREAK_GAP: f32 = 4.0;
const BLOCK_GAP: f32 = 12.0;
const RICH_BLOCK_GAP: f32 = 2.0;
const LIST_ITEM_GAP: f32 = 4.0;
const LIST_NESTING_INDENT: f32 = 18.0;
const BLOCKQUOTE_INDENT: f32 = 18.0;
const LIST_MARKER_GAP: f32 = 10.0;
const CODE_LINE_HEIGHT: f32 = 18.0;
const CODE_BLOCK_PADDING_X: f32 = 12.0;
const CODE_BLOCK_PADDING_Y: f32 = 8.0;
const RULE_HEIGHT: f32 = 18.0;
const RAIL_OFFSET: f32 = 5.0;
const INLINE_CODE_CHROME_WIDTH: f32 = 12.0;
const IMAGE_CHROME_WIDTH: f32 = 14.0;
const INLINE_CODE_BOX_HEIGHT: f32 = 18.0;
const IMAGE_CHIP_BOX_HEIGHT: f32 = 18.0;
const INLINE_CHROME_Y: f32 = 2.0;
const LINK_UNDERLINE_Y: f32 = 16.0;
const SHAPED_TEXT_PAD_X: f32 = 2.0;
const SHAPED_TEXT_PAD_Y: f32 = 2.0;
const UNBOUNDED_WIDTH: f32 = 100_000.0;

const BODY_TEXT_SIZE: f32 = 14.0;
const HEADING_ONE_TEXT_SIZE: f32 = 20.0;
const HEADING_TWO_TEXT_SIZE: f32 = 17.0;
const CODE_TEXT_SIZE: f32 = 12.0;
const IMAGE_TEXT_SIZE: f32 = 11.0;
const MARKER_TEXT_SIZE: f32 = 11.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChatRole {
    Assistant,
    User,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InlineVariant {
    Body,
    HeadingOne,
    HeadingTwo,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MarkState {
    bold: bool,
    italic: bool,
    strike: bool,
    link: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ParseContext {
    list_depth: usize,
    quote_depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InlinePieceKind {
    Text,
    Code,
    ImageChip,
}

#[derive(Clone, Debug)]
struct InlinePieceMeta {
    kind: InlinePieceKind,
    style: TextStyleSpec,
    href: Option<String>,
    strike: bool,
    chrome_width: f32,
}

#[derive(Clone, Debug)]
struct InlinePieceSpec {
    text: String,
    meta: InlinePieceMeta,
    break_mode: RichInlineBreakMode,
}

#[derive(Clone, Debug)]
struct MarkdownChatSeed {
    role: ChatRole,
    markdown: &'static str,
}

#[derive(Clone, Debug)]
enum InlineNode {
    Text(String),
    Strong(Vec<InlineNode>),
    Emphasis(Vec<InlineNode>),
    Strike(Vec<InlineNode>),
    Link {
        href: String,
        children: Vec<InlineNode>,
    },
    Code(String),
    Image {
        alt: String,
    },
    SoftBreak,
    HardBreak,
    Html(String),
}

#[derive(Clone, Debug)]
struct ListItemNode {
    task_marker: Option<bool>,
    blocks: Vec<BlockNode>,
}

#[derive(Clone, Debug)]
enum BlockNode {
    Paragraph(Vec<InlineNode>),
    Heading {
        level: u8,
        content: Vec<InlineNode>,
    },
    CodeBlock(String),
    BlockQuote(Vec<BlockNode>),
    List {
        ordered: bool,
        start: usize,
        items: Vec<ListItemNode>,
    },
    Rule,
}

#[derive(Clone)]
struct PreparedBlockBase {
    content_left: f32,
    margin_top: f32,
    marker_left: Option<f32>,
    marker_text: Option<String>,
    quote_rail_lefts: Vec<f32>,
}

#[derive(Clone)]
struct PreparedInlineBlock {
    base: PreparedBlockBase,
    items: Vec<InlinePieceMeta>,
    flow: PreparedRichInline,
    line_height: f32,
}

#[derive(Clone)]
struct PreparedCodeBlock {
    base: PreparedBlockBase,
    line_height: f32,
    prepared: PreparedTextWithSegments,
}

#[derive(Clone)]
struct PreparedRuleBlock {
    base: PreparedBlockBase,
    height: f32,
}

#[derive(Clone)]
enum PreparedBlock {
    Inline(PreparedInlineBlock),
    Code(PreparedCodeBlock),
    Rule(PreparedRuleBlock),
}

#[derive(Clone)]
struct PreparedChatTemplate {
    role: ChatRole,
    blocks: Vec<PreparedBlock>,
}

#[derive(Clone)]
struct PreparedTemplatesState {
    engine_revision: u64,
    templates: Vec<PreparedChatTemplate>,
}

#[derive(Clone)]
struct BlockFrameBase {
    content_left: f32,
    height: f32,
    marker_left: Option<f32>,
    marker_text: Option<String>,
    quote_rail_lefts: Vec<f32>,
    top: f32,
}

#[derive(Clone)]
enum BlockFrame {
    Inline {
        base: BlockFrameBase,
        line_height: f32,
        used_width: f32,
    },
    Code {
        base: BlockFrameBase,
        line_height: f32,
        width: f32,
    },
    Rule {
        base: BlockFrameBase,
        width: f32,
    },
}

#[derive(Clone)]
struct TemplateFrame {
    blocks: Vec<BlockFrame>,
    bubble_height: f32,
    content_inset_x: f32,
    frame_width: f32,
    layout_content_width: f32,
    role: ChatRole,
    total_height: f32,
}

#[derive(Clone)]
struct MessagePlacement {
    template_index: usize,
    top: f32,
    bottom: f32,
}

#[derive(Clone)]
struct ConversationFrame {
    chat_width: f32,
    messages: Vec<MessagePlacement>,
    occlusion_banner_height: f32,
    template_blocks: Vec<Vec<BlockLayout>>,
    template_frames: Vec<TemplateFrame>,
    total_height: f32,
}

struct ConversationFrameCache {
    engine_revision: u64,
    chat_width_q: u32,
    occlusion_banner_height_q: u32,
    frame: ConversationFrame,
}

#[derive(Clone)]
struct InlineFragmentLayout {
    meta: InlinePieceMeta,
    leading_gap: f32,
    text: String,
    text_width: f32,
    glyph_runs: Vec<PretextGlyphRun>,
    emoji_overlays: Vec<EmojiOverlayRun>,
}

#[derive(Clone)]
struct InlineLineLayout {
    fragments: Vec<InlineFragmentLayout>,
    width: f32,
}

#[derive(Clone)]
enum BlockLayout {
    Inline {
        base: BlockFrameBase,
        line_height: f32,
        lines: Vec<InlineLineLayout>,
    },
    Code {
        base: BlockFrameBase,
        line_height: f32,
        layout: PretextParagraphLayout,
        width: f32,
    },
    Rule {
        base: BlockFrameBase,
        width: f32,
    },
}

pub struct MarkdownChatDemo {
    open: bool,
    show_virtualization_mask: bool,
    requested_chat_width: f32,
    templates_state: Option<PreparedTemplatesState>,
    frame_cache: Option<ConversationFrameCache>,
}

impl Default for MarkdownChatDemo {
    fn default() -> Self {
        Self {
            open: false,
            show_virtualization_mask: false,
            requested_chat_width: DEFAULT_CHAT_WIDTH,
            templates_state: None,
            frame_cache: None,
        }
    }
}

impl DemoWindow for MarkdownChatDemo {
    fn title(&self) -> &str {
        "Markdown Chat"
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
            .default_size(egui::vec2(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT))
            .show(ctx, |ui| {
                self.render_page(ui, ctx, engine, assets);
            });
        self.open = open;
    }
}

impl MarkdownChatDemo {
    fn render_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let outer_width = ui.available_width().max(420.0);
        let page_width = (outer_width - PAGE_GUTTER).min(PAGE_MAX_WIDTH).max(320.0);
        let max_chat_width = (page_width - PAGE_GUTTER * 2.0)
            .clamp(MIN_CHAT_WIDTH, MAX_CHAT_WIDTH)
            .floor();
        self.requested_chat_width = self
            .requested_chat_width
            .clamp(MIN_CHAT_WIDTH, max_chat_width.max(MIN_CHAT_WIDTH))
            .round();

        ui.scope_builder(
            UiBuilder::new().layout(Layout::top_down(Align::Center)),
            |ui| {
                ui.set_min_width(page_width);
                ui.set_max_width(page_width);
                ui.set_width(page_width);

                ui.painter()
                    .rect_filled(ui.max_rect(), CornerRadius::ZERO, PAGE_FILL);

                ui.add_space(PAGE_TOP_GAP);
                paint_header(ui);
                ui.add_space(HEADER_GAP);
                paint_controls(ui, &mut self.requested_chat_width, max_chat_width);
                ui.add_space(14.0);

                let chat_height = ui.available_height().max(CHAT_BODY_DEFAULT_HEIGHT);
                let chat_width = self
                    .requested_chat_width
                    .clamp(MIN_CHAT_WIDTH, max_chat_width)
                    .round();
                self.paint_virtualized_chat(ui, ctx, engine, assets, chat_width, chat_height);
            },
        );
    }

    fn ensure_prepared_templates(&mut self, engine: &PretextEngine) -> &[PreparedChatTemplate] {
        let engine_revision = engine.revision();
        if self
            .templates_state
            .as_ref()
            .is_some_and(|state| state.engine_revision != engine_revision)
        {
            self.templates_state = None;
            self.frame_cache = None;
        }

        if self.templates_state.is_none() {
            let templates = BASE_MESSAGE_SPECS
                .iter()
                .map(|seed| PreparedChatTemplate {
                    role: seed.role,
                    blocks: prepare_markdown_blocks(engine, seed.markdown),
                })
                .collect();
            self.templates_state = Some(PreparedTemplatesState {
                engine_revision,
                templates,
            });
        }

        &self
            .templates_state
            .as_ref()
            .expect("prepared markdown chat templates should exist")
            .templates
    }

    fn ensure_frame(
        &mut self,
        engine: &PretextEngine,
        chat_width: f32,
        occlusion_banner_height: f32,
    ) -> &ConversationFrame {
        let engine_revision = engine.revision();
        let chat_width_q = quantize(chat_width);
        let occlusion_banner_height_q = quantize(occlusion_banner_height);
        let should_rebuild = self.frame_cache.as_ref().is_none_or(|cache| {
            cache.engine_revision != engine_revision
                || cache.chat_width_q != chat_width_q
                || cache.occlusion_banner_height_q != occlusion_banner_height_q
        });

        if should_rebuild {
            let frame = build_conversation_frame(
                engine,
                self.ensure_prepared_templates(engine),
                chat_width,
                occlusion_banner_height,
            );
            self.frame_cache = Some(ConversationFrameCache {
                engine_revision,
                chat_width_q,
                occlusion_banner_height_q,
                frame,
            });
        }

        &self
            .frame_cache
            .as_ref()
            .expect("markdown chat frame cache should exist")
            .frame
    }

    fn paint_virtualized_chat(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
        chat_width: f32,
        chat_height: f32,
    ) {
        egui::Frame::new()
            .fill(PANEL_FILL)
            .stroke(Stroke::new(1.0, PANEL_RULE))
            .corner_radius(CornerRadius::same(CHAT_RADIUS))
            .inner_margin(egui::Margin::same(CHAT_PANEL_PADDING))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let panel_height = chat_height.max(CHAT_BODY_MIN_HEIGHT);
                ui.set_height(panel_height);
                let body_width = ui.available_width().max(chat_width);
                let panel_rect = ui.max_rect();
                let occlusion_banner_height = get_occlusion_banner_height(panel_height);

                self.ensure_frame(engine, chat_width, occlusion_banner_height);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        let frame = &self
                            .frame_cache
                            .as_ref()
                            .expect("markdown chat frame cache should exist")
                            .frame;
                        let visible = find_visible_range(
                            frame,
                            viewport.min.y,
                            viewport.height(),
                            frame.occlusion_banner_height,
                            frame.occlusion_banner_height,
                            VIEWPORT_OVERSCAN,
                        );
                        let content_width = body_width.max(frame.chat_width);
                        let lane_left = ((content_width - frame.chat_width) * 0.5).max(0.0);
                        ui.set_width(content_width);
                        ui.set_height(frame.total_height.max(viewport.height()));

                        let origin = ui.max_rect().min;
                        for index in visible.start..visible.end {
                            let message = &frame.messages[index];
                            let template_frame = &frame.template_frames[message.template_index];
                            let block_layouts = &frame.template_blocks[message.template_index];
                            paint_message(
                                ui,
                                ctx,
                                engine,
                                assets,
                                origin,
                                lane_left,
                                frame.chat_width,
                                index,
                                message.top,
                                template_frame,
                                &block_layouts,
                            );
                        }
                    });

                paint_occlusion_layer(
                    ui.painter(),
                    panel_rect,
                    frame_banner_width(panel_rect, chat_width),
                    occlusion_banner_height,
                    self.show_virtualization_mask,
                );

                if paint_virtualization_toggle(
                    ui,
                    panel_rect,
                    occlusion_banner_height,
                    self.show_virtualization_mask,
                ) {
                    self.show_virtualization_mask = !self.show_virtualization_mask;
                }
            });
    }
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
        RichText::new("Markdown Chat")
            .size(32.0)
            .color(INK)
            .strong(),
    );
    ui.add_space(10.0);
    ui.label(
        RichText::new(
            "A virtualized 10k-message chat that parses markdown into blocks, measures exact heights before paint, and uses the shared rich-inline helper for paragraph-like content.",
        )
        .size(14.0)
        .color(MUTED),
    );
}

fn paint_controls(ui: &mut egui::Ui, requested_chat_width: &mut f32, max_chat_width: f32) {
    egui::Frame::new()
        .fill(Color32::from_rgba_premultiplied(255, 255, 255, 5))
        .stroke(Stroke::new(1.0, PANEL_RULE))
        .corner_radius(CornerRadius::same(CONTROL_RADIUS))
        .inner_margin(egui::Margin::symmetric(18, 14))
        .show(ui, |ui| {
            let value_width = 64.0;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Chat Width")
                        .monospace()
                        .size(11.0)
                        .color(MUTED)
                        .strong(),
                );
                ui.add_space(12.0);
                let slider_width =
                    (ui.available_width() - value_width - ui.spacing().item_spacing.x).max(1.0);
                ui.scope(|ui| {
                    ui.spacing_mut().slider_width = slider_width;
                    ui.add(
                        egui::Slider::new(requested_chat_width, MIN_CHAT_WIDTH..=max_chat_width)
                            .show_value(false)
                            .step_by(1.0),
                    );
                });
                ui.add_sized(
                    egui::vec2(value_width, 18.0),
                    egui::Label::new(
                        RichText::new(format!("{:.0}px", requested_chat_width.round()))
                            .size(13.0)
                            .color(MUTED)
                            .strong(),
                    ),
                );
            });
        });
}

fn build_conversation_frame(
    engine: &PretextEngine,
    templates: &[PreparedChatTemplate],
    chat_width: f32,
    occlusion_banner_height: f32,
) -> ConversationFrame {
    let lane_width = (chat_width - MESSAGE_SIDE_PADDING * 2.0).max(120.0);
    let user_frame_width = lane_width.min((chat_width * BUBBLE_MAX_RATIO).floor().max(240.0));
    let assistant_frame_width = lane_width;
    let template_frames = templates
        .iter()
        .map(|template| {
            let content_inset_x = match template.role {
                ChatRole::Assistant => 0.0,
                ChatRole::User => BUBBLE_PADDING_X,
            };
            let frame_width = match template.role {
                ChatRole::Assistant => assistant_frame_width,
                ChatRole::User => user_frame_width,
            };
            let content_width = (frame_width - content_inset_x * 2.0).max(120.0);
            layout_template_frame(
                engine,
                template,
                frame_width,
                content_width,
                content_inset_x,
            )
        })
        .collect::<Vec<_>>();
    let template_blocks = templates
        .iter()
        .zip(&template_frames)
        .map(|(template, frame)| materialize_template_blocks(engine, template, frame))
        .collect::<Vec<_>>();
    let mut messages = Vec::with_capacity(TOTAL_MESSAGE_COUNT);
    let chat_top_padding = occlusion_banner_height + CHAT_TOP_PADDING_OFFSET;
    let chat_bottom_padding = occlusion_banner_height + CHAT_BOTTOM_PADDING_OFFSET;
    let mut y = chat_top_padding;

    for ordinal in 0..TOTAL_MESSAGE_COUNT {
        let template_index = ordinal % templates.len();
        let top = y;
        let bottom = top + template_frames[template_index].total_height;
        messages.push(MessagePlacement {
            template_index,
            top,
            bottom,
        });
        y = bottom + MESSAGE_GAP;
    }

    let total_height = if messages.is_empty() {
        chat_top_padding + chat_bottom_padding
    } else {
        y - MESSAGE_GAP + chat_bottom_padding
    };

    ConversationFrame {
        chat_width,
        messages,
        occlusion_banner_height,
        template_blocks,
        template_frames,
        total_height,
    }
}

fn layout_template_frame(
    engine: &PretextEngine,
    template: &PreparedChatTemplate,
    max_frame_width: f32,
    max_content_width: f32,
    content_inset_x: f32,
) -> TemplateFrame {
    let mut y = BUBBLE_PADDING_Y;
    let mut blocks = Vec::with_capacity(template.blocks.len());
    let mut used_content_width = 0.0f32;

    for block in &template.blocks {
        y += block_base(block).margin_top;
        let frame = layout_block_frame(engine, block, max_content_width, y);
        used_content_width = used_content_width.max(block_used_width(&frame));
        y += block_frame_base(&frame).height;
        blocks.push(frame);
    }

    let bubble_height = y + BUBBLE_PADDING_Y;
    let frame_width = match template.role {
        ChatRole::Assistant => max_frame_width,
        ChatRole::User => (content_inset_x * 2.0 + used_content_width).clamp(1.0, max_frame_width),
    };

    TemplateFrame {
        blocks,
        bubble_height,
        content_inset_x,
        frame_width,
        layout_content_width: max_content_width,
        role: template.role,
        total_height: bubble_height,
    }
}

fn layout_block_frame(
    engine: &PretextEngine,
    block: &PreparedBlock,
    content_width: f32,
    top: f32,
) -> BlockFrame {
    match block {
        PreparedBlock::Inline(block) => {
            let line_width = (content_width - block.base.content_left).max(1.0);
            let stats = measure_rich_inline_stats(engine, &block.flow, line_width);
            BlockFrame::Inline {
                base: BlockFrameBase {
                    content_left: block.base.content_left,
                    height: stats.line_count as f32 * block.line_height,
                    marker_left: block.base.marker_left,
                    marker_text: block.base.marker_text.clone(),
                    quote_rail_lefts: block.base.quote_rail_lefts.clone(),
                    top,
                },
                line_height: block.line_height,
                used_width: stats.max_line_width,
            }
        }
        PreparedBlock::Code(block) => {
            let box_width = (content_width - block.base.content_left).max(1.0);
            let inner_width = (box_width - CODE_BLOCK_PADDING_X * 2.0).max(1.0);
            let mut line_count = 0usize;
            let mut max_line_width = 0.0f32;
            engine.walk_line_ranges(&block.prepared, inner_width, |line| {
                line_count += 1;
                max_line_width = max_line_width.max(line.width);
            });
            BlockFrame::Code {
                base: BlockFrameBase {
                    content_left: block.base.content_left,
                    height: line_count as f32 * block.line_height + CODE_BLOCK_PADDING_Y * 2.0,
                    marker_left: block.base.marker_left,
                    marker_text: block.base.marker_text.clone(),
                    quote_rail_lefts: block.base.quote_rail_lefts.clone(),
                    top,
                },
                line_height: block.line_height,
                width: max_line_width + CODE_BLOCK_PADDING_X * 2.0,
            }
        }
        PreparedBlock::Rule(block) => BlockFrame::Rule {
            base: BlockFrameBase {
                content_left: block.base.content_left,
                height: block.height,
                marker_left: block.base.marker_left,
                marker_text: block.base.marker_text.clone(),
                quote_rail_lefts: block.base.quote_rail_lefts.clone(),
                top,
            },
            width: (content_width - block.base.content_left).max(1.0),
        },
    }
}

fn block_base(block: &PreparedBlock) -> &PreparedBlockBase {
    match block {
        PreparedBlock::Inline(block) => &block.base,
        PreparedBlock::Code(block) => &block.base,
        PreparedBlock::Rule(block) => &block.base,
    }
}

fn block_frame_base(frame: &BlockFrame) -> &BlockFrameBase {
    match frame {
        BlockFrame::Inline { base, .. } => base,
        BlockFrame::Code { base, .. } => base,
        BlockFrame::Rule { base, .. } => base,
    }
}

fn block_used_width(frame: &BlockFrame) -> f32 {
    match frame {
        BlockFrame::Inline {
            base, used_width, ..
        } => base.content_left + *used_width,
        BlockFrame::Code { base, width, .. } => base.content_left + *width,
        BlockFrame::Rule { base, width } => base.content_left + *width,
    }
}

fn materialize_template_blocks(
    engine: &PretextEngine,
    template: &PreparedChatTemplate,
    frame: &TemplateFrame,
) -> Vec<BlockLayout> {
    template
        .blocks
        .iter()
        .zip(&frame.blocks)
        .map(|(block, block_frame)| {
            materialize_block_layout(engine, block, block_frame, frame.layout_content_width)
        })
        .collect()
}

fn materialize_block_layout(
    engine: &PretextEngine,
    block: &PreparedBlock,
    frame: &BlockFrame,
    content_width: f32,
) -> BlockLayout {
    match (block, frame) {
        (
            PreparedBlock::Inline(block),
            BlockFrame::Inline {
                base, line_height, ..
            },
        ) => {
            let line_width = (content_width - base.content_left).max(1.0);
            let mut lines = Vec::new();
            walk_rich_inline_line_ranges(engine, &block.flow, line_width, |range| {
                let line = materialize_rich_inline_line_range(&block.flow, range);
                let fragments = line
                    .fragments
                    .into_iter()
                    .map(|fragment| {
                        let meta = block.items[fragment.item_index].clone();
                        let prepared = block
                            .flow
                            .item_prepared(fragment.item_index)
                            .expect("inline fragment should resolve to a prepared item");
                        let runs = engine.line_runs(prepared, &fragment.line);
                        let (glyph_runs, emoji_overlays) = split_builtin_emoji_glyphs(
                            &runs.visual_runs,
                            &runs.glyph_runs,
                            fragment_overlay_options(&meta.style, *line_height),
                            engine,
                        );
                        InlineFragmentLayout {
                            meta,
                            leading_gap: fragment.leading_gap,
                            text: fragment.line.text,
                            text_width: fragment.line.width,
                            glyph_runs,
                            emoji_overlays,
                        }
                    })
                    .collect();
                lines.push(InlineLineLayout {
                    fragments,
                    width: line.width,
                });
            });

            BlockLayout::Inline {
                base: base.clone(),
                line_height: *line_height,
                lines,
            }
        }
        (
            PreparedBlock::Code(block),
            BlockFrame::Code {
                base,
                line_height,
                width,
            },
        ) => {
            let inner_width =
                (content_width - base.content_left - CODE_BLOCK_PADDING_X * 2.0).max(1.0);
            BlockLayout::Code {
                base: base.clone(),
                line_height: *line_height,
                layout: block.prepared.layout(engine, inner_width, *line_height),
                width: *width,
            }
        }
        (PreparedBlock::Rule(_), BlockFrame::Rule { base, width }) => BlockLayout::Rule {
            base: base.clone(),
            width: *width,
        },
        _ => panic!("prepared block and frame should stay aligned"),
    }
}

fn find_visible_range(
    frame: &ConversationFrame,
    scroll_top: f32,
    viewport_height: f32,
    top_occlusion_height: f32,
    bottom_occlusion_height: f32,
    overscan: f32,
) -> std::ops::Range<usize> {
    if frame.messages.is_empty() {
        return 0..0;
    }

    let min_y = (scroll_top + top_occlusion_height - overscan).max(0.0);
    let max_y = (scroll_top + viewport_height - bottom_occlusion_height + overscan).max(min_y);

    let mut low = 0usize;
    let mut high = frame.messages.len();
    while low < high {
        let mid = (low + high) >> 1;
        if frame.messages[mid].bottom > min_y {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let start = low;

    low = start;
    high = frame.messages.len();
    while low < high {
        let mid = (low + high) >> 1;
        if frame.messages[mid].top >= max_y {
            high = mid;
        } else {
            low = mid + 1;
        }
    }

    start..low
}

fn get_occlusion_banner_height(viewport_height: f32) -> f32 {
    if viewport_height <= COMPACT_OCCLUSION_VIEWPORT_HEIGHT {
        COMPACT_OCCLUSION_BANNER_HEIGHT
    } else {
        OCCLUSION_BANNER_HEIGHT
    }
}

fn frame_banner_width(panel_rect: Rect, chat_width: f32) -> f32 {
    chat_width.min(panel_rect.width()).max(1.0)
}

fn centered_banner_rect(panel_rect: Rect, chat_width: f32, banner_height: f32) -> Rect {
    Rect::from_center_size(
        egui::pos2(
            panel_rect.center().x,
            panel_rect.top() + banner_height * 0.5,
        ),
        egui::vec2(
            frame_banner_width(panel_rect, chat_width),
            banner_height.max(1.0),
        ),
    )
}

fn paint_occlusion_layer(
    painter: &egui::Painter,
    panel_rect: Rect,
    chat_width: f32,
    occlusion_banner_height: f32,
    visualization_on: bool,
) {
    let top_rect = centered_banner_rect(panel_rect, chat_width, occlusion_banner_height);
    let bottom_rect = Rect::from_min_size(
        egui::pos2(
            top_rect.left(),
            panel_rect.bottom() - occlusion_banner_height,
        ),
        egui::vec2(top_rect.width(), occlusion_banner_height.max(1.0)),
    );
    let tint = if visualization_on {
        OCCLUSION_TINT_ACTIVE
    } else {
        OCCLUSION_TINT
    };
    let edge = if visualization_on {
        OCCLUSION_EDGE_ACTIVE
    } else {
        OCCLUSION_EDGE
    };
    let solid_height = (occlusion_banner_height * 0.68).clamp(1.0, occlusion_banner_height);

    painter.rect_filled(
        Rect::from_min_size(top_rect.min, egui::vec2(top_rect.width(), solid_height)),
        CornerRadius::ZERO,
        tint,
    );
    painter.rect_filled(
        Rect::from_min_size(
            egui::pos2(bottom_rect.left(), bottom_rect.bottom() - solid_height),
            egui::vec2(bottom_rect.width(), solid_height),
        ),
        CornerRadius::ZERO,
        tint,
    );
    painter.add(vertical_alpha_gradient(
        Rect::from_min_max(
            egui::pos2(top_rect.left(), top_rect.top() + solid_height),
            top_rect.max,
        ),
        tint,
        Color32::TRANSPARENT,
    ));
    painter.add(vertical_alpha_gradient(
        Rect::from_min_max(
            bottom_rect.min,
            egui::pos2(bottom_rect.right(), bottom_rect.bottom() - solid_height),
        ),
        Color32::TRANSPARENT,
        tint,
    ));
    painter.line_segment(
        [
            egui::pos2(top_rect.left(), top_rect.top() + solid_height),
            egui::pos2(top_rect.right(), top_rect.top() + solid_height),
        ],
        Stroke::new(1.0, edge),
    );
    painter.line_segment(
        [
            egui::pos2(bottom_rect.left(), bottom_rect.bottom() - solid_height),
            egui::pos2(bottom_rect.right(), bottom_rect.bottom() - solid_height),
        ],
        Stroke::new(1.0, edge),
    );
}

fn paint_virtualization_toggle(
    ui: &mut egui::Ui,
    panel_rect: Rect,
    occlusion_banner_height: f32,
    visualization_on: bool,
) -> bool {
    let compact = occlusion_banner_height < OCCLUSION_BANNER_HEIGHT;
    let button_height = if compact {
        COMPACT_VIRTUALIZATION_BUTTON_HEIGHT
    } else {
        VIRTUALIZATION_BUTTON_HEIGHT
    };
    let pad_x = if compact {
        COMPACT_VIRTUALIZATION_BUTTON_PAD_X
    } else {
        VIRTUALIZATION_BUTTON_PAD_X
    };
    let font_size = if compact {
        COMPACT_VIRTUALIZATION_BUTTON_TEXT_SIZE
    } else {
        VIRTUALIZATION_BUTTON_TEXT_SIZE
    };
    let label = if visualization_on {
        "Hide virtualization mask"
    } else {
        "Show virtualization mask"
    };
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), FontId::monospace(font_size), ACCENT);
    let button_rect = Rect::from_center_size(
        egui::pos2(
            panel_rect.center().x,
            panel_rect.top() + occlusion_banner_height * 0.5,
        ),
        egui::vec2(
            (galley.size().x + pad_x * 2.0).max(140.0),
            button_height.max(1.0),
        ),
    );
    ui.put(
        button_rect,
        Button::new(
            RichText::new(label)
                .monospace()
                .size(font_size)
                .color(ACCENT)
                .strong(),
        )
        .fill(if visualization_on {
            VIRTUALIZATION_BUTTON_FILL_ACTIVE
        } else {
            VIRTUALIZATION_BUTTON_FILL
        })
        .stroke(Stroke::new(1.0, VIRTUALIZATION_BUTTON_STROKE))
        .corner_radius(CornerRadius::same(255)),
    )
    .clicked()
}

fn vertical_alpha_gradient(rect: Rect, top: Color32, bottom: Color32) -> Shape {
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: egui::epaint::WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: egui::epaint::WHITE_UV,
        color: top,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: bottom,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: bottom,
    });
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    Shape::mesh(mesh)
}

fn paint_message(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    origin: egui::Pos2,
    lane_left: f32,
    chat_width: f32,
    message_index: usize,
    message_top: f32,
    frame: &TemplateFrame,
    blocks: &[BlockLayout],
) {
    let painter = ui.painter();
    let row_left = origin.x + lane_left + MESSAGE_SIDE_PADDING;
    let row_width = (chat_width - MESSAGE_SIDE_PADDING * 2.0).max(1.0);
    let bubble_x = match frame.role {
        ChatRole::Assistant => row_left,
        ChatRole::User => row_left + row_width - frame.frame_width,
    };
    let bubble_rect = Rect::from_min_size(
        egui::pos2(bubble_x, origin.y + message_top),
        egui::vec2(frame.frame_width, frame.bubble_height),
    );

    if frame.role == ChatRole::User {
        painter.rect_filled(bubble_rect, CornerRadius::same(16), USER_BUBBLE_FILL);
        painter.rect_stroke(
            bubble_rect,
            CornerRadius::same(16),
            Stroke::new(1.0, USER_BUBBLE_STROKE),
            StrokeKind::Inside,
        );
    }

    let content_left = bubble_rect.left() + frame.content_inset_x;
    let content_top = bubble_rect.top();

    for (block_index, block) in blocks.iter().enumerate() {
        paint_block(
            ui,
            ctx,
            engine,
            assets,
            block,
            message_index,
            block_index,
            egui::pos2(content_left, content_top),
            frame.role,
        );
    }
}

fn paint_block(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    block: &BlockLayout,
    message_index: usize,
    block_index: usize,
    content_origin: egui::Pos2,
    role: ChatRole,
) {
    let painter = ui.painter();
    let base = match block {
        BlockLayout::Inline { base, .. } => base,
        BlockLayout::Code { base, .. } => base,
        BlockLayout::Rule { base, .. } => base,
    };

    for rail_left in &base.quote_rail_lefts {
        let rail_rect = Rect::from_min_size(
            egui::pos2(content_origin.x + rail_left, content_origin.y + base.top),
            egui::vec2(3.0, base.height.max(1.0)),
        );
        painter.rect_filled(rail_rect, CornerRadius::same(2), QUOTE_RAIL);
    }

    if let (Some(marker_text), Some(marker_left)) = (&base.marker_text, base.marker_left) {
        painter.text(
            egui::pos2(
                content_origin.x + marker_left,
                content_origin.y + base.top + marker_top(block),
            ),
            Align2::LEFT_TOP,
            marker_text,
            FontId::monospace(MARKER_TEXT_SIZE),
            MUTED,
        );
    }

    match block {
        BlockLayout::Inline {
            base,
            line_height,
            lines,
        } => {
            for (line_index, line) in lines.iter().enumerate() {
                let mut x = content_origin.x + base.content_left;
                let y = content_origin.y + base.top + line_index as f32 * line_height;
                for (fragment_index, fragment) in line.fragments.iter().enumerate() {
                    x += fragment.leading_gap;
                    paint_inline_fragment(
                        ui,
                        ctx,
                        engine,
                        assets,
                        ui.make_persistent_id((
                            "markdown_chat_link",
                            message_index,
                            block_index,
                            line_index,
                            fragment_index,
                        )),
                        x,
                        y,
                        *line_height,
                        fragment,
                    );
                    x += fragment.text_width + fragment.meta.chrome_width;
                }
                let _ = line.width;
            }
        }
        BlockLayout::Code {
            base,
            line_height,
            layout,
            width,
        } => {
            let box_rect = Rect::from_min_size(
                egui::pos2(
                    content_origin.x + base.content_left,
                    content_origin.y + base.top,
                ),
                egui::vec2(*width, base.height),
            );
            painter.rect_filled(box_rect, CornerRadius::same(12), CODE_BLOCK_FILL);

            for (line_index, line) in layout.lines.iter().enumerate() {
                let y = content_origin.y
                    + base.top
                    + CODE_BLOCK_PADDING_Y
                    + line_index as f32 * line_height;
                let (glyph_runs, emoji_overlays) = split_builtin_emoji_glyphs(
                    &line.runs.visual_runs,
                    &line.runs.glyph_runs,
                    fragment_overlay_options(code_text_style(), *line_height),
                    engine,
                );
                let _ = paint_styled_positioned_text_runs(
                    painter,
                    std::iter::once(StyledPositionedTextRunRef {
                        x: content_origin.x + base.content_left + CODE_BLOCK_PADDING_X,
                        y,
                        text: &line.line.text,
                        glyph_runs: &glyph_runs,
                        emoji_overlays: &emoji_overlays,
                        options: EguiPretextPaintOptions::new(code_text_style(), *line_height)
                            .color(match role {
                                ChatRole::Assistant => INK,
                                ChatRole::User => Color32::WHITE,
                            })
                            .fallback_font(FontId::new(CODE_TEXT_SIZE, FontFamily::Monospace))
                            .fallback_align(Align2::LEFT_TOP)
                            .emoji_size(CODE_TEXT_SIZE)
                            .emoji_slot_height(*line_height),
                    }),
                    ctx,
                    engine,
                    assets,
                );
            }
        }
        BlockLayout::Rule { base, width } => {
            let y = content_origin.y + base.top + base.height * 0.5;
            painter.line_segment(
                [
                    egui::pos2(content_origin.x + base.content_left, y),
                    egui::pos2(content_origin.x + base.content_left + width, y),
                ],
                Stroke::new(1.0, PANEL_RULE),
            );
        }
    }
}

fn marker_top(block: &BlockLayout) -> f32 {
    match block {
        BlockLayout::Code { .. } => CODE_BLOCK_PADDING_Y,
        BlockLayout::Inline { line_height, .. } => ((line_height - 12.0) * 0.5).round().max(0.0),
        BlockLayout::Rule { .. } => 0.0,
    }
}

fn paint_inline_fragment(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    interaction_id: egui::Id,
    x: f32,
    y: f32,
    line_height: f32,
    fragment: &InlineFragmentLayout,
) {
    let painter = ui.painter();
    let chrome_width = fragment.meta.chrome_width;
    match fragment.meta.kind {
        InlinePieceKind::Text => {
            if fragment.meta.href.is_some() {
                painter.line_segment(
                    [
                        egui::pos2(x, y + LINK_UNDERLINE_Y),
                        egui::pos2(x + fragment.text_width, y + LINK_UNDERLINE_Y),
                    ],
                    Stroke::new(1.0, ACCENT),
                );
            }
            if fragment.meta.strike {
                painter.line_segment(
                    [
                        egui::pos2(x, y + line_height * 0.5),
                        egui::pos2(x + fragment.text_width, y + line_height * 0.5),
                    ],
                    Stroke::new(1.0, fragment_color(fragment)),
                );
            }
            paint_text_run(
                painter,
                ctx,
                engine,
                assets,
                x,
                y,
                line_height,
                &fragment.text,
                &fragment.glyph_runs,
                &fragment.emoji_overlays,
                &fragment.meta.style,
                fragment_color(fragment),
                FontFamily::Proportional,
            );
        }
        InlinePieceKind::Code => {
            let box_rect = Rect::from_min_size(
                egui::pos2(x, y + INLINE_CHROME_Y),
                egui::vec2(fragment.text_width + chrome_width, INLINE_CODE_BOX_HEIGHT),
            );
            painter.rect_filled(box_rect, CornerRadius::same(8), INLINE_CODE_FILL);
            paint_text_run(
                painter,
                ctx,
                engine,
                assets,
                x + chrome_width * 0.5,
                y,
                line_height,
                &fragment.text,
                &fragment.glyph_runs,
                &fragment.emoji_overlays,
                &fragment.meta.style,
                INK,
                FontFamily::Monospace,
            );
        }
        InlinePieceKind::ImageChip => {
            let box_rect = Rect::from_min_size(
                egui::pos2(x, y + INLINE_CHROME_Y),
                egui::vec2(fragment.text_width + chrome_width, IMAGE_CHIP_BOX_HEIGHT),
            );
            painter.rect_filled(box_rect, CornerRadius::same(9), IMAGE_CHIP_FILL);
            painter.rect_stroke(
                box_rect,
                CornerRadius::same(9),
                Stroke::new(1.0, IMAGE_CHIP_STROKE),
                StrokeKind::Inside,
            );
            paint_text_run(
                painter,
                ctx,
                engine,
                assets,
                x + chrome_width * 0.5,
                y,
                line_height,
                &fragment.text,
                &fragment.glyph_runs,
                &fragment.emoji_overlays,
                &fragment.meta.style,
                INK,
                FontFamily::Proportional,
            );
        }
    }

    if let Some(href) = &fragment.meta.href {
        let response = ui
            .interact(
                fragment_hit_rect(x, y, line_height, fragment),
                interaction_id,
                Sense::click(),
            )
            .on_hover_text(href);
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if response.clicked_with_open_in_background() || response.clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(href));
        }
    }
}

fn fragment_hit_rect(x: f32, y: f32, line_height: f32, fragment: &InlineFragmentLayout) -> Rect {
    let width = fragment.text_width
        + match fragment.meta.kind {
            InlinePieceKind::Text => 0.0,
            InlinePieceKind::Code | InlinePieceKind::ImageChip => fragment.meta.chrome_width,
        };
    Rect::from_min_size(
        egui::pos2(x, y),
        egui::vec2(width.max(1.0), line_height.max(1.0)),
    )
}

fn paint_text_run(
    painter: &egui::Painter,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    x: f32,
    y: f32,
    line_height: f32,
    text: &str,
    glyph_runs: &[PretextGlyphRun],
    emoji_overlays: &[EmojiOverlayRun],
    style: &TextStyleSpec,
    color: Color32,
    fallback_family: FontFamily,
) {
    let _ = paint_styled_positioned_text_runs(
        painter,
        std::iter::once(StyledPositionedTextRunRef {
            x,
            y,
            text,
            glyph_runs,
            emoji_overlays,
            options: EguiPretextPaintOptions::new(style, line_height)
                .color(color)
                .fallback_font(FontId::new(style.size_px, fallback_family))
                .fallback_align(Align2::LEFT_TOP)
                .emoji_size(style.size_px)
                .emoji_slot_height(line_height),
        }),
        ctx,
        engine,
        assets,
    );
}

fn fragment_color(fragment: &InlineFragmentLayout) -> Color32 {
    if fragment.meta.href.is_some() {
        ACCENT
    } else {
        INK
    }
}

fn fragment_overlay_options(style: &TextStyleSpec, line_height: f32) -> EmojiOverlayOptions<'_> {
    EmojiOverlayOptions {
        style,
        slot_height: line_height,
        padding_x: SHAPED_TEXT_PAD_X,
        padding_y: SHAPED_TEXT_PAD_Y,
        slack_x: 2.0,
        slack_y: 4.0,
        baseline_mode: BaselineMode::AutoFontMetrics,
    }
}

fn prepare_markdown_blocks(engine: &PretextEngine, markdown: &str) -> Vec<PreparedBlock> {
    build_blocks_from_nodes(engine, &parse_markdown(markdown), ParseContext::default())
}

fn parse_markdown(markdown: &str) -> Vec<BlockNode> {
    let parser = Parser::new_ext(
        markdown,
        Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS,
    );
    let mut events = parser.peekable();
    parse_block_nodes(&mut events)
}

fn parse_block_nodes<'a, I>(events: &mut Peekable<I>) -> Vec<BlockNode>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut blocks = Vec::new();

    while let Some(event) = events.peek() {
        if is_block_boundary(event) {
            break;
        }

        let Some(event) = events.next() else {
            break;
        };
        match event {
            Event::Start(Tag::Paragraph) => {
                blocks.push(BlockNode::Paragraph(parse_inline_nodes(
                    events,
                    InlineEnd::Paragraph,
                )));
            }
            Event::Start(Tag::Heading { level, .. }) => {
                blocks.push(BlockNode::Heading {
                    level: heading_level_to_u8(level),
                    content: parse_inline_nodes(events, InlineEnd::Heading),
                });
            }
            Event::Start(Tag::CodeBlock(_kind)) => {
                blocks.push(BlockNode::CodeBlock(collect_code_block_text(events)));
            }
            Event::Start(Tag::Table(_)) => {
                let text = collect_table_block_text(events);
                if !text.is_empty() {
                    blocks.push(BlockNode::CodeBlock(text));
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let children = parse_block_nodes(events);
                consume_block_end(events, BlockEnd::BlockQuote);
                blocks.push(BlockNode::BlockQuote(children));
            }
            Event::Start(Tag::List(start)) => {
                blocks.push(parse_list(events, start));
            }
            Event::Rule => blocks.push(BlockNode::Rule),
            Event::Html(text) => {
                blocks.push(BlockNode::CodeBlock(strip_single_trailing_newline(
                    &text.into_string(),
                )));
            }
            event if starts_implicit_paragraph(&event) => blocks.push(BlockNode::Paragraph(
                parse_inline_nodes_until_block(events, event),
            )),
            _ => {}
        }
    }

    blocks
}

#[derive(Clone, Copy)]
enum InlineEnd {
    Paragraph,
    Heading,
    Strong,
    Emphasis,
    Strike,
    Link,
    Image,
    TableCell,
}

fn parse_inline_nodes<'a, I>(events: &mut Peekable<I>, end: InlineEnd) -> Vec<InlineNode>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut nodes = Vec::new();

    while let Some(event) = events.next() {
        if matches_inline_end(&event, end) {
            break;
        }
        push_inline_node(&mut nodes, events, event);
    }

    nodes
}

fn parse_inline_nodes_until_block<'a, I>(
    events: &mut Peekable<I>,
    first_event: Event<'a>,
) -> Vec<InlineNode>
where
    I: Iterator<Item = Event<'a>>,
{
    let mut nodes = Vec::new();
    push_inline_node(&mut nodes, events, first_event);

    while let Some(event) = events.peek() {
        if is_block_boundary(event) || starts_block(event) {
            break;
        }
        let Some(event) = events.next() else {
            break;
        };
        push_inline_node(&mut nodes, events, event);
    }

    nodes
}

fn push_inline_node<'a, I>(nodes: &mut Vec<InlineNode>, events: &mut Peekable<I>, event: Event<'a>)
where
    I: Iterator<Item = Event<'a>>,
{
    match event {
        Event::Text(text) => nodes.push(InlineNode::Text(text.into_string())),
        Event::Code(text) => nodes.push(InlineNode::Code(text.into_string())),
        Event::Html(text) | Event::InlineHtml(text) => {
            nodes.push(InlineNode::Html(text.into_string()))
        }
        Event::SoftBreak => nodes.push(InlineNode::SoftBreak),
        Event::HardBreak => nodes.push(InlineNode::HardBreak),
        Event::Start(Tag::Strong) => {
            nodes.push(InlineNode::Strong(parse_inline_nodes(
                events,
                InlineEnd::Strong,
            )));
        }
        Event::Start(Tag::Emphasis) => {
            nodes.push(InlineNode::Emphasis(parse_inline_nodes(
                events,
                InlineEnd::Emphasis,
            )));
        }
        Event::Start(Tag::Strikethrough) => {
            nodes.push(InlineNode::Strike(parse_inline_nodes(
                events,
                InlineEnd::Strike,
            )));
        }
        Event::Start(Tag::Link { dest_url, .. }) => {
            nodes.push(InlineNode::Link {
                href: dest_url.into_string(),
                children: parse_inline_nodes(events, InlineEnd::Link),
            });
        }
        Event::Start(Tag::Image { dest_url, .. }) => {
            let alt = inline_nodes_to_plain_text(&parse_inline_nodes(events, InlineEnd::Image));
            nodes.push(InlineNode::Image {
                alt: if alt.is_empty() {
                    dest_url.into_string()
                } else {
                    alt
                },
            });
        }
        Event::TaskListMarker(checked) => {
            nodes.push(InlineNode::Text(if checked {
                "[x] ".to_owned()
            } else {
                "[ ] ".to_owned()
            }));
        }
        _ => {}
    }
}

#[derive(Clone, Copy)]
enum BlockEnd {
    BlockQuote,
    List,
    Item,
}

fn parse_list<'a, I>(events: &mut Peekable<I>, start: Option<u64>) -> BlockNode
where
    I: Iterator<Item = Event<'a>>,
{
    let mut items = Vec::new();

    loop {
        match events.peek() {
            Some(Event::Start(Tag::Item)) => {
                let _ = events.next();
                let task_marker = match events.peek() {
                    Some(Event::TaskListMarker(checked)) => {
                        let checked = *checked;
                        let _ = events.next();
                        Some(checked)
                    }
                    _ => None,
                };
                let blocks = parse_block_nodes(events);
                consume_block_end(events, BlockEnd::Item);
                items.push(ListItemNode {
                    task_marker,
                    blocks,
                });
            }
            Some(Event::End(_)) => {
                consume_block_end(events, BlockEnd::List);
                break;
            }
            Some(_) => {
                let blocks = parse_block_nodes(events);
                if !blocks.is_empty() {
                    items.push(ListItemNode {
                        task_marker: None,
                        blocks,
                    });
                } else {
                    let _ = events.next();
                }
            }
            None => break,
        }
    }

    BlockNode::List {
        ordered: start.is_some(),
        start: start.unwrap_or(1) as usize,
        items,
    }
}

fn collect_code_block_text<'a, I>(events: &mut Peekable<I>) -> String
where
    I: Iterator<Item = Event<'a>>,
{
    let mut text = String::new();
    while let Some(event) = events.next() {
        match event {
            Event::End(_) => break,
            Event::Text(chunk)
            | Event::Code(chunk)
            | Event::Html(chunk)
            | Event::InlineHtml(chunk) => {
                text.push_str(&chunk);
            }
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            _ => {}
        }
    }
    strip_single_trailing_newline(&text)
}

fn collect_table_block_text<'a, I>(events: &mut Peekable<I>) -> String
where
    I: Iterator<Item = Event<'a>>,
{
    let mut header = Vec::<String>::new();
    let mut rows = Vec::<Vec<String>>::new();
    let mut current_row = Vec::<String>::new();
    let mut in_head = false;

    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::TableHead) => in_head = true,
            Event::End(TagEnd::TableHead) => {
                if header.is_empty() && !current_row.is_empty() {
                    header = std::mem::take(&mut current_row);
                }
                in_head = false;
            }
            Event::Start(Tag::TableRow) => current_row.clear(),
            Event::End(TagEnd::TableRow) => {
                if in_head && header.is_empty() {
                    header = std::mem::take(&mut current_row);
                } else if !current_row.is_empty() {
                    rows.push(std::mem::take(&mut current_row));
                }
            }
            Event::Start(Tag::TableCell) => {
                let cell =
                    inline_nodes_to_plain_text(&parse_inline_nodes(events, InlineEnd::TableCell));
                current_row.push(cell);
            }
            Event::End(TagEnd::Table) => {
                if header.is_empty() && !current_row.is_empty() {
                    header = std::mem::take(&mut current_row);
                } else if !current_row.is_empty() {
                    rows.push(std::mem::take(&mut current_row));
                }
                break;
            }
            _ => {}
        }
    }

    if header.is_empty() && !rows.is_empty() {
        header = rows.remove(0);
    }
    if header.is_empty() {
        return String::new();
    }

    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(header.join(" | "));
    lines.push(vec!["---"; header.len()].join(" | "));
    for row in rows {
        lines.push(row.join(" | "));
    }
    lines.join("\n")
}

fn inline_nodes_to_plain_text(nodes: &[InlineNode]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            InlineNode::Text(value) | InlineNode::Html(value) | InlineNode::Code(value) => {
                text.push_str(value);
            }
            InlineNode::SoftBreak => text.push(' '),
            InlineNode::HardBreak => text.push('\n'),
            InlineNode::Strong(children)
            | InlineNode::Emphasis(children)
            | InlineNode::Strike(children) => {
                text.push_str(&inline_nodes_to_plain_text(children));
            }
            InlineNode::Link { children, .. } => {
                text.push_str(&inline_nodes_to_plain_text(children));
            }
            InlineNode::Image { alt } => {
                text.push_str(alt);
            }
        }
    }
    text
}

fn build_blocks_from_nodes(
    engine: &PretextEngine,
    nodes: &[BlockNode],
    ctx: ParseContext,
) -> Vec<PreparedBlock> {
    let mut blocks = Vec::new();
    for node in nodes {
        match node {
            BlockNode::Paragraph(content) => append_block_group(
                &mut blocks,
                build_inline_blocks(engine, content, InlineVariant::Body, ctx),
                BLOCK_GAP,
            ),
            BlockNode::Heading { level, content } => append_block_group(
                &mut blocks,
                build_inline_blocks(engine, content, heading_variant(*level), ctx),
                BLOCK_GAP + 4.0,
            ),
            BlockNode::CodeBlock(text) => append_block_group(
                &mut blocks,
                vec![PreparedBlock::Code(build_code_block(engine, text, ctx))],
                RICH_BLOCK_GAP,
            ),
            BlockNode::BlockQuote(children) => append_block_group(
                &mut blocks,
                build_blocks_from_nodes(
                    engine,
                    children,
                    ParseContext {
                        list_depth: ctx.list_depth,
                        quote_depth: ctx.quote_depth + 1,
                    },
                ),
                RICH_BLOCK_GAP,
            ),
            BlockNode::List {
                ordered,
                start,
                items,
            } => append_block_group(
                &mut blocks,
                build_list_blocks(engine, *ordered, *start, items, ctx),
                BLOCK_GAP,
            ),
            BlockNode::Rule => append_block_group(
                &mut blocks,
                vec![PreparedBlock::Rule(build_rule_block(ctx))],
                BLOCK_GAP + 2.0,
            ),
        }
    }
    blocks
}

fn build_list_blocks(
    engine: &PretextEngine,
    ordered: bool,
    start: usize,
    items: &[ListItemNode],
    ctx: ParseContext,
) -> Vec<PreparedBlock> {
    let mut blocks = Vec::new();
    let item_ctx = ParseContext {
        list_depth: ctx.list_depth + 1,
        quote_depth: ctx.quote_depth,
    };

    for (index, item) in items.iter().enumerate() {
        let mut item_blocks = build_blocks_from_nodes(engine, &item.blocks, item_ctx);
        if item_blocks.is_empty() {
            let text = match item.task_marker {
                Some(true) => "[x]",
                Some(false) => "[ ]",
                None => "",
            };
            item_blocks = build_plain_text_blocks(engine, text, InlineVariant::Body, item_ctx);
        }
        decorate_list_item_blocks(
            engine,
            &mut item_blocks,
            resolve_list_marker_text(ordered, start, index, item.task_marker),
        );
        append_block_group(&mut blocks, item_blocks, LIST_ITEM_GAP);
    }

    blocks
}

fn decorate_list_item_blocks(
    engine: &PretextEngine,
    blocks: &mut [PreparedBlock],
    marker_text: String,
) {
    if blocks.is_empty() {
        return;
    }

    let marker_area = measure_marker_width(engine, &marker_text) + LIST_MARKER_GAP;
    for block in blocks.iter_mut() {
        shift_block(block, marker_area);
    }

    let first = block_base_mut(&mut blocks[0]);
    first.marker_left = Some(first.content_left - marker_area);
    first.marker_text = Some(marker_text);
}

fn build_plain_text_blocks(
    engine: &PretextEngine,
    text: &str,
    variant: InlineVariant,
    ctx: ParseContext,
) -> Vec<PreparedBlock> {
    let Some(piece) = create_text_piece(text, MarkState::default(), variant, None) else {
        return Vec::new();
    };
    build_prepared_inline_blocks(engine, vec![vec![piece]], variant, ctx)
}

fn build_inline_blocks(
    engine: &PretextEngine,
    content: &[InlineNode],
    variant: InlineVariant,
    ctx: ParseContext,
) -> Vec<PreparedBlock> {
    build_prepared_inline_blocks(
        engine,
        collect_inline_piece_lines(content, variant),
        variant,
        ctx,
    )
}

fn build_prepared_inline_blocks(
    engine: &PretextEngine,
    lines: Vec<Vec<InlinePieceSpec>>,
    variant: InlineVariant,
    ctx: ParseContext,
) -> Vec<PreparedBlock> {
    let mut blocks = Vec::new();
    for pieces in lines {
        let Some(block) = build_prepared_inline_block(engine, pieces, variant, ctx) else {
            continue;
        };
        let margin_top = if blocks.is_empty() {
            0.0
        } else {
            HARD_BREAK_GAP
        };
        let mut base = block.base.clone();
        base.margin_top = margin_top;
        blocks.push(PreparedBlock::Inline(PreparedInlineBlock { base, ..block }));
    }
    blocks
}

fn build_prepared_inline_block(
    engine: &PretextEngine,
    pieces: Vec<InlinePieceSpec>,
    variant: InlineVariant,
    ctx: ParseContext,
) -> Option<PreparedInlineBlock> {
    if pieces.is_empty() {
        return None;
    }

    let mut flow_items = Vec::with_capacity(pieces.len());
    let items = pieces
        .iter()
        .map(|piece| piece.meta.clone())
        .collect::<Vec<_>>();
    for piece in &pieces {
        flow_items.push(RichInlineItemSpec {
            text: &piece.text,
            style: &piece.meta.style,
            break_mode: piece.break_mode,
            extra_width: piece.meta.chrome_width,
        });
    }

    Some(PreparedInlineBlock {
        base: create_block_base(ctx),
        items,
        flow: prepare_rich_inline(engine, &flow_items),
        line_height: line_height_for_variant(variant),
    })
}

fn build_code_block(engine: &PretextEngine, text: &str, ctx: ParseContext) -> PreparedCodeBlock {
    PreparedCodeBlock {
        base: create_block_base(ctx),
        line_height: CODE_LINE_HEIGHT,
        prepared: engine.prepare_paragraph(text, code_text_style(), &pre_wrap_options()),
    }
}

fn build_rule_block(ctx: ParseContext) -> PreparedRuleBlock {
    PreparedRuleBlock {
        base: create_block_base(ctx),
        height: RULE_HEIGHT,
    }
}

fn create_block_base(ctx: ParseContext) -> PreparedBlockBase {
    let list_indent = ctx.list_depth.saturating_sub(1) as f32 * LIST_NESTING_INDENT;
    let content_left = list_indent + ctx.quote_depth as f32 * BLOCKQUOTE_INDENT;
    let mut quote_rail_lefts = Vec::with_capacity(ctx.quote_depth);
    for depth in 0..ctx.quote_depth {
        quote_rail_lefts.push(list_indent + depth as f32 * BLOCKQUOTE_INDENT + RAIL_OFFSET);
    }

    PreparedBlockBase {
        content_left,
        margin_top: 0.0,
        marker_left: None,
        marker_text: None,
        quote_rail_lefts,
    }
}

fn append_block_group(
    target: &mut Vec<PreparedBlock>,
    mut group: Vec<PreparedBlock>,
    first_margin: f32,
) {
    if group.is_empty() {
        return;
    }

    for (index, block) in group.iter_mut().enumerate() {
        block_base_mut(block).margin_top = if index == 0 {
            if target.is_empty() {
                0.0
            } else {
                first_margin
            }
        } else {
            block_base(block).margin_top
        };
    }

    target.extend(group);
}

fn shift_block(block: &mut PreparedBlock, delta: f32) {
    block_base_mut(block).content_left += delta;
}

fn block_base_mut(block: &mut PreparedBlock) -> &mut PreparedBlockBase {
    match block {
        PreparedBlock::Inline(block) => &mut block.base,
        PreparedBlock::Code(block) => &mut block.base,
        PreparedBlock::Rule(block) => &mut block.base,
    }
}

fn collect_inline_piece_lines(
    content: &[InlineNode],
    variant: InlineVariant,
) -> Vec<Vec<InlinePieceSpec>> {
    let mut lines = vec![Vec::new()];
    collect_inline_pieces_into(content, variant, MarkState::default(), &mut lines);
    while lines.last().is_some_and(Vec::is_empty) {
        lines.pop();
    }
    lines
}

fn collect_inline_pieces_into(
    nodes: &[InlineNode],
    variant: InlineVariant,
    marks: MarkState,
    lines: &mut Vec<Vec<InlinePieceSpec>>,
) {
    for node in nodes {
        match node {
            InlineNode::Text(text) | InlineNode::Html(text) => {
                push_piece(lines, create_text_piece(text, marks, variant, None));
            }
            InlineNode::Code(text) => {
                push_piece(lines, create_code_piece(text));
            }
            InlineNode::Image { alt } => {
                push_piece(lines, Some(create_image_piece(alt)));
            }
            InlineNode::SoftBreak => {
                push_piece(lines, create_text_piece(" ", marks, variant, None));
            }
            InlineNode::HardBreak => lines.push(Vec::new()),
            InlineNode::Strong(children) => {
                collect_inline_pieces_into(
                    children,
                    variant,
                    MarkState {
                        bold: true,
                        ..marks
                    },
                    lines,
                );
            }
            InlineNode::Emphasis(children) => {
                collect_inline_pieces_into(
                    children,
                    variant,
                    MarkState {
                        italic: true,
                        ..marks
                    },
                    lines,
                );
            }
            InlineNode::Strike(children) => {
                collect_inline_pieces_into(
                    children,
                    variant,
                    MarkState {
                        strike: true,
                        ..marks
                    },
                    lines,
                );
            }
            InlineNode::Link { href, children } => {
                collect_inline_pieces_into(
                    children,
                    variant,
                    MarkState {
                        link: true,
                        ..marks
                    },
                    lines,
                );
                for piece in lines.last_mut().into_iter().flatten().rev() {
                    if piece.meta.href.is_some() {
                        break;
                    }
                    piece.meta.href = Some(href.clone());
                    if piece.meta.kind != InlinePieceKind::Text {
                        break;
                    }
                }
            }
        }
    }
}

fn push_piece(lines: &mut Vec<Vec<InlinePieceSpec>>, piece: Option<InlinePieceSpec>) {
    let Some(piece) = piece else {
        return;
    };

    let line = lines
        .last_mut()
        .expect("inline piece lines should never be empty");
    if let Some(previous) = line.last_mut() {
        if can_merge_inline_pieces(previous, &piece) {
            previous.text.push_str(&piece.text);
            return;
        }
    }
    line.push(piece);
}

fn create_text_piece(
    text: &str,
    marks: MarkState,
    variant: InlineVariant,
    href: Option<String>,
) -> Option<InlinePieceSpec> {
    if text.is_empty() {
        return None;
    }

    Some(InlinePieceSpec {
        text: text.to_owned(),
        meta: InlinePieceMeta {
            kind: InlinePieceKind::Text,
            style: resolve_text_style(variant, marks),
            href,
            strike: marks.strike,
            chrome_width: 0.0,
        },
        break_mode: RichInlineBreakMode::Normal,
    })
}

fn create_code_piece(text: &str) -> Option<InlinePieceSpec> {
    if text.is_empty() {
        return None;
    }

    Some(InlinePieceSpec {
        text: text.to_owned(),
        meta: InlinePieceMeta {
            kind: InlinePieceKind::Code,
            style: code_text_style().clone(),
            href: None,
            strike: false,
            chrome_width: INLINE_CODE_CHROME_WIDTH,
        },
        break_mode: RichInlineBreakMode::Normal,
    })
}

fn create_image_piece(text: &str) -> InlinePieceSpec {
    InlinePieceSpec {
        text: if text.is_empty() {
            "image".to_owned()
        } else {
            text.to_owned()
        },
        meta: InlinePieceMeta {
            kind: InlinePieceKind::ImageChip,
            style: image_text_style().clone(),
            href: None,
            strike: false,
            chrome_width: IMAGE_CHROME_WIDTH,
        },
        break_mode: RichInlineBreakMode::Never,
    }
}

fn can_merge_inline_pieces(a: &InlinePieceSpec, b: &InlinePieceSpec) -> bool {
    a.break_mode == b.break_mode
        && a.meta.kind == b.meta.kind
        && a.meta.href == b.meta.href
        && a.meta.strike == b.meta.strike
        && a.meta.chrome_width.to_bits() == b.meta.chrome_width.to_bits()
        && styles_equal(&a.meta.style, &b.meta.style)
}

fn styles_equal(a: &TextStyleSpec, b: &TextStyleSpec) -> bool {
    a.families == b.families
        && a.size_px.to_bits() == b.size_px.to_bits()
        && a.weight == b.weight
        && a.italic == b.italic
}

fn resolve_text_style(variant: InlineVariant, marks: MarkState) -> TextStyleSpec {
    let (families, size_px, weight) = match variant {
        InlineVariant::HeadingOne => (
            serif_families(),
            HEADING_ONE_TEXT_SIZE,
            if marks.bold { 800 } else { 700 },
        ),
        InlineVariant::HeadingTwo => (
            serif_families(),
            HEADING_TWO_TEXT_SIZE,
            if marks.bold { 800 } else { 700 },
        ),
        InlineVariant::Body => (
            sans_families(),
            BODY_TEXT_SIZE,
            if marks.bold {
                700
            } else if marks.link {
                500
            } else {
                400
            },
        ),
    };

    build_text_style(families, size_px, weight, marks.italic)
}

fn heading_variant(level: u8) -> InlineVariant {
    match level {
        0 | 1 => InlineVariant::HeadingOne,
        2 => InlineVariant::HeadingTwo,
        _ => InlineVariant::Body,
    }
}

fn line_height_for_variant(variant: InlineVariant) -> f32 {
    match variant {
        InlineVariant::HeadingOne => HEADING_ONE_LINE_HEIGHT,
        InlineVariant::HeadingTwo => HEADING_TWO_LINE_HEIGHT,
        InlineVariant::Body => BODY_LINE_HEIGHT,
    }
}

fn resolve_list_marker_text(
    ordered: bool,
    start: usize,
    index: usize,
    task_marker: Option<bool>,
) -> String {
    match task_marker {
        Some(true) => "☑".to_owned(),
        Some(false) => "☐".to_owned(),
        None if ordered => format!("{}.", start + index),
        None => "•".to_owned(),
    }
}

fn measure_marker_width(engine: &PretextEngine, text: &str) -> f32 {
    let prepared = engine.prepare_paragraph(text, marker_text_style(), &normal_options());
    let mut max_width = 0.0f32;
    engine.walk_line_ranges(&prepared, UNBOUNDED_WIDTH, |line| {
        max_width = max_width.max(line.width);
    });
    max_width
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn matches_inline_end(event: &Event<'_>, end: InlineEnd) -> bool {
    match (event, end) {
        (Event::End(TagEnd::Paragraph), InlineEnd::Paragraph) => true,
        (Event::End(TagEnd::Heading(_)), InlineEnd::Heading) => true,
        (Event::End(TagEnd::Strong), InlineEnd::Strong) => true,
        (Event::End(TagEnd::Emphasis), InlineEnd::Emphasis) => true,
        (Event::End(TagEnd::Strikethrough), InlineEnd::Strike) => true,
        (Event::End(TagEnd::Link), InlineEnd::Link) => true,
        (Event::End(TagEnd::Image), InlineEnd::Image) => true,
        (Event::End(TagEnd::TableCell), InlineEnd::TableCell) => true,
        _ => false,
    }
}

fn is_block_boundary(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::End(TagEnd::BlockQuote(_))
            | Event::End(TagEnd::List(_))
            | Event::End(TagEnd::Item)
            | Event::End(TagEnd::CodeBlock)
    )
}

fn starts_block(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::Start(Tag::Paragraph)
            | Event::Start(Tag::Heading { .. })
            | Event::Start(Tag::CodeBlock(_))
            | Event::Start(Tag::Table(_))
            | Event::Start(Tag::BlockQuote(_))
            | Event::Start(Tag::List(_))
            | Event::Start(Tag::Item)
            | Event::Rule
            | Event::Html(_)
    )
}

fn starts_implicit_paragraph(event: &Event<'_>) -> bool {
    matches!(
        event,
        Event::Text(_)
            | Event::Code(_)
            | Event::InlineHtml(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Start(Tag::Strong)
            | Event::Start(Tag::Emphasis)
            | Event::Start(Tag::Strikethrough)
            | Event::Start(Tag::Link { .. })
            | Event::Start(Tag::Image { .. })
            | Event::TaskListMarker(_)
    )
}

fn consume_block_end<'a, I>(events: &mut Peekable<I>, expected: BlockEnd)
where
    I: Iterator<Item = Event<'a>>,
{
    if let Some(event) = events.next() {
        let matches = match (event, expected) {
            (Event::End(TagEnd::BlockQuote(_)), BlockEnd::BlockQuote) => true,
            (Event::End(TagEnd::List(_)), BlockEnd::List) => true,
            (Event::End(TagEnd::Item), BlockEnd::Item) => true,
            _ => false,
        };
        assert!(
            matches,
            "markdown parser ended in an unexpected block state"
        );
    }
}

fn strip_single_trailing_newline(text: &str) -> String {
    text.strip_suffix('\n').unwrap_or(text).to_owned()
}

fn quantize(value: f32) -> u32 {
    (value.max(0.0) * 4.0).round() as u32
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
    }
}

fn pre_wrap_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::PreWrap,
        word_break: WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
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

fn sans_families() -> &'static [&'static str] {
    &[
        "Helvetica Neue",
        "Helvetica",
        "Arial",
        "Noto Sans",
        "Noto Sans Arabic",
        "Noto Sans CJK",
        "Noto Emoji",
        "Noto Color Emoji",
    ]
}

fn serif_families() -> &'static [&'static str] {
    &[
        "Noto Serif",
        "Iowan Old Style",
        "Georgia",
        "Times New Roman",
        "Noto Sans Arabic",
        "Noto Sans CJK",
    ]
}

fn mono_families() -> &'static [&'static str] {
    &[
        "SF Mono",
        "Menlo",
        "Monaco",
        "Noto Sans Mono",
        "Noto Sans Arabic",
        "Noto Sans CJK",
        "Noto Emoji",
        "Noto Color Emoji",
    ]
}

fn code_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| build_text_style(mono_families(), CODE_TEXT_SIZE, 600, false))
}

fn image_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| build_text_style(sans_families(), IMAGE_TEXT_SIZE, 700, false))
}

fn marker_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| build_text_style(mono_families(), MARKER_TEXT_SIZE, 600, false))
}

const BASE_MESSAGE_SPECS: &[MarkdownChatSeed] = &[
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "Can we treat the rich-text inline flow helper (`rich-inline`) as a real primitive, or is it only good for one tiny demo?\n\nI mostly care about:\n- exact bubble heights\n- virtualization without DOM reads\n- markdown-ish inline styling",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "Short answer: **yes, inside a bounded corridor**.\n\nIt already handles rich-text inline flow, `code`, and links like [Pretext](https://github.com/chenglou/pretext), while keeping pills and badges atomic. The real pressure starts once a chat bubble stops being one paragraph.",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "Right. My side is usually short, but your side has the weird stuff: Beijing 北京, Arabic مرحبا, emoji 👩‍🚀, and long URLs like https://example.com/reports/q3?lang=ar&mode=full",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "### What a chat renderer actually needs\n\n1. Parse markdown somewhere else.\n2. Normalize it into blocks and inline runs.\n3. Use the rich-text inline flow helper (`rich-inline`) for paragraph-ish content.\n4. Use the `pre-wrap` path for fenced code.",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "Then let’s stress it with **real markdown**: ***nested emphasis***, ~~deletions~~, `inline code`, [links](https://openai.com/), and a couple messages that are obviously richer on the AI side than on mine.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "> If we know the exact height in advance, then virtualization is no longer guesswork.\n>\n> It becomes geometry.\n\nThat is the whole reason to keep the primitive low-level and composable.",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "Okay, but the design matters too. The left side should feel lighter and more editorial, while my side can stay bubble-y.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "```ts\nconst frame = buildConversationFrame(templates, width)\nconst visible = findVisibleRange(frame, scrollTop, viewportHeight, overscan)\nrenderMessages(frame, visible.start, visible.end)\n```",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "I also want code fences, quotes, and lists to show up often enough that the 10k-thread run actually teaches us something.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "That part is important.\n\n- paragraph layout is one leaf\n- code fences are another leaf\n- the chat message is the block-level container above both\n\nThe assistant side is the real stress test because it keeps hitting headings, bullets, quotes, code fences, and occasional long explanations.",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "Try a checklist too. A product chat is full of little status updates.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "Current polish pass:\n\n- [x] lighter body copy is in\n- [x] the assistant lane is bubble-less\n- [x] exact height prediction is wired up\n- [ ] mobile screenshot smoke tests still remain",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "What about images or chips? Even if they are fake, I want to know the primitive can hold an atomic thing.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "It can. Something like ![diagram](https://example.com/mock-wireframe.png) behaves more like an inline chip than a splittable word, which is exactly the right stress case.",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "Give me one answer that feels more structured, almost like a mini design review.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "## Design review\n\nThe strongest signal so far is that **assistant messages want a different presentation contract from user messages**. The human side reads well as compact bubbles. The assistant side reads better as content on a surface with room to breathe.\n\nThat split also maps nicely to the measurement model because user messages are usually short and AI responses are much more likely to hit rich block transitions.",
    },
    MarkdownChatSeed {
        role: ChatRole::User,
        markdown: "Can we include nested bullets too? I want to see whether the second level still reads okay.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "Things to watch:\n\n- top-level bullets should feel close to body text\n- nested bullets can indent, but only when they truly nest\n  - this second level should not explode the width math\n  - the marker still needs a clean left gutter\n- quotes and code fences should tuck closer to the lead-in paragraph",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "```yaml\nparagraph_leaf: rich-text-inline-flow\ncode_leaf: pre-wrap\nquote_wrapper: block shell\nvirtualization: exact-height-first\n```",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "<aside>Escaped HTML still needs a stable fallback path.</aside>\n\nIf it comes through as a block token, we can still route it through a plain-text or code-like presentation instead of pretending we have a full browser renderer.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "Mixed-script sample: English for the framing, 日本語 for compact line breaks, العربية for punctuation clusters, and emoji like 🧪📐 to keep the grapheme path honest.",
    },
    MarkdownChatSeed {
        role: ChatRole::Assistant,
        markdown: "Yep. Short answers are a useful canary because they should not inherit giant block spacing or overbuilt chrome.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_engine() -> PretextEngine {
        PretextEngine::builder()
            .with_font_data(pretext_egui::experimental::demo_assets::bundled_font_data())
            .include_system_fonts(false)
            .build()
    }

    #[test]
    fn markdown_chat_builds_templates_with_rich_blocks() {
        let engine = bundled_engine();
        let templates = BASE_MESSAGE_SPECS
            .iter()
            .map(|seed| PreparedChatTemplate {
                role: seed.role,
                blocks: prepare_markdown_blocks(&engine, seed.markdown),
            })
            .collect::<Vec<_>>();

        assert!(templates.iter().any(|template| {
            template
                .blocks
                .iter()
                .any(|block| matches!(block, PreparedBlock::Code(_)))
        }));
        assert!(templates.iter().any(|template| {
            template
                .blocks
                .iter()
                .any(|block| matches!(block, PreparedBlock::Inline(_)))
        }));
    }

    #[test]
    fn markdown_chat_tables_fallback_to_code_blocks() {
        let engine = bundled_engine();
        let blocks = prepare_markdown_blocks(
            &engine,
            "| **feature** | [status](https://example.com) |\n| --- | --- |\n| cache | warm |\n| bidi | ready |",
        );

        let PreparedBlock::Code(block) = &blocks[0] else {
            panic!("markdown table should fallback to a code block");
        };
        let layout = block
            .prepared
            .layout(&engine, UNBOUNDED_WIDTH, CODE_LINE_HEIGHT);
        let lines = layout
            .lines
            .into_iter()
            .map(|line| line.line.text)
            .collect::<Vec<_>>();

        assert_eq!(
            lines,
            vec![
                "feature | status".to_owned(),
                "--- | ---".to_owned(),
                "cache | warm".to_owned(),
                "bidi | ready".to_owned(),
            ]
        );
    }

    #[test]
    fn markdown_chat_html_blocks_fallback_to_code_blocks() {
        let engine = bundled_engine();
        let blocks = prepare_markdown_blocks(
            &engine,
            "<aside>Escaped HTML still needs a stable fallback path.</aside>",
        );

        let PreparedBlock::Code(block) = &blocks[0] else {
            panic!("block HTML should fallback to a code block");
        };
        let layout = block
            .prepared
            .layout(&engine, UNBOUNDED_WIDTH, CODE_LINE_HEIGHT);
        let lines = layout
            .lines
            .into_iter()
            .map(|line| line.line.text)
            .collect::<Vec<_>>();

        assert_eq!(
            lines,
            vec!["<aside>Escaped HTML still needs a stable fallback path.</aside>".to_owned()]
        );
    }

    #[test]
    fn markdown_chat_frame_is_deterministic() {
        let engine = bundled_engine();
        let templates = BASE_MESSAGE_SPECS
            .iter()
            .map(|seed| PreparedChatTemplate {
                role: seed.role,
                blocks: prepare_markdown_blocks(&engine, seed.markdown),
            })
            .collect::<Vec<_>>();

        let first = build_conversation_frame(
            &engine,
            &templates,
            DEFAULT_CHAT_WIDTH,
            OCCLUSION_BANNER_HEIGHT,
        );
        let second = build_conversation_frame(
            &engine,
            &templates,
            DEFAULT_CHAT_WIDTH,
            OCCLUSION_BANNER_HEIGHT,
        );

        assert_eq!(first.messages.len(), TOTAL_MESSAGE_COUNT);
        assert_eq!(first.total_height, second.total_height);
        assert_eq!(first.messages[128].top, second.messages[128].top);
        assert_eq!(
            first.template_frames[3].total_height,
            second.template_frames[3].total_height
        );
    }

    #[test]
    fn markdown_chat_visible_range_stays_ordered() {
        let engine = bundled_engine();
        let templates = BASE_MESSAGE_SPECS
            .iter()
            .map(|seed| PreparedChatTemplate {
                role: seed.role,
                blocks: prepare_markdown_blocks(&engine, seed.markdown),
            })
            .collect::<Vec<_>>();
        let frame = build_conversation_frame(
            &engine,
            &templates,
            DEFAULT_CHAT_WIDTH,
            OCCLUSION_BANNER_HEIGHT,
        );
        let visible = find_visible_range(
            &frame,
            2000.0,
            640.0,
            OCCLUSION_BANNER_HEIGHT,
            OCCLUSION_BANNER_HEIGHT,
            VIEWPORT_OVERSCAN,
        );

        assert!(visible.start < visible.end);
        assert!(visible.end <= frame.messages.len());
        assert!(
            frame.messages[visible.start].bottom
                >= 2000.0 + OCCLUSION_BANNER_HEIGHT - VIEWPORT_OVERSCAN
        );
    }

    #[test]
    fn markdown_chat_compact_occlusion_rebuilds_top_padding() {
        let engine = bundled_engine();
        let templates = BASE_MESSAGE_SPECS
            .iter()
            .map(|seed| PreparedChatTemplate {
                role: seed.role,
                blocks: prepare_markdown_blocks(&engine, seed.markdown),
            })
            .collect::<Vec<_>>();

        let regular = build_conversation_frame(
            &engine,
            &templates,
            DEFAULT_CHAT_WIDTH,
            OCCLUSION_BANNER_HEIGHT,
        );
        let compact = build_conversation_frame(
            &engine,
            &templates,
            DEFAULT_CHAT_WIDTH,
            COMPACT_OCCLUSION_BANNER_HEIGHT,
        );

        assert!(regular.messages[0].top > compact.messages[0].top);
        assert!(regular.total_height > compact.total_height);
        assert_eq!(
            get_occlusion_banner_height(COMPACT_OCCLUSION_VIEWPORT_HEIGHT),
            COMPACT_OCCLUSION_BANNER_HEIGHT
        );
    }

    #[test]
    fn markdown_chat_materializes_inline_code_fragments() {
        let engine = bundled_engine();
        let markdown = "### What a chat renderer actually needs\n\n1. Parse markdown somewhere else.\n2. Normalize it into blocks and inline runs.\n3. Use the rich-text inline flow helper (`rich-inline`) for paragraph-ish content.\n4. Use the `pre-wrap` path for fenced code.";
        let template = PreparedChatTemplate {
            role: ChatRole::Assistant,
            blocks: prepare_markdown_blocks(&engine, markdown),
        };
        let frame = layout_template_frame(
            &engine,
            &template,
            DEFAULT_CHAT_WIDTH,
            DEFAULT_CHAT_WIDTH,
            0.0,
        );
        let blocks = materialize_template_blocks(&engine, &template, &frame);

        let lines = blocks
            .iter()
            .filter_map(|block| match block {
                BlockLayout::Inline { lines, .. } => Some(lines),
                _ => None,
            })
            .flat_map(|lines| lines.iter())
            .collect::<Vec<_>>();
        let all_fragments = lines
            .iter()
            .flat_map(|line| line.fragments.iter())
            .collect::<Vec<_>>();
        let code_fragments = all_fragments
            .iter()
            .filter(|fragment| fragment.meta.kind == InlinePieceKind::Code)
            .collect::<Vec<_>>();

        assert!(
            code_fragments
                .iter()
                .any(|fragment| fragment.text == "rich-inline"),
            "expected `rich-inline` inline code fragment"
        );
        assert!(
            code_fragments
                .iter()
                .any(|fragment| fragment.text == "pre-wrap"),
            "expected `pre-wrap` inline code fragment"
        );
        assert!(
            code_fragments
                .iter()
                .any(|fragment| !fragment.glyph_runs.is_empty()),
            "expected inline code fragment to carry glyph runs"
        );
    }
}

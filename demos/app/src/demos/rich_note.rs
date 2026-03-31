use eframe::egui;
use egui::{Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind};
#[cfg(test)]
use pretext::BidiDirection;
use pretext::{
    LayoutCursor, LayoutLineVisualRun, PrepareOptions, PreparedTextWithSegments, PretextEngine,
    WhiteSpaceMode,
};

use crate::assets::AssetRegistry;
use crate::demos::text_runs::paint_visual_runs;
use crate::demos::DemoWindow;

const LINE_START_CURSOR: LayoutCursor = LayoutCursor {
    segment_index: 0,
    grapheme_index: 0,
};

const LINE_HEIGHT: f32 = 34.0;
const LAST_LINE_BLOCK_HEIGHT: f32 = 24.0;
const NOTE_SHELL_CHROME_X: f32 = 40.0;
const NOTE_TOP_PADDING: f32 = 24.0;
const NOTE_BOTTOM_PADDING: f32 = 18.0;
const BODY_MIN_WIDTH: f32 = 260.0;
const BODY_DEFAULT_WIDTH: f32 = 516.0;
const BODY_MAX_WIDTH: f32 = 760.0;
const PAGE_MARGIN: f32 = 28.0;
const CHIP_CHROME_WIDTH: f32 = 22.0;
const CODE_CHROME_WIDTH: f32 = 14.0;
const UNBOUNDED_WIDTH: f32 = 100_000.0;

pub struct RichNoteDemo {
    open: bool,
    requested_width: f32,
    items: Option<Vec<InlineItem>>,
}

impl Default for RichNoteDemo {
    fn default() -> Self {
        Self {
            open: false,
            requested_width: BODY_DEFAULT_WIDTH,
            items: None,
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

struct TextStyleModel {
    style_name: TextStyleName,
    chrome_width: f32,
    spec: pretext::TextStyleSpec,
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
    end_cursor: LayoutCursor,
    full_text: String,
    full_visual_runs: Vec<LayoutLineVisualRun>,
    full_width: f32,
    leading_gap: f32,
    prepared: PreparedTextWithSegments,
}

#[derive(Clone)]
struct ChipInlineItem {
    tone: ChipTone,
    leading_gap: f32,
    text: String,
    visual_runs: Vec<LayoutLineVisualRun>,
    text_width: f32,
    chrome_width: f32,
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
    visual_runs: Vec<LayoutLineVisualRun>,
    text_width: f32,
    chrome_width: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct RichLine {
    fragments: Vec<LineFragment>,
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
    fn title(&self) -> &str {
        "Rich Note"
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
            .default_size(egui::vec2(860.0, 520.0))
            .show(ctx, |ui| {
                let raw_max_body = ui.available_width() - NOTE_SHELL_CHROME_X - PAGE_MARGIN;
                let max_body_width = raw_max_body.max(BODY_MIN_WIDTH).min(BODY_MAX_WIDTH);
                let slider_max = max_body_width.max(BODY_MIN_WIDTH);
                ui.add(
                    egui::Slider::new(&mut self.requested_width, BODY_MIN_WIDTH..=slider_max)
                        .text("Body width"),
                );

                let body_width = self.requested_width.clamp(BODY_MIN_WIDTH, slider_max);
                self.requested_width = body_width;

                let items = self.ensure_items(engine);
                let lines = layout_inline_items(engine, items, body_width);
                let line_count = lines.len();
                let note_body_height = if line_count == 0 {
                    LAST_LINE_BLOCK_HEIGHT
                } else {
                    (line_count as f32 - 1.0) * LINE_HEIGHT + LAST_LINE_BLOCK_HEIGHT
                };
                let note_width = body_width + NOTE_SHELL_CHROME_X;
                let note_height = note_body_height + NOTE_TOP_PADDING + NOTE_BOTTOM_PADDING;

                ui.horizontal(|ui| {
                    ui.label(format!("{line_count} lines"));
                    ui.separator();
                    ui.label(format!("{:.0}px note width", note_width));
                });
                ui.add_space(12.0);

                ui.horizontal_centered(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(note_width, note_height), Sense::hover());
                    paint_rich_note(ui.painter(), rect, &lines);
                });
            });
        self.open = open;
    }
}

impl RichNoteDemo {
    fn ensure_items(&mut self, engine: &PretextEngine) -> &[InlineItem] {
        if self.items.is_none() {
            self.items = Some(prepare_inline_items(engine, INLINE_SPECS));
        }

        self.items
            .as_deref()
            .expect("rich note items should be prepared")
    }
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
            spec: pretext::TextStyleSpec {
                families: vec![
                    "Noto Sans".to_owned(),
                    "Arial".to_owned(),
                    "Helvetica".to_owned(),
                ],
                size_px: 17.0,
                weight: 500,
                italic: false,
            },
        },
        TextStyleName::Link => TextStyleModel {
            style_name,
            chrome_width: 0.0,
            spec: pretext::TextStyleSpec {
                families: vec![
                    "Noto Sans".to_owned(),
                    "Arial".to_owned(),
                    "Helvetica".to_owned(),
                ],
                size_px: 17.0,
                weight: 600,
                italic: false,
            },
        },
        TextStyleName::Code => TextStyleModel {
            style_name,
            chrome_width: CODE_CHROME_WIDTH,
            spec: pretext::TextStyleSpec {
                families: vec![
                    "Noto Sans Mono".to_owned(),
                    "Menlo".to_owned(),
                    "Monaco".to_owned(),
                ],
                size_px: 14.0,
                weight: 600,
                italic: false,
            },
        },
    }
}

fn chip_text_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 12.0,
        weight: 700,
        italic: false,
    }
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
    }
}

fn measure_single_line_width(engine: &PretextEngine, prepared: &PreparedTextWithSegments) -> f32 {
    let mut max_width = 0.0f32;
    engine.walk_line_ranges(prepared, UNBOUNDED_WIDTH, |line| {
        max_width = max_width.max(line.width);
    });
    max_width
}

fn measure_collapsed_space_width(engine: &PretextEngine, style: &pretext::TextStyleSpec) -> f32 {
    let joined = engine.prepare_with_segments("A A", style, &normal_options());
    let compact = engine.prepare_with_segments("AA", style, &normal_options());
    (measure_single_line_width(engine, &joined) - measure_single_line_width(engine, &compact))
        .max(0.0)
}

fn prepare_inline_items(engine: &PretextEngine, specs: &[RichInlineSpec]) -> Vec<InlineItem> {
    let inline_boundary_gap =
        measure_collapsed_space_width(engine, &text_style_model(TextStyleName::Body).spec);
    let mut items = Vec::new();
    let mut pending_gap = 0.0f32;

    for spec in specs {
        match *spec {
            RichInlineSpec::Chip { label, tone } => {
                let label_prepared =
                    engine.prepare_with_segments(label, &chip_text_style(), &normal_options());
                let mut label_cursor = LINE_START_CURSOR;
                let label_line = engine
                    .layout_next_line(&label_prepared, &mut label_cursor, UNBOUNDED_WIDTH)
                    .expect("chip label should fit in an unbounded line");
                let visual_runs = engine.line_visual_runs(&label_prepared, &label_line);
                let text_width = label_line.width.ceil();
                let prepared = engine
                    .prepare_atomic_placeholder(text_width + CHIP_CHROME_WIDTH, &normal_options());
                items.push(InlineItem::Chip(ChipInlineItem {
                    tone,
                    leading_gap: pending_gap,
                    text: label.to_owned(),
                    visual_runs,
                    text_width,
                    chrome_width: CHIP_CHROME_WIDTH,
                    prepared,
                }));
                pending_gap = 0.0;
            }
            RichInlineSpec::Text { text, style } => {
                let carry_gap = pending_gap;
                let has_leading_whitespace = text.chars().next().is_some_and(char::is_whitespace);
                let has_trailing_whitespace = text.chars().last().is_some_and(char::is_whitespace);
                let trimmed_text = text.trim();
                pending_gap = if has_trailing_whitespace {
                    inline_boundary_gap
                } else {
                    0.0
                };

                if trimmed_text.is_empty() {
                    continue;
                }

                let style_model = text_style_model(style);
                let prepared = engine.prepare_with_segments(
                    trimmed_text,
                    &style_model.spec,
                    &normal_options(),
                );
                let mut cursor = LINE_START_CURSOR;
                let Some(whole_line) =
                    engine.layout_next_line(&prepared, &mut cursor, UNBOUNDED_WIDTH)
                else {
                    continue;
                };
                let full_visual_runs = engine.line_visual_runs(&prepared, &whole_line);

                items.push(InlineItem::Text(TextInlineItem {
                    style_name: style_model.style_name,
                    chrome_width: style_model.chrome_width,
                    end_cursor: whole_line.end,
                    full_text: whole_line.text,
                    full_visual_runs,
                    full_width: whole_line.width,
                    leading_gap: if carry_gap > 0.0 || has_leading_whitespace {
                        inline_boundary_gap
                    } else {
                        0.0
                    },
                    prepared,
                }));
            }
        }
    }

    items
}

fn layout_inline_items(
    engine: &PretextEngine,
    items: &[InlineItem],
    max_width: f32,
) -> Vec<RichLine> {
    let safe_width = max_width.max(1.0);
    let mut lines = Vec::new();
    let mut item_index = 0usize;
    let mut text_cursor: Option<LayoutCursor> = None;

    while item_index < items.len() {
        let mut fragments = Vec::new();
        let mut line_width = 0.0f32;
        let mut remaining_width = safe_width;

        while item_index < items.len() {
            match &items[item_index] {
                InlineItem::Chip(item) => {
                    let leading_gap = if fragments.is_empty() {
                        0.0
                    } else {
                        item.leading_gap
                    };
                    let mut line_cursor = LINE_START_CURSOR;
                    let Some(line) = engine.layout_next_line(
                        &item.prepared,
                        &mut line_cursor,
                        (remaining_width - leading_gap).max(1.0),
                    ) else {
                        item_index += 1;
                        text_cursor = None;
                        continue;
                    };

                    if !fragments.is_empty() && leading_gap + line.width > remaining_width {
                        break;
                    }

                    fragments.push(LineFragment {
                        kind: FragmentKind::Chip(item.tone),
                        leading_gap,
                        text: item.text.clone(),
                        visual_runs: item.visual_runs.clone(),
                        text_width: item.text_width,
                        chrome_width: item.chrome_width,
                    });
                    line_width += leading_gap + line.width;
                    remaining_width = (safe_width - line_width).max(0.0);
                    item_index += 1;
                    text_cursor = None;
                }
                InlineItem::Text(item) => {
                    if text_cursor.is_some_and(|cursor| cursor == item.end_cursor) {
                        item_index += 1;
                        text_cursor = None;
                        continue;
                    }

                    let leading_gap = if fragments.is_empty() {
                        0.0
                    } else {
                        item.leading_gap
                    };
                    let reserved_width = leading_gap + item.chrome_width;
                    if !fragments.is_empty() && reserved_width >= remaining_width {
                        break;
                    }

                    if text_cursor.is_none() {
                        let full_width = leading_gap + item.full_width + item.chrome_width;
                        if full_width <= remaining_width {
                            fragments.push(LineFragment {
                                kind: fragment_kind_for_style(item.style_name),
                                leading_gap,
                                text: item.full_text.clone(),
                                visual_runs: item.full_visual_runs.clone(),
                                text_width: item.full_width,
                                chrome_width: item.chrome_width,
                            });
                            line_width += full_width;
                            remaining_width = (safe_width - line_width).max(0.0);
                            item_index += 1;
                            continue;
                        }
                    }

                    let start_cursor = text_cursor.unwrap_or(LINE_START_CURSOR);
                    let mut line_cursor = start_cursor;
                    let Some(line) = engine.layout_next_line(
                        &item.prepared,
                        &mut line_cursor,
                        (remaining_width - reserved_width).max(1.0),
                    ) else {
                        item_index += 1;
                        text_cursor = None;
                        continue;
                    };

                    if start_cursor == line.end {
                        item_index += 1;
                        text_cursor = None;
                        continue;
                    }
                    let visual_runs = engine.line_visual_runs(&item.prepared, &line);

                    fragments.push(LineFragment {
                        kind: fragment_kind_for_style(item.style_name),
                        leading_gap,
                        text: line.text,
                        visual_runs,
                        text_width: line.width,
                        chrome_width: item.chrome_width,
                    });
                    line_width += leading_gap + line.width + item.chrome_width;
                    remaining_width = (safe_width - line_width).max(0.0);

                    if line.end == item.end_cursor {
                        item_index += 1;
                        text_cursor = None;
                    } else {
                        text_cursor = Some(line.end);
                        break;
                    }
                }
            }
        }

        if fragments.is_empty() {
            break;
        }
        lines.push(RichLine { fragments });
    }

    lines
}

fn fragment_kind_for_style(style_name: TextStyleName) -> FragmentKind {
    match style_name {
        TextStyleName::Body => FragmentKind::Body,
        TextStyleName::Link => FragmentKind::Link,
        TextStyleName::Code => FragmentKind::Code,
    }
}

fn paint_rich_note(painter: &egui::Painter, rect: Rect, lines: &[RichLine]) {
    painter.rect_filled(
        rect,
        CornerRadius::same(20),
        Color32::from_rgb(248, 244, 238),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(20),
        Stroke::new(1.0, Color32::from_rgb(220, 210, 196)),
        StrokeKind::Inside,
    );

    let rail_x = rect.left() + 14.0;
    painter.line_segment(
        [
            egui::pos2(rail_x, rect.top() + 18.0),
            egui::pos2(rail_x, rect.bottom() - 18.0),
        ],
        Stroke::new(2.0, Color32::from_rgb(228, 199, 145)),
    );
    painter.circle_filled(
        egui::pos2(rail_x, rect.top() + 26.0),
        4.0,
        Color32::from_rgb(201, 134, 68),
    );

    let body_left = rect.left() + 22.0;
    let body_top = rect.top() + NOTE_TOP_PADDING;
    let body_font = FontId::new(17.0, FontFamily::Proportional);
    let code_font = FontId::new(14.0, FontFamily::Monospace);
    let chip_font = FontId::new(12.0, FontFamily::Proportional);

    for (line_index, line) in lines.iter().enumerate() {
        let mut x = body_left;
        let y = body_top + line_index as f32 * LINE_HEIGHT;

        for fragment in &line.fragments {
            x += fragment.leading_gap;

            match fragment.kind {
                FragmentKind::Body => {
                    paint_visual_runs(
                        painter,
                        x,
                        y,
                        &fragment.text,
                        &fragment.visual_runs,
                        &body_font,
                        Color32::from_rgb(42, 47, 56),
                    );
                }
                FragmentKind::Link => {
                    let color = Color32::from_rgb(45, 101, 193);
                    paint_visual_runs(
                        painter,
                        x,
                        y,
                        &fragment.text,
                        &fragment.visual_runs,
                        &body_font,
                        color,
                    );
                    painter.line_segment(
                        [
                            egui::pos2(x, y + 20.0),
                            egui::pos2(x + fragment.text_width, y + 20.0),
                        ],
                        Stroke::new(1.0, color.gamma_multiply(0.8)),
                    );
                }
                FragmentKind::Code => {
                    let box_rect = Rect::from_min_size(
                        egui::pos2(x, y + 4.0),
                        egui::vec2(fragment.total_width(), 24.0),
                    );
                    painter.rect_filled(
                        box_rect,
                        CornerRadius::same(8),
                        Color32::from_rgb(235, 228, 214),
                    );
                    painter.rect_stroke(
                        box_rect,
                        CornerRadius::same(8),
                        Stroke::new(1.0, Color32::from_rgb(220, 209, 187)),
                        StrokeKind::Inside,
                    );
                    paint_visual_runs(
                        painter,
                        x + fragment.chrome_width * 0.5,
                        y + 9.0,
                        &fragment.text,
                        &fragment.visual_runs,
                        &code_font,
                        Color32::from_rgb(90, 70, 40),
                    );
                }
                FragmentKind::Chip(tone) => {
                    let (fill, stroke, text_color) = chip_palette(tone);
                    let box_rect = Rect::from_min_size(
                        egui::pos2(x, y + 5.0),
                        egui::vec2(fragment.total_width(), 22.0),
                    );
                    painter.rect_filled(box_rect, CornerRadius::same(11), fill);
                    painter.rect_stroke(
                        box_rect,
                        CornerRadius::same(11),
                        Stroke::new(1.0, stroke),
                        StrokeKind::Inside,
                    );
                    paint_visual_runs(
                        painter,
                        x + fragment.chrome_width * 0.5,
                        y + 9.0,
                        &fragment.text,
                        &fragment.visual_runs,
                        &chip_font,
                        text_color,
                    );
                }
            }

            x += fragment.total_width();
        }
    }

    painter.line_segment(
        [
            egui::pos2(body_left, rect.bottom() - 14.0),
            egui::pos2(body_left + 72.0, rect.bottom() - 14.0),
        ],
        Stroke::new(1.5, Color32::from_rgb(217, 208, 194)),
    );
}

fn chip_palette(tone: ChipTone) -> (Color32, Color32, Color32) {
    match tone {
        ChipTone::Mention => (
            Color32::from_rgb(234, 242, 255),
            Color32::from_rgb(191, 210, 242),
            Color32::from_rgb(44, 90, 168),
        ),
        ChipTone::Status => (
            Color32::from_rgb(234, 246, 232),
            Color32::from_rgb(187, 220, 181),
            Color32::from_rgb(53, 109, 63),
        ),
        ChipTone::Priority => (
            Color32::from_rgb(253, 236, 228),
            Color32::from_rgb(239, 190, 167),
            Color32::from_rgb(170, 74, 41),
        ),
        ChipTone::Time => (
            Color32::from_rgb(243, 238, 255),
            Color32::from_rgb(206, 193, 242),
            Color32::from_rgb(92, 70, 160),
        ),
        ChipTone::Count => (
            Color32::from_rgb(241, 240, 234),
            Color32::from_rgb(212, 208, 191),
            Color32::from_rgb(88, 80, 64),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chips_stay_atomic_across_lines() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let items = prepare_inline_items(&engine, INLINE_SPECS);
        let lines = layout_inline_items(&engine, &items, 310.0);

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
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let items = prepare_inline_items(&engine, INLINE_SPECS);
        let first = layout_inline_items(&engine, &items, 420.0);
        let second = layout_inline_items(&engine, &items, 420.0);
        assert_eq!(first, second);
    }

    #[test]
    fn rich_note_keeps_visual_runs_for_mixed_direction_text() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let items = prepare_inline_items(&engine, INLINE_SPECS);
        let mixed = items
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
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let items = prepare_inline_items(&engine, INLINE_SPECS);

        let chip = items
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
}

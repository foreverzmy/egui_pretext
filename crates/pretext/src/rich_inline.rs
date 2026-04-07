//! Rich inline flow helper built on top of `pretext` paragraph layout.
//!
//! This helper targets inline-only, `white-space: normal` rich content:
//! boundary whitespace collapse, atomic inline items, and extra horizontal
//! chrome such as pills or code-span padding.

use std::collections::HashMap;

use crate::advanced::{LayoutCursor, LayoutLine, LayoutLineRange};
use crate::{
    ParagraphDirection, PretextEngine, PretextParagraphOptions, PretextPreparedParagraph,
    PretextStyle, WhiteSpaceMode, WordBreakMode,
};

const UNBOUNDED_WIDTH: f32 = 100_000.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RichInlineBreakMode {
    #[default]
    Normal,
    Never,
}

#[derive(Clone, Copy, Debug)]
pub struct RichInlineItemSpec<'a> {
    pub text: &'a str,
    pub style: &'a PretextStyle,
    pub break_mode: RichInlineBreakMode,
    pub extra_width: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RichInlineCursor {
    pub item_index: usize,
    pub segment_index: usize,
    pub grapheme_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichInlineFragment {
    pub item_index: usize,
    pub leading_gap: f32,
    pub extra_width: f32,
    pub line: LayoutLine,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RichInlineFragmentRange {
    pub item_index: usize,
    pub leading_gap: f32,
    pub extra_width: f32,
    pub line: LayoutLineRange,
    pub add_hyphen: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichInlineLine {
    pub fragments: Vec<RichInlineFragment>,
    pub width: f32,
    pub end: RichInlineCursor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RichInlineLineRange {
    pub fragments: Vec<RichInlineFragmentRange>,
    pub width: f32,
    pub end: RichInlineCursor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RichInlineStats {
    pub line_count: usize,
    pub max_line_width: f32,
}

#[derive(Clone)]
pub struct PreparedRichInline {
    items: Vec<PreparedRichInlineItem>,
    item_by_source_index: Vec<Option<usize>>,
}

#[derive(Clone)]
struct PreparedRichInlineItem {
    source_item_index: usize,
    break_mode: RichInlineBreakMode,
    gap_before: f32,
    extra_width: f32,
    prepared: PretextPreparedParagraph,
    whole_line: LayoutLine,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct StyleCacheKey {
    families: Vec<String>,
    size_px_bits: u32,
    weight: u16,
    italic: bool,
}

impl StyleCacheKey {
    fn new(style: &PretextStyle) -> Self {
        Self {
            families: style.families.clone(),
            size_px_bits: style.size_px.to_bits(),
            weight: style.weight,
            italic: style.italic,
        }
    }
}

impl PreparedRichInline {
    pub fn item_prepared(&self, item_index: usize) -> Option<&PretextPreparedParagraph> {
        let index = self
            .item_by_source_index
            .get(item_index)
            .copied()
            .flatten()?;
        Some(&self.items.get(index)?.prepared)
    }

    pub fn item_line(&self, item_index: usize) -> Option<&LayoutLine> {
        let index = self
            .item_by_source_index
            .get(item_index)
            .copied()
            .flatten()?;
        Some(&self.items.get(index)?.whole_line)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub fn prepare_rich_inline(
    engine: &PretextEngine,
    items: &[RichInlineItemSpec<'_>],
) -> PreparedRichInline {
    let mut prepared_items = Vec::new();
    let mut item_by_source_index = vec![None; items.len()];
    let mut collapsed_space_widths = HashMap::<StyleCacheKey, f32>::new();
    let mut pending_gap = 0.0f32;

    for (index, item) in items.iter().enumerate() {
        let has_leading_whitespace = item.text.starts_with(is_collapsible_boundary_char);
        let has_trailing_whitespace = item.text.ends_with(is_collapsible_boundary_char);
        let trimmed = item.text.trim_matches(is_collapsible_boundary_char);

        if trimmed.is_empty() {
            if item.text.chars().any(is_collapsible_boundary_char) && pending_gap == 0.0 {
                pending_gap =
                    collapsed_space_width(engine, item.style, &mut collapsed_space_widths);
            }
            continue;
        }

        let gap_before = if pending_gap > 0.0 {
            pending_gap
        } else if has_leading_whitespace {
            collapsed_space_width(engine, item.style, &mut collapsed_space_widths)
        } else {
            0.0
        };

        let prepared = engine.prepare_paragraph(trimmed, item.style, &normal_options());
        let mut cursor = LayoutCursor::default();
        let Some(whole_line) = engine.layout_next_line(&prepared, &mut cursor, UNBOUNDED_WIDTH)
        else {
            pending_gap = if has_trailing_whitespace {
                collapsed_space_width(engine, item.style, &mut collapsed_space_widths)
            } else {
                0.0
            };
            continue;
        };

        item_by_source_index[index] = Some(prepared_items.len());
        prepared_items.push(PreparedRichInlineItem {
            source_item_index: index,
            break_mode: item.break_mode,
            gap_before,
            extra_width: item.extra_width.max(0.0),
            prepared,
            whole_line,
        });

        pending_gap = if has_trailing_whitespace {
            collapsed_space_width(engine, item.style, &mut collapsed_space_widths)
        } else {
            0.0
        };
    }

    PreparedRichInline {
        items: prepared_items,
        item_by_source_index,
    }
}

pub fn layout_rich_inline(
    engine: &PretextEngine,
    prepared: &PreparedRichInline,
    max_width: f32,
) -> Vec<RichInlineLine> {
    let mut lines = Vec::new();
    let mut cursor = RichInlineCursor::default();

    while let Some(line) =
        layout_next_rich_inline_line_range(engine, prepared, &mut cursor, max_width)
    {
        lines.push(materialize_rich_inline_line_range(prepared, &line));
    }

    lines
}

pub fn walk_rich_inline_line_ranges(
    engine: &PretextEngine,
    prepared: &PreparedRichInline,
    max_width: f32,
    mut on_line: impl FnMut(&RichInlineLineRange),
) -> RichInlineStats {
    let mut cursor = RichInlineCursor::default();
    let mut stats = RichInlineStats::default();

    while let Some(line) =
        layout_next_rich_inline_line_range(engine, prepared, &mut cursor, max_width)
    {
        stats.line_count += 1;
        stats.max_line_width = stats.max_line_width.max(line.width);
        on_line(&line);
    }

    stats
}

pub fn walk_rich_inline_lines(
    engine: &PretextEngine,
    prepared: &PreparedRichInline,
    max_width: f32,
    mut on_line: impl FnMut(&RichInlineLine),
) -> RichInlineStats {
    walk_rich_inline_line_ranges(engine, prepared, max_width, |line| {
        on_line(&materialize_rich_inline_line_range(prepared, line));
    })
}

pub fn measure_rich_inline_stats(
    engine: &PretextEngine,
    prepared: &PreparedRichInline,
    max_width: f32,
) -> RichInlineStats {
    let mut cursor = RichInlineCursor::default();
    let mut stats = RichInlineStats::default();

    while let Some(line_width) =
        step_rich_inline_line(engine, prepared, &mut cursor, max_width, |_| {})
    {
        stats.line_count += 1;
        stats.max_line_width = stats.max_line_width.max(line_width);
    }

    stats
}

pub fn layout_next_rich_inline_line(
    engine: &PretextEngine,
    prepared: &PreparedRichInline,
    start: &mut RichInlineCursor,
    max_width: f32,
) -> Option<RichInlineLine> {
    let line = layout_next_rich_inline_line_range(engine, prepared, start, max_width)?;
    Some(materialize_rich_inline_line_range(prepared, &line))
}

pub fn layout_next_rich_inline_line_range(
    engine: &PretextEngine,
    prepared: &PreparedRichInline,
    start: &mut RichInlineCursor,
    max_width: f32,
) -> Option<RichInlineLineRange> {
    let mut fragments = Vec::new();
    let width = step_rich_inline_line(engine, prepared, start, max_width, |fragment| {
        fragments.push(fragment);
    })?;

    Some(RichInlineLineRange {
        fragments,
        width,
        end: *start,
    })
}

fn step_rich_inline_line(
    engine: &PretextEngine,
    prepared: &PreparedRichInline,
    start: &mut RichInlineCursor,
    max_width: f32,
    mut on_fragment: impl FnMut(RichInlineFragmentRange),
) -> Option<f32> {
    if prepared.items.is_empty() || start.item_index >= prepared.items.len() {
        return None;
    }

    let safe_width = max_width.max(1.0);
    let mut has_fragments = false;
    let mut line_width = 0.0f32;
    let mut remaining_width = safe_width;
    let mut item_index = start.item_index;
    let mut text_cursor = LayoutCursor {
        segment_index: start.segment_index,
        grapheme_index: start.grapheme_index,
    };

    while item_index < prepared.items.len() {
        let item = &prepared.items[item_index];
        if !is_line_start_cursor(text_cursor) && text_cursor == item.whole_line.end {
            item_index += 1;
            text_cursor = LayoutCursor::default();
            continue;
        }

        let leading_gap = if has_fragments { item.gap_before } else { 0.0 };
        let at_item_start = is_line_start_cursor(text_cursor);

        if item.break_mode == RichInlineBreakMode::Never {
            if !at_item_start {
                item_index += 1;
                text_cursor = LayoutCursor::default();
                continue;
            }

            let total_width = leading_gap + item.whole_line.width + item.extra_width;
            if has_fragments && total_width > remaining_width {
                break;
            }

            on_fragment(RichInlineFragmentRange {
                item_index: item.source_item_index,
                leading_gap,
                extra_width: item.extra_width,
                line: whole_line_range(item),
                add_hyphen: false,
            });
            has_fragments = true;
            line_width += total_width;
            remaining_width = (safe_width - line_width).max(0.0);
            item_index += 1;
            text_cursor = LayoutCursor::default();
            continue;
        }

        let reserved_width = leading_gap + item.extra_width;
        if has_fragments && reserved_width >= remaining_width {
            break;
        }

        if at_item_start {
            let full_width = reserved_width + item.whole_line.width;
            if full_width <= remaining_width {
                on_fragment(RichInlineFragmentRange {
                    item_index: item.source_item_index,
                    leading_gap,
                    extra_width: item.extra_width,
                    line: whole_line_range(item),
                    add_hyphen: false,
                });
                has_fragments = true;
                line_width += full_width;
                remaining_width = (safe_width - line_width).max(0.0);
                item_index += 1;
                text_cursor = LayoutCursor::default();
                continue;
            }
        }

        let mut line_cursor = text_cursor;
        let Some((line, add_hyphen)) = engine.layout_next_line_range(
            &item.prepared,
            &mut line_cursor,
            (remaining_width - reserved_width).max(1.0),
        ) else {
            item_index += 1;
            text_cursor = LayoutCursor::default();
            continue;
        };

        if line.start == line.end {
            item_index += 1;
            text_cursor = LayoutCursor::default();
            continue;
        }

        if has_fragments
            && at_item_start
            && leading_gap > 0.0
            && ends_inside_first_segment(line.end)
        {
            let mut fresh_cursor = LayoutCursor::default();
            if let Some((fresh_line, _)) = engine.layout_next_line_range(
                &item.prepared,
                &mut fresh_cursor,
                (safe_width - item.extra_width).max(1.0),
            ) {
                let fresh_consumes_more = fresh_line.end.segment_index > line.end.segment_index
                    || (fresh_line.end.segment_index == line.end.segment_index
                        && fresh_line.end.grapheme_index > line.end.grapheme_index);
                if fresh_consumes_more {
                    break;
                }
            }
        }

        on_fragment(RichInlineFragmentRange {
            item_index: item.source_item_index,
            leading_gap,
            extra_width: item.extra_width,
            line,
            add_hyphen,
        });
        has_fragments = true;
        line_width += leading_gap + line.width + item.extra_width;
        remaining_width = (safe_width - line_width).max(0.0);

        if line.end == item.whole_line.end {
            item_index += 1;
            text_cursor = LayoutCursor::default();
        } else {
            text_cursor = line.end;
            break;
        }
    }

    if !has_fragments {
        return None;
    }

    *start = RichInlineCursor {
        item_index,
        segment_index: text_cursor.segment_index,
        grapheme_index: text_cursor.grapheme_index,
    };

    Some(line_width)
}

pub fn materialize_rich_inline_line_range(
    prepared: &PreparedRichInline,
    line: &RichInlineLineRange,
) -> RichInlineLine {
    RichInlineLine {
        fragments: line
            .fragments
            .iter()
            .map(|fragment| {
                let item_index = prepared
                    .item_by_source_index
                    .get(fragment.item_index)
                    .copied()
                    .flatten()
                    .expect("rich inline fragment should resolve to a prepared item");
                let item = &prepared.items[item_index];
                RichInlineFragment {
                    item_index: fragment.item_index,
                    leading_gap: fragment.leading_gap,
                    extra_width: fragment.extra_width,
                    line: crate::layout::materialize_line_range(
                        item.prepared.as_prepared(),
                        fragment.line,
                        fragment.add_hyphen,
                    ),
                }
            })
            .collect(),
        width: line.width,
        end: line.end,
    }
}

fn is_line_start_cursor(cursor: LayoutCursor) -> bool {
    cursor.segment_index == 0 && cursor.grapheme_index == 0
}

fn whole_line_range(item: &PreparedRichInlineItem) -> LayoutLineRange {
    LayoutLineRange {
        width: item.whole_line.width,
        start: item.whole_line.start,
        end: item.whole_line.end,
    }
}

fn ends_inside_first_segment(cursor: LayoutCursor) -> bool {
    cursor.segment_index == 0 && cursor.grapheme_index > 0
}

fn normal_options() -> PretextParagraphOptions {
    PretextParagraphOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
    }
}

fn collapsed_space_width(
    engine: &PretextEngine,
    style: &PretextStyle,
    cache: &mut HashMap<StyleCacheKey, f32>,
) -> f32 {
    let key = StyleCacheKey::new(style);
    if let Some(width) = cache.get(&key) {
        return *width;
    }

    let joined = engine.prepare_paragraph("A A", style, &normal_options());
    let compact = engine.prepare_paragraph("AA", style, &normal_options());
    let width = (single_line_width(engine, &joined) - single_line_width(engine, &compact)).max(0.0);
    cache.insert(key, width);
    width
}

fn single_line_width(engine: &PretextEngine, prepared: &PretextPreparedParagraph) -> f32 {
    let mut max_width = 0.0f32;
    engine.walk_line_ranges(prepared, UNBOUNDED_WIDTH, |line| {
        max_width = max_width.max(line.width);
    });
    max_width
}

fn is_collapsible_boundary_char(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000C}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_engine() -> PretextEngine {
        PretextEngine::with_font_data_and_system_fonts(
            vec![
                include_bytes!("../../../demos/app/assets/fonts/NotoSans-Regular.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansArabic-Regular.ttf")
                    .to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansCJK-Regular.ttc").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansMyanmar-Regular.ttf")
                    .to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoEmoji-Regular.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoColorEmoji.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/Noto-COLRv1.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansMono-Regular.ttf").to_vec(),
            ],
            false,
        )
    }

    fn body_style() -> PretextStyle {
        PretextStyle {
            families: vec![
                "Noto Sans".to_owned(),
                "Noto Sans Arabic".to_owned(),
                "Arial".to_owned(),
            ],
            size_px: 17.0,
            weight: 500,
            italic: false,
        }
    }

    fn code_style() -> PretextStyle {
        PretextStyle {
            families: vec![
                "Noto Sans Mono".to_owned(),
                "Noto Sans Arabic".to_owned(),
                "Noto Sans CJK".to_owned(),
            ],
            size_px: 14.0,
            weight: 600,
            italic: false,
        }
    }

    #[test]
    fn atomic_items_stay_whole_and_boundary_spaces_collapse() {
        let engine = bundled_engine();
        let body = body_style();
        let code = code_style();
        let prepared = prepare_rich_inline(
            &engine,
            &[
                RichInlineItemSpec {
                    text: "Ship ",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
                RichInlineItemSpec {
                    text: "@maya",
                    style: &body,
                    break_mode: RichInlineBreakMode::Never,
                    extra_width: 22.0,
                },
                RichInlineItemSpec {
                    text: " 's rich-note ",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
                RichInlineItemSpec {
                    text: "layoutNextLine()",
                    style: &code,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 14.0,
                },
            ],
        );

        let lines = layout_rich_inline(&engine, &prepared, 180.0);

        assert!(lines.len() >= 2);
        let chip_count = lines
            .iter()
            .flat_map(|line| line.fragments.iter())
            .filter(|fragment| fragment.item_index == 1)
            .count();
        assert_eq!(chip_count, 1);
        assert!(lines
            .iter()
            .flat_map(|line| line.fragments.iter())
            .any(|fragment| fragment.leading_gap > 0.0));
    }

    #[test]
    fn rich_inline_stats_match_materialized_layout() {
        let engine = bundled_engine();
        let body = body_style();
        let prepared = prepare_rich_inline(
            &engine,
            &[
                RichInlineItemSpec {
                    text: "English ",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
                RichInlineItemSpec {
                    text: "العربية",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
                RichInlineItemSpec {
                    text: " 北京",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
            ],
        );

        let lines = layout_rich_inline(&engine, &prepared, 120.0);
        let stats = measure_rich_inline_stats(&engine, &prepared, 120.0);
        let max_line_width = lines.iter().map(|line| line.width).fold(0.0f32, f32::max);

        assert_eq!(stats.line_count, lines.len());
        assert!((stats.max_line_width - max_line_width).abs() <= 0.05);
    }

    #[test]
    fn boundary_gap_wraps_before_tiny_first_segment_prefix() {
        let engine = bundled_engine();
        let body = body_style();
        let prepared = prepare_rich_inline(
            &engine,
            &[
                RichInlineItemSpec {
                    text: "Keep ",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
                RichInlineItemSpec {
                    text: "layoutNextLine",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
            ],
        );

        let wide_layout = layout_rich_inline(&engine, &prepared, UNBOUNDED_WIDTH);
        let wide_line = wide_layout
            .first()
            .expect("wide layout should materialize a line");
        assert_eq!(wide_line.fragments.len(), 2);
        let first_width = wide_line.fragments[0].line.width;
        let gap = wide_line.fragments[1].leading_gap;

        let second_prepared = prepared
            .item_prepared(1)
            .expect("second item should stay addressable");
        let second_whole_line = prepared
            .item_line(1)
            .expect("second item should have a whole-line layout");

        let mut candidate_width = None;
        for extra in 1..=64 {
            let max_width = first_width + gap + extra as f32;
            let available_after_gap = extra as f32;

            let mut partial_cursor = LayoutCursor::default();
            let Some(partial_line) = engine.layout_next_line(
                second_prepared,
                &mut partial_cursor,
                available_after_gap.max(1.0),
            ) else {
                continue;
            };
            if partial_line.end == second_whole_line.end {
                continue;
            }

            let mut fresh_cursor = LayoutCursor::default();
            let Some(fresh_line) =
                engine.layout_next_line(second_prepared, &mut fresh_cursor, max_width.max(1.0))
            else {
                continue;
            };
            let fresh_consumes_more = fresh_line.end.segment_index > partial_line.end.segment_index
                || (fresh_line.end.segment_index == partial_line.end.segment_index
                    && fresh_line.end.grapheme_index > partial_line.end.grapheme_index);
            if fresh_consumes_more {
                candidate_width = Some(max_width);
                break;
            }
        }

        let max_width = candidate_width.expect(
            "expected to find a width where the current line could only take a tiny prefix",
        );
        let lines = layout_rich_inline(&engine, &prepared, max_width);

        assert_eq!(lines[0].fragments.len(), 1);
        assert_eq!(lines[0].fragments[0].item_index, 0);
        assert_eq!(lines[1].fragments[0].item_index, 1);
    }

    #[test]
    fn range_materialization_matches_eager_layout() {
        let engine = bundled_engine();
        let body = body_style();
        let code = code_style();
        let prepared = prepare_rich_inline(
            &engine,
            &[
                RichInlineItemSpec {
                    text: "Ship ",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
                RichInlineItemSpec {
                    text: "@maya",
                    style: &body,
                    break_mode: RichInlineBreakMode::Never,
                    extra_width: 22.0,
                },
                RichInlineItemSpec {
                    text: " rich-inline keeps ",
                    style: &body,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 0.0,
                },
                RichInlineItemSpec {
                    text: "layoutNextLineRange()",
                    style: &code,
                    break_mode: RichInlineBreakMode::Normal,
                    extra_width: 14.0,
                },
            ],
        );

        let eager = layout_rich_inline(&engine, &prepared, 170.0);
        let mut ranges = Vec::new();
        let stats = walk_rich_inline_line_ranges(&engine, &prepared, 170.0, |line| {
            ranges.push(materialize_rich_inline_line_range(&prepared, line));
        });

        assert_eq!(ranges, eager);
        assert_eq!(stats.line_count, eager.len());
    }
}

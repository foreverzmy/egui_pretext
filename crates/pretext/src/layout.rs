use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::Arc;

use unicode_bidi::{BidiInfo, Level};

use crate::analysis::{analyze_text, slice_text, GraphemeKind, WhiteSpaceMode};
use crate::bidi::paragraph_to_bidi_runs;
use crate::engine::{
    GraphemeMeta, LayoutCursor, LayoutGlyph, LayoutLine, LayoutLineGlyphRun, LayoutLineRange,
    LayoutLineVisualRun, LayoutResult, LayoutWithLinesResult, PrepareOptions, PreparedText,
    PreparedTextWithSegments, Segment, SegmentKind, SegmentMeta, TextStyleSpec,
};
use crate::font_catalog::FontCatalog;
use crate::line_break::compute_breaks;
use crate::measure::{measure_text, ShapeCache, ShapedGlyph};
use lru::LruCache;
use parking_lot::Mutex;

pub(crate) const LINE_FIT_EPSILON: f32 = 0.005;
pub(crate) const TAB_SIZE_PRE_WRAP: u8 = 8;
const PARAGRAPH_CACHE_CAPACITY: usize = 256;

pub(crate) struct ParagraphCache {
    inner: Mutex<LruCache<ParagraphCacheKey, Arc<CachedParagraphLayout>>>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ParagraphCacheKey {
    text_hash: u64,
    style_hash: u64,
    width_bucket: u32,
    obstacles_hash: u64,
    locale_hash: u64,
    white_space: WhiteSpaceMode,
    paragraph_direction: crate::bidi::ParagraphDirection,
}

#[derive(Clone)]
struct CachedParagraphLine {
    range: LayoutLineRange,
    add_hyphen: bool,
}

#[derive(Clone)]
struct CachedParagraphLayout {
    lines: Arc<[CachedParagraphLine]>,
}

impl ParagraphCache {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(LruCache::new(
                NonZeroUsize::new(PARAGRAPH_CACHE_CAPACITY).expect("paragraph cache capacity"),
            )),
        }
    }

    pub(crate) fn clear(&self) {
        self.inner.lock().clear();
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.inner.lock().len()
    }

    fn get_or_compute(
        &self,
        key: ParagraphCacheKey,
        compute: impl FnOnce() -> Arc<CachedParagraphLayout>,
    ) -> Arc<CachedParagraphLayout> {
        if let Some(cached) = self.inner.lock().get(&key).cloned() {
            return cached;
        }

        let value = compute();
        self.inner.lock().put(key, value.clone());
        value
    }
}

pub(crate) fn prepare_text(
    text: &str,
    style: &TextStyleSpec,
    opts: &PrepareOptions,
    font_catalog: &FontCatalog,
    shape_cache: &Mutex<ShapeCache>,
    locale_hash: u64,
    locale: Option<&str>,
) -> PreparedTextWithSegments {
    let analysis = analyze_text(text, opts, locale);
    let bidi_runs = paragraph_to_bidi_runs(&analysis.normalized, opts.paragraph_direction);
    let measurement = measure_text(&analysis, style, &bidi_runs, font_catalog, shape_cache);
    let break_after = compute_breaks(&analysis);
    let hash = hash_text(&analysis.normalized);
    let style_hash = hash_style(style);

    let graphemes: Vec<GraphemeMeta> = analysis
        .graphemes
        .iter()
        .enumerate()
        .map(|(index, grapheme)| GraphemeMeta {
            byte_range: grapheme.byte_range.clone(),
            advance: match grapheme.kind {
                GraphemeKind::Newline
                | GraphemeKind::ZeroWidthBreak
                | GraphemeKind::SoftHyphen
                | GraphemeKind::WordJoiner
                | GraphemeKind::Tab => 0.0,
                _ => measurement
                    .grapheme_advances
                    .get(index)
                    .copied()
                    .unwrap_or(0.0),
            },
            kind: grapheme.kind,
            break_after: break_after[index],
        })
        .collect();

    let seg_meta: Arc<[SegmentMeta]> = if analysis.segments.is_empty() {
        Arc::from(Vec::<SegmentMeta>::new())
    } else {
        Arc::from(
            analysis
                .segments
                .iter()
                .map(|segment| SegmentMeta {
                    byte_range: segment.byte_range.clone(),
                    graphemes: Arc::from(graphemes[segment.grapheme_range.clone()].to_vec()),
                    tab_stop_advance: measurement.space_advance * TAB_SIZE_PRE_WRAP as f32,
                    discretionary_hyphen_width: measurement.hyphen_advance,
                })
                .collect::<Vec<_>>(),
        )
    };

    let segments: Arc<[Segment]> = if analysis.segments.is_empty() {
        Arc::from(Vec::<Segment>::new())
    } else {
        Arc::from(
            analysis
                .segments
                .iter()
                .map(|segment| Segment {
                    kind: SegmentKind::Text,
                    byte_range: segment.byte_range.clone(),
                    glyphs: segment_glyphs(&measurement.glyphs, &segment.byte_range),
                })
                .collect::<Vec<_>>(),
        )
    };

    let prepared = PreparedText::new(
        Arc::<str>::from(analysis.normalized),
        segments,
        seg_meta.clone(),
        Arc::from(bidi_runs),
        hyphen_glyphs(style, font_catalog, shape_cache),
        hash,
        style_hash,
        locale_hash,
        analysis.white_space,
        opts.paragraph_direction,
    );

    PreparedTextWithSegments {
        inner: prepared,
        seg_meta,
    }
}

pub(crate) fn prepare_atomic_placeholder(
    width: f32,
    opts: &PrepareOptions,
    locale_hash: u64,
) -> PreparedTextWithSegments {
    let safe_width = width.max(0.0);
    let graphemes: Arc<[GraphemeMeta]> = Arc::from(vec![GraphemeMeta {
        byte_range: 0..0,
        advance: safe_width,
        kind: GraphemeKind::Text,
        break_after: crate::line_break::BreakOpportunity::Allowed,
    }]);
    let seg_meta: Arc<[SegmentMeta]> = Arc::from(vec![SegmentMeta {
        byte_range: 0..0,
        graphemes,
        tab_stop_advance: 0.0,
        discretionary_hyphen_width: 0.0,
    }]);
    let segments: Arc<[Segment]> = Arc::from(vec![Segment {
        kind: SegmentKind::AtomicPlaceholder { width: safe_width },
        byte_range: 0..0,
        glyphs: Arc::from(Vec::<ShapedGlyph>::new()),
    }]);
    let prepared = PreparedText::new(
        Arc::<str>::from(""),
        segments,
        seg_meta.clone(),
        Arc::from(Vec::new()),
        Arc::from(Vec::<ShapedGlyph>::new()),
        hash_atomic_placeholder(safe_width),
        0,
        locale_hash,
        opts.white_space,
        opts.paragraph_direction,
    );

    PreparedTextWithSegments {
        inner: prepared,
        seg_meta,
    }
}

pub(crate) fn layout(
    prepared: &PreparedText,
    max_width: f32,
    line_height: f32,
    cache: Option<&ParagraphCache>,
) -> LayoutResult {
    let paragraph = paragraph_layout(prepared, max_width, cache);
    let line_count = paragraph.lines.len();

    LayoutResult {
        height: line_count as f32 * line_height.max(0.0),
        line_count,
    }
}

pub(crate) fn walk_line_ranges(
    prepared: &PreparedTextWithSegments,
    max_width: f32,
    mut on_line: impl FnMut(&LayoutLineRange),
    cache: Option<&ParagraphCache>,
) -> f32 {
    let paragraph = paragraph_layout(prepared.inner(), max_width, cache);
    let mut max_line_width = 0.0f32;

    for line in paragraph.lines.iter() {
        max_line_width = max_line_width.max(line.range.width);
        on_line(&line.range);
    }

    max_line_width
}

pub(crate) fn layout_with_lines(
    prepared: &PreparedTextWithSegments,
    max_width: f32,
    line_height: f32,
    cache: Option<&ParagraphCache>,
) -> LayoutWithLinesResult {
    let paragraph = paragraph_layout(prepared.inner(), max_width, cache);
    let lines = paragraph
        .lines
        .iter()
        .map(|line| materialize_line(prepared.inner(), line.range, line.add_hyphen))
        .collect::<Vec<_>>();

    LayoutWithLinesResult {
        height: lines.len() as f32 * line_height.max(0.0),
        line_count: lines.len(),
        lines,
    }
}

pub(crate) fn layout_next_line(
    prepared: &PreparedTextWithSegments,
    start: &mut LayoutCursor,
    max_width: f32,
    cache: Option<&ParagraphCache>,
) -> Option<LayoutLine> {
    if let Some(cache) = cache {
        let paragraph = paragraph_layout(prepared.inner(), max_width, Some(cache));
        if let Some(normalized) = normalize_line_start(prepared.inner(), *start) {
            if let Some(line) = paragraph
                .lines
                .iter()
                .find(|line| line.range.start == normalized)
            {
                *start = line.range.end;
                return Some(materialize_line(
                    prepared.inner(),
                    line.range,
                    line.add_hyphen,
                ));
            }
        }
    }

    let (range, add_hyphen) = next_line_range_internal(prepared.inner(), start, max_width)?;
    Some(materialize_line(prepared.inner(), range, add_hyphen))
}

pub(crate) fn line_visual_runs(
    prepared: &PreparedText,
    line: &LayoutLine,
) -> Vec<LayoutLineVisualRun> {
    #[derive(Clone)]
    struct VisualFragment {
        text: String,
        width: f32,
        start: LayoutCursor,
        end: LayoutCursor,
        level: Level,
        direction: crate::bidi::BidiDirection,
    }

    let mut fragments = Vec::new();
    let mut cursor = line.start;
    let mut line_width = 0.0f32;

    while cursor != line.end {
        let Some((segment, grapheme)) = grapheme_at(prepared, cursor) else {
            break;
        };
        let next = advance_cursor(prepared, cursor);
        match grapheme.kind {
            GraphemeKind::Newline
            | GraphemeKind::SoftHyphen
            | GraphemeKind::ZeroWidthBreak
            | GraphemeKind::WordJoiner => {
                cursor = next;
                continue;
            }
            _ => {}
        }

        let advance = grapheme_advance(segment, grapheme, line_width);
        let (level, direction) = bidi_props_for_grapheme(prepared, grapheme);
        fragments.push(VisualFragment {
            text: slice_text(prepared.text(), &grapheme.byte_range).to_owned(),
            width: advance,
            start: cursor,
            end: next,
            level,
            direction,
        });
        line_width += advance;
        cursor = next;
    }

    if prepared.white_space() == WhiteSpaceMode::Normal {
        while matches!(fragments.last(), Some(fragment) if fragment.text == " ") {
            fragments.pop();
        }
    }

    let mut logical_runs = Vec::<LayoutLineVisualRun>::new();
    for fragment in fragments {
        if let Some(last) = logical_runs.last_mut() {
            if last.level == fragment.level.number() && last.direction == fragment.direction {
                last.text.push_str(&fragment.text);
                last.width += fragment.width;
                last.end = fragment.end;
                continue;
            }
        }

        logical_runs.push(LayoutLineVisualRun {
            text: fragment.text,
            width: fragment.width,
            start: fragment.start,
            end: fragment.end,
            level: fragment.level.number(),
            direction: fragment.direction,
        });
    }

    if !logical_runs.is_empty() {
        let logical_text = logical_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>();
        let logical_width: f32 = logical_runs.iter().map(|run| run.width).sum();
        if line.text == format!("{logical_text}-") {
            if let Some(last) = logical_runs.last_mut() {
                last.text.push('-');
                last.width += (line.width - logical_width).max(0.0);
            }
        }
    }

    let levels = logical_runs
        .iter()
        .map(|run| {
            Level::new_explicit(run.level).expect("line visual run levels should stay in range")
        })
        .collect::<Vec<_>>();
    let order = BidiInfo::reorder_visual(&levels);
    order
        .into_iter()
        .map(|index| logical_runs[index].clone())
        .collect()
}

pub(crate) fn line_glyph_runs(
    prepared: &PreparedText,
    line: &LayoutLine,
) -> Vec<LayoutLineGlyphRun> {
    let visual_runs = line_visual_runs(prepared, line);
    let mut run_left = 0.0f32;
    let mut output = Vec::with_capacity(visual_runs.len());

    for run in visual_runs {
        let mut glyphs = collect_run_glyphs(prepared, &run, line.text.ends_with('-'));
        shift_glyphs_x(&mut glyphs, run_left);
        output.push(LayoutLineGlyphRun {
            width: run.width,
            start: run.start,
            end: run.end,
            level: run.level,
            direction: run.direction,
            glyphs,
        });
        run_left += run.width;
    }

    output
}

fn next_line_range_internal(
    prepared: &PreparedText,
    start: &mut LayoutCursor,
    max_width: f32,
) -> Option<(LayoutLineRange, bool)> {
    let mut cursor = normalize_line_start(prepared, *start)?;
    let line_start = cursor;
    let mut line_width = 0.0f32;
    let mut last_break: Option<(LayoutCursor, f32, bool)> = None;

    loop {
        let Some((segment, grapheme)) = grapheme_at(prepared, cursor) else {
            let range = LayoutLineRange {
                width: line_width,
                start: line_start,
                end: cursor,
            };
            *start = cursor;
            return Some((range, false));
        };

        if grapheme.kind == GraphemeKind::Newline {
            let next = advance_cursor(prepared, cursor);
            let range = LayoutLineRange {
                width: line_width,
                start: line_start,
                end: next,
            };
            *start = next;
            return Some((range, false));
        }

        let advance = grapheme_advance(segment, grapheme, line_width);
        let next_width = line_width + advance;
        let next_cursor = advance_cursor(prepared, cursor);

        if next_width > max_width + LINE_FIT_EPSILON && cursor != line_start {
            if let Some((break_cursor, break_width, add_hyphen)) = last_break {
                let range = LayoutLineRange {
                    width: break_width,
                    start: line_start,
                    end: break_cursor,
                };
                *start = break_cursor;
                return Some((range, add_hyphen));
            }

            let range = LayoutLineRange {
                width: line_width,
                start: line_start,
                end: cursor,
            };
            *start = cursor;
            return Some((range, false));
        }

        line_width = next_width;
        if matches!(
            grapheme.break_after,
            crate::line_break::BreakOpportunity::Allowed
                | crate::line_break::BreakOpportunity::Forced
        ) {
            last_break = Some((
                next_cursor,
                if grapheme.kind == GraphemeKind::SoftHyphen {
                    line_width + segment.discretionary_hyphen_width
                } else {
                    line_width
                },
                grapheme.kind == GraphemeKind::SoftHyphen,
            ));
            if grapheme.break_after == crate::line_break::BreakOpportunity::Forced {
                let range = LayoutLineRange {
                    width: last_break
                        .as_ref()
                        .map(|(_, width, _)| *width)
                        .unwrap_or(line_width),
                    start: line_start,
                    end: next_cursor,
                };
                *start = next_cursor;
                return Some((range, grapheme.kind == GraphemeKind::SoftHyphen));
            }
        }

        cursor = next_cursor;
    }
}

fn segment_glyphs(glyphs: &[ShapedGlyph], byte_range: &Range<usize>) -> Arc<[ShapedGlyph]> {
    Arc::from(
        glyphs
            .iter()
            .filter(|glyph| {
                glyph.cluster_byte >= byte_range.start && glyph.cluster_byte < byte_range.end
            })
            .map(|glyph| ShapedGlyph {
                cluster_byte: glyph.cluster_byte - byte_range.start,
                ..*glyph
            })
            .collect::<Vec<_>>(),
    )
}

fn hyphen_glyphs(
    style: &TextStyleSpec,
    font_catalog: &FontCatalog,
    shape_cache: &Mutex<ShapeCache>,
) -> Arc<[ShapedGlyph]> {
    let preferred = font_catalog.resolve_style_chain(style);
    let Some(face) = font_catalog.face_for_cluster("-", &preferred) else {
        return Arc::from(Vec::<ShapedGlyph>::new());
    };
    let mut cache = shape_cache.lock();
    crate::measure::shape_run(
        "-",
        &face,
        unicode_script::Script::Latin,
        crate::bidi::BidiDirection::Ltr,
        style,
        &mut cache,
    )
}

fn collect_run_glyphs(
    prepared: &PreparedText,
    run: &LayoutLineVisualRun,
    line_has_hyphen: bool,
) -> Vec<LayoutGlyph> {
    let start_byte = cursor_byte_pos(prepared, run.start);
    let end_byte = cursor_byte_pos(prepared, run.end);
    let mut glyphs = Vec::new();
    let mut pen_x = 0.0f32;
    let segment_range = if run.direction == crate::bidi::BidiDirection::Rtl {
        EitherSegmentIter::Reverse(run.start.segment_index..=run.end.segment_index)
    } else {
        EitherSegmentIter::Forward(run.start.segment_index..=run.end.segment_index)
    };

    for segment_index in segment_range {
        let Some(segment) = prepared.core.segments.get(segment_index) else {
            continue;
        };
        let slice_start = start_byte.max(segment.byte_range.start);
        let slice_end = end_byte.min(segment.byte_range.end);
        if slice_start >= slice_end {
            continue;
        }

        let local_start = slice_start - segment.byte_range.start;
        let local_end = slice_end - segment.byte_range.start;
        for glyph in segment.glyphs.iter() {
            if glyph.cluster_byte < local_start || glyph.cluster_byte >= local_end {
                continue;
            }
            glyphs.push(LayoutGlyph {
                face_id: glyph.face_id,
                glyph_id: glyph.glyph_id,
                x: pen_x,
                advance: glyph.advance,
                x_offset: glyph.x_offset,
                y_offset: glyph.y_offset,
            });
            pen_x += glyph.advance;
        }
    }

    let extracted = extract_text(prepared, run.start, run.end, prepared.white_space());
    if line_has_hyphen && run.text.ends_with('-') && !extracted.ends_with('-') {
        for glyph in prepared.hyphen_glyphs() {
            glyphs.push(LayoutGlyph {
                face_id: glyph.face_id,
                glyph_id: glyph.glyph_id,
                x: pen_x,
                advance: glyph.advance,
                x_offset: glyph.x_offset,
                y_offset: glyph.y_offset,
            });
            pen_x += glyph.advance;
        }
    }

    glyphs
}

fn shift_glyphs_x(glyphs: &mut [LayoutGlyph], delta: f32) {
    if delta == 0.0 {
        return;
    }
    for glyph in glyphs {
        glyph.x += delta;
    }
}

fn cursor_byte_pos(prepared: &PreparedText, cursor: LayoutCursor) -> usize {
    if let Some(segment) = prepared.seg_meta().get(cursor.segment_index) {
        if let Some(grapheme) = segment.graphemes.get(cursor.grapheme_index) {
            return grapheme.byte_range.start;
        }
        return segment.byte_range.end;
    }
    prepared.text().len()
}

enum EitherSegmentIter {
    Forward(std::ops::RangeInclusive<usize>),
    Reverse(std::ops::RangeInclusive<usize>),
}

impl Iterator for EitherSegmentIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Forward(range) => range.next(),
            Self::Reverse(range) => range.next_back(),
        }
    }
}

fn materialize_line(
    prepared: &PreparedText,
    range: LayoutLineRange,
    add_hyphen: bool,
) -> LayoutLine {
    let mut text = extract_text(prepared, range.start, range.end, prepared.white_space());
    if add_hyphen {
        text.push('-');
    }

    LayoutLine {
        text,
        width: range.width,
        start: range.start,
        end: range.end,
    }
}

fn normalize_line_start(prepared: &PreparedText, mut cursor: LayoutCursor) -> Option<LayoutCursor> {
    while let Some((_, grapheme)) = grapheme_at(prepared, cursor) {
        let skip = match prepared.white_space() {
            WhiteSpaceMode::Normal => matches!(
                grapheme.kind,
                GraphemeKind::Space
                    | GraphemeKind::ZeroWidthBreak
                    | GraphemeKind::SoftHyphen
                    | GraphemeKind::WordJoiner
                    | GraphemeKind::Newline
            ),
            WhiteSpaceMode::PreWrap => matches!(
                grapheme.kind,
                GraphemeKind::ZeroWidthBreak | GraphemeKind::SoftHyphen | GraphemeKind::WordJoiner
            ),
        };
        if !skip {
            return Some(cursor);
        }
        cursor = advance_cursor(prepared, cursor);
    }

    if is_eof(prepared, cursor) {
        None
    } else {
        Some(cursor)
    }
}

fn extract_text(
    prepared: &PreparedText,
    start: LayoutCursor,
    end: LayoutCursor,
    white_space: WhiteSpaceMode,
) -> String {
    let mut text = String::new();
    let mut cursor = start;

    while cursor != end {
        let Some((_, grapheme)) = grapheme_at(prepared, cursor) else {
            break;
        };
        match grapheme.kind {
            GraphemeKind::Newline
            | GraphemeKind::SoftHyphen
            | GraphemeKind::ZeroWidthBreak
            | GraphemeKind::WordJoiner => {}
            _ => {
                let source = slice_text(prepared.text(), &grapheme.byte_range);
                text.push_str(source);
            }
        }
        cursor = advance_cursor(prepared, cursor);
    }

    if white_space == WhiteSpaceMode::Normal {
        text.trim_end_matches(' ').to_owned()
    } else {
        text
    }
}

fn grapheme_advance(segment: &SegmentMeta, grapheme: &GraphemeMeta, line_width: f32) -> f32 {
    match grapheme.kind {
        GraphemeKind::Tab => next_tab_advance(line_width, segment.tab_stop_advance),
        GraphemeKind::Newline
        | GraphemeKind::SoftHyphen
        | GraphemeKind::ZeroWidthBreak
        | GraphemeKind::WordJoiner => 0.0,
        _ => grapheme.advance,
    }
}

fn bidi_props_for_grapheme(
    prepared: &PreparedText,
    grapheme: &GraphemeMeta,
) -> (Level, crate::bidi::BidiDirection) {
    if grapheme.byte_range.is_empty() {
        return (Level::ltr(), crate::bidi::BidiDirection::Ltr);
    }

    let byte = grapheme.byte_range.start;
    prepared
        .bidi_runs()
        .iter()
        .find(|run| byte >= run.byte_range.start && byte < run.byte_range.end)
        .map(|run| (run.level, run.direction))
        .unwrap_or((Level::ltr(), crate::bidi::BidiDirection::Ltr))
}

fn next_tab_advance(line_width: f32, tab_stop_advance: f32) -> f32 {
    if tab_stop_advance <= 0.0 {
        return 0.0;
    }

    let remainder = line_width % tab_stop_advance;
    if remainder.abs() <= LINE_FIT_EPSILON {
        tab_stop_advance
    } else {
        tab_stop_advance - remainder
    }
}

fn grapheme_at(
    prepared: &PreparedText,
    cursor: LayoutCursor,
) -> Option<(&SegmentMeta, &GraphemeMeta)> {
    let segment = prepared.seg_meta().get(cursor.segment_index)?;
    let grapheme = segment.graphemes.get(cursor.grapheme_index)?;
    Some((segment, grapheme))
}

fn advance_cursor(prepared: &PreparedText, cursor: LayoutCursor) -> LayoutCursor {
    if let Some(segment) = prepared.seg_meta().get(cursor.segment_index) {
        if cursor.grapheme_index + 1 < segment.graphemes.len() {
            return LayoutCursor {
                segment_index: cursor.segment_index,
                grapheme_index: cursor.grapheme_index + 1,
            };
        }
    }

    LayoutCursor {
        segment_index: cursor.segment_index + 1,
        grapheme_index: 0,
    }
}

fn is_eof(prepared: &PreparedText, cursor: LayoutCursor) -> bool {
    prepared.seg_meta().get(cursor.segment_index).is_none()
}

fn hash_text(text: &str) -> u64 {
    let mut state = ahash::AHasher::default();
    text.hash(&mut state);
    state.finish()
}

fn hash_style(style: &TextStyleSpec) -> u64 {
    let mut state = ahash::AHasher::default();
    style.hash(&mut state);
    state.finish()
}

fn hash_atomic_placeholder(width: f32) -> u64 {
    let mut state = ahash::AHasher::default();
    "atomic-placeholder".hash(&mut state);
    ((width.max(0.0) * 64.0).round() as u32).hash(&mut state);
    state.finish()
}

fn paragraph_layout(
    prepared: &PreparedText,
    max_width: f32,
    cache: Option<&ParagraphCache>,
) -> Arc<CachedParagraphLayout> {
    if let Some(cache) = cache {
        let key = ParagraphCacheKey {
            text_hash: prepared.text_hash(),
            style_hash: prepared.style_hash(),
            width_bucket: quantize_width(max_width),
            obstacles_hash: 0,
            locale_hash: prepared.locale_hash(),
            white_space: prepared.white_space(),
            paragraph_direction: prepared.paragraph_direction(),
        };
        return cache.get_or_compute(key, || {
            Arc::new(compute_paragraph_layout(prepared, max_width))
        });
    }

    Arc::new(compute_paragraph_layout(prepared, max_width))
}

fn compute_paragraph_layout(prepared: &PreparedText, max_width: f32) -> CachedParagraphLayout {
    let mut cursor = LayoutCursor {
        segment_index: 0,
        grapheme_index: 0,
    };
    let mut lines = Vec::new();

    while let Some((range, add_hyphen)) = next_line_range_internal(prepared, &mut cursor, max_width)
    {
        lines.push(CachedParagraphLine { range, add_hyphen });
    }

    CachedParagraphLayout {
        lines: Arc::from(lines),
    }
}

#[inline]
fn quantize_width(width: f32) -> u32 {
    (width.max(0.0) / 2.0).round() as u32
}

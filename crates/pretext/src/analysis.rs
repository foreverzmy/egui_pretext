use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use icu_locale::Locale;
use icu_segmenter::{
    options::{WordBreakInvariantOptions, WordBreakOptions},
    WordSegmenter,
};
use unicode_script::UnicodeScript;
use unicode_segmentation::UnicodeSegmentation;

use crate::engine::PrepareOptions;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum WhiteSpaceMode {
    Normal,
    PreWrap,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum WordBreakMode {
    Normal,
    KeepAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GraphemeKind {
    Text,
    Space,
    Tab,
    Newline,
    SoftHyphen,
    ZeroWidthBreak,
    WordJoiner,
}

#[derive(Clone, Debug)]
pub(crate) struct AnalyzedGrapheme {
    pub byte_range: Range<usize>,
    pub kind: GraphemeKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnalysisSegmentKind {
    Text,
    Space,
    Tab,
    Newline,
    Glue,
    ZeroWidthBreak,
    SoftHyphen,
}

#[derive(Clone, Debug)]
pub(crate) struct AnalyzedSegment {
    pub byte_range: Range<usize>,
    pub grapheme_range: Range<usize>,
    #[allow(dead_code)]
    pub kind: AnalysisSegmentKind,
}

#[derive(Clone, Debug)]
pub(crate) struct TextAnalysis {
    pub normalized: String,
    pub graphemes: Vec<AnalyzedGrapheme>,
    pub segments: Vec<AnalyzedSegment>,
    pub urls: Vec<Range<usize>>,
    pub white_space: WhiteSpaceMode,
    pub word_break: WordBreakMode,
}

type LocaleWordSegmenterCache = RefCell<HashMap<String, Option<Rc<WordSegmenter>>>>;

thread_local! {
    static LOCALE_WORD_SEGMENTER_CACHE: LocaleWordSegmenterCache = RefCell::new(HashMap::new());
}

pub(crate) fn clear_runtime_caches() {
    LOCALE_WORD_SEGMENTER_CACHE.with(|cache| cache.borrow_mut().clear());
}

pub(crate) fn analyze_text(
    text: &str,
    opts: &PrepareOptions,
    locale: Option<&str>,
) -> TextAnalysis {
    let white_space = opts.white_space;
    let word_break = opts.word_break;
    let normalized = match white_space {
        WhiteSpaceMode::Normal => normalize_whitespace_normal(text),
        WhiteSpaceMode::PreWrap => normalize_whitespace_pre_wrap(text),
    };

    let graphemes: Vec<AnalyzedGrapheme> =
        UnicodeSegmentation::grapheme_indices(normalized.as_str(), true)
            .map(|(start, grapheme)| {
                let end = start + grapheme.len();
                AnalyzedGrapheme {
                    byte_range: start..end,
                    kind: classify_grapheme(grapheme, white_space),
                }
            })
            .collect();
    let urls = find_url_spans(&normalized);
    let segments = build_segments(
        &normalized,
        &graphemes,
        white_space,
        word_break,
        &urls,
        locale,
    );

    TextAnalysis {
        normalized,
        graphemes,
        segments,
        urls,
        white_space,
        word_break,
    }
}

pub fn normalize_whitespace_normal(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    let mut normalized = String::with_capacity(text.len());
    let mut saw_collapsible_space = false;

    for ch in text.chars() {
        match ch {
            ' ' | '\t' | '\n' | '\r' | '\u{000C}' => {
                saw_collapsible_space = true;
            }
            _ => {
                if saw_collapsible_space && !normalized.is_empty() {
                    normalized.push(' ');
                }
                saw_collapsible_space = false;
                normalized.push(ch);
            }
        }
    }

    if normalized.ends_with(' ') {
        normalized.pop();
    }

    normalized
}

pub fn normalize_whitespace_pre_wrap(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\u{000C}', "\n")
}

pub(crate) fn slice_text<'a>(text: &'a str, range: &Range<usize>) -> &'a str {
    &text[range.clone()]
}

pub(crate) fn is_cjk(text: &str) -> bool {
    text.chars().all(is_cjk_char)
}

pub(crate) fn contains_cjk_text(text: &str) -> bool {
    text.chars().any(is_cjk_char)
}

pub(crate) fn can_continue_keep_all_text_run(text: &str) -> bool {
    text.chars()
        .last()
        .is_some_and(|ch| !is_left_sticky_punctuation_char(ch) && !is_keep_all_glue_char(ch))
}

pub(crate) fn is_cjk_char(ch: char) -> bool {
    let c = ch as u32;
    matches!(
        c,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2CEB0..=0x2EBEF
            | 0x2EBF0..=0x2EE5D
            | 0x30000..=0x3134F
            | 0x31350..=0x323AF
            | 0x323B0..=0x33479
            | 0xF900..=0xFAFF
            | 0x2F800..=0x2FA1F
            | 0x3000..=0x303F
            | 0x3040..=0x309F
            | 0x30A0..=0x30FF
            | 0xAC00..=0xD7AF
            | 0xFF00..=0xFFEF
    )
}

fn is_keep_all_glue_char(ch: char) -> bool {
    matches!(ch, '\u{00A0}' | '\u{202F}' | '\u{2060}' | '\u{FEFF}')
}

pub(crate) fn is_cjk_line_start_prohibited(text: &str) -> bool {
    text.chars().all(|ch| {
        matches!(
            ch,
            '\u{FF0C}'
                | '\u{FF0E}'
                | '\u{FF01}'
                | '\u{FF1A}'
                | '\u{FF1B}'
                | '\u{FF1F}'
                | '\u{3001}'
                | '\u{3002}'
                | '\u{30FB}'
                | '\u{FF09}'
                | '\u{3015}'
                | '\u{3009}'
                | '\u{300B}'
                | '\u{300D}'
                | '\u{300F}'
                | '\u{3011}'
                | '\u{3017}'
                | '\u{3019}'
                | '\u{301B}'
                | '\u{30FC}'
                | '\u{3005}'
                | '\u{303B}'
                | '\u{309D}'
                | '\u{309E}'
                | '\u{30FD}'
                | '\u{30FE}'
                | '.'
                | ','
                | '!'
                | '?'
                | ':'
                | ';'
                | ')'
                | ']'
                | '}'
                | '%'
                | '"'
                | '”'
                | '’'
                | '»'
                | '›'
                | '…'
        )
    })
}

pub(crate) fn is_cjk_line_end_prohibited(text: &str) -> bool {
    text.chars().all(|ch| {
        matches!(
            ch,
            '"' | '('
                | '['
                | '{'
                | '“'
                | '‘'
                | '«'
                | '‹'
                | '\u{FF08}'
                | '\u{3014}'
                | '\u{3008}'
                | '\u{300A}'
                | '\u{300C}'
                | '\u{300E}'
                | '\u{3010}'
                | '\u{3016}'
                | '\u{3018}'
                | '\u{301A}'
        )
    })
}

fn classify_grapheme(grapheme: &str, white_space: WhiteSpaceMode) -> GraphemeKind {
    match grapheme {
        "\n" => GraphemeKind::Newline,
        "\t" if white_space == WhiteSpaceMode::PreWrap => GraphemeKind::Tab,
        "\u{00AD}" => GraphemeKind::SoftHyphen,
        "\u{200B}" => GraphemeKind::ZeroWidthBreak,
        "\u{2060}" => GraphemeKind::WordJoiner,
        " " => GraphemeKind::Space,
        _ => GraphemeKind::Text,
    }
}

fn find_url_spans(text: &str) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut start = 0usize;

    while start < text.len() {
        let remaining = &text[start..];
        let Some(offset) = remaining
            .find("http://")
            .or_else(|| remaining.find("https://"))
            .or_else(|| remaining.find("www."))
        else {
            break;
        };

        let absolute_start = start + offset;
        let mut absolute_end = text.len();
        for (idx, ch) in text[absolute_start..].char_indices() {
            if ch.is_whitespace() {
                absolute_end = absolute_start + idx;
                break;
            }
        }

        if absolute_end > absolute_start {
            spans.push(absolute_start..absolute_end);
        }
        start = absolute_end.saturating_add(1);
    }

    spans
}

#[derive(Clone, Debug)]
struct SegmentationPiece {
    byte_range: Range<usize>,
    grapheme_range: Range<usize>,
    kind: AnalysisSegmentKind,
    word_like: bool,
}

fn build_segments(
    normalized: &str,
    graphemes: &[AnalyzedGrapheme],
    white_space: WhiteSpaceMode,
    word_break: WordBreakMode,
    urls: &[Range<usize>],
    locale: Option<&str>,
) -> Vec<AnalyzedSegment> {
    if graphemes.is_empty() {
        return Vec::new();
    }

    let grapheme_starts: HashMap<usize, usize> = graphemes
        .iter()
        .enumerate()
        .map(|(index, grapheme)| (grapheme.byte_range.start, index))
        .collect();

    let mut pieces = build_initial_pieces(normalized, graphemes, white_space, locale);
    pieces = merge_initial_text_pieces(normalized, pieces);
    merge_escaped_quote_clusters_into_previous(normalized, &mut pieces);
    merge_forward_sticky_into_next(normalized, &mut pieces);
    pieces = merge_glue_connected_text_runs(normalized, word_break, pieces);
    pieces = merge_url_like_runs(normalized, urls, &grapheme_starts, graphemes, pieces);
    pieces = merge_numeric_runs(normalized, pieces);
    pieces = split_hyphenated_numeric_runs(normalized, &grapheme_starts, graphemes, pieces);
    pieces = merge_ascii_punctuation_chains(normalized, pieces);
    pieces = carry_trailing_forward_sticky_across_cjk_boundary(
        normalized,
        &grapheme_starts,
        graphemes,
        pieces,
    );
    merge_leading_marks_after_space(normalized, &mut pieces);
    if word_break == WordBreakMode::KeepAll {
        pieces = merge_keep_all_text_runs(normalized, pieces);
    }

    pieces
        .into_iter()
        .map(|piece| AnalyzedSegment {
            byte_range: piece.byte_range,
            grapheme_range: piece.grapheme_range,
            kind: piece.kind,
        })
        .collect()
}

fn build_initial_pieces(
    normalized: &str,
    graphemes: &[AnalyzedGrapheme],
    white_space: WhiteSpaceMode,
    locale: Option<&str>,
) -> Vec<SegmentationPiece> {
    let mut pieces = Vec::new();
    let mut grapheme_index = 0usize;

    for piece_range in word_piece_ranges(normalized, graphemes, locale) {
        let piece_start = piece_range.start;
        let piece_end = piece_range.end;
        let mut current_kind: Option<AnalysisSegmentKind> = None;
        let mut current_grapheme_start = grapheme_index;
        let mut current_piece_start = piece_start;

        while grapheme_index < graphemes.len()
            && graphemes[grapheme_index].byte_range.start < piece_end
        {
            let grapheme = &graphemes[grapheme_index];
            let grapheme_text = slice_text(normalized, &grapheme.byte_range);
            let kind = classify_segment_kind(grapheme_text, grapheme.kind, white_space);

            if current_kind.is_some_and(|current| current != kind) {
                let byte_range = current_piece_start..grapheme.byte_range.start;
                pieces.push(SegmentationPiece {
                    word_like: is_word_like_text(slice_text(normalized, &byte_range)),
                    byte_range,
                    grapheme_range: current_grapheme_start..grapheme_index,
                    kind: current_kind.expect("current kind must exist"),
                });
                current_piece_start = grapheme.byte_range.start;
                current_grapheme_start = grapheme_index;
            }

            current_kind = Some(kind);
            grapheme_index += 1;
        }

        if let Some(kind) = current_kind {
            let byte_range = current_piece_start..piece_end;
            pieces.push(SegmentationPiece {
                word_like: is_word_like_text(slice_text(normalized, &byte_range)),
                byte_range,
                grapheme_range: current_grapheme_start..grapheme_index,
                kind,
            });
        }
    }

    pieces
}

fn locale_word_segmenter(locale: Option<&str>) -> Option<Rc<WordSegmenter>> {
    let locale = locale?.trim();
    if locale.is_empty() {
        return None;
    }

    LOCALE_WORD_SEGMENTER_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(locale).cloned() {
            return cached;
        }

        let segmenter = build_locale_word_segmenter(locale);
        cache
            .borrow_mut()
            .insert(locale.to_owned(), segmenter.clone());
        segmenter
    })
}

fn build_locale_word_segmenter(locale: &str) -> Option<Rc<WordSegmenter>> {
    let locale = locale.parse::<Locale>().ok()?;
    let mut options = WordBreakOptions::default();
    options.content_locale = Some(&locale.id);
    WordSegmenter::try_new_auto(options).ok().map(Rc::new)
}

fn word_piece_ranges(
    normalized: &str,
    graphemes: &[AnalyzedGrapheme],
    locale: Option<&str>,
) -> Vec<Range<usize>> {
    let boundaries = if let Some(segmenter) = locale_word_segmenter(locale) {
        segmenter
            .as_borrowed()
            .segment_str(normalized)
            .collect::<Vec<_>>()
    } else {
        WordSegmenter::new_auto(WordBreakInvariantOptions::default())
            .segment_str(normalized)
            .collect::<Vec<_>>()
    };

    boundaries
        .windows(2)
        .filter_map(|window| {
            let start = window[0];
            let end = window[1];
            (start < end).then_some(start..end)
        })
        .flat_map(|piece_range| {
            let piece_text = slice_text(normalized, &piece_range);
            if !is_cjk(piece_text) {
                return vec![piece_range];
            }

            graphemes
                .iter()
                .filter(|grapheme| {
                    grapheme.byte_range.start >= piece_range.start
                        && grapheme.byte_range.start < piece_range.end
                })
                .map(|grapheme| grapheme.byte_range.clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn merge_initial_text_pieces(
    normalized: &str,
    pieces: Vec<SegmentationPiece>,
) -> Vec<SegmentationPiece> {
    let mut merged: Vec<SegmentationPiece> = Vec::with_capacity(pieces.len());

    for piece in pieces {
        if let Some(previous) = merged.last_mut() {
            if previous.kind == AnalysisSegmentKind::Text
                && piece.kind == AnalysisSegmentKind::Text
                && should_merge_text_piece(normalized, previous, &piece)
            {
                previous.byte_range.end = piece.byte_range.end;
                previous.grapheme_range.end = piece.grapheme_range.end;
                previous.word_like = previous.word_like || piece.word_like;
                continue;
            }
        }

        merged.push(piece);
    }

    merged
}

fn should_merge_text_piece(
    normalized: &str,
    previous: &SegmentationPiece,
    next: &SegmentationPiece,
) -> bool {
    let previous_text = slice_text(normalized, &previous.byte_range);
    let next_text = slice_text(normalized, &next.byte_range);

    if is_cjk(next_text) && is_cjk(previous_text) && is_cjk_line_start_prohibited(next_text) {
        return true;
    }

    if ends_with_myanmar_medial_glue(previous_text) {
        return true;
    }

    if next.word_like
        && contains_arabic_script(next_text)
        && ends_with_arabic_no_space_punctuation(previous_text)
    {
        return true;
    }

    if !next.word_like
        && next_text != "-"
        && next_text != "—"
        && is_repeated_single_char_run(previous_text, next_text)
    {
        return true;
    }

    if !previous.word_like && is_forward_sticky_cluster_segment(previous_text) {
        return true;
    }

    !next.word_like
        && (is_left_sticky_punctuation_segment(next_text)
            || (next_text == "-" && previous.word_like))
}

fn merge_escaped_quote_clusters_into_previous(
    normalized: &str,
    pieces: &mut Vec<SegmentationPiece>,
) {
    let mut index = 1usize;
    while index < pieces.len() {
        let merge = pieces[index].kind == AnalysisSegmentKind::Text
            && !pieces[index].word_like
            && pieces[index - 1].kind == AnalysisSegmentKind::Text
            && is_escaped_quote_cluster_segment(slice_text(normalized, &pieces[index].byte_range));

        if merge {
            let current = pieces.remove(index);
            pieces[index - 1].byte_range.end = current.byte_range.end;
            pieces[index - 1].grapheme_range.end = current.grapheme_range.end;
            pieces[index - 1].word_like = pieces[index - 1].word_like || current.word_like;
        } else {
            index += 1;
        }
    }
}

fn merge_forward_sticky_into_next(normalized: &str, pieces: &mut Vec<SegmentationPiece>) {
    let mut index = 0usize;
    while index + 1 < pieces.len() {
        let merge = pieces[index].kind == AnalysisSegmentKind::Text
            && !pieces[index].word_like
            && pieces[index + 1].kind == AnalysisSegmentKind::Text
            && is_forward_sticky_cluster_segment(slice_text(normalized, &pieces[index].byte_range));

        if merge {
            let current = pieces.remove(index);
            pieces[index].byte_range.start = current.byte_range.start;
            pieces[index].grapheme_range.start = current.grapheme_range.start;
            pieces[index].word_like = pieces[index].word_like || current.word_like;
            index = index.saturating_sub(1);
        } else {
            index += 1;
        }
    }
}

fn merge_glue_connected_text_runs(
    normalized: &str,
    word_break: WordBreakMode,
    pieces: Vec<SegmentationPiece>,
) -> Vec<SegmentationPiece> {
    let mut merged = Vec::with_capacity(pieces.len());
    let mut read = 0usize;

    while read < pieces.len() {
        let mut piece = pieces[read].clone();

        if piece.kind == AnalysisSegmentKind::Glue {
            let glue_start = piece.byte_range.start;
            let glue_grapheme_start = piece.grapheme_range.start;
            read += 1;
            while read < pieces.len() && pieces[read].kind == AnalysisSegmentKind::Glue {
                piece.byte_range.end = pieces[read].byte_range.end;
                piece.grapheme_range.end = pieces[read].grapheme_range.end;
                read += 1;
            }

            if read < pieces.len() && pieces[read].kind == AnalysisSegmentKind::Text {
                piece.kind = AnalysisSegmentKind::Text;
                piece.byte_range.end = pieces[read].byte_range.end;
                piece.grapheme_range.end = pieces[read].grapheme_range.end;
                piece.byte_range.start = glue_start;
                piece.grapheme_range.start = glue_grapheme_start;
                piece.word_like = pieces[read].word_like;
                read += 1;
            } else {
                merged.push(piece);
                continue;
            }
        } else {
            read += 1;
        }

        if piece.kind == AnalysisSegmentKind::Text {
            while read < pieces.len() && pieces[read].kind == AnalysisSegmentKind::Glue {
                let glue = pieces[read].clone();
                read += 1;
                let mut trailing_glue = glue.clone();
                while read < pieces.len() && pieces[read].kind == AnalysisSegmentKind::Glue {
                    trailing_glue.byte_range.end = pieces[read].byte_range.end;
                    trailing_glue.grapheme_range.end = pieces[read].grapheme_range.end;
                    read += 1;
                }

                if read < pieces.len() && pieces[read].kind == AnalysisSegmentKind::Text {
                    let current_text = slice_text(normalized, &piece.byte_range);
                    let next_text = slice_text(normalized, &pieces[read].byte_range);
                    if word_break == WordBreakMode::KeepAll
                        && !contains_cjk_text(current_text)
                        && contains_cjk_text(next_text)
                    {
                        piece.byte_range.end = trailing_glue.byte_range.end;
                        piece.grapheme_range.end = trailing_glue.grapheme_range.end;
                        break;
                    }
                    piece.byte_range.end = pieces[read].byte_range.end;
                    piece.grapheme_range.end = pieces[read].grapheme_range.end;
                    piece.word_like = piece.word_like || pieces[read].word_like;
                    read += 1;
                } else {
                    piece.byte_range.end = trailing_glue.byte_range.end;
                    piece.grapheme_range.end = trailing_glue.grapheme_range.end;
                }
            }
        }

        merged.push(piece);
    }

    merged
}

fn merge_url_like_runs(
    normalized: &str,
    urls: &[Range<usize>],
    grapheme_starts: &HashMap<usize, usize>,
    graphemes: &[AnalyzedGrapheme],
    pieces: Vec<SegmentationPiece>,
) -> Vec<SegmentationPiece> {
    if urls.is_empty() {
        return pieces;
    }

    let mut merged = Vec::new();
    let mut piece_index = 0usize;
    let mut url_index = 0usize;

    while piece_index < pieces.len() {
        while url_index < urls.len() && pieces[piece_index].byte_range.start >= urls[url_index].end
        {
            url_index += 1;
        }

        if url_index < urls.len() {
            let span = &urls[url_index];
            if pieces[piece_index].byte_range.start >= span.start
                && pieces[piece_index].byte_range.end <= span.end
            {
                while piece_index < pieces.len() && pieces[piece_index].byte_range.end <= span.end {
                    piece_index += 1;
                }

                let url_text = &normalized[span.clone()];
                if let Some(query_offset) = url_text.find('?') {
                    let query_split = span.start + query_offset + '?'.len_utf8();
                    if query_split < span.end {
                        merged.push(make_text_piece(
                            span.start,
                            query_split,
                            grapheme_starts,
                            graphemes,
                        ));
                        merged.push(make_text_piece(
                            query_split,
                            span.end,
                            grapheme_starts,
                            graphemes,
                        ));
                        url_index += 1;
                        continue;
                    }
                }

                merged.push(make_text_piece(
                    span.start,
                    span.end,
                    grapheme_starts,
                    graphemes,
                ));
                url_index += 1;
                continue;
            }
        }

        merged.push(pieces[piece_index].clone());
        piece_index += 1;
    }

    merged
}

fn merge_numeric_runs(normalized: &str, pieces: Vec<SegmentationPiece>) -> Vec<SegmentationPiece> {
    let mut merged = Vec::new();
    let mut index = 0usize;

    while index < pieces.len() {
        let piece = &pieces[index];
        let piece_text = slice_text(normalized, &piece.byte_range);

        if piece.kind == AnalysisSegmentKind::Text
            && is_numeric_run_segment(piece_text)
            && contains_decimal_digit(piece_text)
        {
            let mut merged_piece = piece.clone();
            index += 1;
            while index < pieces.len()
                && pieces[index].kind == AnalysisSegmentKind::Text
                && is_numeric_run_segment(slice_text(normalized, &pieces[index].byte_range))
            {
                merged_piece.byte_range.end = pieces[index].byte_range.end;
                merged_piece.grapheme_range.end = pieces[index].grapheme_range.end;
                merged_piece.word_like = true;
                index += 1;
            }
            merged.push(merged_piece);
            continue;
        }

        merged.push(piece.clone());
        index += 1;
    }

    merged
}

fn split_hyphenated_numeric_runs(
    normalized: &str,
    grapheme_starts: &HashMap<usize, usize>,
    graphemes: &[AnalyzedGrapheme],
    pieces: Vec<SegmentationPiece>,
) -> Vec<SegmentationPiece> {
    let mut split = Vec::new();

    for piece in pieces {
        let text = slice_text(normalized, &piece.byte_range);
        if piece.kind == AnalysisSegmentKind::Text && text.contains('-') {
            let parts: Vec<&str> = text.split('-').collect();
            let should_split = parts.len() > 1
                && parts.iter().all(|part| {
                    !part.is_empty() && contains_decimal_digit(part) && is_numeric_run_segment(part)
                });

            if should_split {
                let mut offset = piece.byte_range.start;
                for (index, part) in parts.iter().enumerate() {
                    let mut part_end = offset + part.len();
                    if index + 1 < parts.len() {
                        part_end += 1;
                    }
                    split.push(make_text_piece(
                        offset,
                        part_end,
                        grapheme_starts,
                        graphemes,
                    ));
                    offset = part_end;
                }
                continue;
            }
        }

        split.push(piece);
    }

    split
}

fn merge_ascii_punctuation_chains(
    normalized: &str,
    pieces: Vec<SegmentationPiece>,
) -> Vec<SegmentationPiece> {
    let mut merged = Vec::new();
    let mut index = 0usize;

    while index < pieces.len() {
        let piece = &pieces[index];
        let text = slice_text(normalized, &piece.byte_range);

        if piece.kind == AnalysisSegmentKind::Text
            && piece.word_like
            && is_ascii_punctuation_chain_segment(text)
        {
            let mut merged_piece = piece.clone();
            let mut merged_text = text.to_owned();
            index += 1;

            while has_ascii_chain_trailing_joiners(&merged_text)
                && index < pieces.len()
                && pieces[index].kind == AnalysisSegmentKind::Text
                && pieces[index].word_like
                && is_ascii_punctuation_chain_segment(slice_text(
                    normalized,
                    &pieces[index].byte_range,
                ))
            {
                merged_piece.byte_range.end = pieces[index].byte_range.end;
                merged_piece.grapheme_range.end = pieces[index].grapheme_range.end;
                merged_text.push_str(slice_text(normalized, &pieces[index].byte_range));
                index += 1;
            }

            merged.push(merged_piece);
            continue;
        }

        merged.push(piece.clone());
        index += 1;
    }

    merged
}

fn carry_trailing_forward_sticky_across_cjk_boundary(
    normalized: &str,
    grapheme_starts: &HashMap<usize, usize>,
    graphemes: &[AnalyzedGrapheme],
    mut pieces: Vec<SegmentationPiece>,
) -> Vec<SegmentationPiece> {
    if pieces.len() < 2 {
        return pieces;
    }

    for index in 0..pieces.len() - 1 {
        if pieces[index].kind != AnalysisSegmentKind::Text
            || pieces[index + 1].kind != AnalysisSegmentKind::Text
        {
            continue;
        }

        let left = slice_text(normalized, &pieces[index].byte_range);
        let right = slice_text(normalized, &pieces[index + 1].byte_range);
        if !is_cjk(left) || !is_cjk(right) {
            continue;
        }

        let Some(split_offset) = split_trailing_forward_sticky_cluster(left) else {
            continue;
        };
        let split_byte = pieces[index].byte_range.start + split_offset;

        pieces[index].byte_range.end = split_byte;
        pieces[index].grapheme_range.end =
            grapheme_index_for_byte(grapheme_starts, graphemes, split_byte);
        pieces[index + 1].byte_range.start = split_byte;
        pieces[index + 1].grapheme_range.start =
            grapheme_index_for_byte(grapheme_starts, graphemes, split_byte);
    }

    pieces
}

fn merge_leading_marks_after_space(normalized: &str, pieces: &mut Vec<SegmentationPiece>) {
    let mut index = 1usize;
    while index + 1 < pieces.len() {
        let merge = matches!(
            pieces[index - 1].kind,
            AnalysisSegmentKind::Space | AnalysisSegmentKind::Glue
        ) && pieces[index].kind == AnalysisSegmentKind::Text
            && !pieces[index].word_like
            && is_marks_only(slice_text(normalized, &pieces[index].byte_range))
            && pieces[index + 1].kind == AnalysisSegmentKind::Text
            && contains_arabic_script(slice_text(normalized, &pieces[index + 1].byte_range));

        if merge {
            let current = pieces.remove(index);
            pieces[index].byte_range.start = current.byte_range.start;
            pieces[index].grapheme_range.start = current.grapheme_range.start;
        } else {
            index += 1;
        }
    }
}

fn merge_keep_all_text_runs(
    normalized: &str,
    pieces: Vec<SegmentationPiece>,
) -> Vec<SegmentationPiece> {
    let mut merged: Vec<SegmentationPiece> = Vec::with_capacity(pieces.len());

    for piece in pieces {
        if let Some(previous) = merged.last_mut() {
            let previous_text = slice_text(normalized, &previous.byte_range);
            if previous.kind == AnalysisSegmentKind::Text
                && piece.kind == AnalysisSegmentKind::Text
                && can_continue_keep_all_text_run(previous_text)
                && contains_cjk_text(previous_text)
            {
                previous.byte_range.end = piece.byte_range.end;
                previous.grapheme_range.end = piece.grapheme_range.end;
                previous.word_like = previous.word_like || piece.word_like;
                continue;
            }
        }

        merged.push(piece);
    }

    merged
}

fn make_text_piece(
    start: usize,
    end: usize,
    grapheme_starts: &HashMap<usize, usize>,
    graphemes: &[AnalyzedGrapheme],
) -> SegmentationPiece {
    SegmentationPiece {
        byte_range: start..end,
        grapheme_range: grapheme_index_for_byte(grapheme_starts, graphemes, start)
            ..grapheme_index_for_byte(grapheme_starts, graphemes, end),
        kind: AnalysisSegmentKind::Text,
        word_like: true,
    }
}

fn grapheme_index_for_byte(
    grapheme_starts: &HashMap<usize, usize>,
    graphemes: &[AnalyzedGrapheme],
    byte: usize,
) -> usize {
    grapheme_starts
        .get(&byte)
        .copied()
        .unwrap_or_else(|| graphemes.len())
}

fn classify_segment_kind(
    grapheme: &str,
    grapheme_kind: GraphemeKind,
    white_space: WhiteSpaceMode,
) -> AnalysisSegmentKind {
    match grapheme_kind {
        GraphemeKind::Space => AnalysisSegmentKind::Space,
        GraphemeKind::Tab if white_space == WhiteSpaceMode::PreWrap => AnalysisSegmentKind::Tab,
        GraphemeKind::Newline if white_space == WhiteSpaceMode::PreWrap => {
            AnalysisSegmentKind::Newline
        }
        GraphemeKind::ZeroWidthBreak => AnalysisSegmentKind::ZeroWidthBreak,
        GraphemeKind::SoftHyphen => AnalysisSegmentKind::SoftHyphen,
        GraphemeKind::WordJoiner => AnalysisSegmentKind::Glue,
        _ if matches!(grapheme, "\u{00A0}" | "\u{202F}" | "\u{FEFF}") => AnalysisSegmentKind::Glue,
        _ => AnalysisSegmentKind::Text,
    }
}

fn is_word_like_text(text: &str) -> bool {
    let mut saw_word = false;
    for ch in text.chars() {
        if is_word_char(ch) {
            saw_word = true;
            continue;
        }
        if is_combining_mark(ch) {
            continue;
        }
        return false;
    }
    saw_word
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric()
        || ch == '_'
        || (is_cjk_char(ch)
            && !ch.is_whitespace()
            && !is_left_sticky_punctuation_char(ch)
            && !is_forward_sticky_char(ch))
}

fn is_combining_mark(ch: char) -> bool {
    matches!(
        ch as u32,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05BD
            | 0x05BF
            | 0x05C1..=0x05C2
            | 0x05C4..=0x05C5
            | 0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x06E7..=0x06E8
            | 0x06EA..=0x06ED
            | 0x0711
            | 0x0730..=0x074A
            | 0x07A6..=0x07B0
            | 0x07EB..=0x07F3
            | 0x0816..=0x0819
            | 0x081B..=0x0823
            | 0x0825..=0x0827
            | 0x0829..=0x082D
            | 0x0859..=0x085B
            | 0x08D3..=0x08FF
            | 0x1AB0..=0x1AFF
            | 0x1DC0..=0x1DFF
            | 0x20D0..=0x20FF
            | 0xFE20..=0xFE2F
    )
}

fn contains_arabic_script(text: &str) -> bool {
    text.chars()
        .any(|ch| ch.script() == unicode_script::Script::Arabic)
}

fn ends_with_arabic_no_space_punctuation(text: &str) -> bool {
    text.chars()
        .last()
        .is_some_and(|ch| matches!(ch, ':' | '.' | '\u{060C}' | '\u{061B}'))
}

fn ends_with_myanmar_medial_glue(text: &str) -> bool {
    text.chars().last().is_some_and(|ch| ch == '\u{104F}')
}

fn is_left_sticky_punctuation_segment(segment: &str) -> bool {
    let mut saw_punctuation = false;
    for ch in segment.chars() {
        if is_left_sticky_punctuation_char(ch) {
            saw_punctuation = true;
            continue;
        }
        if saw_punctuation && is_combining_mark(ch) {
            continue;
        }
        return false;
    }
    saw_punctuation
}

fn is_forward_sticky_cluster_segment(segment: &str) -> bool {
    if is_escaped_quote_cluster_segment(segment) {
        return true;
    }

    !segment.is_empty()
        && segment
            .chars()
            .all(|ch| is_forward_sticky_char(ch) || is_combining_mark(ch))
}

fn is_escaped_quote_cluster_segment(segment: &str) -> bool {
    let mut saw_quote = false;
    for ch in segment.chars() {
        if ch == '\\' || is_combining_mark(ch) {
            continue;
        }
        if is_forward_sticky_char(ch) || is_left_sticky_punctuation_char(ch) {
            saw_quote = true;
            continue;
        }
        return false;
    }
    saw_quote
}

fn is_left_sticky_punctuation_char(ch: char) -> bool {
    matches!(
        ch,
        '.' | ','
            | '!'
            | '?'
            | ':'
            | ';'
            | ')'
            | ']'
            | '}'
            | '%'
            | '"'
            | '”'
            | '’'
            | '»'
            | '›'
            | '…'
            | '\u{060C}'
            | '\u{061B}'
            | '\u{061F}'
            | '\u{0964}'
            | '\u{0965}'
            | '\u{104A}'
            | '\u{104B}'
            | '\u{104C}'
            | '\u{104D}'
            | '\u{104F}'
            | '\u{FF0C}'
            | '\u{FF0E}'
            | '\u{FF01}'
            | '\u{FF1A}'
            | '\u{FF1B}'
            | '\u{FF1F}'
            | '\u{3001}'
            | '\u{3002}'
            | '\u{30FB}'
            | '\u{FF09}'
            | '\u{3015}'
            | '\u{3009}'
            | '\u{300B}'
            | '\u{300D}'
            | '\u{300F}'
            | '\u{3011}'
            | '\u{3017}'
            | '\u{3019}'
            | '\u{301B}'
            | '\u{30FC}'
            | '\u{3005}'
            | '\u{303B}'
            | '\u{309D}'
            | '\u{309E}'
            | '\u{30FD}'
            | '\u{30FE}'
    )
}

fn is_forward_sticky_char(ch: char) -> bool {
    matches!(
        ch,
        '"' | '('
            | '['
            | '{'
            | '“'
            | '‘'
            | '«'
            | '‹'
            | '\''
            | '’'
            | '\u{FF08}'
            | '\u{3014}'
            | '\u{3008}'
            | '\u{300A}'
            | '\u{300C}'
            | '\u{300E}'
            | '\u{3010}'
            | '\u{3016}'
            | '\u{3018}'
            | '\u{301A}'
    )
}

fn split_trailing_forward_sticky_cluster(text: &str) -> Option<usize> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut split_index = chars.len();

    while split_index > 0 {
        let ch = chars[split_index - 1].1;
        if is_combining_mark(ch) || is_forward_sticky_char(ch) {
            split_index -= 1;
            continue;
        }
        break;
    }

    if split_index == 0 || split_index == chars.len() {
        return None;
    }

    Some(chars[split_index].0)
}

fn is_repeated_single_char_run(segment: &str, ch_segment: &str) -> bool {
    let mut chars = ch_segment.chars();
    let Some(ch) = chars.next() else {
        return false;
    };
    if chars.next().is_some() {
        return false;
    }

    !segment.is_empty() && segment.chars().all(|part| part == ch)
}

fn contains_decimal_digit(text: &str) -> bool {
    text.chars().any(|ch| ch.to_digit(10).is_some())
}

fn is_numeric_run_segment(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch.to_digit(10).is_some() || is_numeric_joiner_char(ch))
}

fn is_numeric_joiner_char(ch: char) -> bool {
    matches!(
        ch,
        ':' | '-' | '/' | '×' | ',' | '.' | '+' | '\u{2013}' | '\u{2014}'
    )
}

fn is_ascii_punctuation_chain_segment(text: &str) -> bool {
    let mut saw_word = false;
    let mut saw_trailing_joiner = false;

    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if saw_trailing_joiner {
                return false;
            }
            saw_word = true;
            continue;
        }

        if saw_word && matches!(ch, ',' | ':' | ';') {
            saw_trailing_joiner = true;
            continue;
        }

        return false;
    }

    saw_word
}

fn has_ascii_chain_trailing_joiners(text: &str) -> bool {
    let mut saw_joiner = false;
    for ch in text.chars().rev() {
        if matches!(ch, ',' | ':' | ';') {
            saw_joiner = true;
            continue;
        }
        break;
    }
    saw_joiner
}

fn is_marks_only(text: &str) -> bool {
    !text.is_empty() && text.chars().all(is_combining_mark)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::PrepareOptions;

    fn segment_texts(text: &str, white_space: WhiteSpaceMode) -> Vec<String> {
        segment_texts_with_locale(text, white_space, None)
    }

    fn segment_texts_with_word_break(
        text: &str,
        white_space: WhiteSpaceMode,
        word_break: WordBreakMode,
    ) -> Vec<String> {
        let analysis = analyze_text(
            text,
            &PrepareOptions {
                white_space,
                word_break,
                paragraph_direction: crate::bidi::ParagraphDirection::Auto,
            },
            None,
        );
        analysis
            .segments
            .iter()
            .map(|segment| slice_text(&analysis.normalized, &segment.byte_range).to_owned())
            .collect()
    }

    fn segment_texts_with_locale(
        text: &str,
        white_space: WhiteSpaceMode,
        locale: Option<&str>,
    ) -> Vec<String> {
        let analysis = analyze_text(
            text,
            &PrepareOptions {
                white_space,
                word_break: WordBreakMode::Normal,
                paragraph_direction: crate::bidi::ParagraphDirection::Auto,
            },
            locale,
        );
        analysis
            .segments
            .iter()
            .map(|segment| slice_text(&analysis.normalized, &segment.byte_range).to_owned())
            .collect()
    }

    #[test]
    fn normal_mode_builds_expected_segments() {
        assert_eq!(
            segment_texts("  Hello\t \n  World  ", WhiteSpaceMode::Normal),
            vec!["Hello".to_owned(), " ".to_owned(), "World".to_owned()]
        );
    }

    #[test]
    fn pre_wrap_keeps_spaces_tabs_and_breaks_visible() {
        assert_eq!(
            segment_texts("Hello\tWorld\n  Tail", WhiteSpaceMode::PreWrap),
            vec![
                "Hello".to_owned(),
                "\t".to_owned(),
                "World".to_owned(),
                "\n".to_owned(),
                "  ".to_owned(),
                "Tail".to_owned()
            ]
        );
    }

    #[test]
    fn segmentation_keeps_urls_numeric_runs_and_punctuation_attached() {
        assert_eq!(
            segment_texts(
                "see https://example.com/reports/q3?lang=ar&mode=full now",
                WhiteSpaceMode::Normal
            ),
            vec![
                "see".to_owned(),
                " ".to_owned(),
                "https://example.com/reports/q3?".to_owned(),
                "lang=ar&mode=full".to_owned(),
                " ".to_owned(),
                "now".to_owned()
            ]
        );
        assert_eq!(
            segment_texts("window 7:00-9:00 only", WhiteSpaceMode::Normal),
            vec![
                "window".to_owned(),
                " ".to_owned(),
                "7:00-".to_owned(),
                "9:00".to_owned(),
                " ".to_owned(),
                "only".to_owned()
            ]
        );
        assert_eq!(
            segment_texts("foo;bar === heading ===", WhiteSpaceMode::Normal),
            vec![
                "foo;bar".to_owned(),
                " ".to_owned(),
                "===".to_owned(),
                " ".to_owned(),
                "heading".to_owned(),
                " ".to_owned(),
                "===".to_owned()
            ]
        );
    }

    #[test]
    fn segmentation_applies_cjk_and_arabic_attachment_rules() {
        assert_eq!(
            segment_texts("中文，测试。", WhiteSpaceMode::Normal),
            vec![
                "中".to_owned(),
                "文，".to_owned(),
                "测".to_owned(),
                "试。".to_owned()
            ]
        );
        assert_eq!(
            segment_texts("作者はさつき、「下人", WhiteSpaceMode::Normal),
            vec![
                "作".to_owned(),
                "者".to_owned(),
                "は".to_owned(),
                "さ".to_owned(),
                "つ".to_owned(),
                "き、".to_owned(),
                "「下".to_owned(),
                "人".to_owned()
            ]
        );
        assert_eq!(
            segment_texts("مرحبا، عالم؟", WhiteSpaceMode::Normal),
            vec!["مرحبا،".to_owned(), " ".to_owned(), "عالم؟".to_owned()]
        );
        assert_eq!(
            segment_texts("فيقول:وعليك السلام", WhiteSpaceMode::Normal),
            vec!["فيقول:وعليك".to_owned(), " ".to_owned(), "السلام".to_owned()]
        );
    }

    #[test]
    fn locale_can_change_word_boundaries() {
        assert_eq!(
            segment_texts_with_locale("EU:ssä", WhiteSpaceMode::Normal, None),
            vec!["EU:".to_owned(), "ssä".to_owned()]
        );
        assert_eq!(
            segment_texts_with_locale("EU:ssä", WhiteSpaceMode::Normal, Some("fi")),
            vec!["EU:ssä".to_owned()]
        );
    }

    #[test]
    fn keep_all_keeps_cjk_leading_no_space_runs_cohesive() {
        assert_eq!(
            segment_texts_with_word_break(
                "中文，测试。",
                WhiteSpaceMode::Normal,
                WordBreakMode::KeepAll
            ),
            vec!["中文，".to_owned(), "测试。".to_owned()]
        );
        assert_eq!(
            segment_texts_with_word_break(
                "한국어테스트",
                WhiteSpaceMode::Normal,
                WordBreakMode::KeepAll
            ),
            vec!["한국어테스트".to_owned()]
        );
        assert_eq!(
            segment_texts_with_word_break(
                "日本語foo-bar",
                WhiteSpaceMode::Normal,
                WordBreakMode::KeepAll
            ),
            vec!["日本語foo-bar".to_owned()]
        );
        assert_eq!(
            segment_texts_with_word_break(
                "foo-bar日本語",
                WhiteSpaceMode::Normal,
                WordBreakMode::KeepAll
            ),
            vec!["foo-".to_owned(), "bar".to_owned(), "日本語".to_owned()]
        );
        assert_eq!(
            segment_texts_with_word_break(
                "foo\u{00A0}世界",
                WhiteSpaceMode::Normal,
                WordBreakMode::KeepAll
            ),
            vec!["foo\u{00A0}".to_owned(), "世界".to_owned()]
        );
    }

    #[test]
    fn is_cjk_covers_newer_extension_blocks() {
        assert!(is_cjk("\u{2EBF0}"));
        assert!(is_cjk("\u{31350}"));
        assert!(is_cjk("\u{323B0}"));
        assert!(!is_cjk("hello"));
    }
}

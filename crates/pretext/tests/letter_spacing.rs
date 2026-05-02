mod support;

use pretext::advanced::LayoutCursor;
use pretext::rich_inline::{
    measure_rich_inline_stats, prepare_rich_inline, RichInlineBreakMode, RichInlineItemSpec,
};
use pretext::{ParagraphDirection, PretextParagraphOptions, WhiteSpaceMode};

const EPSILON: f32 = 0.05;

fn options(letter_spacing: f32) -> PretextParagraphOptions {
    PretextParagraphOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
        letter_spacing,
        ..PretextParagraphOptions::default()
    }
}

fn pre_wrap_options(letter_spacing: f32) -> PretextParagraphOptions {
    PretextParagraphOptions {
        white_space: WhiteSpaceMode::PreWrap,
        paragraph_direction: ParagraphDirection::Auto,
        letter_spacing,
        ..PretextParagraphOptions::default()
    }
}

#[track_caller]
fn assert_close(left: f32, right: f32) {
    assert!(
        (left - right).abs() <= EPSILON,
        "expected {left} to be close to {right}"
    );
}

#[test]
fn letter_spacing_adds_inter_grapheme_gaps_without_trailing_gap() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = 4.0;

    let single = engine.prepare_paragraph("A", &style, &options(spacing));
    let pair = engine.prepare_paragraph("AB", &style, &options(spacing));
    let segmented = engine.prepare_paragraph("A B", &style, &options(spacing));

    assert_close(
        engine.layout_with_lines(&single, 200.0, 20.0).lines[0].width,
        engine.prefix_widths("A", &style)[1],
    );
    assert_close(
        engine.layout_with_lines(&pair, 200.0, 20.0).lines[0].width,
        engine.prefix_widths("AB", &style)[2] + spacing,
    );
    assert_close(
        engine.layout_with_lines(&segmented, 200.0, 20.0).lines[0].width,
        engine.prefix_widths("A B", &style)[3] + spacing * 2.0,
    );
}

#[test]
fn letter_spacing_wraps_inside_words_with_line_local_gaps() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = 5.0;
    let prepared = engine.prepare_paragraph("abcd", &style, &options(spacing));
    let two_graphemes_width = engine.prefix_widths("ab", &style)[2] + spacing;
    let wrapped = engine.layout_with_lines(&prepared, two_graphemes_width + spacing + 0.1, 20.0);

    assert_eq!(wrapped.line_count, 2);
    assert_eq!(wrapped.lines[0].text, "ab");
    assert_eq!(wrapped.lines[1].text, "cd");
    assert_close(wrapped.lines[0].width, two_graphemes_width);
    assert_close(
        wrapped.lines[1].width,
        engine.prefix_widths("cd", &style)[2] + spacing,
    );
    assert_eq!(
        engine
            .layout(
                prepared.as_prepared(),
                two_graphemes_width + spacing + 0.1,
                20.0
            )
            .line_count,
        wrapped.line_count
    );
}

#[test]
fn letter_spacing_trims_gap_before_hanging_collapsible_space() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = 6.0;
    let line_a_width = engine.prefix_widths("A", &style)[1];
    let prepared = engine.prepare_paragraph("A B", &style, &options(spacing));
    let wrapped = engine.layout_with_lines(&prepared, line_a_width + 0.1, 20.0);

    assert_eq!(wrapped.lines[0].text, "A");
    assert_close(wrapped.lines[0].width, line_a_width);
}

#[test]
fn letter_spacing_applies_across_cjk_emoji_and_punctuation() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = 2.0;
    let text = "A😀春?";
    let prepared = engine.prepare_paragraph(text, &style, &options(spacing));
    let line = &engine.layout_with_lines(&prepared, 300.0, 20.0).lines[0];
    let base_width = engine
        .prefix_widths(text, &style)
        .last()
        .copied()
        .unwrap_or(0.0);

    assert_eq!(line.text, text);
    assert_close(line.width, base_width + spacing * 3.0);
}

#[test]
fn negative_letter_spacing_tightens_inter_grapheme_gaps() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = -1.5;
    let prepared = engine.prepare_paragraph("AB", &style, &options(spacing));
    let line = &engine.layout_with_lines(&prepared, 200.0, 20.0).lines[0];

    assert_close(line.width, engine.prefix_widths("AB", &style)[2] + spacing);
}

#[test]
fn letter_spacing_stays_line_local_across_hard_breaks() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let prepared = engine.prepare_paragraph("A\nB", &style, &pre_wrap_options(4.0));
    let lines = engine.layout_with_lines(&prepared, 200.0, 20.0).lines;

    assert_eq!(lines.len(), 2);
    assert_close(lines[0].width, engine.prefix_widths("A", &style)[1]);
    assert_close(lines[1].width, engine.prefix_widths("B", &style)[1]);
}

#[test]
fn letter_spacing_participates_in_pre_wrap_tab_positioning() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = 4.0;
    let prepared = engine.prepare_paragraph("A\tB", &style, &pre_wrap_options(spacing));
    let line = &engine.layout_with_lines(&prepared, 200.0, 20.0).lines[0];
    let a_width = engine.prefix_widths("A", &style)[1];
    let b_width = engine.prefix_widths("B", &style)[1];
    let space_width = engine.prefix_widths(" ", &style)[1];
    let tab_stop = space_width * 8.0;
    let tab_input_width = a_width + spacing;
    let remainder = tab_input_width % tab_stop;
    let tab_advance = if remainder.abs() <= 0.005 {
        tab_stop
    } else {
        tab_stop - remainder
    };

    assert_eq!(line.text, "A\tB");
    assert_close(
        line.width,
        a_width + spacing + tab_advance + spacing + b_width,
    );
}

#[test]
fn letter_spacing_is_reflected_in_glyph_run_positions() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = 3.0;
    let prepared = engine.prepare_paragraph("AB", &style, &options(spacing));
    let line = engine.layout_with_lines(&prepared, 200.0, 20.0).lines[0].clone();
    let glyph_runs = engine.line_glyph_runs(&prepared, &line);
    let glyphs = &glyph_runs[0].glyphs;

    assert!(glyphs.len() >= 2);
    assert_close(glyphs[1].x, glyphs[0].advance + spacing);
    assert_close(glyph_runs[0].width, line.width);
}

#[test]
fn rich_inline_letter_spacing_applies_inside_items() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let spacing = 3.0;
    let prepared = prepare_rich_inline(
        &engine,
        &[RichInlineItemSpec {
            text: "AB",
            style: &style,
            break_mode: RichInlineBreakMode::Normal,
            extra_width: 0.0,
            letter_spacing: spacing,
        }],
    );
    let stats = measure_rich_inline_stats(&engine, &prepared, 200.0);

    assert_eq!(stats.line_count, 1);
    assert_close(
        stats.max_line_width,
        engine.prefix_widths("AB", &style)[2] + spacing,
    );
}

#[test]
fn letter_spacing_streaming_matches_batched_layout() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let prepared =
        engine.prepare_paragraph("Hello 世界 مرحبا emoji 😀 tracking", &style, &options(2.5));
    let batched = engine.layout_with_lines(&prepared, 120.0, 20.0);
    let mut cursor = LayoutCursor::default();
    let mut streamed = Vec::new();

    while let Some(line) = engine.layout_next_line(&prepared, &mut cursor, 120.0) {
        streamed.push(line);
    }

    assert_eq!(streamed, batched.lines);
}

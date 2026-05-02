mod support;

use pretext::advanced::{LayoutCursor, LayoutLine};
use pretext::{
    BidiDirection, PretextEngine, PretextParagraphOptions, PretextPreparedParagraph, WhiteSpaceMode,
};

const WIDTH_EPSILON: f32 = 0.05;

fn assert_run_parity(
    engine: &PretextEngine,
    prepared: &PretextPreparedParagraph,
    line: &LayoutLine,
) {
    let visual_runs = engine.line_visual_runs(prepared, line);
    let glyph_runs = engine.line_glyph_runs(prepared, line);
    let combined_runs = engine.line_runs(prepared, line);

    assert_eq!(visual_runs.len(), glyph_runs.len());
    assert_eq!(combined_runs.visual_runs, visual_runs);
    assert_eq!(combined_runs.glyph_runs, glyph_runs);

    for (visual, glyphs) in visual_runs.iter().zip(glyph_runs.iter()) {
        assert_eq!(visual.start, glyphs.start);
        assert_eq!(visual.end, glyphs.end);
        assert_eq!(visual.level, glyphs.level);
        assert_eq!(visual.direction, glyphs.direction);
        assert!((visual.width - glyphs.width).abs() <= WIDTH_EPSILON);

        let glyph_width: f32 = glyphs.glyphs.iter().map(|glyph| glyph.advance).sum();
        assert!((glyph_width - glyphs.width).abs() <= WIDTH_EPSILON);
    }
}

#[test]
fn mixed_bidi_visual_and_glyph_runs_stay_in_lockstep() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_paragraph(
        "English قبل العربية and then back again",
        &support::default_style(),
        &PretextParagraphOptions::default(),
    );
    let layout = engine.layout_paragraph(&prepared, 220.0, 22.0);

    assert!(layout.line_count >= 1);
    assert!(layout.lines.iter().any(|line| {
        line.runs
            .visual_runs
            .iter()
            .any(|run| run.direction == BidiDirection::Rtl)
    }));

    for line in &layout.lines {
        assert_run_parity(&engine, &prepared, &line.line);
    }
}

#[test]
fn soft_hyphen_visual_and_glyph_runs_share_synthetic_hyphen_width() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let prefix_widths = engine.prefix_widths("hy", &style);
    let max_width = prefix_widths[2] + engine.glyph_advance('-', &style) + 0.5;
    let prepared = engine.prepare_paragraph(
        "hy\u{00AD}phenation demo",
        &style,
        &PretextParagraphOptions::default(),
    );
    let layout = engine.layout_paragraph(&prepared, max_width, 20.0);
    let first_line = &layout.lines.first().expect("expected first line").line;

    assert!(first_line.text.ends_with('-'));
    assert_run_parity(&engine, &prepared, first_line);

    let visual_runs = engine.line_visual_runs(&prepared, first_line);
    assert!(visual_runs
        .last()
        .is_some_and(|run| run.text.ends_with('-')));
}

#[test]
fn normal_whitespace_trims_trailing_spaces_before_visual_and_glyph_runs() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_paragraph(
        "abc   ",
        &support::default_style(),
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );
    let layout = engine.layout_paragraph(&prepared, 240.0, 20.0);
    let line = &layout.lines.first().expect("expected single line").line;
    let visual_runs = &layout
        .lines
        .first()
        .expect("expected single line")
        .runs
        .visual_runs;

    assert_eq!(line.text, "abc");
    assert_eq!(
        visual_runs
            .iter()
            .map(|run| run.text.as_str())
            .collect::<String>(),
        "abc"
    );
    assert!(visual_runs.iter().all(|run| !run.text.ends_with(' ')));
    assert_run_parity(&engine, &prepared, line);
}

#[test]
fn layout_paragraph_matches_measurement_and_line_runs() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_paragraph(
        "English قبل العربية and then back again",
        &support::default_style(),
        &PretextParagraphOptions::default(),
    );
    let metrics = engine.measure_paragraph(&prepared, 220.0, 22.0);
    let layout = engine.layout_paragraph(&prepared, 220.0, 22.0);

    assert_eq!(layout.height, metrics.height);
    assert_eq!(layout.line_count, metrics.line_count);

    for line in &layout.lines {
        assert_eq!(line.runs, engine.line_runs(&prepared, &line.line));
    }
}

#[test]
fn layout_next_line_helpers_match_legacy_streaming_paths() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_paragraph(
        "English قبل العربية and hy\u{00AD}phenation demo back again",
        &support::default_style(),
        &PretextParagraphOptions::default(),
    );
    let mut line_cursor = LayoutCursor::default();
    let mut glyph_cursor = LayoutCursor::default();
    let mut runs_cursor = LayoutCursor::default();

    loop {
        let line = engine.layout_next_line(&prepared, &mut line_cursor, 120.0);
        let with_glyph_runs =
            engine.layout_next_line_with_glyph_runs(&prepared, &mut glyph_cursor, 120.0);
        let with_runs = engine.layout_next_line_with_runs(&prepared, &mut runs_cursor, 120.0);

        assert_eq!(glyph_cursor, line_cursor);
        assert_eq!(runs_cursor, line_cursor);

        match line {
            Some(line) => {
                let with_glyph_runs =
                    with_glyph_runs.expect("glyph-runs helper should yield the same line");
                let with_runs = with_runs.expect("runs helper should yield the same line");
                assert_eq!(with_glyph_runs.line, line);
                assert_eq!(with_runs.line, line);
                assert_eq!(
                    with_glyph_runs.glyph_runs,
                    engine.line_glyph_runs(&prepared, &line)
                );
                assert_eq!(with_runs.runs, engine.line_runs(&prepared, &line));
            }
            None => {
                assert!(with_glyph_runs.is_none());
                assert!(with_runs.is_none());
                break;
            }
        }
    }
}

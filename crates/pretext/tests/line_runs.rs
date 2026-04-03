mod support;

use pretext::{BidiDirection, PrepareOptions, WhiteSpaceMode};

const WIDTH_EPSILON: f32 = 0.05;

fn assert_run_parity(
    engine: &pretext::PretextEngine,
    prepared: &pretext::PreparedTextWithSegments,
    line: &pretext::LayoutLine,
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
    let prepared = engine.prepare_with_segments(
        "English قبل العربية and then back again",
        &support::default_style(),
        &PrepareOptions::default(),
    );
    let layout = engine.layout_with_lines(&prepared, 220.0, 22.0);

    assert!(layout.line_count >= 1);
    assert!(layout.lines.iter().any(|line| {
        engine
            .line_visual_runs(&prepared, line)
            .iter()
            .any(|run| run.direction == BidiDirection::Rtl)
    }));

    for line in &layout.lines {
        assert_run_parity(&engine, &prepared, line);
    }
}

#[test]
fn soft_hyphen_visual_and_glyph_runs_share_synthetic_hyphen_width() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let prefix_widths = engine.prefix_widths("hy", &style);
    let max_width = prefix_widths[2] + engine.glyph_advance('-', &style) + 0.5;
    let prepared = engine.prepare_with_segments(
        "hy\u{00AD}phenation demo",
        &style,
        &PrepareOptions::default(),
    );
    let layout = engine.layout_with_lines(&prepared, max_width, 20.0);
    let first_line = layout.lines.first().expect("expected first line");

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
    let prepared = engine.prepare_with_segments(
        "abc   ",
        &support::default_style(),
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            ..PrepareOptions::default()
        },
    );
    let layout = engine.layout_with_lines(&prepared, 240.0, 20.0);
    let line = layout.lines.first().expect("expected single line");
    let visual_runs = engine.line_visual_runs(&prepared, line);

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
fn layout_with_runs_matches_layout_with_lines_and_line_runs() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_with_segments(
        "English قبل العربية and then back again",
        &support::default_style(),
        &PrepareOptions::default(),
    );
    let with_lines = engine.layout_with_lines(&prepared, 220.0, 22.0);
    let with_runs = engine.layout_with_runs(&prepared, 220.0, 22.0);

    assert_eq!(with_runs.height, with_lines.height);
    assert_eq!(with_runs.line_count, with_lines.line_count);
    assert_eq!(with_runs.lines.len(), with_lines.lines.len());

    for (with_run_line, with_line) in with_runs.lines.iter().zip(with_lines.lines.iter()) {
        assert_eq!(&with_run_line.line, with_line);
        assert_eq!(with_run_line.runs, engine.line_runs(&prepared, with_line));
    }
}

#[test]
fn layout_next_line_helpers_match_legacy_streaming_paths() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_with_segments(
        "English قبل العربية and hy\u{00AD}phenation demo back again",
        &support::default_style(),
        &PrepareOptions::default(),
    );
    let mut line_cursor = pretext::LayoutCursor::default();
    let mut glyph_cursor = pretext::LayoutCursor::default();
    let mut runs_cursor = pretext::LayoutCursor::default();

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

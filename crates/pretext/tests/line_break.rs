mod support;

use pretext::{PrepareOptions, WhiteSpaceMode};

#[test]
fn pre_wrap_newline_forces_break() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let prepared = engine.prepare_with_segments(
        "alpha\nbeta",
        &style,
        &PrepareOptions {
            white_space: WhiteSpaceMode::PreWrap,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );

    let result = engine.layout_with_lines(&prepared, 400.0, 20.0);
    assert_eq!(result.line_count, 2);
    assert_eq!(result.lines[0].text, "alpha");
    assert_eq!(result.lines[1].text, "beta");
}

#[test]
fn soft_hyphen_only_appears_when_line_break_uses_it() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let word_widths = engine.prefix_widths("hy", &style);
    let max_width = word_widths[2] + engine.glyph_advance('-', &style) + 0.5;
    let prepared = engine.prepare_with_segments(
        "hy\u{00AD}phen",
        &style,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );

    let result = engine.layout_with_lines(&prepared, max_width, 20.0);
    assert!(result.line_count >= 2);
    assert!(result.lines[0].text.ends_with('-'));
}

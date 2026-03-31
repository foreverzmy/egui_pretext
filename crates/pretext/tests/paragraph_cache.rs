mod support;

use pretext::{PrepareOptions, TextStyleSpec, WhiteSpaceMode};

#[test]
fn paragraph_cache_is_enabled() {
    let engine = support::bundled_engine();
    assert!(engine.has_paragraph_cache());
}

#[test]
fn paragraph_cache_separates_styles() {
    let engine = support::bundled_engine();
    let small = support::default_style();
    let large = TextStyleSpec {
        size_px: 28.0,
        ..support::default_style()
    };
    let text = "The same paragraph should not reuse cached lines across text styles.";

    let prepared_small = engine.prepare_with_segments(
        text,
        &small,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );
    let prepared_large = engine.prepare_with_segments(
        text,
        &large,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );

    let small_layout = engine.layout_with_lines(&prepared_small, 190.0, 22.0);
    let large_layout = engine.layout_with_lines(&prepared_large, 190.0, 34.0);

    let fresh_engine = support::bundled_engine();
    let fresh_prepared_large = fresh_engine.prepare_with_segments(
        text,
        &large,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );
    let fresh_large_layout = fresh_engine.layout_with_lines(&fresh_prepared_large, 190.0, 34.0);

    assert_ne!(small_layout.lines, large_layout.lines);
    assert_eq!(large_layout.lines, fresh_large_layout.lines);
}

#[test]
fn paragraph_cache_separates_whitespace_modes() {
    let engine = support::bundled_engine();
    let text = "alpha\nbeta";
    let style = support::default_style();

    let normal = engine.prepare_with_segments(
        text,
        &style,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );
    let pre_wrap = engine.prepare_with_segments(
        text,
        &style,
        &PrepareOptions {
            white_space: WhiteSpaceMode::PreWrap,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );

    let normal_layout = engine.layout_with_lines(&normal, 400.0, 20.0);
    let pre_wrap_layout = engine.layout_with_lines(&pre_wrap, 400.0, 20.0);

    assert_eq!(normal_layout.line_count, 1);
    assert_eq!(pre_wrap_layout.line_count, 2);
    assert_ne!(normal_layout.lines, pre_wrap_layout.lines);
}

#[test]
fn paragraph_cache_separates_atomic_placeholder_widths() {
    let engine = support::bundled_engine();
    let narrow = engine.prepare_atomic_placeholder(
        36.0,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );
    let wide = engine.prepare_atomic_placeholder(
        84.0,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );

    let narrow_layout = engine.layout_with_lines(&narrow, 24.0, 20.0);
    let wide_layout = engine.layout_with_lines(&wide, 24.0, 20.0);

    let fresh_engine = support::bundled_engine();
    let fresh_wide = fresh_engine.prepare_atomic_placeholder(
        84.0,
        &PrepareOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: pretext::ParagraphDirection::Auto,
        },
    );
    let fresh_wide_layout = fresh_engine.layout_with_lines(&fresh_wide, 24.0, 20.0);

    assert_eq!(narrow_layout.line_count, 1);
    assert_eq!(wide_layout.line_count, 1);
    assert_ne!(narrow_layout.lines[0].width, wide_layout.lines[0].width);
    assert_eq!(wide_layout, fresh_wide_layout);
}

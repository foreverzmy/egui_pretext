mod support;

use pretext::{ParagraphDirection, PretextParagraphOptions, PretextStyle, WhiteSpaceMode};

#[test]
fn paragraph_cache_is_enabled() {
    let engine = support::bundled_engine();
    assert!(engine.has_paragraph_cache());
}

#[test]
fn paragraph_cache_separates_styles() {
    let engine = support::bundled_engine();
    let small = support::default_style();
    let large = PretextStyle {
        size_px: 28.0,
        ..support::default_style()
    };
    let text = "The same paragraph should not reuse cached lines across text styles.";

    let prepared_small = engine.prepare_paragraph(
        text,
        &small,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );
    let prepared_large = engine.prepare_paragraph(
        text,
        &large,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );

    let small_layout = engine.layout_paragraph(&prepared_small, 190.0, 22.0);
    let large_layout = engine.layout_paragraph(&prepared_large, 190.0, 34.0);

    let fresh_engine = support::bundled_engine();
    let fresh_prepared_large = fresh_engine.prepare_paragraph(
        text,
        &large,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );
    let fresh_large_layout = fresh_engine.layout_paragraph(&fresh_prepared_large, 190.0, 34.0);

    assert_ne!(small_layout.lines, large_layout.lines);
    assert_eq!(large_layout.lines, fresh_large_layout.lines);
}

#[test]
fn paragraph_cache_separates_whitespace_modes() {
    let engine = support::bundled_engine();
    let text = "alpha\nbeta";
    let style = support::default_style();

    let normal = engine.prepare_paragraph(
        text,
        &style,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );
    let pre_wrap = engine.prepare_paragraph(
        text,
        &style,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::PreWrap,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );

    let normal_layout = engine.layout_paragraph(&normal, 400.0, 20.0);
    let pre_wrap_layout = engine.layout_paragraph(&pre_wrap, 400.0, 20.0);

    assert_eq!(normal_layout.line_count, 1);
    assert_eq!(pre_wrap_layout.line_count, 2);
    assert_ne!(normal_layout.lines, pre_wrap_layout.lines);
}

#[test]
fn paragraph_cache_separates_letter_spacing() {
    let engine = support::bundled_engine();
    let text = "spacing sensitive paragraph";
    let style = support::default_style();

    let normal = engine.prepare_paragraph(
        text,
        &style,
        &PretextParagraphOptions {
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );
    let tracked = engine.prepare_paragraph(
        text,
        &style,
        &PretextParagraphOptions {
            letter_spacing: 4.0,
            ..PretextParagraphOptions::default()
        },
    );

    let normal_layout = engine.layout_paragraph(&normal, 180.0, 20.0);
    let tracked_layout = engine.layout_paragraph(&tracked, 180.0, 20.0);

    let fresh_engine = support::bundled_engine();
    let fresh_tracked = fresh_engine.prepare_paragraph(
        text,
        &style,
        &PretextParagraphOptions {
            letter_spacing: 4.0,
            ..PretextParagraphOptions::default()
        },
    );
    let fresh_tracked_layout = fresh_engine.layout_paragraph(&fresh_tracked, 180.0, 20.0);

    assert_ne!(normal_layout.lines, tracked_layout.lines);
    assert_eq!(tracked_layout.lines, fresh_tracked_layout.lines);
}

#[test]
fn paragraph_cache_separates_atomic_placeholder_widths() {
    let engine = support::bundled_engine();
    let narrow = engine.prepare_atomic_placeholder(
        36.0,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );
    let wide = engine.prepare_atomic_placeholder(
        84.0,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );

    let narrow_layout = engine.layout_paragraph(&narrow, 24.0, 20.0);
    let wide_layout = engine.layout_paragraph(&wide, 24.0, 20.0);

    let fresh_engine = support::bundled_engine();
    let fresh_wide = fresh_engine.prepare_atomic_placeholder(
        84.0,
        &PretextParagraphOptions {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
            letter_spacing: 0.0,
            ..PretextParagraphOptions::default()
        },
    );
    let fresh_wide_layout = fresh_engine.layout_paragraph(&fresh_wide, 24.0, 20.0);

    assert_eq!(narrow_layout.line_count, 1);
    assert_eq!(wide_layout.line_count, 1);
    assert_ne!(
        narrow_layout.lines[0].line.width,
        wide_layout.lines[0].line.width
    );
    assert_eq!(wide_layout, fresh_wide_layout);
}

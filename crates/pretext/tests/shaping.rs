mod support;

#[test]
fn prefix_widths_are_monotonic() {
    let engine = support::bundled_engine();
    let widths = engine.prefix_widths("hello", &support::default_style());

    assert_eq!(widths.len(), 6);
    assert_eq!(widths[0], 0.0);
    assert!(widths.windows(2).all(|pair| pair[1] >= pair[0]));
}

#[test]
fn glyph_advance_returns_non_zero_for_visible_glyphs() {
    let engine = support::bundled_engine();
    assert!(engine.glyph_advance('h', &support::default_style()) > 0.0);
    assert!(engine.glyph_advance('م', &support::default_style()) >= 0.0);
}

#[test]
fn emoji_zwj_sequence_is_treated_as_single_cluster() {
    let engine = support::bundled_engine();
    let widths = engine.prefix_widths("👨‍👩‍👧‍👦", &support::default_style());

    assert_eq!(widths.len(), 2);
    assert_eq!(widths[0], 0.0);
    assert!(widths[1] >= 0.0);
}

mod support;

#[test]
fn zwj_family_emoji_counts_as_one_grapheme_prefix_step() {
    let engine = support::bundled_engine();
    let widths = engine.prefix_widths("a👨‍👩‍👧‍👦b", &support::default_style());

    assert_eq!(widths.len(), 4);
    assert_eq!(widths[0], 0.0);
    assert!(widths.windows(2).all(|pair| pair[1] >= pair[0]));
}

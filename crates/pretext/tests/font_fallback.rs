mod support;

use pretext::{font_catalog::FontCatalog, TextStyleSpec};

#[test]
fn face_for_cluster_requires_full_coverage() {
    let catalog = FontCatalog::with_font_data_and_system_fonts(support::bundled_font_data(), false);
    let style = support::default_style();
    let preferred = catalog.resolve_style_chain(&style);

    let face = catalog
        .face_for_cluster("ab", &preferred)
        .expect("expected a face for latin cluster");

    assert!(face.has_glyph('a'));
    assert!(face.has_glyph('b'));
}

#[test]
fn missing_cluster_returns_fallback_face_instead_of_none() {
    let catalog = FontCatalog::with_font_data_and_system_fonts(support::bundled_font_data(), false);
    let style = support::default_style();
    let preferred = catalog.resolve_style_chain(&style);

    let face = catalog.face_for_cluster("\u{E000}", &preferred);
    assert!(face.is_some());
}

#[test]
fn mixed_script_prefix_widths_are_monotonic() {
    let engine = support::bundled_engine();
    let widths = engine.prefix_widths("Hello 😀 مرحبا 漢字", &support::default_style());

    assert_eq!(widths[0], 0.0);
    assert!(widths.windows(2).all(|pair| pair[1] >= pair[0]));
    assert!(widths.last().copied().unwrap_or_default() >= 0.0);
}

#[test]
fn bundled_fonts_measure_arabic_and_rocket_emoji() {
    let engine = support::bundled_engine();
    let style = support::default_style();

    assert!(engine.glyph_advance('ب', &style) > 0.0);
    assert!(engine.glyph_advance('🚀', &style) > 0.0);
}

#[test]
fn font_for_char_still_prefers_requested_family_over_global_coverage_map() {
    let catalog = FontCatalog::with_font_data_and_system_fonts(support::bundled_font_data(), false);
    let style = TextStyleSpec {
        families: vec!["Noto Sans Mono".to_owned(), "Noto Sans".to_owned()],
        ..support::default_style()
    };
    let preferred = catalog.resolve_style_chain(&style);

    let face = catalog
        .face_for_char('A', &preferred)
        .expect("expected a face for ASCII letter");

    assert_eq!(face.family_name(), "Noto Sans Mono");
}

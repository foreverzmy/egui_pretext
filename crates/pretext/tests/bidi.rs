mod support;

use pretext::{
    bidi::paragraph_to_bidi_runs, BidiDirection, ParagraphDirection, PretextParagraphOptions,
};

#[test]
fn mixed_direction_text_contains_rtl_run() {
    let runs = paragraph_to_bidi_runs("abc مرحبا def", ParagraphDirection::Auto);
    assert!(runs.iter().any(|run| run.direction == BidiDirection::Rtl));
    assert!(runs.iter().any(|run| run.direction == BidiDirection::Ltr));
}

#[test]
fn bidi_runs_stay_aligned_to_utf8_boundaries() {
    let text = "Latin مرحبا 漢字";
    let runs = paragraph_to_bidi_runs(text, ParagraphDirection::Auto);

    assert!(!runs.is_empty());
    for run in runs {
        assert!(text.is_char_boundary(run.byte_range.start));
        assert!(text.is_char_boundary(run.byte_range.end));
        assert!(!text[run.byte_range].is_empty());
    }
}

#[test]
fn pure_rtl_text_coalesces_into_one_run() {
    let runs = paragraph_to_bidi_runs("مرحبا بالعالم", ParagraphDirection::Auto);

    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].direction, BidiDirection::Rtl);
}

#[test]
fn visual_runs_reorder_mixed_direction_line_without_mutating_logical_text() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_paragraph(
        "אבג abc",
        &support::default_style(),
        &PretextParagraphOptions::default(),
    );
    let layout = engine.layout_paragraph(&prepared, 240.0, 22.0);
    let line = layout.lines.first().expect("single line layout");
    let visual_runs = &line.runs.visual_runs;

    assert_eq!(line.line.text, "אבג abc");
    assert_eq!(visual_runs.len(), 2);
    assert_eq!(visual_runs[0].direction, BidiDirection::Ltr);
    assert_eq!(visual_runs[1].direction, BidiDirection::Rtl);
    assert!(visual_runs[0].text.contains("abc"));
    assert!(visual_runs[1].text.contains("אבג"));
}

#[test]
fn explicit_ltr_paragraph_direction_keeps_rtl_prefix_at_visual_start() {
    let engine = support::bundled_engine();
    let prepared = engine.prepare_paragraph(
        "كل شيء! Mixed bidi",
        &support::default_style(),
        &PretextParagraphOptions {
            paragraph_direction: ParagraphDirection::Ltr,
            ..PretextParagraphOptions::default()
        },
    );
    let layout = engine.layout_paragraph(&prepared, 400.0, 22.0);
    let line = layout.lines.first().expect("single line layout");
    let visual_runs = &line.runs.visual_runs;

    assert_eq!(visual_runs.len(), 2);
    assert_eq!(visual_runs[0].direction, BidiDirection::Rtl);
    assert!(visual_runs[0].text.contains("كل شيء"));
    assert_eq!(visual_runs[1].direction, BidiDirection::Ltr);
    assert!(visual_runs[1].text.contains("Mixed bidi"));
}

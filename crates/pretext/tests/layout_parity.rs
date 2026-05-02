mod support;

use pretext::advanced::LayoutCursor;
use pretext::{ParagraphDirection, PretextParagraphOptions, WhiteSpaceMode};

#[test]
fn layout_interfaces_stay_in_sync() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let opts = PretextParagraphOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
        letter_spacing: 0.0,
        ..PretextParagraphOptions::default()
    };
    let text = "English العربية mixed with CJK 漢字 and emoji 👨‍👩‍👧‍👦 for parity.";

    let prepared = engine.prepare_paragraph(text, &style, &opts);
    let metrics = prepared.measure(&engine, 180.0, 22.0);
    let layout = prepared.layout(&engine, 180.0, 22.0);

    let mut walked = 0usize;
    engine.walk_line_ranges(&prepared, 180.0, |_| {
        walked += 1;
    });

    let mut streamed = 0usize;
    let mut cursor = LayoutCursor::default();
    while engine
        .layout_next_line(&prepared, &mut cursor, 180.0)
        .is_some()
    {
        streamed += 1;
    }

    assert_eq!(layout.line_count, metrics.line_count);
    assert_eq!(layout.line_count, walked);
    assert_eq!(layout.line_count, streamed);
}

mod support;

use pretext::{LayoutCursor, PrepareOptions, WhiteSpaceMode};

#[test]
fn layout_interfaces_stay_in_sync() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let opts = PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
    };
    let text = "English العربية mixed with CJK 漢字 and emoji 👨‍👩‍👧‍👦 for parity.";

    let prepared = engine.prepare(text, &style, &opts);
    let prepared_segments = engine.prepare_with_segments(text, &style, &opts);

    let layout = engine.layout(&prepared, 180.0, 22.0);
    let with_lines = engine.layout_with_lines(&prepared_segments, 180.0, 22.0);

    let mut walked = 0usize;
    engine.walk_line_ranges(&prepared_segments, 180.0, |_| {
        walked += 1;
    });

    let mut streamed = 0usize;
    let mut cursor = LayoutCursor::default();
    while engine
        .layout_next_line(&prepared_segments, &mut cursor, 180.0)
        .is_some()
    {
        streamed += 1;
    }

    assert_eq!(layout.line_count, with_lines.line_count);
    assert_eq!(layout.line_count, walked);
    assert_eq!(layout.line_count, streamed);
}

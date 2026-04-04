//! Advanced `pretext` APIs.
//!
//! These exports expose lower-level layout, cursor, shaping, and font details
//! used by custom editors, obstacle-aware flow, and debugging tools.

use crate::{
    PretextEngine, PretextParagraphMetrics, PretextParagraphOptions, PretextPreparedParagraph,
    PretextStyle,
};

pub use crate::engine::{
    LayoutCursor, LayoutGlyph, LayoutLine, LayoutLineRange, LayoutLineRuns,
    LayoutLineWithGlyphRuns, LayoutWithLinesResult, PreparedText, SegmentKind, ShapedTextSpan,
};
pub use crate::font_catalog::{FontId, LoadedFace};
pub use crate::line_break::BreakOpportunity;
pub use crate::measure::{shape_run, ShapeCache, ShapedGlyph};

pub fn prepare_text(
    engine: &PretextEngine,
    text: &str,
    style: &PretextStyle,
    options: &PretextParagraphOptions,
) -> PreparedText {
    engine.prepare(text, style, options)
}

pub fn measure_text(
    engine: &PretextEngine,
    prepared: &PreparedText,
    max_width: f32,
    line_height: f32,
) -> PretextParagraphMetrics {
    engine.layout(prepared, max_width, line_height)
}

pub fn layout_lines(
    engine: &PretextEngine,
    prepared: &PretextPreparedParagraph,
    max_width: f32,
    line_height: f32,
) -> LayoutWithLinesResult {
    engine.layout_with_lines(prepared, max_width, line_height)
}

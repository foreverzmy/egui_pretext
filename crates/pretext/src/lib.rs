//! Stable `pretext` SDK surface.
//!
//! Use the root exports for standard paragraph preparation and layout.
//! Reach for [`advanced`] when you need cursor-driven continuation,
//! atomic placeholders, glyph shaping details, or lower-level layout data.

#[doc(hidden)]
pub mod analysis;
#[doc(hidden)]
pub mod bidi;
#[doc(hidden)]
pub mod engine;
#[doc(hidden)]
pub mod font_catalog;
#[doc(hidden)]
pub mod layout;
#[doc(hidden)]
pub mod line_break;
#[doc(hidden)]
pub mod measure;

pub mod advanced;

pub use crate::analysis::WhiteSpaceMode;
pub use crate::bidi::{BidiDirection, ParagraphDirection};
pub use crate::engine::{
    EngineRuntimeStats, LayoutLineGlyphRun as PretextGlyphRun,
    LayoutLineVisualRun as PretextVisualRun, LayoutLineWithRuns as PretextLine,
    LayoutResult as PretextParagraphMetrics, LayoutWithRunsResult as PretextParagraphLayout,
    PrepareOptions as PretextParagraphOptions,
    PreparedTextWithSegments as PretextPreparedParagraph, PretextEngine, PretextEngineBuilder,
    TextStyleSpec as PretextStyle,
};

use pretext::{BidiDirection, PretextGlyphRun, PretextStyle};

use crate::glyph_atlas::GlyphSceneBuilder;
use crate::{BaselineMetrics, BaselineMode, EguiPretextPaintOptions};

/// Built-in demo emoji assets that can be painted as SVG overlays.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum EmojiAssetId {
    Rocket,
    PartyPopper,
    CheckMark,
}

/// Built-in SVG assets bundled with `pretext-egui`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum SvgAssetId {
    OpenAiLogo,
    ClaudeLogo,
    Emoji(EmojiAssetId),
}

/// Layout inputs used when turning built-in emoji graphemes into overlay runs.
#[derive(Clone, Copy)]
pub struct EmojiOverlayOptions<'a> {
    pub style: &'a PretextStyle,
    pub slot_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub slack_x: f32,
    pub slack_y: f32,
    pub baseline_mode: BaselineMode,
}

/// One emoji overlay slot within a visual run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmojiOverlay {
    pub start: f32,
    pub end: f32,
    pub emoji_id: EmojiAssetId,
}

/// A visual run worth of emoji overlays, with enough metrics to paint them later.
#[derive(Clone, Debug, PartialEq)]
pub struct EmojiOverlayRun {
    pub line_offset: f32,
    pub width: f32,
    pub direction: BidiDirection,
    pub baseline_metrics: BaselineMetrics,
    pub emojis: Vec<EmojiOverlay>,
}

/// A borrowed positioned text fragment with one shared paint style.
#[derive(Clone, Copy)]
pub struct PositionedTextRunRef<'a> {
    pub x: f32,
    pub y: f32,
    pub text: &'a str,
    pub glyph_runs: &'a [PretextGlyphRun],
    pub emoji_overlays: &'a [EmojiOverlayRun],
}

/// A borrowed positioned text fragment with its own per-fragment paint options.
#[derive(Clone)]
pub struct StyledPositionedTextRunRef<'a, 'b> {
    pub x: f32,
    pub y: f32,
    pub text: &'a str,
    pub glyph_runs: &'a [PretextGlyphRun],
    pub emoji_overlays: &'a [EmojiOverlayRun],
    pub options: EguiPretextPaintOptions<'b>,
}

/// Preset glyph-atlas warmup buckets used by the demos.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum AtlasWarmupBucket {
    CommonSans,
    CommonSerif,
    SerifDisplay,
    Mono,
    Arabic,
    Cjk,
    Myanmar,
}

#[derive(Clone)]
pub(crate) struct PendingFallbackText {
    pub(crate) origin: egui::Pos2,
    pub(crate) text: String,
    pub(crate) fallback_align: egui::Align2,
    pub(crate) fallback_font: egui::FontId,
    pub(crate) color: egui::Color32,
}

#[derive(Clone)]
pub(crate) struct PendingEmojiPaint {
    pub(crate) line_left: f32,
    pub(crate) slot_top: f32,
    pub(crate) overlay_runs: Vec<EmojiOverlayRun>,
    pub(crate) emoji_size: f32,
    pub(crate) slot_height: f32,
}

/// Batches glyph-scene painting with optional text fallback and emoji overlays.
pub struct PretextFragmentPainter {
    pub(crate) scene: GlyphSceneBuilder,
    pub(crate) pending_fallbacks: Vec<PendingFallbackText>,
    pub(crate) pending_emoji: Vec<PendingEmojiPaint>,
}

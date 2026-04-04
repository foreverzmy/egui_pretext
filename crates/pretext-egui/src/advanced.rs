//! Advanced `pretext-egui` APIs.
//!
//! These exports cover lower-level glyph-scene, positioned-run, emoji-overlay,
//! and warmup helpers used by the workspace demos and custom renderers.

use egui::{Color32, Context, Painter, TextureHandle};
use pretext::{PretextEngine, PretextGlyphRun, PretextStyle, PretextVisualRun};

pub use crate::advanced_types::{
    AtlasWarmupBucket, EmojiOverlay, EmojiOverlayOptions, EmojiOverlayRun, PositionedTextRunRef,
    PretextFragmentPainter, StyledPositionedTextRunRef,
};
pub use crate::advanced_types::{EmojiAssetId, SvgAssetId};
pub use crate::glyph_atlas::GlyphSceneBuilder;
pub use crate::{
    EguiPretextPaintOptions, PretextTextTexture, PretextTextureRasterError,
    PretextTextureRasterRequest,
};

/// Starts a new glyph-atlas scene for batching multiple fragments together.
pub fn new_glyph_scene(assets: &crate::EguiPretextRenderer) -> GlyphSceneBuilder {
    crate::new_glyph_scene(assets)
}

#[allow(clippy::too_many_arguments)]
/// Appends one line's glyph runs into an existing glyph-atlas scene.
pub fn append_glyph_runs(
    scene: &mut GlyphSceneBuilder,
    x: f32,
    y: f32,
    glyph_runs: &[PretextGlyphRun],
    style: &PretextStyle,
    line_height: f32,
    color: Color32,
    ctx: &Context,
    engine: &PretextEngine,
    assets: &mut crate::EguiPretextRenderer,
) -> bool {
    crate::append_glyph_runs(
        scene,
        x,
        y,
        glyph_runs,
        style,
        line_height,
        color,
        ctx,
        engine,
        assets,
    )
}

/// Flushes a glyph scene to the current painter.
pub fn flush_glyph_scene(
    painter: &Painter,
    scene: &mut GlyphSceneBuilder,
    assets: &mut crate::EguiPretextRenderer,
) -> bool {
    crate::flush_glyph_scene(painter, scene, assets)
}

/// Splits built-in emoji graphemes out of glyph runs and returns SVG overlay metadata.
pub fn split_builtin_emoji_glyphs(
    visual_runs: &[PretextVisualRun],
    glyph_runs: &[PretextGlyphRun],
    options: EmojiOverlayOptions<'_>,
    engine: &PretextEngine,
) -> (Vec<PretextGlyphRun>, Vec<EmojiOverlayRun>) {
    crate::split_builtin_emoji_glyphs(visual_runs, glyph_runs, options, engine)
}

/// Removes built-in emoji glyphs from shaped glyph runs without producing overlay metadata.
pub fn strip_builtin_emoji_glyphs(
    visual_runs: &[PretextVisualRun],
    glyph_runs: &[PretextGlyphRun],
    style: &PretextStyle,
    engine: &PretextEngine,
) -> Vec<PretextGlyphRun> {
    crate::strip_builtin_emoji_glyphs(visual_runs, glyph_runs, style, engine)
}

#[allow(clippy::too_many_arguments)]
/// Paints precomputed SVG emoji overlays for a positioned line fragment.
pub fn paint_emoji_overlays(
    painter: &Painter,
    line_left: f32,
    slot_top: f32,
    overlay_runs: &[EmojiOverlayRun],
    emoji_size: f32,
    slot_height: f32,
    ctx: &Context,
    assets: &mut crate::EguiPretextRenderer,
) {
    crate::paint_emoji_overlays(
        painter,
        line_left,
        slot_top,
        overlay_runs,
        emoji_size,
        slot_height,
        ctx,
        assets,
    );
}

/// Paints positioned fragments that all share the same paint options.
pub fn paint_positioned_text_runs<'a>(
    painter: &Painter,
    lines: impl IntoIterator<Item = PositionedTextRunRef<'a>>,
    options: &EguiPretextPaintOptions<'_>,
    ctx: &Context,
    engine: &PretextEngine,
    assets: &mut crate::EguiPretextRenderer,
) -> bool {
    crate::paint_positioned_text_runs(painter, lines, options, ctx, engine, assets)
}

/// Paints positioned fragments where each fragment carries its own paint options.
pub fn paint_styled_positioned_text_runs<'a, 'b>(
    painter: &Painter,
    lines: impl IntoIterator<Item = StyledPositionedTextRunRef<'a, 'b>>,
    ctx: &Context,
    engine: &PretextEngine,
    assets: &mut crate::EguiPretextRenderer,
) -> bool {
    crate::paint_styled_positioned_text_runs(painter, lines, ctx, engine, assets)
}

/// Computes baseline metrics for a shaped text raster request without allocating a texture.
pub fn shaped_text_baseline_metrics(
    engine: &PretextEngine,
    request: PretextTextureRasterRequest<'_>,
) -> crate::BaselineMetrics {
    crate::shaped_text_baseline_metrics(engine, request)
}

/// Loads or reuses one built-in emoji SVG as an egui texture.
pub fn emoji_texture(
    renderer: &mut crate::EguiPretextRenderer,
    emoji_id: EmojiAssetId,
    size: [usize; 2],
    ctx: &Context,
) -> TextureHandle {
    renderer.emoji_texture(emoji_id, size, ctx)
}

#[allow(clippy::too_many_arguments)]
/// Paints one line of glyph runs directly through the renderer's glyph atlas.
pub fn paint_line_glyph_runs(
    renderer: &mut crate::EguiPretextRenderer,
    painter: &Painter,
    x: f32,
    y: f32,
    glyph_runs: &[PretextGlyphRun],
    style: &PretextStyle,
    line_height: f32,
    color: Color32,
    ctx: &Context,
    engine: &PretextEngine,
) -> bool {
    renderer.paint_line_glyph_runs(
        painter,
        x,
        y,
        glyph_runs,
        style,
        line_height,
        color,
        ctx,
        engine,
    )
}

/// Queues a preset warmup bucket into the glyph atlas.
pub fn enqueue_atlas_warmup(
    renderer: &mut crate::EguiPretextRenderer,
    bucket: AtlasWarmupBucket,
    style: &PretextStyle,
    seed_texts: &[&str],
    engine: &PretextEngine,
    ctx: &Context,
) {
    renderer.enqueue_atlas_warmup(bucket, style, seed_texts, engine, ctx);
}

/// Advances pending glyph-atlas warmup work within the given budgets.
pub fn tick_atlas_warmup(
    renderer: &mut crate::EguiPretextRenderer,
    ctx: &Context,
    engine: &PretextEngine,
    glyph_budget: usize,
    page_budget: usize,
) -> bool {
    renderer.tick_atlas_warmup(ctx, engine, glyph_budget, page_budget)
}

/// Returns the built-in emoji asset mapped to a grapheme, if any.
pub fn builtin_emoji_for_grapheme(grapheme: &str) -> Option<EmojiAssetId> {
    crate::EguiPretextRenderer::builtin_emoji_for_grapheme(grapheme)
}

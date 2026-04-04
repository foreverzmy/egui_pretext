//! Experimental `pretext-egui` APIs and demo-only helpers.

pub mod demo_assets {
    use egui::{Context, TextureHandle};

    use crate::EguiPretextRenderer;

    pub use crate::{EmojiAssetId, SvgAssetId};

    /// Returns owned copies of the bundled demo fonts for building a fresh engine.
    pub fn bundled_font_data() -> Vec<Vec<u8>> {
        EguiPretextRenderer::bundled_font_data()
    }

    /// Installs the demo-oriented egui font stack into the current context.
    pub fn install_demo_fonts(ctx: &Context) {
        ctx.set_fonts(EguiPretextRenderer::demo_font_definitions());
    }

    /// Returns the raw bundled SVG bytes for a built-in asset.
    pub fn svg_bytes(asset_id: SvgAssetId) -> &'static [u8] {
        EguiPretextRenderer::svg_bytes(asset_id)
    }

    /// Loads or reuses a bundled SVG texture through the renderer cache.
    pub fn bundled_svg_texture(
        renderer: &mut EguiPretextRenderer,
        asset_id: SvgAssetId,
        size: [usize; 2],
        ctx: &Context,
    ) -> TextureHandle {
        renderer.bundled_svg_texture(asset_id, size, ctx)
    }
}

pub mod svg_emoji_overlay {
    //! Demo-oriented helpers for replacing selected graphemes with SVG emoji overlays.

    pub use crate::advanced::{
        paint_emoji_overlays, split_builtin_emoji_glyphs, strip_builtin_emoji_glyphs,
    };
    pub use crate::{EmojiAssetId, EmojiOverlay, EmojiOverlayOptions, EmojiOverlayRun};
}

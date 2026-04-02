use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

mod glyph_atlas;

use egui::{ColorImage, FontData, FontDefinitions, FontFamily, TextureHandle, TextureOptions};
use image::{ImageBuffer, Rgba};
use lru::LruCache;
use pretext::{BidiDirection, LayoutLineGlyphRun, PretextEngine, TextStyleSpec};
pub use pretext_render::{BaselineMetrics, BaselineMode};
use pretext_render::{RenderStatsSnapshot, TextRasterRequest, TextRenderCache};
use resvg::usvg;

use crate::glyph_atlas::GlyphAtlas;

const SHAPED_TEXT_TEXTURE_CACHE_CAPACITY: usize = 1024;

macro_rules! include_demo_asset {
    ($path:literal) => {
        include_bytes!(concat!("../../../demos/app/assets/", $path))
    };
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum EmojiAssetId {
    Rocket,
    PartyPopper,
    CheckMark,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum SvgAssetId {
    OpenAiLogo,
    ClaudeLogo,
    Emoji(EmojiAssetId),
}

#[derive(Clone, Copy, Debug)]
pub struct ShapedTextRasterRequest<'a> {
    pub text: &'a str,
    pub style: &'a TextStyleSpec,
    pub direction: BidiDirection,
    pub color: egui::Color32,
    pub fragment_width: f32,
    pub slot_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub slack_x: f32,
    pub slack_y: f32,
    pub baseline_mode: BaselineMode,
    pub texture_options: TextureOptions,
}

#[derive(Clone)]
pub struct ShapedTextTexture {
    pub handle: TextureHandle,
    pub logical_size: egui::Vec2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetRegistryStats {
    pub static_svg_textures: usize,
    pub shaped_text_textures: usize,
    pub texture_cache_hits: u64,
    pub texture_cache_misses: u64,
    pub texture_uploads: u64,
    pub texture_upload_bytes: u64,
    pub render: RenderStatsSnapshot,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct SvgTextureKey {
    asset_id: SvgAssetId,
    size: [usize; 2],
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ShapedTextTextureKey {
    raster_cache_id: u64,
    texture_options: TextureOptions,
}

pub struct AssetRegistry {
    static_svg_textures: HashMap<SvgTextureKey, TextureHandle>,
    shaped_text_textures: LruCache<ShapedTextTextureKey, TextureHandle>,
    glyph_atlas: GlyphAtlas,
    render_cache: TextRenderCache,
    texture_cache_hits: u64,
    texture_cache_misses: u64,
    texture_uploads: u64,
    texture_upload_bytes: u64,
}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self {
            static_svg_textures: HashMap::new(),
            shaped_text_textures: LruCache::new(
                NonZeroUsize::new(SHAPED_TEXT_TEXTURE_CACHE_CAPACITY)
                    .expect("shaped text texture cache capacity"),
            ),
            glyph_atlas: GlyphAtlas::default(),
            render_cache: TextRenderCache::default(),
            texture_cache_hits: 0,
            texture_cache_misses: 0,
            texture_uploads: 0,
            texture_upload_bytes: 0,
        }
    }
}

impl AssetRegistry {
    pub fn bundled_font_data() -> Vec<Vec<u8>> {
        vec![
            include_demo_asset!("fonts/NotoSans-Regular.ttf").to_vec(),
            include_demo_asset!("fonts/NotoSerif-Regular.ttf").to_vec(),
            include_demo_asset!("fonts/NotoSerif-Italic.ttf").to_vec(),
            include_demo_asset!("fonts/NotoSerif-Bold.ttf").to_vec(),
            include_demo_asset!("fonts/NotoSansArabic-Regular.ttf").to_vec(),
            include_demo_asset!("fonts/NotoSansCJK-Regular.ttc").to_vec(),
            include_demo_asset!("fonts/NotoSansMyanmar-Regular.ttf").to_vec(),
            include_demo_asset!("fonts/Noto-COLRv1.ttf").to_vec(),
            include_demo_asset!("fonts/NotoSansMono-Regular.ttf").to_vec(),
        ]
    }

    pub fn install_fonts(&mut self, ctx: &egui::Context) {
        ctx.set_fonts(Self::font_definitions());
    }

    pub fn svg_bytes(asset_id: SvgAssetId) -> &'static [u8] {
        match asset_id {
            SvgAssetId::OpenAiLogo => include_demo_asset!("logos/openai-symbol.svg"),
            SvgAssetId::ClaudeLogo => include_demo_asset!("logos/claude-symbol.svg"),
            SvgAssetId::Emoji(EmojiAssetId::Rocket) => include_demo_asset!("emoji_u1f680.svg"),
            SvgAssetId::Emoji(EmojiAssetId::PartyPopper) => include_demo_asset!("emoji_u1f389.svg"),
            SvgAssetId::Emoji(EmojiAssetId::CheckMark) => include_demo_asset!("emoji_u2705.svg"),
        }
    }

    pub fn bundled_svg_texture(
        &mut self,
        asset_id: SvgAssetId,
        size: [usize; 2],
        ctx: &egui::Context,
    ) -> TextureHandle {
        let key = SvgTextureKey { asset_id, size };
        if let Some(texture) = self.static_svg_textures.get(&key) {
            return texture.clone();
        }

        let image = rasterize_svg(Self::svg_bytes(asset_id), size, false)
            .unwrap_or_else(|| transparent_image(size));
        let texture = ctx.load_texture(svg_texture_name(key), image, TextureOptions::LINEAR);
        self.static_svg_textures.insert(key, texture.clone());
        texture
    }

    pub fn emoji_texture(
        &mut self,
        emoji_id: EmojiAssetId,
        size: [usize; 2],
        ctx: &egui::Context,
    ) -> TextureHandle {
        self.bundled_svg_texture(SvgAssetId::Emoji(emoji_id), size, ctx)
    }

    pub fn shaped_text_texture(
        &mut self,
        engine: &PretextEngine,
        request: ShapedTextRasterRequest<'_>,
        ctx: &egui::Context,
    ) -> Option<ShapedTextTexture> {
        let rasterized = self.render_cache.rasterized_text(
            engine,
            text_raster_request(request),
            ctx.pixels_per_point().max(1.0),
        )?;
        let logical_size = egui::vec2(
            rasterized.logical_size().width,
            rasterized.logical_size().height,
        );
        let key = ShapedTextTextureKey {
            raster_cache_id: rasterized.cache_id(),
            texture_options: request.texture_options,
        };
        if let Some(texture) = self.shaped_text_textures.get(&key).cloned() {
            self.texture_cache_hits += 1;
            return Some(ShapedTextTexture {
                handle: texture,
                logical_size,
            });
        }

        self.texture_cache_misses += 1;
        let image = alpha_mask_image(rasterized.pixel_size(), rasterized.alpha_pixels().as_ref());
        let texture = ctx.load_texture(
            shaped_text_texture_name(key),
            image,
            request.texture_options,
        );
        self.shaped_text_textures.put(key, texture.clone());
        self.texture_uploads += 1;
        self.texture_upload_bytes +=
            (rasterized.pixel_size()[0] * rasterized.pixel_size()[1] * 4) as u64;

        Some(ShapedTextTexture {
            handle: texture,
            logical_size,
        })
    }

    pub fn paint_line_glyph_runs(
        &mut self,
        painter: &egui::Painter,
        x: f32,
        y: f32,
        glyph_runs: &[LayoutLineGlyphRun],
        style: &TextStyleSpec,
        line_height: f32,
        color: egui::Color32,
        ctx: &egui::Context,
        engine: &PretextEngine,
    ) -> bool {
        self.glyph_atlas.paint_line_glyph_runs(
            painter,
            x,
            y,
            glyph_runs,
            style,
            line_height,
            color,
            ctx,
            engine,
            &mut self.texture_uploads,
            &mut self.texture_upload_bytes,
        )
    }

    pub fn builtin_emoji_for_grapheme(grapheme: &str) -> Option<EmojiAssetId> {
        match grapheme {
            "🚀" => Some(EmojiAssetId::Rocket),
            "🎉" => Some(EmojiAssetId::PartyPopper),
            "✅" => Some(EmojiAssetId::CheckMark),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn static_svg_texture_count(&self) -> usize {
        self.static_svg_textures.len()
    }

    #[cfg(test)]
    pub(crate) fn shaped_text_texture_count(&self) -> usize {
        self.shaped_text_textures.len()
    }

    #[cfg(test)]
    pub(crate) fn glyph_path_count(&self) -> usize {
        self.render_cache.stats_snapshot().glyph_path_entries
    }

    #[cfg(test)]
    pub(crate) fn glyph_atlas_entry_count(&self) -> usize {
        self.glyph_atlas.stats().entries
    }

    pub fn stats_snapshot(&self) -> AssetRegistryStats {
        AssetRegistryStats {
            static_svg_textures: self.static_svg_textures.len(),
            shaped_text_textures: self.shaped_text_textures.len(),
            texture_cache_hits: self.texture_cache_hits,
            texture_cache_misses: self.texture_cache_misses,
            texture_uploads: self.texture_uploads,
            texture_upload_bytes: self.texture_upload_bytes,
            render: self.render_cache.stats_snapshot(),
        }
    }

    fn font_definitions() -> FontDefinitions {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "noto-sans".to_owned(),
            FontData::from_static(include_demo_asset!("fonts/NotoSans-Regular.ttf")).into(),
        );
        fonts.font_data.insert(
            "noto-sans-arabic".to_owned(),
            FontData::from_static(include_demo_asset!("fonts/NotoSansArabic-Regular.ttf")).into(),
        );
        fonts.font_data.insert(
            "noto-sans-cjk".to_owned(),
            FontData::from_static(include_demo_asset!("fonts/NotoSansCJK-Regular.ttc")).into(),
        );
        fonts.font_data.insert(
            "noto-sans-myanmar".to_owned(),
            FontData::from_static(include_demo_asset!("fonts/NotoSansMyanmar-Regular.ttf")).into(),
        );
        fonts.font_data.insert(
            "noto-colr-emoji".to_owned(),
            FontData::from_static(include_demo_asset!("fonts/Noto-COLRv1.ttf")).into(),
        );
        fonts.font_data.insert(
            "noto-sans-mono".to_owned(),
            FontData::from_static(include_demo_asset!("fonts/NotoSansMono-Regular.ttf")).into(),
        );

        let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
        proportional.insert(0, "noto-sans".to_owned());
        proportional.insert(1, "noto-sans-arabic".to_owned());
        proportional.insert(2, "noto-sans-cjk".to_owned());
        proportional.insert(3, "noto-sans-myanmar".to_owned());
        proportional.insert(4, "noto-colr-emoji".to_owned());

        let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
        monospace.insert(0, "noto-sans-mono".to_owned());
        monospace.insert(1, "noto-sans-arabic".to_owned());
        monospace.insert(2, "noto-sans-cjk".to_owned());
        monospace.insert(3, "noto-sans-myanmar".to_owned());
        monospace.insert(4, "noto-colr-emoji".to_owned());

        fonts
    }
}

pub fn shaped_text_baseline_metrics(
    engine: &PretextEngine,
    request: ShapedTextRasterRequest<'_>,
) -> BaselineMetrics {
    pretext_render::text_baseline_metrics(engine, text_raster_request(request))
}

fn text_raster_request(request: ShapedTextRasterRequest<'_>) -> TextRasterRequest<'_> {
    TextRasterRequest {
        text: request.text,
        style: request.style,
        direction: request.direction,
        slot_height: request.slot_height,
        padding_x: request.padding_x,
        padding_y: request.padding_y,
        slack_x: request.slack_x,
        slack_y: request.slack_y,
        baseline_mode: request.baseline_mode,
    }
}

fn svg_texture_name(key: SvgTextureKey) -> String {
    format!(
        "pretext-egui/svg/{:?}/{:?}x{:?}",
        key.asset_id, key.size[0], key.size[1]
    )
}

fn shaped_text_texture_name(key: ShapedTextTextureKey) -> String {
    let mut state = std::collections::hash_map::DefaultHasher::new();
    key.hash(&mut state);
    format!("pretext-egui/shaped-text/{:016x}", state.finish())
}

fn alpha_mask_image(size: [usize; 2], alpha_pixels: &[u8]) -> ColorImage {
    let pixels = alpha_pixels
        .iter()
        .map(|alpha| egui::Color32::from_white_alpha(*alpha))
        .collect();
    ColorImage::new(size, pixels)
}

fn rasterize_svg(
    svg_bytes: &[u8],
    size: [usize; 2],
    load_bundled_fonts: bool,
) -> Option<ColorImage> {
    let mut options = usvg::Options::default();
    if load_bundled_fonts {
        let fontdb = options.fontdb_mut();
        for data in AssetRegistry::bundled_font_data() {
            fontdb.load_font_data(data);
        }
        fontdb.set_sans_serif_family("Noto Sans");
        fontdb.set_monospace_family("Noto Sans Mono");
    }
    let tree = usvg::Tree::from_data(svg_bytes, &options).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(size[0] as u32, size[1] as u32)?;
    let svg_size = tree.size();
    let scale_x = size[0] as f32 / svg_size.width();
    let scale_y = size[1] as f32 / svg_size.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        size[0] as u32,
        size[1] as u32,
        pixmap.data().to_vec(),
    )?;
    let pixels = image
        .pixels()
        .map(|pixel| egui::Color32::from_rgba_premultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
        .collect();
    Some(ColorImage::new(size, pixels))
}

fn transparent_image(size: [usize; 2]) -> ColorImage {
    let pixels = vec![egui::Color32::from_rgba_premultiplied(0, 0, 0, 0); size[0] * size[1]];
    ColorImage::new(size, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{FontId, RawInput, TextureId};
    use pretext::{ParagraphDirection, PrepareOptions, WhiteSpaceMode};

    fn engine() -> PretextEngine {
        PretextEngine::with_font_data_and_system_fonts(AssetRegistry::bundled_font_data(), false)
    }

    fn default_style() -> TextStyleSpec {
        TextStyleSpec {
            families: vec![
                "Noto Sans".to_owned(),
                "Noto Sans Arabic".to_owned(),
                "Noto Color Emoji".to_owned(),
            ],
            size_px: 16.0,
            weight: 400,
            italic: false,
        }
    }

    #[test]
    fn ui_fonts_prefer_local_colr_emoji_font() {
        let fonts = AssetRegistry::font_definitions();
        let proportional = fonts
            .families
            .get(&FontFamily::Proportional)
            .expect("proportional family");
        let local_emoji = proportional
            .iter()
            .position(|name| name == "noto-colr-emoji")
            .expect("expected local COLRv1 emoji font in proportional family");
        let builtin_emoji = proportional
            .iter()
            .position(|name| name == "NotoEmoji-Regular")
            .expect("expected builtin emoji fallback in proportional family");

        assert!(local_emoji < builtin_emoji);
        assert!(fonts.font_data.contains_key("noto-colr-emoji"));
    }

    #[test]
    fn installed_ui_fonts_cover_mixed_arabic_and_emoji_text() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        assets.install_fonts(&ctx);

        let mut probe = None;
        let _ = ctx.run(RawInput::default(), |ctx| {
            let font_id = FontId::new(16.0, FontFamily::Proportional);
            probe = Some(ctx.fonts_mut(|fonts| {
                (
                    fonts.has_glyphs(&font_id, "بدأت الرحلة 🚀"),
                    fonts.glyph_width(&font_id, '🚀'),
                )
            }));
        });
        let (supports_sample, rocket_width) = probe.expect("expected probe result");

        assert!(supports_sample);
        assert!(rocket_width > 0.0);
    }

    #[test]
    fn bundled_svg_texture_reuses_canonical_cache_entry() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        let first =
            assets.bundled_svg_texture(SvgAssetId::Emoji(EmojiAssetId::Rocket), [96, 96], &ctx);
        let second =
            assets.bundled_svg_texture(SvgAssetId::Emoji(EmojiAssetId::Rocket), [96, 96], &ctx);

        assert_eq!(assets.static_svg_texture_count(), 1);
        assert_eq!(first.id(), second.id());
    }

    #[test]
    fn alpha_mask_image_uses_valid_white_alpha_pixels() {
        let image = alpha_mask_image([3, 1], &[0, 64, 255]);

        assert_eq!(image.pixels[0], egui::Color32::from_white_alpha(0));
        assert_eq!(image.pixels[1], egui::Color32::from_white_alpha(64));
        assert_eq!(image.pixels[2], egui::Color32::from_white_alpha(255));
    }

    #[test]
    fn shaped_text_texture_reuses_generated_texture_and_glyph_paths() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        let engine = engine();
        let style = default_style();
        let request = ShapedTextRasterRequest {
            text: "بدأت الرحلة",
            style: &style,
            direction: BidiDirection::Rtl,
            color: egui::Color32::WHITE,
            fragment_width: 72.0,
            slot_height: 22.0,
            padding_x: 2.0,
            padding_y: 2.0,
            slack_x: 2.0,
            slack_y: 2.0,
            baseline_mode: BaselineMode::AutoFontMetrics,
            texture_options: TextureOptions::NEAREST,
        };

        let first = assets
            .shaped_text_texture(&engine, request, &ctx)
            .expect("expected texture");
        let after_first_textures = assets.shaped_text_texture_count();
        let after_first_paths = assets.glyph_path_count();
        let second = assets
            .shaped_text_texture(&engine, request, &ctx)
            .expect("expected cached texture");

        assert_eq!(after_first_textures, assets.shaped_text_texture_count());
        assert_eq!(after_first_paths, assets.glyph_path_count());
        assert_eq!(first.handle.id(), second.handle.id());
        assert_ne!(first.handle.id(), TextureId::default());
    }

    #[test]
    fn bundled_font_data_drives_pretext_engine() {
        let engine = engine();
        let prepared = engine.prepare(
            "emoji ✅ and Arabic العربية",
            &default_style(),
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );
        let layout = engine.layout(&prepared, 220.0, 20.0);

        assert!(layout.line_count >= 1);
    }

    #[test]
    fn paint_line_glyph_runs_reuses_cached_atlas_entries() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        let engine = engine();
        let style = default_style();
        let prepared = engine.prepare_with_segments(
            "Atlas العربية",
            &style,
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );
        let layout = engine.layout_with_lines(&prepared, 240.0, 22.0);
        let glyph_runs = engine.line_glyph_runs(&prepared, &layout.lines[0]);

        let _ = ctx.run(RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("glyph-atlas-first"),
            ));
            assert!(assets.paint_line_glyph_runs(
                &painter,
                8.0,
                8.0,
                &glyph_runs,
                &style,
                22.0,
                egui::Color32::WHITE,
                ctx,
                &engine,
            ));
        });
        let entries_after_first = assets.glyph_atlas_entry_count();
        let uploads_after_first = assets.stats_snapshot().texture_uploads;

        let _ = ctx.run(RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("glyph-atlas-second"),
            ));
            assert!(assets.paint_line_glyph_runs(
                &painter,
                8.0,
                8.0,
                &glyph_runs,
                &style,
                22.0,
                egui::Color32::WHITE,
                ctx,
                &engine,
            ));
        });

        assert!(entries_after_first > 0);
        assert_eq!(entries_after_first, assets.glyph_atlas_entry_count());
        assert_eq!(uploads_after_first, assets.stats_snapshot().texture_uploads);
    }
}

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

use ahash::AHashSet;
mod glyph_atlas;

use egui::{ColorImage, FontData, FontDefinitions, FontFamily, TextureHandle, TextureOptions};
use image::{ImageBuffer, Rgba};
use lru::LruCache;
use pretext::font_catalog::FontId;
use pretext::{
    BidiDirection, LayoutLine, LayoutLineGlyphRun, LayoutLineVisualRun, LayoutWithLinesResult,
    LayoutWithRunsResult, PrepareOptions, PreparedTextWithSegments, PretextEngine, TextStyleSpec,
};
pub use pretext_render::{BaselineMetrics, BaselineMode};
use pretext_render::{RenderStatsSnapshot, TextRasterRequest, TextRenderCache};
use resvg::usvg;
use unicode_segmentation::UnicodeSegmentation;

use crate::glyph_atlas::{GlyphAtlas, GlyphWarmResult};
pub use crate::glyph_atlas::{GlyphAtlasStats, GlyphSceneBuilder};

const SHAPED_TEXT_TEXTURE_CACHE_CAPACITY: usize = 1024;
const WARMUP_LINE_HEIGHT_MULTIPLIER: f32 = 1.5;

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

#[derive(Clone, Copy)]
pub struct EmojiOverlayOptions<'a> {
    pub style: &'a TextStyleSpec,
    pub slot_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub slack_x: f32,
    pub slack_y: f32,
    pub baseline_mode: BaselineMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmojiOverlay {
    pub start: f32,
    pub end: f32,
    pub emoji_id: EmojiAssetId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmojiOverlayRun {
    pub line_offset: f32,
    pub width: f32,
    pub direction: BidiDirection,
    pub baseline_metrics: BaselineMetrics,
    pub emojis: Vec<EmojiOverlay>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PretextParagraphLine {
    pub line: LayoutLine,
    pub visual_runs: Vec<LayoutLineVisualRun>,
    pub glyph_runs: Vec<LayoutLineGlyphRun>,
    pub emoji_overlays: Vec<EmojiOverlayRun>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PretextParagraphLayout {
    pub width: f32,
    pub height: f32,
    pub line_count: usize,
    pub lines: Vec<PretextParagraphLine>,
}

#[derive(Clone, Debug)]
pub struct PretextParagraphPaintOptions<'a> {
    pub style: &'a TextStyleSpec,
    pub line_height: f32,
    pub color: egui::Color32,
    pub fallback_font: egui::FontId,
    pub fallback_align: egui::Align2,
    pub emoji_size: f32,
    pub emoji_slot_height: f32,
}

#[derive(Clone, Debug)]
pub struct PretextFragmentPaintOptions<'a> {
    pub style: &'a TextStyleSpec,
    pub line_height: f32,
    pub color: egui::Color32,
    pub fallback_font: egui::FontId,
    pub fallback_align: egui::Align2,
    pub emoji_size: f32,
    pub emoji_slot_height: f32,
}

#[derive(Clone, Copy)]
pub struct PositionedTextRunRef<'a> {
    pub x: f32,
    pub y: f32,
    pub text: &'a str,
    pub glyph_runs: &'a [LayoutLineGlyphRun],
    pub emoji_overlays: &'a [EmojiOverlayRun],
}

#[derive(Clone)]
pub struct StyledPositionedTextRunRef<'a, 'b> {
    pub x: f32,
    pub y: f32,
    pub text: &'a str,
    pub glyph_runs: &'a [LayoutLineGlyphRun],
    pub emoji_overlays: &'a [EmojiOverlayRun],
    pub options: PretextFragmentPaintOptions<'b>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssetRegistryStats {
    pub static_svg_textures: usize,
    pub shaped_text_textures: usize,
    pub texture_cache_hits: u64,
    pub texture_cache_misses: u64,
    pub texture_uploads: u64,
    pub texture_upload_bytes: u64,
    pub atlas_hits: u64,
    pub atlas_misses: u64,
    pub atlas_pages: usize,
    pub atlas_entries: usize,
    pub warmup_queue_depth: usize,
    pub mesh_flushes: u64,
    pub glyph_quads: u64,
    pub render: RenderStatsSnapshot,
}

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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct AtlasWarmupKey {
    engine_revision: u64,
    bucket: AtlasWarmupBucket,
    size_px_q: u32,
    pixels_per_point_q: u32,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct WarmupGlyphKey {
    face_id: FontId,
    glyph_id: u16,
}

struct AtlasWarmupJob {
    key: AtlasWarmupKey,
    size_px: f32,
    pixels_per_point: f32,
    glyphs: Vec<WarmupGlyphKey>,
    cursor: usize,
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
    mesh_flushes: u64,
    glyph_quads: u64,
    warmup_engine_revision: Option<u64>,
    pending_warmups: VecDeque<AtlasWarmupJob>,
    completed_warmups: AHashSet<AtlasWarmupKey>,
}

pub struct PretextParagraph<'a> {
    layout: &'a PretextParagraphLayout,
    engine: &'a PretextEngine,
    assets: &'a mut AssetRegistry,
    paint_options: PretextParagraphPaintOptions<'a>,
    desired_width: Option<f32>,
    sense: egui::Sense,
}

#[derive(Clone)]
struct PendingFallbackText {
    origin: egui::Pos2,
    text: String,
    fallback_align: egui::Align2,
    fallback_font: egui::FontId,
    color: egui::Color32,
}

#[derive(Clone)]
struct PendingEmojiPaint {
    line_left: f32,
    slot_top: f32,
    overlay_runs: Vec<EmojiOverlayRun>,
    emoji_size: f32,
    slot_height: f32,
}

pub struct PretextFragmentPainter {
    scene: GlyphSceneBuilder,
    pending_fallbacks: Vec<PendingFallbackText>,
    pending_emoji: Vec<PendingEmojiPaint>,
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
            mesh_flushes: 0,
            glyph_quads: 0,
            warmup_engine_revision: None,
            pending_warmups: VecDeque::new(),
            completed_warmups: AHashSet::new(),
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

    pub fn begin_glyph_scene(&self) -> GlyphSceneBuilder {
        self.glyph_atlas.begin_scene()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append_line_glyph_runs_to_scene(
        &mut self,
        scene: &mut GlyphSceneBuilder,
        x: f32,
        y: f32,
        glyph_runs: &[LayoutLineGlyphRun],
        style: &TextStyleSpec,
        line_height: f32,
        color: egui::Color32,
        ctx: &egui::Context,
        engine: &PretextEngine,
    ) -> bool {
        self.glyph_atlas.append_line_glyph_runs(
            scene,
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

    pub fn flush_glyph_scene(
        &mut self,
        painter: &egui::Painter,
        scene: &mut GlyphSceneBuilder,
    ) -> bool {
        let flush_stats = self.glyph_atlas.flush_scene(painter, scene);
        self.mesh_flushes += flush_stats.mesh_flushes;
        self.glyph_quads += flush_stats.glyph_quads;
        flush_stats.painted
    }

    pub fn enqueue_atlas_warmup(
        &mut self,
        bucket: AtlasWarmupBucket,
        style: &TextStyleSpec,
        seed_texts: &[&str],
        engine: &PretextEngine,
        ctx: &egui::Context,
    ) {
        self.reset_warmups_if_engine_changed(engine.revision());
        let pixels_per_point = ctx.pixels_per_point().max(1.0);
        let key = AtlasWarmupKey {
            engine_revision: engine.revision(),
            bucket,
            size_px_q: quantize_bucket(style.size_px),
            pixels_per_point_q: quantize_bucket(pixels_per_point),
        };
        if self.completed_warmups.contains(&key)
            || self.pending_warmups.iter().any(|job| job.key == key)
        {
            return;
        }

        let glyphs = collect_warmup_glyphs(engine, style, seed_texts);
        if glyphs.is_empty() {
            self.completed_warmups.insert(key);
            return;
        }

        self.pending_warmups.push_back(AtlasWarmupJob {
            key,
            size_px: style.size_px,
            pixels_per_point,
            glyphs,
            cursor: 0,
        });
    }

    pub fn tick_atlas_warmup(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        glyph_budget: usize,
        page_budget: usize,
    ) -> bool {
        self.reset_warmups_if_engine_changed(engine.revision());
        if self.pending_warmups.is_empty() || glyph_budget == 0 {
            return false;
        }

        let mut misses = 0usize;
        while let Some(job) = self.pending_warmups.front_mut() {
            if self.glyph_atlas.stats().pages >= page_budget {
                self.pending_warmups.clear();
                return false;
            }
            while job.cursor < job.glyphs.len() {
                let glyph = job.glyphs[job.cursor];
                job.cursor += 1;
                let Some(result) = self.glyph_atlas.warm_glyph(
                    ctx,
                    engine,
                    glyph.face_id,
                    glyph.glyph_id,
                    job.size_px,
                    job.pixels_per_point,
                    &mut self.texture_uploads,
                    &mut self.texture_upload_bytes,
                ) else {
                    continue;
                };
                if result == GlyphWarmResult::Miss {
                    misses += 1;
                    if misses >= glyph_budget {
                        ctx.request_repaint();
                        return true;
                    }
                }
            }

            let finished = self
                .pending_warmups
                .pop_front()
                .expect("warmup job should exist");
            self.completed_warmups.insert(finished.key);
            if misses >= glyph_budget {
                break;
            }
        }

        if !self.pending_warmups.is_empty() {
            ctx.request_repaint();
        }
        !self.pending_warmups.is_empty()
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
        let atlas = self.glyph_atlas.stats();
        AssetRegistryStats {
            static_svg_textures: self.static_svg_textures.len(),
            shaped_text_textures: self.shaped_text_textures.len(),
            texture_cache_hits: self.texture_cache_hits,
            texture_cache_misses: self.texture_cache_misses,
            texture_uploads: self.texture_uploads,
            texture_upload_bytes: self.texture_upload_bytes,
            atlas_hits: atlas.hits,
            atlas_misses: atlas.misses,
            atlas_pages: atlas.pages,
            atlas_entries: atlas.entries,
            warmup_queue_depth: self.warmup_queue_depth(),
            mesh_flushes: self.mesh_flushes,
            glyph_quads: self.glyph_quads,
            render: self.render_cache.stats_snapshot(),
        }
    }

    fn reset_warmups_if_engine_changed(&mut self, engine_revision: u64) {
        if self.warmup_engine_revision == Some(engine_revision) {
            return;
        }
        self.warmup_engine_revision = Some(engine_revision);
        self.pending_warmups.clear();
        self.completed_warmups.clear();
    }

    fn warmup_queue_depth(&self) -> usize {
        self.pending_warmups
            .iter()
            .map(|job| job.glyphs.len().saturating_sub(job.cursor))
            .sum()
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

fn collect_warmup_glyphs(
    engine: &PretextEngine,
    style: &TextStyleSpec,
    seed_texts: &[&str],
) -> Vec<WarmupGlyphKey> {
    let mut seen = AHashSet::new();
    let mut output = Vec::new();

    for text in seed_texts {
        if text.is_empty() {
            continue;
        }
        let prepared = engine.prepare_with_segments(text, style, &PrepareOptions::default());
        let layout =
            engine.layout_with_runs(&prepared, 100_000.0, warmup_line_height(style.size_px));
        for line in &layout.lines {
            for run in &line.runs.glyph_runs {
                for glyph in &run.glyphs {
                    let key = WarmupGlyphKey {
                        face_id: glyph.face_id,
                        glyph_id: glyph.glyph_id,
                    };
                    if seen.insert(key) {
                        output.push(key);
                    }
                }
            }
        }
    }

    output
}

pub fn shaped_text_baseline_metrics(
    engine: &PretextEngine,
    request: ShapedTextRasterRequest<'_>,
) -> BaselineMetrics {
    pretext_render::text_baseline_metrics(engine, text_raster_request(request))
}

impl PretextParagraphLayout {
    pub fn from_prepared(
        engine: &PretextEngine,
        prepared: &PreparedTextWithSegments,
        wrap_width: f32,
        line_height: f32,
        emoji_options: Option<EmojiOverlayOptions<'_>>,
    ) -> Self {
        let layout = engine.layout_with_runs(prepared, wrap_width.max(1.0), line_height);
        Self::from_layout_with_runs(engine, layout, emoji_options)
    }

    pub fn from_layout(
        engine: &PretextEngine,
        prepared: &PreparedTextWithSegments,
        layout: LayoutWithLinesResult,
        emoji_options: Option<EmojiOverlayOptions<'_>>,
    ) -> Self {
        let width = layout
            .lines
            .iter()
            .fold(0.0f32, |max_width, line| max_width.max(line.width));
        let height = layout.height;
        let line_count = layout.line_count;
        let lines = layout
            .lines
            .into_iter()
            .map(|line| {
                let visual_runs = engine.line_visual_runs(prepared, &line);
                let raw_glyph_runs = engine.line_glyph_runs(prepared, &line);
                let (glyph_runs, emoji_overlays) = match emoji_options {
                    Some(options) => {
                        split_builtin_emoji_glyphs(&visual_runs, &raw_glyph_runs, options, engine)
                    }
                    None => (raw_glyph_runs, Vec::new()),
                };

                PretextParagraphLine {
                    line,
                    visual_runs,
                    glyph_runs,
                    emoji_overlays,
                }
            })
            .collect();

        Self {
            width,
            height,
            line_count,
            lines,
        }
    }

    pub fn from_layout_with_runs(
        engine: &PretextEngine,
        layout: LayoutWithRunsResult,
        emoji_options: Option<EmojiOverlayOptions<'_>>,
    ) -> Self {
        let width = layout
            .lines
            .iter()
            .fold(0.0f32, |max_width, line| max_width.max(line.line.width));
        let height = layout.height;
        let line_count = layout.line_count;
        let lines = layout
            .lines
            .into_iter()
            .map(|line_with_runs| {
                let (glyph_runs, emoji_overlays) = match emoji_options {
                    Some(options) => split_builtin_emoji_glyphs(
                        &line_with_runs.runs.visual_runs,
                        &line_with_runs.runs.glyph_runs,
                        options,
                        engine,
                    ),
                    None => (line_with_runs.runs.glyph_runs.clone(), Vec::new()),
                };

                PretextParagraphLine {
                    line: line_with_runs.line,
                    visual_runs: line_with_runs.runs.visual_runs,
                    glyph_runs,
                    emoji_overlays,
                }
            })
            .collect();

        Self {
            width,
            height,
            line_count,
            lines,
        }
    }
}

impl<'a> PretextParagraphPaintOptions<'a> {
    pub fn new(style: &'a TextStyleSpec, line_height: f32) -> Self {
        Self {
            style,
            line_height,
            color: egui::Color32::WHITE,
            fallback_font: egui::FontId::new(style.size_px, FontFamily::Proportional),
            fallback_align: egui::Align2::LEFT_TOP,
            emoji_size: line_height,
            emoji_slot_height: line_height,
        }
    }

    pub fn color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    pub fn fallback_font(mut self, fallback_font: egui::FontId) -> Self {
        self.fallback_font = fallback_font;
        self
    }

    pub fn fallback_align(mut self, fallback_align: egui::Align2) -> Self {
        self.fallback_align = fallback_align;
        self
    }

    pub fn emoji_size(mut self, emoji_size: f32) -> Self {
        self.emoji_size = emoji_size;
        self
    }

    pub fn emoji_slot_height(mut self, emoji_slot_height: f32) -> Self {
        self.emoji_slot_height = emoji_slot_height;
        self
    }
}

impl<'a> PretextFragmentPaintOptions<'a> {
    pub fn new(style: &'a TextStyleSpec, line_height: f32) -> Self {
        Self {
            style,
            line_height,
            color: egui::Color32::WHITE,
            fallback_font: egui::FontId::new(style.size_px, FontFamily::Proportional),
            fallback_align: egui::Align2::LEFT_TOP,
            emoji_size: line_height,
            emoji_slot_height: line_height,
        }
    }

    pub fn color(mut self, color: egui::Color32) -> Self {
        self.color = color;
        self
    }

    pub fn fallback_font(mut self, fallback_font: egui::FontId) -> Self {
        self.fallback_font = fallback_font;
        self
    }

    pub fn fallback_align(mut self, fallback_align: egui::Align2) -> Self {
        self.fallback_align = fallback_align;
        self
    }

    pub fn emoji_size(mut self, emoji_size: f32) -> Self {
        self.emoji_size = emoji_size;
        self
    }

    pub fn emoji_slot_height(mut self, emoji_slot_height: f32) -> Self {
        self.emoji_slot_height = emoji_slot_height;
        self
    }
}

impl PretextFragmentPainter {
    pub fn new(assets: &AssetRegistry) -> Self {
        Self {
            scene: new_glyph_scene(assets),
            pending_fallbacks: Vec::new(),
            pending_emoji: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_fragment(
        &mut self,
        x: f32,
        slot_top: f32,
        text: &str,
        glyph_runs: &[LayoutLineGlyphRun],
        emoji_overlays: &[EmojiOverlayRun],
        options: &PretextFragmentPaintOptions<'_>,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut AssetRegistry,
    ) {
        if !append_glyph_runs(
            &mut self.scene,
            x,
            slot_top,
            glyph_runs,
            options.style,
            options.line_height,
            options.color,
            ctx,
            engine,
            assets,
        ) && !(glyph_runs.is_empty() && !emoji_overlays.is_empty())
        {
            self.pending_fallbacks.push(PendingFallbackText {
                origin: egui::pos2(x, slot_top),
                text: text.to_owned(),
                fallback_align: options.fallback_align,
                fallback_font: options.fallback_font.clone(),
                color: options.color,
            });
        }

        if !emoji_overlays.is_empty() {
            self.pending_emoji.push(PendingEmojiPaint {
                line_left: x,
                slot_top,
                overlay_runs: emoji_overlays.to_vec(),
                emoji_size: options.emoji_size,
                slot_height: options.emoji_slot_height,
            });
        }
    }

    pub fn finish(
        mut self,
        painter: &egui::Painter,
        ctx: &egui::Context,
        assets: &mut AssetRegistry,
    ) -> bool {
        let painted = flush_glyph_scene(painter, &mut self.scene, assets);
        let had_fallbacks = !self.pending_fallbacks.is_empty();
        let had_emoji = !self.pending_emoji.is_empty();
        for fallback in self.pending_fallbacks {
            painter.text(
                fallback.origin,
                fallback.fallback_align,
                fallback.text,
                fallback.fallback_font,
                fallback.color,
            );
        }
        for emoji in self.pending_emoji {
            paint_emoji_overlays(
                painter,
                emoji.line_left,
                emoji.slot_top,
                &emoji.overlay_runs,
                emoji.emoji_size,
                emoji.slot_height,
                ctx,
                assets,
            );
        }

        painted || had_fallbacks || had_emoji
    }
}

impl<'a> PretextParagraph<'a> {
    pub fn new(
        layout: &'a PretextParagraphLayout,
        style: &'a TextStyleSpec,
        line_height: f32,
        engine: &'a PretextEngine,
        assets: &'a mut AssetRegistry,
    ) -> Self {
        Self {
            layout,
            engine,
            assets,
            paint_options: PretextParagraphPaintOptions::new(style, line_height),
            desired_width: None,
            sense: egui::Sense::hover(),
        }
    }

    pub fn color(mut self, color: egui::Color32) -> Self {
        self.paint_options = self.paint_options.color(color);
        self
    }

    pub fn fallback_font(mut self, fallback_font: egui::FontId) -> Self {
        self.paint_options = self.paint_options.fallback_font(fallback_font);
        self
    }

    pub fn fallback_align(mut self, fallback_align: egui::Align2) -> Self {
        self.paint_options = self.paint_options.fallback_align(fallback_align);
        self
    }

    pub fn emoji_size(mut self, emoji_size: f32) -> Self {
        self.paint_options = self.paint_options.emoji_size(emoji_size);
        self
    }

    pub fn emoji_slot_height(mut self, emoji_slot_height: f32) -> Self {
        self.paint_options = self.paint_options.emoji_slot_height(emoji_slot_height);
        self
    }

    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    pub fn sense(mut self, sense: egui::Sense) -> Self {
        self.sense = sense;
        self
    }
}

impl egui::Widget for PretextParagraph<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let desired_size = egui::vec2(
            self.desired_width.unwrap_or(self.layout.width).max(0.0),
            self.layout.height.max(0.0),
        );
        let (rect, response) = ui.allocate_exact_size(desired_size, self.sense);
        let painter = ui.painter_at(rect);
        paint_pretext_paragraph(
            &painter,
            rect.min,
            self.layout,
            &self.paint_options,
            ui.ctx(),
            self.engine,
            self.assets,
        );
        response
    }
}

pub fn new_glyph_scene(assets: &AssetRegistry) -> GlyphSceneBuilder {
    assets.begin_glyph_scene()
}

#[allow(clippy::too_many_arguments)]
pub fn append_glyph_runs(
    scene: &mut GlyphSceneBuilder,
    x: f32,
    y: f32,
    glyph_runs: &[LayoutLineGlyphRun],
    style: &TextStyleSpec,
    line_height: f32,
    color: egui::Color32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) -> bool {
    assets.append_line_glyph_runs_to_scene(
        scene,
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

pub fn flush_glyph_scene(
    painter: &egui::Painter,
    scene: &mut GlyphSceneBuilder,
    assets: &mut AssetRegistry,
) -> bool {
    assets.flush_glyph_scene(painter, scene)
}

pub fn split_builtin_emoji_glyphs(
    visual_runs: &[LayoutLineVisualRun],
    glyph_runs: &[LayoutLineGlyphRun],
    options: EmojiOverlayOptions<'_>,
    engine: &PretextEngine,
) -> (Vec<LayoutLineGlyphRun>, Vec<EmojiOverlayRun>) {
    let mut run_left = 0.0f32;
    let mut output = Vec::with_capacity(glyph_runs.len());
    let mut overlays = Vec::new();

    for (visual_run, glyph_run) in visual_runs.iter().zip(glyph_runs.iter()) {
        if !contains_builtin_emoji(&visual_run.text) {
            output.push(glyph_run.clone());
            run_left += visual_run.width;
            continue;
        }

        let prefix_widths = engine.prefix_widths(&visual_run.text, options.style);
        let emoji_ranges = visual_run
            .text
            .grapheme_indices(true)
            .enumerate()
            .filter_map(|(index, (_, grapheme))| {
                AssetRegistry::builtin_emoji_for_grapheme(grapheme).map(|emoji_id| EmojiOverlay {
                    start: prefix_widths[index],
                    end: prefix_widths[index + 1],
                    emoji_id,
                })
            })
            .collect::<Vec<_>>();
        if emoji_ranges.is_empty() {
            output.push(glyph_run.clone());
            run_left += visual_run.width;
            continue;
        }

        let mut filtered = glyph_run.clone();
        filtered.glyphs.retain(|glyph| {
            let center_x = glyph.x + glyph.x_offset + glyph.advance * 0.5;
            !emoji_ranges.iter().any(|overlay| {
                let start = run_left + overlay.start;
                let end = run_left + overlay.end;
                center_x >= start && center_x <= end
            })
        });
        output.push(filtered);
        overlays.push(EmojiOverlayRun {
            line_offset: run_left,
            width: visual_run.width,
            direction: visual_run.direction,
            baseline_metrics: shaped_text_baseline_metrics(
                engine,
                ShapedTextRasterRequest {
                    text: &visual_run.text,
                    style: options.style,
                    direction: visual_run.direction,
                    color: egui::Color32::WHITE,
                    fragment_width: visual_run.width,
                    slot_height: options.slot_height,
                    padding_x: options.padding_x,
                    padding_y: options.padding_y,
                    slack_x: options.slack_x,
                    slack_y: options.slack_y,
                    baseline_mode: options.baseline_mode,
                    texture_options: egui::TextureOptions::NEAREST,
                },
            ),
            emojis: emoji_ranges,
        });
        run_left += visual_run.width;
    }

    if glyph_runs.len() > visual_runs.len() {
        output.extend(glyph_runs[visual_runs.len()..].iter().cloned());
    }

    (output, overlays)
}

pub fn strip_builtin_emoji_glyphs(
    visual_runs: &[LayoutLineVisualRun],
    glyph_runs: &[LayoutLineGlyphRun],
    style: &TextStyleSpec,
    engine: &PretextEngine,
) -> Vec<LayoutLineGlyphRun> {
    split_builtin_emoji_glyphs(
        visual_runs,
        glyph_runs,
        EmojiOverlayOptions {
            style,
            slot_height: style.size_px,
            padding_x: 0.0,
            padding_y: 0.0,
            slack_x: 0.0,
            slack_y: 0.0,
            baseline_mode: BaselineMode::AutoFontMetrics,
        },
        engine,
    )
    .0
}

pub fn paint_emoji_overlays(
    painter: &egui::Painter,
    line_left: f32,
    slot_top: f32,
    overlay_runs: &[EmojiOverlayRun],
    emoji_size: f32,
    slot_height: f32,
    ctx: &egui::Context,
    assets: &mut AssetRegistry,
) {
    for overlay_run in overlay_runs {
        let run_left = line_left + overlay_run.line_offset;
        for emoji in &overlay_run.emojis {
            let slot_width = (emoji.end - emoji.start).max(1.0);
            let slot_left = match overlay_run.direction {
                BidiDirection::Ltr => run_left + emoji.start,
                BidiDirection::Rtl => run_left + overlay_run.width - emoji.start - slot_width,
            };
            let size = emoji_size.min(slot_height).min(slot_width).max(1.0);
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    slot_left + (slot_width - size).max(0.0) * 0.5,
                    slot_top + overlay_run.baseline_metrics.square_top(size),
                ),
                egui::vec2(size, size),
            );
            let texture = assets.emoji_texture(emoji.emoji_id, [96, 96], ctx);
            painter.image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }
}

pub fn paint_pretext_paragraph(
    painter: &egui::Painter,
    origin: egui::Pos2,
    layout: &PretextParagraphLayout,
    options: &PretextParagraphPaintOptions<'_>,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    let fragment_options = PretextFragmentPaintOptions::new(options.style, options.line_height)
        .color(options.color)
        .fallback_font(options.fallback_font.clone())
        .fallback_align(options.fallback_align)
        .emoji_size(options.emoji_size)
        .emoji_slot_height(options.emoji_slot_height);
    let mut y = origin.y;
    let mut fragment_painter = PretextFragmentPainter::new(assets);
    for paragraph_line in &layout.lines {
        fragment_painter.push_fragment(
            origin.x,
            y,
            &paragraph_line.line.text,
            &paragraph_line.glyph_runs,
            &paragraph_line.emoji_overlays,
            &fragment_options,
            ctx,
            engine,
            assets,
        );
        y += options.line_height;
    }
    let _ = fragment_painter.finish(painter, ctx, assets);
}

pub fn paint_positioned_text_runs<'a>(
    painter: &egui::Painter,
    lines: impl IntoIterator<Item = PositionedTextRunRef<'a>>,
    options: &PretextFragmentPaintOptions<'_>,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) -> bool {
    let mut fragment_painter = PretextFragmentPainter::new(assets);
    for line in lines {
        fragment_painter.push_fragment(
            line.x,
            line.y,
            line.text,
            line.glyph_runs,
            line.emoji_overlays,
            options,
            ctx,
            engine,
            assets,
        );
    }
    fragment_painter.finish(painter, ctx, assets)
}

pub fn paint_styled_positioned_text_runs<'a, 'b>(
    painter: &egui::Painter,
    lines: impl IntoIterator<Item = StyledPositionedTextRunRef<'a, 'b>>,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) -> bool {
    let mut fragment_painter = PretextFragmentPainter::new(assets);
    for line in lines {
        fragment_painter.push_fragment(
            line.x,
            line.y,
            line.text,
            line.glyph_runs,
            line.emoji_overlays,
            &line.options,
            ctx,
            engine,
            assets,
        );
    }
    fragment_painter.finish(painter, ctx, assets)
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

fn contains_builtin_emoji(text: &str) -> bool {
    text.graphemes(true)
        .any(|grapheme| AssetRegistry::builtin_emoji_for_grapheme(grapheme).is_some())
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

fn quantize_bucket(value: f32) -> u32 {
    (value.max(0.0) * 64.0).round() as u32
}

fn warmup_line_height(size_px: f32) -> f32 {
    (size_px * WARMUP_LINE_HEIGHT_MULTIPLIER).max(size_px + 4.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{FontId, RawInput, Rect, TextureId};
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

    fn mono_style() -> TextStyleSpec {
        TextStyleSpec {
            families: vec![
                "Noto Sans Mono".to_owned(),
                "Noto Sans Arabic".to_owned(),
                "Noto Color Emoji".to_owned(),
            ],
            size_px: 18.0,
            weight: 400,
            italic: false,
        }
    }

    fn shape_uses_user_texture(shape: &egui::Shape) -> bool {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().any(shape_uses_user_texture),
            egui::Shape::Mesh(mesh) => mesh.texture_id != TextureId::default(),
            _ => shape.texture_id() != TextureId::default(),
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
    fn pretext_paragraph_layout_keeps_visual_runs_and_builtin_emoji_overlays() {
        let engine = engine();
        let style = default_style();
        let prepared = engine.prepare_with_segments(
            "بدأت الرحلة 🚀 and then kept going",
            &style,
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );
        let layout = PretextParagraphLayout::from_prepared(
            &engine,
            &prepared,
            220.0,
            22.0,
            Some(EmojiOverlayOptions {
                style: &style,
                slot_height: 22.0,
                padding_x: 2.0,
                padding_y: 2.0,
                slack_x: 2.0,
                slack_y: 2.0,
                baseline_mode: BaselineMode::AutoFontMetrics,
            }),
        );

        assert!(layout.line_count >= 1);
        assert!(layout.width > 0.0);
        assert!(layout
            .lines
            .iter()
            .flat_map(|line| line.visual_runs.iter())
            .any(|run| run.direction == BidiDirection::Rtl));
        assert!(layout
            .lines
            .iter()
            .flat_map(|line| line.emoji_overlays.iter())
            .flat_map(|overlay| overlay.emojis.iter())
            .any(|emoji| emoji.emoji_id == EmojiAssetId::Rocket));
    }

    #[test]
    fn pretext_paragraph_layout_from_prepared_uses_layout_with_runs() {
        let engine = engine();
        let style = default_style();
        let prepared = engine.prepare_with_segments(
            "Atlas العربية ✅",
            &style,
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );

        let before = engine.runtime_stats();
        let layout = PretextParagraphLayout::from_prepared(
            &engine,
            &prepared,
            240.0,
            22.0,
            Some(EmojiOverlayOptions {
                style: &style,
                slot_height: 22.0,
                padding_x: 2.0,
                padding_y: 2.0,
                slack_x: 2.0,
                slack_y: 2.0,
                baseline_mode: BaselineMode::AutoFontMetrics,
            }),
        );
        let after = engine.runtime_stats();

        assert!(layout.line_count >= 1);
        assert!(after.layout_with_runs_calls > before.layout_with_runs_calls);
        assert_eq!(
            after.layout_with_lines_calls,
            before.layout_with_lines_calls
        );
        assert_eq!(after.line_visual_runs_calls, before.line_visual_runs_calls);
        assert_eq!(after.line_glyph_runs_calls, before.line_glyph_runs_calls);
        assert_eq!(after.line_runs_calls, before.line_runs_calls);
    }

    #[test]
    fn pretext_paragraph_widget_uses_atlas_and_svg_textures_without_shaped_text_cache() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        let engine = engine();
        let style = default_style();
        let prepared = engine.prepare_with_segments(
            "Atlas العربية ✅",
            &style,
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );
        let layout = PretextParagraphLayout::from_prepared(
            &engine,
            &prepared,
            240.0,
            22.0,
            Some(EmojiOverlayOptions {
                style: &style,
                slot_height: 22.0,
                padding_x: 2.0,
                padding_y: 2.0,
                slack_x: 2.0,
                slack_y: 2.0,
                baseline_mode: BaselineMode::AutoFontMetrics,
            }),
        );
        let desired_width = 180.0;
        let mut response_rect = None;

        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(480.0, 240.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = ui.add(
                        PretextParagraph::new(&layout, &style, 22.0, &engine, &mut assets)
                            .color(egui::Color32::WHITE)
                            .emoji_size(18.0)
                            .emoji_slot_height(20.0)
                            .desired_width(desired_width),
                    );
                    response_rect = Some(response.rect);
                });
            },
        );
        let response_rect = response_rect.expect("paragraph widget should allocate a rect");
        let stats = assets.stats_snapshot();

        assert!((response_rect.width() - desired_width).abs() < 0.01);
        assert!((response_rect.height() - layout.height).abs() < 0.01);
        assert!(stats.atlas_entries > 0);
        assert_eq!(stats.shaped_text_textures, 0);
        assert!(output
            .shapes
            .iter()
            .any(|clipped| shape_uses_user_texture(&clipped.shape)));
    }

    #[test]
    fn positioned_text_runs_use_atlas_without_shaped_text_cache() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        let engine = engine();
        let style = default_style();
        let prepared = engine.prepare_with_segments(
            "Positioned العربية",
            &style,
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );
        let layout = engine.layout_with_lines(&prepared, 320.0, 22.0);
        let glyph_runs = engine.line_glyph_runs(&prepared, &layout.lines[0]);
        let options = PretextFragmentPaintOptions::new(&style, 22.0)
            .color(egui::Color32::WHITE)
            .fallback_align(egui::Align2::LEFT_TOP);

        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(480.0, 240.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("positioned-text-runs"),
                ));
                let _ = paint_positioned_text_runs(
                    &painter,
                    [PositionedTextRunRef {
                        x: 24.0,
                        y: 32.0,
                        text: &layout.lines[0].text,
                        glyph_runs: &glyph_runs,
                        emoji_overlays: &[],
                    }],
                    &options,
                    ctx,
                    &engine,
                    &mut assets,
                );
            },
        );
        let stats = assets.stats_snapshot();

        assert!(stats.atlas_entries > 0);
        assert_eq!(stats.shaped_text_textures, 0);
        assert!(output
            .shapes
            .iter()
            .any(|clipped| shape_uses_user_texture(&clipped.shape)));
    }

    #[test]
    fn styled_positioned_text_runs_use_atlas_without_shaped_text_cache() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        let engine = engine();
        let style = default_style();
        let mono = mono_style();
        let prepared_body = engine.prepare_with_segments(
            "Styled body العربية",
            &style,
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );
        let prepared_mono = engine.prepare_with_segments(
            "Mono 101",
            &mono,
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        );
        let layout_body = engine.layout_with_lines(&prepared_body, 320.0, 22.0);
        let layout_mono = engine.layout_with_lines(&prepared_mono, 320.0, 24.0);
        let glyph_runs_body = engine.line_glyph_runs(&prepared_body, &layout_body.lines[0]);
        let glyph_runs_mono = engine.line_glyph_runs(&prepared_mono, &layout_mono.lines[0]);

        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(480.0, 240.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("styled-positioned-text-runs"),
                ));
                let _ = paint_styled_positioned_text_runs(
                    &painter,
                    [
                        StyledPositionedTextRunRef {
                            x: 24.0,
                            y: 32.0,
                            text: &layout_body.lines[0].text,
                            glyph_runs: &glyph_runs_body,
                            emoji_overlays: &[],
                            options: PretextFragmentPaintOptions::new(&style, 22.0)
                                .color(egui::Color32::WHITE)
                                .fallback_font(FontId::new(style.size_px, FontFamily::Proportional))
                                .fallback_align(egui::Align2::LEFT_TOP),
                        },
                        StyledPositionedTextRunRef {
                            x: 24.0,
                            y: 66.0,
                            text: &layout_mono.lines[0].text,
                            glyph_runs: &glyph_runs_mono,
                            emoji_overlays: &[],
                            options: PretextFragmentPaintOptions::new(&mono, 24.0)
                                .color(egui::Color32::LIGHT_GRAY)
                                .fallback_font(FontId::new(mono.size_px, FontFamily::Monospace))
                                .fallback_align(egui::Align2::LEFT_TOP),
                        },
                    ],
                    ctx,
                    &engine,
                    &mut assets,
                );
            },
        );
        let stats = assets.stats_snapshot();

        assert!(stats.atlas_entries > 0);
        assert_eq!(stats.shaped_text_textures, 0);
        assert!(output
            .shapes
            .iter()
            .any(|clipped| shape_uses_user_texture(&clipped.shape)));
    }

    #[test]
    fn fragment_painter_falls_back_without_atlas_glyphs() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        let engine = engine();
        let style = default_style();
        let mut painted = false;

        let output = ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 120.0),
                )),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("fragment-fallback"),
                ));
                let options = PretextFragmentPaintOptions::new(&style, 22.0)
                    .color(egui::Color32::WHITE)
                    .fallback_font(FontId::new(style.size_px, FontFamily::Proportional))
                    .fallback_align(egui::Align2::LEFT_TOP);
                let mut fragment_painter = PretextFragmentPainter::new(&assets);
                fragment_painter.push_fragment(
                    24.0,
                    32.0,
                    "Fallback only",
                    &[],
                    &[],
                    &options,
                    ctx,
                    &engine,
                    &mut assets,
                );
                painted = fragment_painter.finish(&painter, ctx, &mut assets);
            },
        );
        let stats = assets.stats_snapshot();

        assert!(painted);
        assert_eq!(stats.atlas_entries, 0);
        assert_eq!(stats.shaped_text_textures, 0);
        assert!(!output.shapes.is_empty());
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

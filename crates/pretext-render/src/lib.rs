use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use lru::LruCache;
use parking_lot::Mutex;
use pretext::advanced::ShapedTextSpan;
use pretext::font_catalog::FontId;
use pretext::{BidiDirection, PretextEngine, PretextStyle as TextStyleSpec};

const RASTER_CACHE_CAPACITY: usize = 1024;
const GLYPH_PATH_CACHE_CAPACITY: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BaselineMode {
    AutoFontMetrics,
    FixedBaselinePx(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BaselineMetrics {
    pub baseline_px: f32,
    pub ascent_px: f32,
    pub descent_px: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct TextRasterRequest<'a> {
    pub text: &'a str,
    pub style: &'a TextStyleSpec,
    pub direction: BidiDirection,
    pub slot_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub slack_x: f32,
    pub slack_y: f32,
    pub baseline_mode: BaselineMode,
}

#[derive(Clone)]
pub struct RasterizedText {
    cache_id: u64,
    logical_size: LogicalSize,
    pixel_size: [usize; 2],
    alpha_pixels: Arc<[u8]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalSize {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStatsSnapshot {
    pub raster_cache_entries: usize,
    pub glyph_path_entries: usize,
    pub raster_cache_hits: u64,
    pub raster_cache_misses: u64,
    pub rasterizations: u64,
    pub glyph_path_hits: u64,
    pub glyph_path_misses: u64,
}

#[derive(Default)]
struct RenderStats {
    raster_cache_hits: AtomicU64,
    raster_cache_misses: AtomicU64,
    rasterizations: AtomicU64,
    glyph_path_hits: AtomicU64,
    glyph_path_misses: AtomicU64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum BaselineModeKey {
    AutoFontMetrics,
    FixedBaselinePx { baseline_px_q: u32 },
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct RasterCacheKey {
    text_hash: u64,
    style_hash: u64,
    direction: BidiDirection,
    slot_height_q: u32,
    padding_x_q: u32,
    padding_y_q: u32,
    slack_x_q: u32,
    slack_y_q: u32,
    baseline_mode: BaselineModeKey,
    raster_scale_q: u32,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct GlyphPathKey {
    face_id: FontId,
    glyph_id: u16,
}

pub struct TextRenderCache {
    rasterized_text: Mutex<LruCache<RasterCacheKey, Arc<RasterizedText>>>,
    glyph_paths: Mutex<LruCache<GlyphPathKey, Arc<tiny_skia::Path>>>,
    stats: RenderStats,
}

impl Default for TextRenderCache {
    fn default() -> Self {
        Self {
            rasterized_text: Mutex::new(LruCache::new(
                NonZeroUsize::new(RASTER_CACHE_CAPACITY).expect("raster cache capacity"),
            )),
            glyph_paths: Mutex::new(LruCache::new(
                NonZeroUsize::new(GLYPH_PATH_CACHE_CAPACITY).expect("glyph path cache capacity"),
            )),
            stats: RenderStats::default(),
        }
    }
}

impl TextRasterRequest<'_> {
    pub fn logical_size(&self, content_width: f32) -> LogicalSize {
        LogicalSize {
            width: content_width.ceil().max(1.0) + self.padding_x * 2.0 + self.slack_x,
            height: self.slot_height.ceil().max(1.0) + self.padding_y * 2.0 + self.slack_y,
        }
    }
}

impl RasterizedText {
    pub fn cache_id(&self) -> u64 {
        self.cache_id
    }

    pub fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    pub fn pixel_size(&self) -> [usize; 2] {
        self.pixel_size
    }

    pub fn alpha_pixels(&self) -> Arc<[u8]> {
        self.alpha_pixels.clone()
    }
}

impl BaselineMetrics {
    pub fn content_top_px(&self) -> f32 {
        self.baseline_px - self.ascent_px
    }

    pub fn content_height_px(&self) -> f32 {
        (self.ascent_px + self.descent_px).max(1.0)
    }

    pub fn square_top(&self, square_size: f32) -> f32 {
        self.content_top_px() + (self.content_height_px() - square_size) * 0.5
    }
}

pub fn text_baseline_metrics(
    engine: &PretextEngine,
    request: TextRasterRequest<'_>,
) -> BaselineMetrics {
    let spans = engine.shape_text_spans_shared(request.text, request.style, request.direction);
    shaped_text_baseline_metrics(&spans, &request, 1.0)
}

impl TextRenderCache {
    pub fn rasterized_text(
        &self,
        engine: &PretextEngine,
        request: TextRasterRequest<'_>,
        raster_scale: f32,
    ) -> Option<Arc<RasterizedText>> {
        let key = RasterCacheKey::new(&request, raster_scale);
        if let Some(cached) = self.rasterized_text.lock().get(&key).cloned() {
            self.stats.raster_cache_hits.fetch_add(1, Ordering::Relaxed);
            return Some(cached);
        }

        self.stats
            .raster_cache_misses
            .fetch_add(1, Ordering::Relaxed);

        let value = Arc::new(self.build_rasterized_text(engine, request, key)?);
        self.rasterized_text.lock().put(key, value.clone());
        Some(value)
    }

    pub fn stats_snapshot(&self) -> RenderStatsSnapshot {
        RenderStatsSnapshot {
            raster_cache_entries: self.rasterized_text.lock().len(),
            glyph_path_entries: self.glyph_paths.lock().len(),
            raster_cache_hits: self.stats.raster_cache_hits.load(Ordering::Relaxed),
            raster_cache_misses: self.stats.raster_cache_misses.load(Ordering::Relaxed),
            rasterizations: self.stats.rasterizations.load(Ordering::Relaxed),
            glyph_path_hits: self.stats.glyph_path_hits.load(Ordering::Relaxed),
            glyph_path_misses: self.stats.glyph_path_misses.load(Ordering::Relaxed),
        }
    }

    pub fn clear(&self) {
        self.rasterized_text.lock().clear();
        self.glyph_paths.lock().clear();
    }

    fn build_rasterized_text(
        &self,
        engine: &PretextEngine,
        request: TextRasterRequest<'_>,
        key: RasterCacheKey,
    ) -> Option<RasterizedText> {
        let spans = engine.shape_text_spans_shared(request.text, request.style, request.direction);
        let content_width = spans.iter().map(|span| span.width).sum::<f32>();
        let logical_size = request.logical_size(content_width);
        let pixel_size = [
            (logical_size.width * key.raster_scale()).ceil().max(1.0) as usize,
            (logical_size.height * key.raster_scale()).ceil().max(1.0) as usize,
        ];
        let mut pixmap = tiny_skia::Pixmap::new(pixel_size[0] as u32, pixel_size[1] as u32)?;
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        paint.anti_alias = true;

        let baseline_metrics = shaped_text_baseline_metrics(&spans, &request, key.raster_scale());
        let baseline = request.padding_y * key.raster_scale() + baseline_metrics.baseline_px;
        let mut span_left = request.padding_x * key.raster_scale();
        for span in spans.iter() {
            self.rasterize_shaped_text_span(
                &mut pixmap,
                &paint,
                span,
                request.style.size_px,
                span_left,
                baseline,
                key.raster_scale(),
            );
            span_left += span.width * key.raster_scale();
        }

        self.stats.rasterizations.fetch_add(1, Ordering::Relaxed);

        let alpha_pixels: Arc<[u8]> = pixmap
            .pixels()
            .iter()
            .map(|pixel| pixel.alpha())
            .collect::<Vec<_>>()
            .into();

        Some(RasterizedText {
            cache_id: key.cache_id(),
            logical_size,
            pixel_size,
            alpha_pixels,
        })
    }

    fn rasterize_shaped_text_span(
        &self,
        pixmap: &mut tiny_skia::Pixmap,
        paint: &tiny_skia::Paint<'_>,
        span: &ShapedTextSpan,
        font_size: f32,
        span_left: f32,
        baseline: f32,
        raster_scale: f32,
    ) {
        let Ok(face) = ttf_parser::Face::parse(span.face.data(), span.face.face_index()) else {
            return;
        };
        let units_per_em = span.face.units_per_em().max(1) as f32;
        let glyph_scale = font_size * raster_scale / units_per_em;
        let mut pen_x = span_left;

        for glyph in span.glyphs.iter() {
            let advance = glyph.advance * raster_scale;
            let glyph_x = pen_x + glyph.x_offset * raster_scale;
            pen_x += advance;
            let Some(path) = self.glyph_path(&face, span.face.id(), glyph.glyph_id) else {
                continue;
            };
            let transform = tiny_skia::Transform::from_row(
                glyph_scale,
                0.0,
                0.0,
                -glyph_scale,
                glyph_x,
                baseline - glyph.y_offset * raster_scale,
            );
            pixmap.fill_path(
                path.as_ref(),
                paint,
                tiny_skia::FillRule::Winding,
                transform,
                None,
            );
        }
    }

    fn glyph_path(
        &self,
        face: &ttf_parser::Face<'_>,
        face_id: FontId,
        glyph_id: u16,
    ) -> Option<Arc<tiny_skia::Path>> {
        let key = GlyphPathKey { face_id, glyph_id };
        if let Some(path) = self.glyph_paths.lock().get(&key).cloned() {
            self.stats.glyph_path_hits.fetch_add(1, Ordering::Relaxed);
            return Some(path);
        }

        let mut builder = GlyphPathBuilder::default();
        face.outline_glyph(ttf_parser::GlyphId(glyph_id), &mut builder)?;
        let path = Arc::new(builder.finish()?);
        self.glyph_paths.lock().put(key, path.clone());
        self.stats.glyph_path_misses.fetch_add(1, Ordering::Relaxed);
        Some(path)
    }
}

impl RasterCacheKey {
    fn new(request: &TextRasterRequest<'_>, raster_scale: f32) -> Self {
        Self {
            text_hash: hash_text(request.text),
            style_hash: hash_style(request.style),
            direction: request.direction,
            slot_height_q: quantize_f32(request.slot_height),
            padding_x_q: quantize_f32(request.padding_x),
            padding_y_q: quantize_f32(request.padding_y),
            slack_x_q: quantize_f32(request.slack_x),
            slack_y_q: quantize_f32(request.slack_y),
            baseline_mode: normalize_baseline_mode(request.baseline_mode),
            raster_scale_q: quantize_f32(raster_scale),
        }
    }

    fn raster_scale(self) -> f32 {
        self.raster_scale_q as f32 / 64.0
    }

    fn cache_id(self) -> u64 {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut state);
        state.finish()
    }
}

fn normalize_baseline_mode(mode: BaselineMode) -> BaselineModeKey {
    match mode {
        BaselineMode::AutoFontMetrics => BaselineModeKey::AutoFontMetrics,
        BaselineMode::FixedBaselinePx(value) => BaselineModeKey::FixedBaselinePx {
            baseline_px_q: quantize_f32(value),
        },
    }
}

fn shaped_text_baseline_metrics(
    spans: &[ShapedTextSpan],
    request: &TextRasterRequest<'_>,
    raster_scale: f32,
) -> BaselineMetrics {
    let (ascent, descent) = shaped_text_vertical_extents(spans, request, raster_scale);
    match request.baseline_mode {
        BaselineMode::AutoFontMetrics => {
            let content_height = (ascent + descent).max(1.0);
            let top_inset = ((request.slot_height * raster_scale - content_height).max(0.0)) * 0.5;
            BaselineMetrics {
                baseline_px: top_inset + ascent,
                ascent_px: ascent,
                descent_px: descent,
            }
        }
        BaselineMode::FixedBaselinePx(value) => BaselineMetrics {
            baseline_px: value * raster_scale,
            ascent_px: ascent,
            descent_px: descent,
        },
    }
}

fn shaped_text_vertical_extents(
    spans: &[ShapedTextSpan],
    request: &TextRasterRequest<'_>,
    raster_scale: f32,
) -> (f32, f32) {
    let mut ascent = request.style.size_px * raster_scale * 0.8;
    let mut descent = request.style.size_px * raster_scale * 0.2;

    for span in spans {
        let Ok(face) = ttf_parser::Face::parse(span.face.data(), span.face.face_index()) else {
            continue;
        };
        let units_per_em = span.face.units_per_em().max(1) as f32;
        let scale = request.style.size_px * raster_scale / units_per_em;
        ascent = ascent.max(face.ascender() as f32 * scale);
        descent = descent.max((-(face.descender() as f32) * scale).max(0.0));
    }

    (ascent.max(1.0), descent.max(0.0))
}

fn quantize_f32(value: f32) -> u32 {
    (value.max(0.0) * 64.0).round() as u32
}

fn hash_text(text: &str) -> u64 {
    let mut state = ahash::AHasher::default();
    text.hash(&mut state);
    state.finish()
}

fn hash_style(style: &TextStyleSpec) -> u64 {
    let mut state = ahash::AHasher::default();
    style.hash(&mut state);
    state.finish()
}

#[derive(Default)]
struct GlyphPathBuilder {
    inner: tiny_skia::PathBuilder,
}

impl GlyphPathBuilder {
    fn finish(self) -> Option<tiny_skia::Path> {
        self.inner.finish()
    }
}

impl ttf_parser::OutlineBuilder for GlyphPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.inner.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.inner.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.inner.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.inner.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.inner.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> PretextEngine {
        PretextEngine::builder()
            .with_font_data(vec![
                include_bytes!("../../../demos/app/assets/fonts/NotoSans-Regular.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansArabic-Regular.ttf")
                    .to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansCJK-Regular.ttc").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansMyanmar-Regular.ttf")
                    .to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/Noto-COLRv1.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansMono-Regular.ttf").to_vec(),
            ])
            .include_system_fonts(false)
            .build()
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
    fn rasterized_text_reuses_cached_entry() {
        let engine = engine();
        let cache = TextRenderCache::default();
        let request = TextRasterRequest {
            text: "English العربية",
            style: &default_style(),
            direction: BidiDirection::Ltr,
            slot_height: 22.0,
            padding_x: 2.0,
            padding_y: 2.0,
            slack_x: 2.0,
            slack_y: 2.0,
            baseline_mode: BaselineMode::AutoFontMetrics,
        };

        let first = cache
            .rasterized_text(&engine, request, 2.0)
            .expect("first rasterized text");
        let second = cache
            .rasterized_text(&engine, request, 2.0)
            .expect("cached rasterized text");

        assert!(Arc::ptr_eq(&first, &second));
        let stats = cache.stats_snapshot();
        assert_eq!(stats.raster_cache_hits, 1);
        assert_eq!(stats.rasterizations, 1);
    }

    #[test]
    fn rasterized_text_size_tracks_content_width_instead_of_external_slot_width() {
        let engine = engine();
        let cache = TextRenderCache::default();
        let request = TextRasterRequest {
            text: "Cache me",
            style: &default_style(),
            direction: BidiDirection::Ltr,
            slot_height: 22.0,
            padding_x: 2.0,
            padding_y: 2.0,
            slack_x: 2.0,
            slack_y: 2.0,
            baseline_mode: BaselineMode::AutoFontMetrics,
        };

        let raster = cache
            .rasterized_text(&engine, request, 1.0)
            .expect("rasterized text");

        assert!(raster.logical_size().width > 8.0);
        assert!(raster.logical_size().height >= 22.0);
        assert_eq!(
            raster.pixel_size()[0],
            raster.logical_size().width.ceil() as usize
        );
    }

    #[test]
    fn fixed_baseline_metrics_preserve_requested_baseline_offset() {
        let engine = engine();
        let request = TextRasterRequest {
            text: "emoji 🎉",
            style: &default_style(),
            direction: BidiDirection::Ltr,
            slot_height: 20.0,
            padding_x: 2.0,
            padding_y: 2.0,
            slack_x: 2.0,
            slack_y: 2.0,
            baseline_mode: BaselineMode::FixedBaselinePx(18.0),
        };

        let metrics = text_baseline_metrics(&engine, request);

        assert_eq!(metrics.baseline_px, 18.0);
        assert!(metrics.ascent_px > 0.0);
        assert!(metrics.descent_px >= 0.0);
    }

    #[test]
    fn baseline_metrics_square_top_centers_within_content_box() {
        let metrics = BaselineMetrics {
            baseline_px: 14.0,
            ascent_px: 10.0,
            descent_px: 4.0,
        };

        assert_eq!(metrics.content_top_px(), 4.0);
        assert_eq!(metrics.content_height_px(), 14.0);
        assert_eq!(metrics.square_top(10.0), 6.0);
    }
}

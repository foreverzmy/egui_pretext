use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use lru::LruCache;
use parking_lot::{Mutex, RwLock};

use crate::analysis::{GraphemeKind, WhiteSpaceMode};
use crate::bidi::{BidiDirection, BidiRun, ParagraphDirection};
use crate::font_catalog::{FontCatalog, FontId, LoadedFace};
use crate::layout::{self, ParagraphCache};
use crate::line_break::BreakOpportunity;
use crate::measure::{self, ShapeCache, ShapedGlyph};

const PREPARED_TEXT_CACHE_CAPACITY: usize = 256;
const ATOMIC_PLACEHOLDER_CACHE_CAPACITY: usize = 128;

pub struct PretextEngine {
    font_catalog: FontCatalog,
    shape_cache: Mutex<ShapeCache>,
    para_cache: Option<ParagraphCache>,
    prepare_cache: PrepareCache,
    locale: RwLock<Option<String>>,
    revision: u64,
    runtime_stats: RuntimeStats,
}

#[derive(Clone, Debug)]
pub struct TextStyleSpec {
    pub families: Vec<String>,
    pub size_px: f32,
    pub weight: u16,
    pub italic: bool,
}

impl Hash for TextStyleSpec {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.families.hash(state);
        ((self.size_px * 64.0).round() as u32).hash(state);
        self.weight.hash(state);
        self.italic.hash(state);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PrepareOptions {
    pub white_space: WhiteSpaceMode,
    pub paragraph_direction: ParagraphDirection,
}

impl Default for PrepareOptions {
    fn default() -> Self {
        Self {
            white_space: WhiteSpaceMode::Normal,
            paragraph_direction: ParagraphDirection::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayoutCursor {
    pub segment_index: usize,
    pub grapheme_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutLine {
    pub text: String,
    pub width: f32,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutLineVisualRun {
    pub text: String,
    pub width: f32,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
    pub level: u8,
    pub direction: BidiDirection,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutGlyph {
    pub face_id: FontId,
    pub glyph_id: u16,
    pub x: f32,
    pub advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutLineGlyphRun {
    pub width: f32,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
    pub level: u8,
    pub direction: BidiDirection,
    pub glyphs: Vec<LayoutGlyph>,
}

#[derive(Clone)]
pub struct ShapedTextSpan {
    pub text: String,
    pub byte_range: Range<usize>,
    pub width: f32,
    pub direction: BidiDirection,
    pub face: Arc<crate::font_catalog::LoadedFace>,
    pub glyphs: Arc<[ShapedGlyph]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutLineRange {
    pub width: f32,
    pub start: LayoutCursor,
    pub end: LayoutCursor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutResult {
    pub height: f32,
    pub line_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutWithLinesResult {
    pub height: f32,
    pub line_count: usize,
    pub lines: Vec<LayoutLine>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EngineRuntimeStats {
    pub prepare_calls: u64,
    pub prepare_with_segments_calls: u64,
    pub prepare_atomic_placeholder_calls: u64,
    pub prepare_cache_hits: u64,
    pub prepare_cache_misses: u64,
    pub atomic_placeholder_cache_hits: u64,
    pub atomic_placeholder_cache_misses: u64,
    pub layout_calls: u64,
    pub layout_with_lines_calls: u64,
    pub walk_line_ranges_calls: u64,
    pub layout_next_line_calls: u64,
    pub line_visual_runs_calls: u64,
    pub line_glyph_runs_calls: u64,
    pub glyph_advance_calls: u64,
    pub prefix_widths_calls: u64,
    pub shape_text_spans_calls: u64,
}

#[derive(Clone, Copy, Debug)]
pub enum SegmentKind {
    Text,
    AtomicPlaceholder { width: f32 },
}

#[derive(Clone)]
pub struct PreparedText {
    pub(crate) core: Arc<PreparedCore>,
}

#[derive(Clone)]
pub struct PreparedTextWithSegments {
    pub(crate) inner: PreparedText,
    #[allow(dead_code)]
    pub(crate) seg_meta: Arc<[SegmentMeta]>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct PreparedCore {
    pub text: Arc<str>,
    pub segments: Arc<[Segment]>,
    pub seg_meta: Arc<[SegmentMeta]>,
    pub bidi_runs: Arc<[BidiRun]>,
    pub hyphen_glyphs: Arc<[ShapedGlyph]>,
    pub hash: u64,
    pub style_hash: u64,
    pub locale_hash: u64,
    pub white_space: WhiteSpaceMode,
    pub paragraph_direction: ParagraphDirection,
}

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct Segment {
    pub kind: SegmentKind,
    pub byte_range: Range<usize>,
    pub glyphs: Arc<[ShapedGlyph]>,
}

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct SegmentMeta {
    pub byte_range: Range<usize>,
    pub graphemes: Arc<[GraphemeMeta]>,
    pub tab_stop_advance: f32,
    pub discretionary_hyphen_width: f32,
}

#[derive(Clone)]
pub(crate) struct GraphemeMeta {
    pub byte_range: Range<usize>,
    pub advance: f32,
    pub kind: GraphemeKind,
    pub break_after: BreakOpportunity,
}

#[derive(Default)]
struct RuntimeStats {
    prepare_calls: AtomicU64,
    prepare_with_segments_calls: AtomicU64,
    prepare_atomic_placeholder_calls: AtomicU64,
    prepare_cache_hits: AtomicU64,
    prepare_cache_misses: AtomicU64,
    atomic_placeholder_cache_hits: AtomicU64,
    atomic_placeholder_cache_misses: AtomicU64,
    layout_calls: AtomicU64,
    layout_with_lines_calls: AtomicU64,
    walk_line_ranges_calls: AtomicU64,
    layout_next_line_calls: AtomicU64,
    line_visual_runs_calls: AtomicU64,
    line_glyph_runs_calls: AtomicU64,
    glyph_advance_calls: AtomicU64,
    prefix_widths_calls: AtomicU64,
    shape_text_spans_calls: AtomicU64,
}

struct PrepareCache {
    text: Mutex<LruCache<PrepareCacheKey, PreparedTextWithSegments>>,
    atomic_placeholders: Mutex<LruCache<AtomicPlaceholderCacheKey, PreparedTextWithSegments>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct PrepareCacheKey {
    text: String,
    style: TextStyleCacheKey,
    locale: Option<String>,
    white_space: WhiteSpaceMode,
    paragraph_direction: ParagraphDirection,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct TextStyleCacheKey {
    families: Vec<String>,
    size_px_bits: u32,
    weight: u16,
    italic: bool,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct AtomicPlaceholderCacheKey {
    width_bits: u32,
    locale: Option<String>,
    white_space: WhiteSpaceMode,
    paragraph_direction: ParagraphDirection,
}

impl PreparedText {
    pub(crate) fn new(
        text: Arc<str>,
        segments: Arc<[Segment]>,
        seg_meta: Arc<[SegmentMeta]>,
        bidi_runs: Arc<[BidiRun]>,
        hyphen_glyphs: Arc<[ShapedGlyph]>,
        hash: u64,
        style_hash: u64,
        locale_hash: u64,
        white_space: WhiteSpaceMode,
        paragraph_direction: ParagraphDirection,
    ) -> Self {
        Self {
            core: Arc::new(PreparedCore {
                text,
                segments,
                seg_meta,
                bidi_runs,
                hyphen_glyphs,
                hash,
                style_hash,
                locale_hash,
                white_space,
                paragraph_direction,
            }),
        }
    }

    pub(crate) fn seg_meta(&self) -> &[SegmentMeta] {
        &self.core.seg_meta
    }

    pub(crate) fn text(&self) -> &str {
        &self.core.text
    }

    pub(crate) fn white_space(&self) -> WhiteSpaceMode {
        self.core.white_space
    }

    pub(crate) fn paragraph_direction(&self) -> ParagraphDirection {
        self.core.paragraph_direction
    }

    pub(crate) fn text_hash(&self) -> u64 {
        self.core.hash
    }

    #[allow(dead_code)]
    pub(crate) fn bidi_runs(&self) -> &[BidiRun] {
        &self.core.bidi_runs
    }

    pub(crate) fn style_hash(&self) -> u64 {
        self.core.style_hash
    }

    pub(crate) fn locale_hash(&self) -> u64 {
        self.core.locale_hash
    }

    pub(crate) fn hyphen_glyphs(&self) -> &[ShapedGlyph] {
        &self.core.hyphen_glyphs
    }
}

impl PreparedTextWithSegments {
    pub(crate) fn inner(&self) -> &PreparedText {
        &self.inner
    }

    pub fn as_prepared(&self) -> &PreparedText {
        &self.inner
    }
}

impl PrepareCache {
    fn new() -> Self {
        Self {
            text: Mutex::new(LruCache::new(
                NonZeroUsize::new(PREPARED_TEXT_CACHE_CAPACITY)
                    .expect("prepared text cache capacity"),
            )),
            atomic_placeholders: Mutex::new(LruCache::new(
                NonZeroUsize::new(ATOMIC_PLACEHOLDER_CACHE_CAPACITY)
                    .expect("atomic placeholder cache capacity"),
            )),
        }
    }

    fn clear(&self) {
        self.text.lock().clear();
        self.atomic_placeholders.lock().clear();
    }

    fn get_or_compute_text(
        &self,
        key: PrepareCacheKey,
        compute: impl FnOnce() -> PreparedTextWithSegments,
    ) -> (PreparedTextWithSegments, bool) {
        if let Some(cached) = self.text.lock().get(&key).cloned() {
            return (cached, true);
        }

        let value = compute();
        let mut cache = self.text.lock();
        if let Some(cached) = cache.get(&key).cloned() {
            return (cached, true);
        }
        cache.put(key, value.clone());
        (value, false)
    }

    fn get_or_compute_atomic_placeholder(
        &self,
        key: AtomicPlaceholderCacheKey,
        compute: impl FnOnce() -> PreparedTextWithSegments,
    ) -> (PreparedTextWithSegments, bool) {
        if let Some(cached) = self.atomic_placeholders.lock().get(&key).cloned() {
            return (cached, true);
        }

        let value = compute();
        let mut cache = self.atomic_placeholders.lock();
        if let Some(cached) = cache.get(&key).cloned() {
            return (cached, true);
        }
        cache.put(key, value.clone());
        (value, false)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.text.lock().len() + self.atomic_placeholders.lock().len()
    }
}

impl PrepareCacheKey {
    fn new(
        text: &str,
        style: &TextStyleSpec,
        opts: &PrepareOptions,
        locale: Option<String>,
    ) -> Self {
        Self {
            text: text.to_owned(),
            style: TextStyleCacheKey::new(style),
            locale,
            white_space: opts.white_space,
            paragraph_direction: opts.paragraph_direction,
        }
    }
}

impl TextStyleCacheKey {
    fn new(style: &TextStyleSpec) -> Self {
        Self {
            families: style.families.clone(),
            size_px_bits: style.size_px.to_bits(),
            weight: style.weight,
            italic: style.italic,
        }
    }
}

impl AtomicPlaceholderCacheKey {
    fn new(width: f32, opts: &PrepareOptions, locale: Option<String>) -> Self {
        Self {
            width_bits: width.max(0.0).to_bits(),
            locale,
            white_space: opts.white_space,
            paragraph_direction: opts.paragraph_direction,
        }
    }
}

impl PretextEngine {
    pub fn new() -> Self {
        Self {
            font_catalog: FontCatalog::new(),
            shape_cache: Mutex::new(ShapeCache::new()),
            para_cache: Some(ParagraphCache::new()),
            prepare_cache: PrepareCache::new(),
            locale: RwLock::new(None),
            revision: next_engine_revision(),
            runtime_stats: RuntimeStats::default(),
        }
    }

    pub fn with_font_data<I>(font_data: I) -> Self
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        Self::with_font_data_and_system_fonts(font_data, true)
    }

    pub fn with_font_data_and_system_fonts<I>(font_data: I, include_system_fonts: bool) -> Self
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        Self {
            font_catalog: FontCatalog::with_font_data_and_system_fonts(
                font_data,
                include_system_fonts,
            ),
            shape_cache: Mutex::new(ShapeCache::new()),
            para_cache: Some(ParagraphCache::new()),
            prepare_cache: PrepareCache::new(),
            locale: RwLock::new(None),
            revision: next_engine_revision(),
            runtime_stats: RuntimeStats::default(),
        }
    }

    pub fn prepare(
        &self,
        text: &str,
        style: &TextStyleSpec,
        opts: &PrepareOptions,
    ) -> PreparedText {
        self.runtime_stats
            .prepare_calls
            .fetch_add(1, Ordering::Relaxed);
        self.prepare_cached(text, style, opts).inner
    }

    pub fn prepare_with_segments(
        &self,
        text: &str,
        style: &TextStyleSpec,
        opts: &PrepareOptions,
    ) -> PreparedTextWithSegments {
        self.runtime_stats
            .prepare_with_segments_calls
            .fetch_add(1, Ordering::Relaxed);
        self.prepare_cached(text, style, opts)
    }

    pub fn prepare_atomic_placeholder(
        &self,
        width: f32,
        opts: &PrepareOptions,
    ) -> PreparedTextWithSegments {
        self.runtime_stats
            .prepare_atomic_placeholder_calls
            .fetch_add(1, Ordering::Relaxed);
        self.prepare_atomic_placeholder_cached(width, opts)
    }

    pub fn layout(
        &self,
        prepared: &PreparedText,
        max_width: f32,
        line_height: f32,
    ) -> LayoutResult {
        self.runtime_stats
            .layout_calls
            .fetch_add(1, Ordering::Relaxed);
        layout::layout(prepared, max_width, line_height, self.para_cache.as_ref())
    }

    pub fn layout_with_lines(
        &self,
        prepared: &PreparedTextWithSegments,
        max_width: f32,
        line_height: f32,
    ) -> LayoutWithLinesResult {
        self.runtime_stats
            .layout_with_lines_calls
            .fetch_add(1, Ordering::Relaxed);
        layout::layout_with_lines(prepared, max_width, line_height, self.para_cache.as_ref())
    }

    pub fn walk_line_ranges(
        &self,
        prepared: &PreparedTextWithSegments,
        max_width: f32,
        on_line: impl FnMut(&LayoutLineRange),
    ) -> f32 {
        self.runtime_stats
            .walk_line_ranges_calls
            .fetch_add(1, Ordering::Relaxed);
        layout::walk_line_ranges(prepared, max_width, on_line, self.para_cache.as_ref())
    }

    pub fn layout_next_line(
        &self,
        prepared: &PreparedTextWithSegments,
        start: &mut LayoutCursor,
        max_width: f32,
    ) -> Option<LayoutLine> {
        self.runtime_stats
            .layout_next_line_calls
            .fetch_add(1, Ordering::Relaxed);
        layout::layout_next_line(prepared, start, max_width, self.para_cache.as_ref())
    }

    pub fn line_visual_runs(
        &self,
        prepared: &PreparedTextWithSegments,
        line: &LayoutLine,
    ) -> Vec<LayoutLineVisualRun> {
        self.runtime_stats
            .line_visual_runs_calls
            .fetch_add(1, Ordering::Relaxed);
        layout::line_visual_runs(prepared.inner(), line)
    }

    pub fn line_glyph_runs(
        &self,
        prepared: &PreparedTextWithSegments,
        line: &LayoutLine,
    ) -> Vec<LayoutLineGlyphRun> {
        self.runtime_stats
            .line_glyph_runs_calls
            .fetch_add(1, Ordering::Relaxed);
        layout::line_glyph_runs(prepared.inner(), line)
    }

    pub fn clear_cache(&mut self) {
        self.shape_cache.get_mut().clear();
        crate::analysis::clear_runtime_caches();
        self.font_catalog.clear_runtime_caches();
        self.prepare_cache.clear();
        if let Some(cache) = &self.para_cache {
            cache.clear();
        }
    }

    pub fn set_locale(&mut self, locale: Option<&str>) {
        let next = locale.and_then(|value| (!value.is_empty()).then(|| value.to_owned()));
        {
            let current = self.locale.get_mut();
            if *current == next {
                return;
            }
            *current = next;
        }
        self.clear_cache();
        self.revision = next_engine_revision();
    }

    pub fn glyph_advance(&self, ch: char, style: &TextStyleSpec) -> f32 {
        self.runtime_stats
            .glyph_advance_calls
            .fetch_add(1, Ordering::Relaxed);
        measure::measure_cluster_width(
            &ch.to_string(),
            style,
            &self.font_catalog,
            &self.shape_cache,
        )
    }

    pub fn prefix_widths(&self, text: &str, style: &TextStyleSpec) -> Arc<[f32]> {
        self.runtime_stats
            .prefix_widths_calls
            .fetch_add(1, Ordering::Relaxed);
        measure::prefix_widths(text, style, &self.font_catalog, &self.shape_cache)
    }

    pub fn shape_text_spans_shared(
        &self,
        text: &str,
        style: &TextStyleSpec,
        direction: BidiDirection,
    ) -> Arc<[ShapedTextSpan]> {
        self.runtime_stats
            .shape_text_spans_calls
            .fetch_add(1, Ordering::Relaxed);
        measure::shape_text_spans_shared(
            text,
            direction,
            style,
            &self.font_catalog,
            &self.shape_cache,
        )
    }

    pub fn shape_text_spans(
        &self,
        text: &str,
        style: &TextStyleSpec,
        direction: BidiDirection,
    ) -> Vec<ShapedTextSpan> {
        self.shape_text_spans_shared(text, style, direction)
            .iter()
            .cloned()
            .collect()
    }

    pub fn locale(&self) -> Option<String> {
        self.locale.read().clone()
    }

    pub fn load_face(&self, id: FontId) -> Option<Arc<LoadedFace>> {
        self.font_catalog.load_face(id)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn has_paragraph_cache(&self) -> bool {
        self.para_cache.is_some()
    }

    pub fn runtime_stats(&self) -> EngineRuntimeStats {
        self.runtime_stats.snapshot()
    }

    fn prepare_cached(
        &self,
        text: &str,
        style: &TextStyleSpec,
        opts: &PrepareOptions,
    ) -> PreparedTextWithSegments {
        let locale = self.locale();
        let key = PrepareCacheKey::new(text, style, opts, locale.clone());
        let (prepared, hit) = self.prepare_cache.get_or_compute_text(key, || {
            layout::prepare_text(
                text,
                style,
                opts,
                &self.font_catalog,
                &self.shape_cache,
                hash_locale(locale.clone()),
                locale.as_deref(),
            )
        });
        let counter = if hit {
            &self.runtime_stats.prepare_cache_hits
        } else {
            &self.runtime_stats.prepare_cache_misses
        };
        counter.fetch_add(1, Ordering::Relaxed);
        prepared
    }

    fn prepare_atomic_placeholder_cached(
        &self,
        width: f32,
        opts: &PrepareOptions,
    ) -> PreparedTextWithSegments {
        let locale = self.locale();
        let key = AtomicPlaceholderCacheKey::new(width, opts, locale.clone());
        let (prepared, hit) = self
            .prepare_cache
            .get_or_compute_atomic_placeholder(key, || {
                layout::prepare_atomic_placeholder(width, opts, hash_locale(locale.clone()))
            });
        let counter = if hit {
            &self.runtime_stats.atomic_placeholder_cache_hits
        } else {
            &self.runtime_stats.atomic_placeholder_cache_misses
        };
        counter.fetch_add(1, Ordering::Relaxed);
        prepared
    }
}

impl Default for PretextEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeStats {
    fn snapshot(&self) -> EngineRuntimeStats {
        EngineRuntimeStats {
            prepare_calls: self.prepare_calls.load(Ordering::Relaxed),
            prepare_with_segments_calls: self.prepare_with_segments_calls.load(Ordering::Relaxed),
            prepare_atomic_placeholder_calls: self
                .prepare_atomic_placeholder_calls
                .load(Ordering::Relaxed),
            prepare_cache_hits: self.prepare_cache_hits.load(Ordering::Relaxed),
            prepare_cache_misses: self.prepare_cache_misses.load(Ordering::Relaxed),
            atomic_placeholder_cache_hits: self
                .atomic_placeholder_cache_hits
                .load(Ordering::Relaxed),
            atomic_placeholder_cache_misses: self
                .atomic_placeholder_cache_misses
                .load(Ordering::Relaxed),
            layout_calls: self.layout_calls.load(Ordering::Relaxed),
            layout_with_lines_calls: self.layout_with_lines_calls.load(Ordering::Relaxed),
            walk_line_ranges_calls: self.walk_line_ranges_calls.load(Ordering::Relaxed),
            layout_next_line_calls: self.layout_next_line_calls.load(Ordering::Relaxed),
            line_visual_runs_calls: self.line_visual_runs_calls.load(Ordering::Relaxed),
            line_glyph_runs_calls: self.line_glyph_runs_calls.load(Ordering::Relaxed),
            glyph_advance_calls: self.glyph_advance_calls.load(Ordering::Relaxed),
            prefix_widths_calls: self.prefix_widths_calls.load(Ordering::Relaxed),
            shape_text_spans_calls: self.shape_text_spans_calls.load(Ordering::Relaxed),
        }
    }
}

fn hash_locale(locale: Option<String>) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut state = ahash::AHasher::default();
    match locale {
        Some(locale) => locale.hash(&mut state),
        None => 0u8.hash(&mut state),
    }
    state.finish()
}

fn next_engine_revision() -> u64 {
    static NEXT_ENGINE_REVISION: AtomicU64 = AtomicU64::new(1);
    NEXT_ENGINE_REVISION.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_engine() -> PretextEngine {
        PretextEngine::with_font_data_and_system_fonts(
            vec![
                include_bytes!("../../../demos/app/assets/fonts/NotoSans-Regular.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansArabic-Regular.ttf")
                    .to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansCJK-Regular.ttc").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansMyanmar-Regular.ttf")
                    .to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/Noto-COLRv1.ttf").to_vec(),
                include_bytes!("../../../demos/app/assets/fonts/NotoSansMono-Regular.ttf").to_vec(),
            ],
            false,
        )
    }

    fn bundled_style() -> TextStyleSpec {
        TextStyleSpec {
            families: vec![
                "Noto Sans".to_owned(),
                "Noto Sans Arabic".to_owned(),
                "Arial".to_owned(),
                "Helvetica".to_owned(),
            ],
            size_px: 16.0,
            weight: 400,
            italic: false,
        }
    }

    #[test]
    fn set_locale_clears_layout_and_shape_caches() {
        let mut engine = bundled_engine();
        engine.set_locale(Some("th"));

        let prepared = engine.prepare_with_segments(
            "ภาษาไทย hello world",
            &bundled_style(),
            &PrepareOptions::default(),
        );
        let layout = engine.layout_with_lines(&prepared, 120.0, 24.0);
        assert!(layout.line_count > 0);
        assert!(engine.shape_cache.lock().len() > 0);
        assert_eq!(engine.prepare_cache.len(), 1);
        assert_eq!(
            engine
                .para_cache
                .as_ref()
                .expect("paragraph cache should exist")
                .len(),
            1
        );

        engine.set_locale(Some("en"));

        assert_eq!(engine.shape_cache.lock().len(), 0);
        assert_eq!(engine.prepare_cache.len(), 0);
        assert_eq!(
            engine
                .para_cache
                .as_ref()
                .expect("paragraph cache should exist")
                .len(),
            0
        );
    }

    #[test]
    fn prepared_state_keeps_its_original_locale_identity() {
        let mut engine = bundled_engine();
        let style = bundled_style();
        engine.set_locale(Some("th"));

        let thai_prepared =
            engine.prepare_with_segments("ภาษาไทย hello world", &style, &PrepareOptions::default());
        let thai_layout = engine.layout_with_lines(&thai_prepared, 120.0, 24.0);

        engine.set_locale(None);

        let reset_layout = engine.layout_with_lines(&thai_prepared, 120.0, 24.0);
        let latin_prepared =
            engine.prepare_with_segments("hello world", &style, &PrepareOptions::default());

        assert_eq!(
            thai_prepared.inner().locale_hash(),
            hash_locale(Some("th".to_owned()))
        );
        assert_eq!(reset_layout, thai_layout);
        assert_eq!(latin_prepared.inner().locale_hash(), hash_locale(None));
        assert_ne!(
            thai_prepared.inner().locale_hash(),
            latin_prepared.inner().locale_hash()
        );
    }

    #[test]
    fn locale_changes_analysis_boundaries_for_finnish_loanwords() {
        let mut engine = bundled_engine();
        let style = bundled_style();
        let invariant = engine.prepare_with_segments("EU:ssä", &style, &PrepareOptions::default());

        engine.set_locale(Some("fi"));
        let finnish = engine.prepare_with_segments("EU:ssä", &style, &PrepareOptions::default());

        assert_eq!(invariant.seg_meta.len(), 2);
        assert_eq!(finnish.seg_meta.len(), 1);
    }

    #[test]
    fn atomic_placeholder_stays_single_line_across_all_layout_interfaces() {
        let engine = bundled_engine();
        let prepared = engine.prepare_atomic_placeholder(72.0, &PrepareOptions::default());
        let plain = prepared.inner().clone();

        let layout = engine.layout(&plain, 24.0, 20.0);
        let with_lines = engine.layout_with_lines(&prepared, 24.0, 20.0);

        let mut walked = 0usize;
        let mut walked_width = 0.0f32;
        engine.walk_line_ranges(&prepared, 24.0, |line| {
            walked += 1;
            walked_width = line.width;
        });

        let mut cursor = LayoutCursor::default();
        let streamed = engine
            .layout_next_line(&prepared, &mut cursor, 24.0)
            .expect("placeholder should yield one line");

        assert_eq!(layout.line_count, 1);
        assert_eq!(with_lines.line_count, 1);
        assert_eq!(walked, 1);
        assert_eq!(with_lines.lines[0].text, "");
        assert_eq!(with_lines.lines[0].width, 72.0);
        assert_eq!(walked_width, 72.0);
        assert_eq!(streamed.text, "");
        assert_eq!(streamed.width, 72.0);
        assert_eq!(
            cursor,
            LayoutCursor {
                segment_index: 1,
                grapheme_index: 0,
            }
        );
    }

    #[test]
    fn prepared_segments_keep_local_glyph_subsets() {
        let engine = bundled_engine();
        let prepared = engine.prepare_with_segments(
            "alpha beta",
            &bundled_style(),
            &PrepareOptions::default(),
        );
        let core = &prepared.inner().core;
        let segment_slices = core
            .segments
            .iter()
            .map(|segment| &core.text[segment.byte_range.clone()])
            .collect::<Vec<_>>();

        assert_eq!(segment_slices, vec!["alpha", " ", "beta"]);
        assert!(!core.segments[0].glyphs.is_empty());
        assert!(!core.segments[2].glyphs.is_empty());

        for (segment, slice) in core.segments.iter().zip(segment_slices.iter()) {
            assert!(
                segment
                    .glyphs
                    .iter()
                    .all(|glyph| glyph.cluster_byte < slice.len()),
                "segment {slice:?} carried glyphs outside its local byte range: {:?}",
                segment
                    .glyphs
                    .iter()
                    .map(|glyph| glyph.cluster_byte)
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn prefix_widths_reuses_cached_arc() {
        let engine = bundled_engine();
        let style = bundled_style();

        let first = engine.prefix_widths("cache me", &style);
        let after_first = engine.shape_cache.lock().len();
        let second = engine.prefix_widths("cache me", &style);
        let after_second = engine.shape_cache.lock().len();

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn prepare_with_segments_reuses_cached_prepared_core() {
        let engine = bundled_engine();
        let style = bundled_style();
        let opts = PrepareOptions::default();

        let first = engine.prepare_with_segments("cache me", &style, &opts);
        let second = engine.prepare_with_segments("cache me", &style, &opts);

        assert!(Arc::ptr_eq(&first.inner.core, &second.inner.core));
        assert!(Arc::ptr_eq(&first.seg_meta, &second.seg_meta));
        assert_eq!(engine.prepare_cache.len(), 1);

        let stats = engine.runtime_stats();
        assert_eq!(stats.prepare_cache_hits, 1);
        assert_eq!(stats.prepare_cache_misses, 1);
    }

    #[test]
    fn prepare_and_prepare_with_segments_share_cached_prepared_core() {
        let engine = bundled_engine();
        let style = bundled_style();
        let opts = PrepareOptions::default();

        let first = engine.prepare("shared cache", &style, &opts);
        let second = engine.prepare_with_segments("shared cache", &style, &opts);

        assert!(Arc::ptr_eq(&first.core, &second.inner.core));
        assert_eq!(engine.prepare_cache.len(), 1);

        let stats = engine.runtime_stats();
        assert_eq!(stats.prepare_cache_hits, 1);
        assert_eq!(stats.prepare_cache_misses, 1);
    }

    #[test]
    fn atomic_placeholder_reuses_cached_prepared_core() {
        let engine = bundled_engine();
        let opts = PrepareOptions::default();

        let first = engine.prepare_atomic_placeholder(72.0, &opts);
        let second = engine.prepare_atomic_placeholder(72.0, &opts);

        assert!(Arc::ptr_eq(&first.inner.core, &second.inner.core));
        assert!(Arc::ptr_eq(&first.seg_meta, &second.seg_meta));
        assert_eq!(engine.prepare_cache.len(), 1);

        let stats = engine.runtime_stats();
        assert_eq!(stats.atomic_placeholder_cache_hits, 1);
        assert_eq!(stats.atomic_placeholder_cache_misses, 1);
    }

    #[test]
    fn engine_revision_tracks_semantic_engine_changes() {
        let first = bundled_engine();
        let second = bundled_engine();
        assert_ne!(first.revision(), second.revision());

        let mut engine = bundled_engine();
        let initial = engine.revision();
        engine.clear_cache();
        assert_eq!(engine.revision(), initial);

        engine.set_locale(Some("th"));
        let thai = engine.revision();
        assert_ne!(thai, initial);

        engine.set_locale(Some("th"));
        assert_eq!(engine.revision(), thai);

        engine.set_locale(None);
        assert_ne!(engine.revision(), thai);
    }

    #[test]
    fn shape_text_spans_shared_reuses_cached_arc() {
        let engine = bundled_engine();
        let style = bundled_style();

        let first = engine.shape_text_spans_shared("بدأت الرحلة", &style, BidiDirection::Rtl);
        let after_first = engine.shape_cache.lock().len();
        let second = engine.shape_text_spans_shared("بدأت الرحلة", &style, BidiDirection::Rtl);
        let after_second = engine.shape_cache.lock().len();

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn shape_text_spans_matches_shared_contents() {
        let engine = bundled_engine();
        let style = bundled_style();
        let shared =
            engine.shape_text_spans_shared("English العربية mixed", &style, BidiDirection::Ltr);
        let owned = engine.shape_text_spans("English العربية mixed", &style, BidiDirection::Ltr);

        assert_eq!(shared.len(), owned.len());
        for (shared_span, owned_span) in shared.iter().zip(owned.iter()) {
            assert_eq!(shared_span.text, owned_span.text);
            assert_eq!(shared_span.byte_range, owned_span.byte_range);
            assert_eq!(shared_span.width, owned_span.width);
            assert_eq!(shared_span.direction, owned_span.direction);
            assert_eq!(shared_span.face.id(), owned_span.face.id());
            assert_eq!(shared_span.glyphs.len(), owned_span.glyphs.len());
        }
    }

    #[test]
    fn prepared_text_keeps_bidi_runs_for_future_visual_ordering() {
        let engine = bundled_engine();
        let prepared = engine.prepare_with_segments(
            "English العربية mixed",
            &bundled_style(),
            &PrepareOptions::default(),
        );

        assert!(prepared.inner().bidi_runs().len() >= 2);
        assert!(prepared
            .inner()
            .bidi_runs()
            .iter()
            .any(|run| matches!(run.direction, crate::bidi::BidiDirection::Rtl)));
    }

    #[test]
    fn shape_text_spans_emit_rtl_segments_in_visual_order() {
        let engine = bundled_engine();
        let spans = engine.shape_text_spans(
            "بدأت الرحلة",
            &bundled_style(),
            crate::bidi::BidiDirection::Rtl,
        );

        let texts = spans
            .iter()
            .map(|span| span.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, vec!["الرحلة", " ", "بدأت"]);
        assert!(
            spans[0].glyphs.first().unwrap().cluster_byte
                > spans[0].glyphs.last().unwrap().cluster_byte
        );
    }
}

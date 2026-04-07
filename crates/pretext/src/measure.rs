use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::Arc;

use ahash::AHashMap;
use lru::LruCache;
use parking_lot::Mutex;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

use crate::analysis::{AnalyzedGrapheme, GraphemeKind, TextAnalysis};
use crate::bidi::{BidiDirection, BidiRun};
use crate::engine::{ShapedTextSpan, TextStyleSpec};
use crate::font_catalog::{FontCatalog, FontId, LoadedFace};

const SHAPE_CACHE_CAPACITY: usize = 2048;
const PREFIX_WIDTHS_CACHE_CAPACITY: usize = 512;
const SHAPED_SPANS_CACHE_CAPACITY: usize = 1024;

#[derive(Clone, Copy, Debug)]
pub struct ShapedGlyph {
    pub face_id: FontId,
    pub glyph_id: u16,
    pub cluster_byte: usize,
    pub advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ShapeCacheKey {
    text_hash: u64,
    font_id: FontId,
    size_px_q: u32,
    weight: u16,
    italic: bool,
    direction: BidiDirection,
    script: u32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct PrefixWidthsCacheKey {
    text_hash: u64,
    style_hash: u64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct ShapedSpansCacheKey {
    text_hash: u64,
    style_hash: u64,
    direction: BidiDirection,
}

pub struct ShapeCache {
    inner: LruCache<ShapeCacheKey, Arc<[ShapedGlyph]>>,
    prefix_widths: LruCache<PrefixWidthsCacheKey, Arc<[f32]>>,
    shaped_spans: LruCache<ShapedSpansCacheKey, Arc<[ShapedTextSpan]>>,
}

impl ShapeCache {
    pub fn new() -> Self {
        Self {
            inner: LruCache::new(NonZeroUsize::new(SHAPE_CACHE_CAPACITY).unwrap()),
            prefix_widths: LruCache::new(NonZeroUsize::new(PREFIX_WIDTHS_CACHE_CAPACITY).unwrap()),
            shaped_spans: LruCache::new(NonZeroUsize::new(SHAPED_SPANS_CACHE_CAPACITY).unwrap()),
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.prefix_widths.clear();
        self.shaped_spans.clear();
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.inner.len() + self.prefix_widths.len() + self.shaped_spans.len()
    }
}

pub(crate) struct MeasurementResult {
    pub glyphs: Arc<[ShapedGlyph]>,
    pub grapheme_advances: Vec<f32>,
    pub space_advance: f32,
    pub hyphen_advance: f32,
}

pub(crate) fn measure_text(
    analysis: &TextAnalysis,
    style: &TextStyleSpec,
    bidi_runs: &[BidiRun],
    catalog: &FontCatalog,
    cache: &Mutex<ShapeCache>,
) -> MeasurementResult {
    let preferred = catalog.resolve_style_chain(style);
    let mut grapheme_advances = vec![0.0f32; analysis.graphemes.len()];
    let mut glyphs = Vec::new();
    let mut starts = AHashMap::new();
    for (index, grapheme) in analysis.graphemes.iter().enumerate() {
        starts.insert(grapheme.byte_range.start, index);
    }

    for run in bidi_runs {
        let Some(run_glyphs) = measure_bidi_run(
            &analysis.normalized,
            &analysis.graphemes,
            run,
            style,
            catalog,
            &preferred,
            cache,
            &starts,
            &mut grapheme_advances,
        ) else {
            continue;
        };
        glyphs.extend_from_slice(&run_glyphs);
    }

    let space_advance = measure_cluster_width(" ", style, catalog, cache);
    let hyphen_advance = measure_cluster_width("-", style, catalog, cache);

    MeasurementResult {
        glyphs: Arc::from(glyphs),
        grapheme_advances,
        space_advance,
        hyphen_advance,
    }
}

pub(crate) fn shape_text_spans_shared(
    text: &str,
    direction: BidiDirection,
    style: &TextStyleSpec,
    catalog: &FontCatalog,
    cache: &Mutex<ShapeCache>,
) -> Arc<[ShapedTextSpan]> {
    if text.is_empty() {
        return Arc::from(Vec::<ShapedTextSpan>::new());
    }

    let key = ShapedSpansCacheKey {
        text_hash: hash_text(text),
        style_hash: hash_style(style),
        direction,
    };
    if let Some(cached) = cache.lock().shaped_spans.get(&key).cloned() {
        return cached;
    }

    let graphemes: Vec<AnalyzedGrapheme> = UnicodeSegmentation::grapheme_indices(text, true)
        .map(|(start, grapheme)| AnalyzedGrapheme {
            byte_range: start..start + grapheme.len(),
            kind: match grapheme {
                "\n" => GraphemeKind::Newline,
                "\t" => GraphemeKind::Tab,
                "\u{00AD}" => GraphemeKind::SoftHyphen,
                "\u{200B}" => GraphemeKind::ZeroWidthBreak,
                "\u{2060}" => GraphemeKind::WordJoiner,
                " " => GraphemeKind::Space,
                _ => GraphemeKind::Text,
            },
        })
        .collect();
    let preferred = catalog.resolve_style_chain(style);
    let mut font_runs = split_into_font_runs(text, &graphemes, 0..text.len(), catalog, &preferred);
    if direction == BidiDirection::Rtl {
        font_runs.reverse();
    }

    let mut output = Vec::with_capacity(font_runs.len());
    for font_run in font_runs {
        let segment_text = &text[font_run.byte_range.clone()];
        if segment_text.is_empty() {
            continue;
        }

        let glyphs = {
            let mut guard = cache.lock();
            shape_run(
                segment_text,
                &font_run.face,
                font_run.script,
                direction,
                style,
                &mut guard,
            )
        };
        let width = glyphs.iter().map(|glyph| glyph.advance).sum();

        output.push(ShapedTextSpan {
            text: segment_text.to_owned(),
            byte_range: font_run.byte_range,
            width,
            direction,
            face: font_run.face,
            glyphs,
        });
    }

    let output: Arc<[ShapedTextSpan]> = Arc::from(output);
    cache.lock().shaped_spans.put(key, output.clone());
    output
}

pub(crate) fn prefix_widths(
    text: &str,
    style: &TextStyleSpec,
    catalog: &FontCatalog,
    cache: &Mutex<ShapeCache>,
) -> Arc<[f32]> {
    let key = PrefixWidthsCacheKey {
        text_hash: hash_text(text),
        style_hash: hash_style(style),
    };
    if let Some(cached) = cache.lock().prefix_widths.get(&key).cloned() {
        return cached;
    }

    let analysis = TextAnalysis {
        normalized: text.to_owned(),
        graphemes: unicode_segmentation::UnicodeSegmentation::grapheme_indices(text, true)
            .map(|(start, grapheme)| AnalyzedGrapheme {
                byte_range: start..start + grapheme.len(),
                kind: match grapheme {
                    "\n" => GraphemeKind::Newline,
                    "\t" => GraphemeKind::Tab,
                    "\u{00AD}" => GraphemeKind::SoftHyphen,
                    "\u{200B}" => GraphemeKind::ZeroWidthBreak,
                    "\u{2060}" => GraphemeKind::WordJoiner,
                    " " => GraphemeKind::Space,
                    _ => GraphemeKind::Text,
                },
            })
            .collect(),
        segments: Vec::new(),
        urls: Vec::new(),
        white_space: crate::analysis::WhiteSpaceMode::PreWrap,
        word_break: crate::analysis::WordBreakMode::Normal,
    };
    let bidi_runs =
        crate::bidi::paragraph_to_bidi_runs(text, crate::bidi::ParagraphDirection::Auto);
    let measurement = measure_text(&analysis, style, &bidi_runs, catalog, cache);

    let mut acc = 0.0f64;
    let mut widths = Vec::with_capacity(measurement.grapheme_advances.len() + 1);
    widths.push(0.0);
    for advance in measurement.grapheme_advances {
        acc += advance as f64;
        widths.push(acc as f32);
    }
    let widths: Arc<[f32]> = Arc::from(widths);
    cache.lock().prefix_widths.put(key, widths.clone());
    widths
}

pub(crate) fn measure_cluster_width(
    cluster: &str,
    style: &TextStyleSpec,
    catalog: &FontCatalog,
    cache: &Mutex<ShapeCache>,
) -> f32 {
    let preferred = catalog.resolve_style_chain(style);
    let Some(face) = catalog.face_for_cluster(cluster, &preferred) else {
        return style.size_px.max(1.0) * 0.5;
    };

    let script = detect_script(cluster);
    let mut guard = cache.lock();
    shape_run(
        cluster,
        &face,
        script,
        BidiDirection::Ltr,
        style,
        &mut guard,
    )
    .iter()
    .map(|glyph| glyph.advance)
    .sum()
}

pub fn shape_run(
    text: &str,
    face: &LoadedFace,
    script: Script,
    direction: BidiDirection,
    style: &TextStyleSpec,
    cache: &mut ShapeCache,
) -> Arc<[ShapedGlyph]> {
    let key = ShapeCacheKey {
        text_hash: hash_text(text),
        font_id: face.id(),
        size_px_q: (style.size_px * 64.0).round() as u32,
        weight: style.weight,
        italic: style.italic,
        direction,
        script: script as u32,
    };

    if let Some(cached) = cache.inner.get(&key) {
        return cached.clone();
    }

    let Some(rb_face) = rustybuzz::Face::from_slice(face.data(), face.face_index()) else {
        return Arc::from(Vec::<ShapedGlyph>::new());
    };

    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    buffer.set_direction(match direction {
        BidiDirection::Ltr => rustybuzz::Direction::LeftToRight,
        BidiDirection::Rtl => rustybuzz::Direction::RightToLeft,
    });
    buffer.set_script(map_script(script));

    let glyph_buffer = rustybuzz::shape(&rb_face, &[], buffer);
    let units_per_em = face.units_per_em().max(1) as f32;
    let scale = style.size_px / units_per_em;
    let glyphs: Vec<ShapedGlyph> = glyph_buffer
        .glyph_infos()
        .iter()
        .zip(glyph_buffer.glyph_positions())
        .map(|(info, pos)| ShapedGlyph {
            face_id: face.id(),
            glyph_id: info.glyph_id as u16,
            cluster_byte: info.cluster as usize,
            advance: pos.x_advance as f32 * scale,
            x_offset: pos.x_offset as f32 * scale,
            y_offset: pos.y_offset as f32 * scale,
        })
        .collect();
    let glyphs: Arc<[ShapedGlyph]> = Arc::from(glyphs);
    cache.inner.put(key, glyphs.clone());
    glyphs
}

fn measure_bidi_run(
    text: &str,
    graphemes: &[AnalyzedGrapheme],
    bidi_run: &BidiRun,
    style: &TextStyleSpec,
    catalog: &FontCatalog,
    preferred: &[FontId],
    cache: &Mutex<ShapeCache>,
    starts: &AHashMap<usize, usize>,
    grapheme_advances: &mut [f32],
) -> Option<Vec<ShapedGlyph>> {
    if bidi_run.byte_range.start >= bidi_run.byte_range.end {
        return None;
    }

    let runs = split_into_font_runs(
        text,
        graphemes,
        bidi_run.byte_range.clone(),
        catalog,
        preferred,
    );
    let mut output = Vec::new();

    for font_run in runs {
        let segment_text = &text[font_run.byte_range.clone()];
        if segment_text.is_empty() {
            continue;
        }

        let shaped = {
            let mut guard = cache.lock();
            shape_run(
                segment_text,
                &font_run.face,
                font_run.script,
                bidi_run.direction,
                style,
                &mut guard,
            )
        };
        accumulate_cluster_advances(
            &shaped,
            graphemes,
            font_run.byte_range.clone(),
            starts,
            grapheme_advances,
        );
        output.extend(offset_cluster_bytes(&shaped, font_run.byte_range.start));
    }

    Some(output)
}

struct FontRun {
    byte_range: Range<usize>,
    face: Arc<LoadedFace>,
    script: Script,
}

struct ScriptRun {
    byte_range: Range<usize>,
    script: Script,
}

fn split_into_font_runs(
    text: &str,
    graphemes: &[AnalyzedGrapheme],
    byte_range: Range<usize>,
    catalog: &FontCatalog,
    preferred: &[FontId],
) -> Vec<FontRun> {
    let mut output = Vec::new();
    for script_run in split_by_script(text, graphemes, byte_range) {
        if is_complex_script(script_run.script) {
            output.extend(split_complex_script_run(
                text,
                graphemes,
                &script_run,
                catalog,
                preferred,
            ));
        } else {
            output.extend(split_by_coverage(
                text,
                graphemes,
                &script_run,
                catalog,
                preferred,
            ));
        }
    }
    output
}

fn split_by_script(
    text: &str,
    graphemes: &[AnalyzedGrapheme],
    byte_range: Range<usize>,
) -> Vec<ScriptRun> {
    let indices: Vec<usize> = graphemes
        .iter()
        .enumerate()
        .filter_map(|(idx, grapheme)| {
            (grapheme.byte_range.start >= byte_range.start
                && grapheme.byte_range.start < byte_range.end)
                .then_some(idx)
        })
        .collect();

    if indices.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let first = &graphemes[indices[0]];
    let mut current_script = normalize_script(
        detect_script(&text[first.byte_range.clone()]),
        Script::Latin,
    );
    let mut current_start = first.byte_range.start;
    let mut current_end = first.byte_range.end;

    for index in indices.into_iter().skip(1) {
        let grapheme = &graphemes[index];
        let grapheme_script = normalize_script(
            detect_script(&text[grapheme.byte_range.clone()]),
            current_script,
        );

        if grapheme_script == current_script {
            current_end = grapheme.byte_range.end;
            continue;
        }

        runs.push(ScriptRun {
            byte_range: current_start..current_end,
            script: current_script,
        });
        current_script = grapheme_script;
        current_start = grapheme.byte_range.start;
        current_end = grapheme.byte_range.end;
    }

    runs.push(ScriptRun {
        byte_range: current_start..current_end,
        script: current_script,
    });
    runs
}

fn split_complex_script_run(
    text: &str,
    graphemes: &[AnalyzedGrapheme],
    run: &ScriptRun,
    catalog: &FontCatalog,
    preferred: &[FontId],
) -> Vec<FontRun> {
    let run_text = &text[run.byte_range.clone()];
    let Some(face) = catalog.best_face_for_run(run_text, preferred) else {
        return Vec::new();
    };

    if run_text.chars().all(|ch| face.has_glyph(ch)) {
        return vec![FontRun {
            byte_range: run.byte_range.clone(),
            face,
            script: run.script,
        }];
    }

    split_by_coverage(text, graphemes, run, catalog, preferred)
}

fn split_by_coverage(
    text: &str,
    graphemes: &[AnalyzedGrapheme],
    run: &ScriptRun,
    catalog: &FontCatalog,
    preferred: &[FontId],
) -> Vec<FontRun> {
    let indices: Vec<usize> = graphemes
        .iter()
        .enumerate()
        .filter_map(|(idx, grapheme)| {
            (grapheme.byte_range.start >= run.byte_range.start
                && grapheme.byte_range.start < run.byte_range.end)
                .then_some(idx)
        })
        .collect();

    if indices.is_empty() {
        return Vec::new();
    }

    let first = &graphemes[indices[0]];
    let first_text = &text[first.byte_range.clone()];
    let Some(mut current_face) = catalog.face_for_cluster(first_text, preferred) else {
        return Vec::new();
    };
    let mut current_start = first.byte_range.start;
    let mut current_end = first.byte_range.end;
    let mut runs = Vec::new();

    for index in indices.into_iter().skip(1) {
        let grapheme = &graphemes[index];
        let grapheme_text = &text[grapheme.byte_range.clone()];
        let face = catalog
            .face_for_cluster(grapheme_text, preferred)
            .unwrap_or_else(|| current_face.clone());
        if face.id() == current_face.id() {
            current_end = grapheme.byte_range.end;
            continue;
        }

        runs.push(FontRun {
            byte_range: current_start..current_end,
            face: current_face,
            script: run.script,
        });

        current_face = face;
        current_start = grapheme.byte_range.start;
        current_end = grapheme.byte_range.end;
    }

    runs.push(FontRun {
        byte_range: current_start..current_end,
        face: current_face,
        script: run.script,
    });
    runs
}

fn is_complex_script(script: Script) -> bool {
    matches!(
        script,
        Script::Arabic
            | Script::Hebrew
            | Script::Devanagari
            | Script::Bengali
            | Script::Gurmukhi
            | Script::Myanmar
            | Script::Khmer
            | Script::Thai
    )
}

fn accumulate_cluster_advances(
    shaped: &[ShapedGlyph],
    graphemes: &[AnalyzedGrapheme],
    run_byte_range: Range<usize>,
    starts: &AHashMap<usize, usize>,
    grapheme_advances: &mut [f32],
) {
    if shaped.is_empty() {
        return;
    }

    let mut cluster_totals = Vec::new();
    for glyph in shaped {
        if let Some((cluster, width)) = cluster_totals.last_mut() {
            if *cluster == glyph.cluster_byte {
                *width += glyph.advance;
                continue;
            }
        }
        cluster_totals.push((glyph.cluster_byte, glyph.advance));
    }

    for index in 0..cluster_totals.len() {
        let (cluster_start, cluster_width) = cluster_totals[index];
        let cluster_end = cluster_totals
            .get(index + 1)
            .map(|(start, _)| *start)
            .unwrap_or(run_byte_range.end - run_byte_range.start);
        let global_start = run_byte_range.start + cluster_start;
        let global_end = run_byte_range.start + cluster_end;

        let mut hit = Vec::new();
        for grapheme in graphemes {
            if grapheme.byte_range.start >= global_start && grapheme.byte_range.start < global_end {
                if let Some(grapheme_index) = starts.get(&grapheme.byte_range.start).copied() {
                    hit.push(grapheme_index);
                }
            }
        }

        if hit.is_empty() {
            if let Some(grapheme_index) = graphemes
                .iter()
                .find(|grapheme| {
                    grapheme.byte_range.start <= global_start
                        && grapheme.byte_range.end > global_start
                })
                .and_then(|grapheme| starts.get(&grapheme.byte_range.start))
                .copied()
            {
                hit.push(grapheme_index);
            }
        }

        if hit.is_empty() {
            continue;
        }

        let share = cluster_width / hit.len() as f32;
        for grapheme_index in hit {
            grapheme_advances[grapheme_index] += share;
        }
    }
}

fn offset_cluster_bytes(shaped: &[ShapedGlyph], byte_offset: usize) -> Vec<ShapedGlyph> {
    shaped
        .iter()
        .map(|glyph| ShapedGlyph {
            cluster_byte: glyph.cluster_byte + byte_offset,
            ..*glyph
        })
        .collect()
}

fn detect_script(text: &str) -> Script {
    text.chars()
        .map(|ch| ch.script())
        .find(|script| !matches!(script, Script::Common | Script::Inherited | Script::Unknown))
        .unwrap_or(Script::Latin)
}

fn normalize_script(script: Script, fallback: Script) -> Script {
    match script {
        Script::Common | Script::Inherited | Script::Unknown => fallback,
        _ => script,
    }
}

fn map_script(script: Script) -> rustybuzz::Script {
    match script {
        Script::Arabic => rustybuzz::script::ARABIC,
        Script::Hebrew => rustybuzz::script::HEBREW,
        Script::Devanagari => rustybuzz::script::DEVANAGARI,
        Script::Bengali => rustybuzz::script::BENGALI,
        Script::Gurmukhi => rustybuzz::script::GURMUKHI,
        Script::Myanmar => rustybuzz::script::MYANMAR,
        Script::Khmer => rustybuzz::script::KHMER,
        Script::Thai => rustybuzz::script::THAI,
        Script::Han => rustybuzz::script::HAN,
        Script::Hiragana => rustybuzz::script::HIRAGANA,
        Script::Katakana => rustybuzz::script::KATAKANA,
        Script::Hangul => rustybuzz::script::HANGUL,
        Script::Cyrillic => rustybuzz::script::CYRILLIC,
        Script::Greek => rustybuzz::script::GREEK,
        _ => rustybuzz::script::LATIN,
    }
}

fn hash_text(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut state = ahash::AHasher::default();
    text.hash(&mut state);
    state.finish()
}

fn hash_style(style: &TextStyleSpec) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut state = ahash::AHasher::default();
    style.hash(&mut state);
    state.finish()
}

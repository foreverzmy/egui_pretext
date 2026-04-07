use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use ahash::AHashMap;
use fontdb::{Database, Family, Query, Source, Stretch, Style, Weight, ID};
use parking_lot::RwLock;

use crate::engine::TextStyleSpec;

pub type FontId = ID;

#[derive(Clone, Copy, Debug)]
struct CoverageRange {
    start: u32,
    end: u32,
}

struct CoverageCacheEntry {
    codepoints: Arc<[u32]>,
    ranges: Arc<[CoverageRange]>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum CoverageCacheKey {
    Binary { hash: u64, len: usize, index: u32 },
    Path { path: PathBuf, index: u32 },
}

#[derive(Clone)]
pub struct LoadedFace {
    id: FontId,
    family_name: Arc<str>,
    data: Arc<[u8]>,
    face_index: u32,
}

impl LoadedFace {
    pub fn id(&self) -> FontId {
        self.id
    }

    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn face_index(&self) -> u32 {
        self.face_index
    }

    pub fn units_per_em(&self) -> u16 {
        self.with_ttf_face(|face| face.units_per_em())
            .unwrap_or(1000)
    }

    pub fn has_glyph(&self, ch: char) -> bool {
        if !requires_glyph(ch) {
            return true;
        }
        self.with_ttf_face(|face| face.glyph_index(ch).is_some())
            .unwrap_or(false)
    }

    fn with_ttf_face<T>(&self, f: impl FnOnce(&ttf_parser::Face<'_>) -> T) -> Option<T> {
        let face = ttf_parser::Face::parse(&self.data, self.face_index).ok()?;
        Some(f(&face))
    }
}

pub struct FontCatalog {
    db: Database,
    faces: Vec<FontId>,
    default_face: Option<FontId>,
    char_to_font: RwLock<AHashMap<char, FontId>>,
    face_coverage: RwLock<AHashMap<FontId, Arc<[CoverageRange]>>>,
    loaded_faces: RwLock<AHashMap<FontId, Arc<LoadedFace>>>,
}

impl FontCatalog {
    pub fn new() -> Self {
        let mut db = Database::new();
        db.load_system_fonts();
        Self::build(&db)
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
        let mut db = Database::new();
        for data in font_data {
            db.load_font_data(data);
        }
        if include_system_fonts {
            db.load_system_fonts();
        }
        Self::build(&db)
    }

    pub fn build(db: &Database) -> Self {
        let faces: Vec<FontId> = db.faces().map(|face| face.id).collect();
        let default_face = faces.first().copied();
        let (char_to_font, face_coverage) = build_prewarmed_coverage_maps(db, &faces);
        Self {
            db: db.clone(),
            faces,
            default_face,
            char_to_font: RwLock::new(char_to_font),
            face_coverage: RwLock::new(face_coverage),
            loaded_faces: RwLock::new(AHashMap::new()),
        }
    }

    pub fn clear_runtime_caches(&self) {
        self.loaded_faces.write().clear();
    }

    pub fn resolve_style_chain(&self, style: &TextStyleSpec) -> Vec<FontId> {
        let mut resolved = Vec::new();
        for family in &style.families {
            let families = [Family::Name(family.as_str())];
            let query = Query {
                families: &families,
                weight: Weight(style.weight),
                stretch: Stretch::Normal,
                style: if style.italic {
                    Style::Italic
                } else {
                    Style::Normal
                },
            };
            if let Some(id) = self.db.query(&query) {
                if !resolved.contains(&id) {
                    resolved.push(id);
                }
            }
        }

        if resolved.is_empty() {
            if let Some(id) = self.default_face {
                resolved.push(id);
            }
        }

        resolved
    }

    pub fn font_for_char(&self, ch: char, preferred: &[FontId]) -> Option<FontId> {
        if !requires_glyph(ch) {
            return preferred.first().copied().or(self.default_face);
        }

        for id in preferred {
            if self.face_covers_char(*id, ch) {
                return Some(*id);
            }
        }

        if let Some(id) = self.char_to_font.read().get(&ch).copied() {
            return Some(id);
        }

        self.find_fallback_font_for_char(ch)
    }

    pub fn face_for_char(&self, ch: char, preferred: &[FontId]) -> Option<Arc<LoadedFace>> {
        self.font_for_char(ch, preferred)
            .and_then(|id| self.load_face(id))
            .or_else(|| self.default_face())
    }

    pub fn face_for_cluster(&self, cluster: &str, preferred: &[FontId]) -> Option<Arc<LoadedFace>> {
        self.candidate_ids(preferred)
            .into_iter()
            .find_map(|id| {
                self.face_covers_cluster(id, cluster)
                    .then(|| self.load_face(id))
                    .flatten()
            })
            .or_else(|| self.default_face())
    }

    pub(crate) fn best_face_for_run(
        &self,
        run_text: &str,
        preferred: &[FontId],
    ) -> Option<Arc<LoadedFace>> {
        let mut best: Option<(usize, FontId)> = None;
        let chars: Vec<char> = run_text.chars().collect();

        for id in self.candidate_ids(preferred) {
            let score = chars
                .iter()
                .filter(|&&ch| self.face_covers_char(id, ch))
                .count();
            if score == chars.len() {
                return self.load_face(id).or_else(|| self.default_face());
            }
            if best
                .as_ref()
                .map(|(best_score, _)| score > *best_score)
                .unwrap_or(true)
            {
                best = Some((score, id));
            }
        }

        best.and_then(|(_, id)| self.load_face(id))
            .or_else(|| self.default_face())
    }

    pub fn load_face(&self, id: FontId) -> Option<Arc<LoadedFace>> {
        if let Some(face) = self.loaded_faces.read().get(&id) {
            return Some(face.clone());
        }

        let family_name: Arc<str> = self
            .db
            .face(id)
            .and_then(|face| face.families.first().map(|family| family.0.clone()))
            .unwrap_or_else(|| "Unknown".to_owned())
            .into();

        let (data, face_index) = self
            .db
            .with_face_data(id, |data, face_index| (Arc::<[u8]>::from(data), face_index))?;
        let face = Arc::new(LoadedFace {
            id,
            family_name,
            data,
            face_index,
        });
        self.loaded_faces.write().insert(id, face.clone());
        Some(face)
    }

    fn candidate_ids(&self, preferred: &[FontId]) -> Vec<FontId> {
        let mut ids = Vec::with_capacity(preferred.len() + self.faces.len());
        for id in preferred.iter().copied().chain(self.faces.iter().copied()) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }

    fn default_face(&self) -> Option<Arc<LoadedFace>> {
        self.default_face.and_then(|id| self.load_face(id))
    }

    fn find_fallback_font_for_char(&self, ch: char) -> Option<FontId> {
        let fallback = self
            .faces
            .iter()
            .copied()
            .find(|id| self.face_covers_char(*id, ch))
            .or(self.default_face);

        if let Some(id) = fallback {
            self.char_to_font.write().entry(ch).or_insert(id);
        }

        fallback
    }

    fn face_covers_cluster(&self, id: FontId, cluster: &str) -> bool {
        cluster.chars().all(|ch| self.face_covers_char(id, ch))
    }

    fn face_covers_char(&self, id: FontId, ch: char) -> bool {
        if !requires_glyph(ch) {
            return true;
        }

        self.coverage_ranges_for_face(id)
            .as_deref()
            .map(|ranges| coverage_contains(ranges, ch))
            .unwrap_or_else(|| {
                self.load_face(id)
                    .map(|face| face.has_glyph(ch))
                    .unwrap_or(false)
            })
    }

    fn coverage_ranges_for_face(&self, id: FontId) -> Option<Arc<[CoverageRange]>> {
        if let Some(ranges) = self.face_coverage.read().get(&id).cloned() {
            return Some(ranges);
        }

        let entry = coverage_entry_for_face(&self.db, id)?;
        {
            let mut face_coverage = self.face_coverage.write();
            if let Some(ranges) = face_coverage.get(&id).cloned() {
                return Some(ranges);
            }
            face_coverage.insert(id, entry.ranges.clone());
        }
        self.seed_char_to_font(id, entry.codepoints.as_ref());
        Some(entry.ranges.clone())
    }

    fn seed_char_to_font(&self, id: FontId, codepoints: &[u32]) {
        let mut char_to_font = self.char_to_font.write();
        for &codepoint in codepoints {
            if let Some(ch) = char::from_u32(codepoint) {
                char_to_font.entry(ch).or_insert(id);
            }
        }
    }
}

fn requires_glyph(ch: char) -> bool {
    !matches!(ch, '\n' | '\r' | '\t' | '\u{200B}' | '\u{2060}')
}

fn coverage_cache() -> &'static RwLock<AHashMap<CoverageCacheKey, Arc<CoverageCacheEntry>>> {
    static CACHE: OnceLock<RwLock<AHashMap<CoverageCacheKey, Arc<CoverageCacheEntry>>>> =
        OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(AHashMap::new()))
}

fn build_prewarmed_coverage_maps(
    db: &Database,
    faces: &[FontId],
) -> (
    AHashMap<char, FontId>,
    AHashMap<FontId, Arc<[CoverageRange]>>,
) {
    let mut char_to_font = AHashMap::new();
    let mut face_coverage = AHashMap::new();

    for id in faces {
        if !should_prewarm_face(db, *id) {
            continue;
        }

        let Some(entry) = coverage_entry_for_face(db, *id) else {
            continue;
        };

        for &codepoint in entry.codepoints.iter() {
            if let Some(ch) = char::from_u32(codepoint) {
                char_to_font.entry(ch).or_insert(*id);
            }
        }

        face_coverage.insert(*id, entry.ranges.clone());
    }

    (char_to_font, face_coverage)
}

fn should_prewarm_face(db: &Database, id: FontId) -> bool {
    matches!(db.face(id), Some(face) if matches!(&face.source, Source::Binary(_)))
}

fn coverage_entry_for_face(db: &Database, id: FontId) -> Option<Arc<CoverageCacheEntry>> {
    let info = db.face(id)?;

    match &info.source {
        Source::Binary(_) => db
            .with_face_data(id, |data, face_index| {
                let key = CoverageCacheKey::Binary {
                    hash: hash_bytes(data),
                    len: data.len(),
                    index: face_index,
                };
                coverage_entry_with_key(key, || build_coverage_entry(data, face_index))
            })
            .flatten(),
        Source::File(path) | Source::SharedFile(path, _) => coverage_entry_with_key(
            CoverageCacheKey::Path {
                path: path.clone(),
                index: info.index,
            },
            || {
                db.with_face_data(id, |data, face_index| {
                    build_coverage_entry(data, face_index)
                })
                .flatten()
            },
        ),
    }
}

fn coverage_entry_with_key(
    key: CoverageCacheKey,
    build: impl FnOnce() -> Option<CoverageCacheEntry>,
) -> Option<Arc<CoverageCacheEntry>> {
    if let Some(entry) = coverage_cache().read().get(&key).cloned() {
        return Some(entry);
    }

    let entry = Arc::new(build()?);
    let mut cache = coverage_cache().write();
    Some(cache.entry(key).or_insert_with(|| entry.clone()).clone())
}

fn build_coverage_entry(data: &[u8], face_index: u32) -> Option<CoverageCacheEntry> {
    let codepoints: Arc<[u32]> = collect_face_codepoints(data, face_index)?.into();
    let ranges = collapse_codepoints_to_ranges(&codepoints);
    Some(CoverageCacheEntry { codepoints, ranges })
}

fn collect_face_codepoints(data: &[u8], face_index: u32) -> Option<Vec<u32>> {
    let face = ttf_parser::Face::parse(data, face_index).ok()?;
    let cmap = face.tables().cmap?;
    let mut codepoints = Vec::new();

    for subtable in cmap.subtables {
        if !subtable.is_unicode() {
            continue;
        }

        subtable.codepoints(|codepoint| {
            if char::from_u32(codepoint).is_some() && subtable.glyph_index(codepoint).is_some() {
                codepoints.push(codepoint);
            }
        });
    }

    codepoints.sort_unstable();
    codepoints.dedup();
    Some(codepoints)
}

fn collapse_codepoints_to_ranges(codepoints: &[u32]) -> Arc<[CoverageRange]> {
    let Some(&first) = codepoints.first() else {
        return Arc::from(Vec::<CoverageRange>::new());
    };

    let mut ranges = Vec::new();
    let mut start = first;
    let mut end = first;

    for &codepoint in &codepoints[1..] {
        if codepoint == end.saturating_add(1) {
            end = codepoint;
            continue;
        }

        ranges.push(CoverageRange { start, end });
        start = codepoint;
        end = codepoint;
    }

    ranges.push(CoverageRange { start, end });
    Arc::from(ranges)
}

fn coverage_contains(ranges: &[CoverageRange], ch: char) -> bool {
    let target = ch as u32;
    let mut low = 0usize;
    let mut high = ranges.len();

    while low < high {
        let mid = low + (high - low) / 2;
        let range = &ranges[mid];
        if target < range.start {
            high = mid;
        } else if target > range.end {
            low = mid + 1;
        } else {
            return true;
        }
    }

    false
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut state = ahash::AHasher::default();
    bytes.hash(&mut state);
    state.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_font_data() -> Vec<Vec<u8>> {
        vec![
            include_bytes!("../../../demos/app/assets/fonts/NotoSans-Regular.ttf").to_vec(),
            include_bytes!("../../../demos/app/assets/fonts/NotoSansArabic-Regular.ttf").to_vec(),
            include_bytes!("../../../demos/app/assets/fonts/NotoSansCJK-Regular.ttc").to_vec(),
            include_bytes!("../../../demos/app/assets/fonts/NotoSansMyanmar-Regular.ttf").to_vec(),
            include_bytes!("../../../demos/app/assets/fonts/NotoEmoji-Regular.ttf").to_vec(),
            include_bytes!("../../../demos/app/assets/fonts/NotoColorEmoji.ttf").to_vec(),
            include_bytes!("../../../demos/app/assets/fonts/Noto-COLRv1.ttf").to_vec(),
            include_bytes!("../../../demos/app/assets/fonts/NotoSansMono-Regular.ttf").to_vec(),
        ]
    }

    fn file_backed_face_with_glyph(catalog: &FontCatalog, ch: char) -> Option<FontId> {
        catalog.faces.iter().copied().find(|id| {
            let Some(face_info) = catalog.db.face(*id) else {
                return false;
            };
            if !matches!(
                &face_info.source,
                Source::File(_) | Source::SharedFile(_, _)
            ) {
                return false;
            }

            catalog
                .db
                .with_face_data(*id, |data, face_index| {
                    ttf_parser::Face::parse(data, face_index)
                        .ok()
                        .and_then(|face| face.glyph_index(ch))
                        .is_some()
                })
                .unwrap_or(false)
        })
    }

    #[test]
    fn build_only_prewarms_binary_face_coverage() {
        let catalog = FontCatalog::with_font_data_and_system_fonts(bundled_font_data(), true);
        let expected_binary_count = catalog
            .faces
            .iter()
            .copied()
            .filter(|id| should_prewarm_face(&catalog.db, *id))
            .count();

        assert_eq!(catalog.face_coverage.read().len(), expected_binary_count);

        if let Some(file_backed) = file_backed_face_with_glyph(&catalog, 'A') {
            assert!(!catalog.face_coverage.read().contains_key(&file_backed));
        }
    }

    #[test]
    fn file_backed_face_coverage_is_built_lazily() {
        let catalog = FontCatalog::with_font_data_and_system_fonts(bundled_font_data(), true);
        let Some(file_backed) = file_backed_face_with_glyph(&catalog, 'A') else {
            return;
        };

        let before = catalog.face_coverage.read().len();
        assert!(!catalog.face_coverage.read().contains_key(&file_backed));
        assert!(catalog.face_covers_char(file_backed, 'A'));
        assert!(catalog.face_coverage.read().contains_key(&file_backed));
        assert_eq!(catalog.face_coverage.read().len(), before + 1);
    }

    #[test]
    fn best_face_for_run_only_loads_selected_face_data() {
        let catalog = FontCatalog::with_font_data_and_system_fonts(bundled_font_data(), true);
        let style = TextStyleSpec {
            families: vec![
                "Helvetica".to_owned(),
                "Arial".to_owned(),
                "Noto Sans".to_owned(),
            ],
            size_px: 16.0,
            weight: 400,
            italic: false,
        };
        let preferred = catalog.resolve_style_chain(&style);

        assert_eq!(catalog.loaded_faces.read().len(), 0);
        let face = catalog
            .best_face_for_run("Hello", &preferred)
            .expect("expected a face for ASCII run");
        let cached = catalog
            .loaded_faces
            .read()
            .get(&face.id())
            .cloned()
            .expect("expected the selected face to be cached");

        assert_eq!(catalog.loaded_faces.read().len(), 1);
        assert!(Arc::ptr_eq(&cached, &face));
    }
}

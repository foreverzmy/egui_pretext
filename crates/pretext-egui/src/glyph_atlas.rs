use ahash::{AHashMap, AHashSet};
use egui::epaint::Mesh;
use egui::{Color32, ColorImage, Context, Painter, Rect, Shape, TextureHandle, TextureOptions};
use image::ImageFormat;
use pretext::font_catalog::FontId;
use pretext::{LayoutLineGlyphRun, PretextEngine, TextStyleSpec};
use resvg::usvg;

const ATLAS_PAGE_SIZE: usize = 2048;
const ATLAS_GLYPH_PADDING_PX: usize = 1;
const ATLAS_TEXTURE_OPTIONS: TextureOptions = TextureOptions::LINEAR;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphAtlasStats {
    pub pages: usize,
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
}

#[derive(Default)]
pub struct GlyphSceneBuilder {
    meshes: AHashMap<egui::TextureId, Mesh>,
    glyph_quads: u64,
    painted: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GlyphSceneFlushStats {
    pub mesh_flushes: u64,
    pub glyph_quads: u64,
    pub painted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphWarmResult {
    Hit,
    Miss,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct GlyphRasterKey {
    engine_revision: u64,
    face_id: FontId,
    glyph_id: u16,
    size_px_q: u32,
    pixels_per_point_q: u32,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct FaceMetricKey {
    engine_revision: u64,
    face_id: FontId,
    size_px_q: u32,
}

#[derive(Clone)]
struct GlyphAtlasEntry {
    page_index: usize,
    uv_rect: Rect,
    logical_size: egui::Vec2,
    offset: egui::Vec2,
    is_color: bool,
}

#[derive(Clone, Copy)]
struct FaceMetrics {
    ascent: f32,
    descent: f32,
}

struct AtlasPage {
    texture: TextureHandle,
    next_x: usize,
    next_y: usize,
    row_height: usize,
}

pub struct GlyphAtlas {
    pages: Vec<AtlasPage>,
    entries: AHashMap<GlyphRasterKey, GlyphAtlasEntry>,
    face_metrics: AHashMap<FaceMetricKey, FaceMetrics>,
    hits: u64,
    misses: u64,
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        Self {
            pages: Vec::new(),
            entries: AHashMap::new(),
            face_metrics: AHashMap::new(),
            hits: 0,
            misses: 0,
        }
    }
}

impl GlyphAtlas {
    pub fn stats(&self) -> GlyphAtlasStats {
        GlyphAtlasStats {
            pages: self.pages.len(),
            entries: self.entries.len(),
            hits: self.hits,
            misses: self.misses,
        }
    }

    pub fn begin_scene(&self) -> GlyphSceneBuilder {
        GlyphSceneBuilder::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn paint_line_glyph_runs(
        &mut self,
        painter: &Painter,
        x: f32,
        y: f32,
        glyph_runs: &[LayoutLineGlyphRun],
        style: &TextStyleSpec,
        line_height: f32,
        color: Color32,
        ctx: &Context,
        engine: &PretextEngine,
        texture_uploads: &mut u64,
        texture_upload_bytes: &mut u64,
    ) -> bool {
        let mut scene = self.begin_scene();
        let painted = self.append_line_glyph_runs(
            &mut scene,
            x,
            y,
            glyph_runs,
            style,
            line_height,
            color,
            ctx,
            engine,
            texture_uploads,
            texture_upload_bytes,
        );
        let _ = self.flush_scene(painter, &mut scene);
        painted
    }

    #[allow(clippy::too_many_arguments)]
    pub fn append_line_glyph_runs(
        &mut self,
        scene: &mut GlyphSceneBuilder,
        x: f32,
        y: f32,
        glyph_runs: &[LayoutLineGlyphRun],
        style: &TextStyleSpec,
        line_height: f32,
        color: Color32,
        ctx: &Context,
        engine: &PretextEngine,
        texture_uploads: &mut u64,
        texture_upload_bytes: &mut u64,
    ) -> bool {
        if glyph_runs.is_empty() {
            return false;
        }

        let pixels_per_point = ctx.pixels_per_point().max(1.0);
        let baseline_y = y + self.baseline_px(glyph_runs, style, line_height, engine);

        for run in glyph_runs {
            for glyph in &run.glyphs {
                let Some(lookup) = self.glyph_entry(
                    ctx,
                    engine,
                    glyph.face_id,
                    glyph.glyph_id,
                    style.size_px,
                    pixels_per_point,
                    texture_uploads,
                    texture_upload_bytes,
                ) else {
                    continue;
                };

                let rect_min = egui::pos2(
                    snap_to_pixel(
                        x + glyph.x + glyph.x_offset + lookup.entry.offset.x,
                        pixels_per_point,
                    ),
                    snap_to_pixel(
                        baseline_y - glyph.y_offset + lookup.entry.offset.y,
                        pixels_per_point,
                    ),
                );
                let rect = Rect::from_min_size(rect_min, lookup.entry.logical_size);
                let texture_id = self.pages[lookup.entry.page_index].texture.id();
                let mesh = scene
                    .meshes
                    .entry(texture_id)
                    .or_insert_with(|| Mesh::with_texture(texture_id));
                mesh.add_rect_with_uv(
                    rect,
                    lookup.entry.uv_rect,
                    if lookup.entry.is_color {
                        Color32::WHITE
                    } else {
                        color
                    },
                );
                scene.painted = true;
                scene.glyph_quads += 1;
            }
        }

        scene.painted
    }

    pub fn flush_scene(
        &mut self,
        painter: &Painter,
        scene: &mut GlyphSceneBuilder,
    ) -> GlyphSceneFlushStats {
        let mut flush_stats = GlyphSceneFlushStats {
            glyph_quads: scene.glyph_quads,
            painted: scene.painted,
            ..GlyphSceneFlushStats::default()
        };

        for mesh in std::mem::take(&mut scene.meshes).into_values() {
            if mesh.is_empty() {
                continue;
            }
            painter.add(Shape::mesh(mesh));
            flush_stats.mesh_flushes += 1;
        }

        scene.glyph_quads = 0;
        scene.painted = false;
        flush_stats
    }

    #[allow(clippy::too_many_arguments)]
    pub fn warm_glyph(
        &mut self,
        ctx: &Context,
        engine: &PretextEngine,
        face_id: FontId,
        glyph_id: u16,
        size_px: f32,
        pixels_per_point: f32,
        texture_uploads: &mut u64,
        texture_upload_bytes: &mut u64,
    ) -> Option<GlyphWarmResult> {
        self.glyph_entry(
            ctx,
            engine,
            face_id,
            glyph_id,
            size_px,
            pixels_per_point,
            texture_uploads,
            texture_upload_bytes,
        )
        .map(|lookup| lookup.result)
    }

    fn baseline_px(
        &mut self,
        glyph_runs: &[LayoutLineGlyphRun],
        style: &TextStyleSpec,
        line_height: f32,
        engine: &PretextEngine,
    ) -> f32 {
        let mut ascent = style.size_px * 0.8;
        let mut descent = style.size_px * 0.2;
        let mut seen = AHashSet::new();

        for glyph in glyph_runs.iter().flat_map(|run| run.glyphs.iter()) {
            if !seen.insert(glyph.face_id) {
                continue;
            }
            let Some(metrics) = self.face_metrics(engine, glyph.face_id, style.size_px) else {
                continue;
            };
            ascent = ascent.max(metrics.ascent);
            descent = descent.max(metrics.descent);
        }

        let content_height = (ascent + descent).max(1.0);
        ((line_height - content_height).max(0.0)) * 0.5 + ascent
    }

    fn face_metrics(
        &mut self,
        engine: &PretextEngine,
        face_id: FontId,
        size_px: f32,
    ) -> Option<FaceMetrics> {
        let key = FaceMetricKey {
            engine_revision: engine.revision(),
            face_id,
            size_px_q: quantize(size_px),
        };
        if let Some(metrics) = self.face_metrics.get(&key).copied() {
            return Some(metrics);
        }

        let face = engine.load_face(face_id)?;
        let ttf = ttf_parser::Face::parse(face.data(), face.face_index()).ok()?;
        let scale = size_px / face.units_per_em().max(1) as f32;
        let metrics = FaceMetrics {
            ascent: (ttf.ascender() as f32 * scale).max(1.0),
            descent: (-(ttf.descender() as f32) * scale).max(0.0),
        };
        self.face_metrics.insert(key, metrics);
        Some(metrics)
    }

    fn glyph_entry(
        &mut self,
        ctx: &Context,
        engine: &PretextEngine,
        face_id: FontId,
        glyph_id: u16,
        size_px: f32,
        pixels_per_point: f32,
        texture_uploads: &mut u64,
        texture_upload_bytes: &mut u64,
    ) -> Option<GlyphLookup> {
        let key = GlyphRasterKey {
            engine_revision: engine.revision(),
            face_id,
            glyph_id,
            size_px_q: quantize(size_px),
            pixels_per_point_q: quantize(pixels_per_point),
        };
        if let Some(entry) = self.entries.get(&key).cloned() {
            self.hits += 1;
            return Some(GlyphLookup {
                entry,
                result: GlyphWarmResult::Hit,
            });
        }
        self.misses += 1;

        let face = engine.load_face(face_id)?;
        let raster = rasterize_glyph(&face, glyph_id, size_px, pixels_per_point)?;
        let placement = self.allocate(raster.image.size, ctx)?;
        let page = self.pages.get_mut(placement.page_index)?;
        page.texture
            .set_partial(placement.pos, raster.image, ATLAS_TEXTURE_OPTIONS);
        *texture_uploads += 1;
        *texture_upload_bytes += (placement.upload_size[0] * placement.upload_size[1] * 4) as u64;

        let entry = GlyphAtlasEntry {
            page_index: placement.page_index,
            uv_rect: placement.uv_rect,
            logical_size: raster.logical_size,
            offset: raster.offset,
            is_color: raster.is_color,
        };
        self.entries.insert(key, entry.clone());
        Some(GlyphLookup {
            entry,
            result: GlyphWarmResult::Miss,
        })
    }

    fn allocate(&mut self, size: [usize; 2], ctx: &Context) -> Option<Allocation> {
        if size[0] > ATLAS_PAGE_SIZE || size[1] > ATLAS_PAGE_SIZE {
            return None;
        }

        for (page_index, page) in self.pages.iter_mut().enumerate() {
            if let Some(pos) = allocate_on_page(page, size) {
                return Some(allocation(page_index, pos, size));
            }
        }

        let image = ColorImage::filled([ATLAS_PAGE_SIZE, ATLAS_PAGE_SIZE], Color32::TRANSPARENT);
        let texture = ctx.load_texture(
            format!("pretext-egui/glyph-atlas/{}", self.pages.len()),
            image,
            ATLAS_TEXTURE_OPTIONS,
        );
        let mut page = AtlasPage {
            texture,
            next_x: 0,
            next_y: 0,
            row_height: 0,
        };
        let pos = allocate_on_page(&mut page, size)?;
        let page_index = self.pages.len();
        self.pages.push(page);
        Some(allocation(page_index, pos, size))
    }
}

struct Allocation {
    page_index: usize,
    pos: [usize; 2],
    upload_size: [usize; 2],
    uv_rect: Rect,
}

struct GlyphLookup {
    entry: GlyphAtlasEntry,
    result: GlyphWarmResult,
}

struct RasterizedGlyph {
    image: ColorImage,
    logical_size: egui::Vec2,
    offset: egui::Vec2,
    is_color: bool,
}

fn allocate_on_page(page: &mut AtlasPage, size: [usize; 2]) -> Option<[usize; 2]> {
    if page.next_x + size[0] > ATLAS_PAGE_SIZE {
        page.next_x = 0;
        page.next_y += page.row_height;
        page.row_height = 0;
    }
    if page.next_y + size[1] > ATLAS_PAGE_SIZE {
        return None;
    }

    let pos = [page.next_x, page.next_y];
    page.next_x += size[0];
    page.row_height = page.row_height.max(size[1]);
    Some(pos)
}

fn allocation(page_index: usize, pos: [usize; 2], size: [usize; 2]) -> Allocation {
    let inv = 1.0 / ATLAS_PAGE_SIZE as f32;
    Allocation {
        page_index,
        pos,
        upload_size: size,
        uv_rect: Rect::from_min_max(
            egui::pos2(pos[0] as f32 * inv, pos[1] as f32 * inv),
            egui::pos2(
                (pos[0] + size[0]) as f32 * inv,
                (pos[1] + size[1]) as f32 * inv,
            ),
        ),
    }
}

fn rasterize_glyph(
    face: &pretext::font_catalog::LoadedFace,
    glyph_id: u16,
    size_px: f32,
    pixels_per_point: f32,
) -> Option<RasterizedGlyph> {
    let ttf = ttf_parser::Face::parse(face.data(), face.face_index()).ok()?;
    let glyph_id = ttf_parser::GlyphId(glyph_id);

    rasterize_bitmap_glyph(&ttf, glyph_id, size_px, pixels_per_point)
        .or_else(|| rasterize_svg_glyph(&ttf, glyph_id, size_px, pixels_per_point))
        .or_else(|| rasterize_outline_glyph(&ttf, glyph_id, size_px, pixels_per_point))
}

fn rasterize_outline_glyph(
    face: &ttf_parser::Face<'_>,
    glyph_id: ttf_parser::GlyphId,
    size_px: f32,
    pixels_per_point: f32,
) -> Option<RasterizedGlyph> {
    let bbox = face.glyph_bounding_box(glyph_id)?;
    let units_per_em = face.units_per_em().max(1) as f32;
    let scale = size_px * pixels_per_point / units_per_em;
    let left_px = (bbox.x_min as f32 * scale).floor();
    let right_px = (bbox.x_max as f32 * scale).ceil();
    let top_px = (-bbox.y_max as f32 * scale).floor();
    let bottom_px = (-bbox.y_min as f32 * scale).ceil();
    let width = (right_px - left_px).max(1.0) as usize + ATLAS_GLYPH_PADDING_PX * 2;
    let height = (bottom_px - top_px).max(1.0) as usize + ATLAS_GLYPH_PADDING_PX * 2;

    let mut builder = GlyphPathBuilder::default();
    face.outline_glyph(glyph_id, &mut builder)?;
    let path = builder.finish()?;
    let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32)?;
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(255, 255, 255, 255);
    paint.anti_alias = true;
    let transform = tiny_skia::Transform::from_row(
        scale,
        0.0,
        0.0,
        -scale,
        -left_px + ATLAS_GLYPH_PADDING_PX as f32,
        -top_px + ATLAS_GLYPH_PADDING_PX as f32,
    );
    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);

    Some(RasterizedGlyph {
        image: ColorImage::from_rgba_premultiplied([width, height], pixmap.data()),
        logical_size: egui::vec2(
            width as f32 / pixels_per_point,
            height as f32 / pixels_per_point,
        ),
        offset: egui::vec2(
            left_px / pixels_per_point - ATLAS_GLYPH_PADDING_PX as f32 / pixels_per_point,
            top_px / pixels_per_point - ATLAS_GLYPH_PADDING_PX as f32 / pixels_per_point,
        ),
        is_color: false,
    })
}

fn rasterize_svg_glyph(
    face: &ttf_parser::Face<'_>,
    glyph_id: ttf_parser::GlyphId,
    size_px: f32,
    pixels_per_point: f32,
) -> Option<RasterizedGlyph> {
    let svg = face.glyph_svg_image(glyph_id)?;
    let bbox = face.glyph_bounding_box(glyph_id)?;
    let units_per_em = face.units_per_em().max(1) as f32;
    let scale = size_px * pixels_per_point / units_per_em;
    let left_px = (bbox.x_min as f32 * scale).floor();
    let right_px = (bbox.x_max as f32 * scale).ceil();
    let top_px = (-bbox.y_max as f32 * scale).floor();
    let bottom_px = (-bbox.y_min as f32 * scale).ceil();
    let width = (right_px - left_px).max(1.0) as usize;
    let height = (bottom_px - top_px).max(1.0) as usize;
    let upload_size = [
        width + ATLAS_GLYPH_PADDING_PX * 2,
        height + ATLAS_GLYPH_PADDING_PX * 2,
    ];

    let mut pixmap = tiny_skia::Pixmap::new(upload_size[0] as u32, upload_size[1] as u32)?;
    let tree = usvg::Tree::from_data(svg.data, &usvg::Options::default()).ok()?;
    let svg_size = tree.size();
    let transform = tiny_skia::Transform::from_row(
        width as f32 / svg_size.width().max(1.0),
        0.0,
        0.0,
        height as f32 / svg_size.height().max(1.0),
        ATLAS_GLYPH_PADDING_PX as f32,
        ATLAS_GLYPH_PADDING_PX as f32,
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(RasterizedGlyph {
        image: ColorImage::from_rgba_premultiplied(upload_size, pixmap.data()),
        logical_size: egui::vec2(
            upload_size[0] as f32 / pixels_per_point,
            upload_size[1] as f32 / pixels_per_point,
        ),
        offset: egui::vec2(
            left_px / pixels_per_point - ATLAS_GLYPH_PADDING_PX as f32 / pixels_per_point,
            top_px / pixels_per_point - ATLAS_GLYPH_PADDING_PX as f32 / pixels_per_point,
        ),
        is_color: true,
    })
}

fn rasterize_bitmap_glyph(
    face: &ttf_parser::Face<'_>,
    glyph_id: ttf_parser::GlyphId,
    size_px: f32,
    pixels_per_point: f32,
) -> Option<RasterizedGlyph> {
    let desired_ppem = (size_px * pixels_per_point)
        .round()
        .clamp(1.0, u16::MAX as f32) as u16;
    let image = face.glyph_raster_image(glyph_id, desired_ppem)?;
    let scale = desired_ppem as f32 / image.pixels_per_em.max(1) as f32;
    let rgba = decode_raster_image(&image)?;
    let upload_size = [
        image.width as usize + ATLAS_GLYPH_PADDING_PX * 2,
        image.height as usize + ATLAS_GLYPH_PADDING_PX * 2,
    ];
    let mut pixels = vec![Color32::TRANSPARENT; upload_size[0] * upload_size[1]];
    for row in 0..image.height as usize {
        let src_start = row * image.width as usize;
        let dst_start = (row + ATLAS_GLYPH_PADDING_PX) * upload_size[0] + ATLAS_GLYPH_PADDING_PX;
        pixels[dst_start..dst_start + image.width as usize]
            .copy_from_slice(&rgba[src_start..src_start + image.width as usize]);
    }

    Some(RasterizedGlyph {
        image: ColorImage::new(upload_size, pixels),
        logical_size: egui::vec2(
            upload_size[0] as f32 * scale / pixels_per_point,
            upload_size[1] as f32 * scale / pixels_per_point,
        ),
        offset: egui::vec2(
            image.x as f32 * scale / pixels_per_point
                - ATLAS_GLYPH_PADDING_PX as f32 * scale / pixels_per_point,
            -(image.y as f32) * scale / pixels_per_point
                - ATLAS_GLYPH_PADDING_PX as f32 * scale / pixels_per_point,
        ),
        is_color: true,
    })
}

fn decode_raster_image(image: &ttf_parser::RasterGlyphImage<'_>) -> Option<Vec<Color32>> {
    match image.format {
        ttf_parser::RasterImageFormat::PNG => {
            let decoded = image::load_from_memory_with_format(image.data, ImageFormat::Png)
                .ok()?
                .to_rgba8();
            Some(
                decoded
                    .pixels()
                    .map(|pixel| {
                        Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])
                    })
                    .collect(),
            )
        }
        ttf_parser::RasterImageFormat::BitmapPremulBgra32 => Some(
            image
                .data
                .chunks_exact(4)
                .map(|pixel| {
                    Color32::from_rgba_premultiplied(pixel[2], pixel[1], pixel[0], pixel[3])
                })
                .collect(),
        ),
        ttf_parser::RasterImageFormat::BitmapGray8 => Some(
            image
                .data
                .iter()
                .map(|alpha| Color32::from_white_alpha(*alpha))
                .collect(),
        ),
        _ => None,
    }
}

fn snap_to_pixel(value: f32, pixels_per_point: f32) -> f32 {
    (value * pixels_per_point).round() / pixels_per_point
}

fn quantize(value: f32) -> u32 {
    (value.max(0.0) * 64.0).round() as u32
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

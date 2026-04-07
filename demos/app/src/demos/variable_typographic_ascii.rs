use std::time::Duration;

use eframe::egui;
use egui::{
    Color32, ColorImage, CornerRadius, Label, Rect, RichText, Sense, Stroke, StrokeKind,
    TextureHandle, TextureOptions,
};
use pretext::advanced::LayoutCursor;
use pretext::{
    BidiDirection, PretextEngine, PretextGlyphRun as LayoutLineGlyphRun,
    PretextParagraphOptions as PrepareOptions, PretextStyle as TextStyleSpec,
};
use pretext_egui::{
    advanced::{paint_styled_positioned_text_runs, StyledPositionedTextRunRef},
    EguiPretextPaintOptions, EguiPretextRenderer,
};
use pretext_render::{BaselineMode, TextRasterRequest, TextRenderCache};

use crate::demos::{format_warmup_status, DemoWarmupStatus, DemoWindow};

const COLS: usize = 50;
const ROWS: usize = 28;
const FONT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 16.0;
const TARGET_ROW_W: f32 = 440.0;
const ART_HEIGHT: f32 = ROWS as f32 * LINE_HEIGHT;
const ART_BOX_PADDING: f32 = 14.0;
const ART_BOX_HEIGHT: f32 = ART_HEIGHT + ART_BOX_PADDING * 2.0;
const PANEL_WIDTH: f32 = 468.0;
const PANEL_GAP: f32 = 28.0;
const PANEL_LABEL_GAP: f32 = 10.0;
const SUBTITLE_MAX_W: f32 = 720.0;

const FIELD_OVERSAMPLE: usize = 2;
const FIELD_COLS: usize = COLS * FIELD_OVERSAMPLE;
const FIELD_ROWS: usize = ROWS * FIELD_OVERSAMPLE;

const CANVAS_W: usize = 220;
const CANVAS_H: usize = 224;
const FIELD_SCALE_X: f32 = FIELD_COLS as f32 / CANVAS_W as f32;
const FIELD_SCALE_Y: f32 = FIELD_ROWS as f32 / CANVAS_H as f32;

const PARTICLE_N: usize = 120;
const SPRITE_R: f32 = 14.0;
const ATTRACTOR_R: f32 = 12.0;
const LARGE_ATTRACTOR_R: f32 = 30.0;
const ATTRACTOR_FORCE_1: f32 = 0.22;
const ATTRACTOR_FORCE_2: f32 = 0.05;
const FIELD_DECAY: f32 = 0.82;
const TARGET_CELL_W: f32 = TARGET_ROW_W / COLS as f32;
const CHARSET: &str =
    " .,:;!+-=*#@%&abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const MONO_RAMP: &str = " .`-_:,;^=+/|)\\!?0oOQ#%@";
const WEIGHTS: [u16; 3] = [300, 500, 800];
const STYLES: [bool; 2] = [false, true];

const TITLE: &str = "Variable Typographic ASCII";
const SUBTITLE: &str = "Proportional font (Georgia) rendered at 3 font-weights × normal/italic — each variant measured by pretext for precise width. A shared particle-and-attractor brightness field drives all three panels, then characters are chosen by brightness AND width to preserve the shape in proportional type.";
const CREDIT: &str = "Made by @somnai_dreams";
const SOURCE_FIELD_LABEL: &str = "Source Field";
const PROPORTIONAL_LABEL: &str = "Proportional × 3 Weights × Italic";
const MONOSPACE_LABEL: &str = "Monospace × Single Weight";
const WINDOW_DEFAULT_WIDTH: f32 = 1080.0;
const WINDOW_DEFAULT_HEIGHT: f32 = 1520.0;

const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const FRAME_DT_SECONDS: f32 = 1.0 / 60.0;
const FRAME_DT_MILLIS: f32 = 1000.0 / 60.0;

const BRIGHTNESS_CANVAS_SIZE: f32 = 28.0;
const BRIGHTNESS_CANVAS_PIXELS: usize = BRIGHTNESS_CANVAS_SIZE as usize;
const PROP_COLOR_RGB: [u8; 3] = [196, 163, 90];
const WINDOW_BG_TOP: Color32 = Color32::from_rgb(10, 10, 18);
const WINDOW_BG_BOTTOM: Color32 = Color32::from_rgb(6, 6, 10);
const WINDOW_STROKE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 18);
const PALETTE_WARMUP_BATCH: usize = 12;

pub struct VariableTypographicAsciiDemo {
    open: bool,
    particles: Vec<Particle>,
    rng: DeterministicRng,
    last_time: Option<f64>,
    frame_accumulator: f32,
    simulation_time_ms: f32,
    source_pixels: Vec<f32>,
    brightness_field: Vec<f32>,
    stamps: Stamps,
    palette: Option<PaletteCache>,
    palette_builder: Option<PaletteWarmupState>,
    palette_engine_revision: Option<u64>,
    rows: Vec<RowRender>,
    source_texture: Option<SizedTexture>,
}

struct PaletteWarmupState {
    engine_revision: u64,
    raster_cache: TextRenderCache,
    variants: Vec<VariantStyle>,
    chars: Vec<char>,
    mono_chars: Vec<char>,
    variant_index: usize,
    char_index: usize,
    mono_index: usize,
    entries: Vec<PaletteEntry>,
    mono_entries: Vec<MonoPaletteEntry>,
    prop_space_width: f32,
    mono_style: TextStyleSpec,
    mono_cell_width: f32,
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

#[derive(Clone, Copy, Debug)]
struct Attractor {
    x: f32,
    y: f32,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct VariantStyle {
    weight: u16,
    italic: bool,
    style: TextStyleSpec,
}

#[derive(Clone, Debug)]
struct PaletteEntry {
    text: String,
    variant_index: usize,
    width: f32,
    brightness: f32,
    glyph_runs: Vec<LayoutLineGlyphRun>,
}

#[derive(Clone, Copy, Debug, Default)]
struct BrightnessEntry {
    mono_char: char,
    mono_palette_index: u8,
    prop: PropLookup,
}

#[derive(Clone, Copy, Debug, Default)]
enum PropLookup {
    #[default]
    Blank,
    Glyph {
        palette_index: usize,
        alpha_step: u8,
    },
}

#[derive(Clone, Debug)]
struct MonoPaletteEntry {
    ch: char,
    text: String,
    glyph_runs: Vec<LayoutLineGlyphRun>,
}

#[derive(Clone, Debug)]
struct PaletteCache {
    variants: Vec<VariantStyle>,
    entries: Vec<PaletteEntry>,
    mono_entries: Vec<MonoPaletteEntry>,
    lookup: [BrightnessEntry; 256],
    prop_space_width: f32,
    mono_style: TextStyleSpec,
    mono_cell_width: f32,
    mono_row_width: f32,
}

#[derive(Clone, Copy, Debug)]
struct MonoGlyph {
    column: u8,
    x_offset: f32,
    palette_index: usize,
}

#[derive(Clone, Copy, Debug)]
struct RowGlyph {
    column: u8,
    x_offset: f32,
    palette_index: usize,
    alpha_step: u8,
}

#[derive(Clone, Debug)]
struct RowRender {
    initialized: bool,
    brightness_bytes: [u8; COLS],
    cells: [BrightnessEntry; COLS],
    mono_bytes: [u8; COLS],
    mono_glyphs: Vec<MonoGlyph>,
    prop_glyphs: Vec<RowGlyph>,
    prop_prefix_widths: [f32; COLS + 1],
    prop_width: f32,
}

impl Default for RowRender {
    fn default() -> Self {
        Self {
            initialized: false,
            brightness_bytes: [0; COLS],
            cells: [BrightnessEntry::default(); COLS],
            mono_bytes: [b' '; COLS],
            mono_glyphs: Vec::with_capacity(COLS),
            prop_glyphs: Vec::with_capacity(COLS),
            prop_prefix_widths: [0.0; COLS + 1],
            prop_width: 0.0,
        }
    }
}

impl RowRender {
    #[cfg(test)]
    fn mono_text(&self) -> &str {
        std::str::from_utf8(&self.mono_bytes).expect("row mono bytes should stay ASCII")
    }
}

#[derive(Clone, Debug)]
struct Stamp {
    radius_x: i32,
    radius_y: i32,
    size_x: usize,
    values: Vec<f32>,
}

#[derive(Clone, Debug)]
struct Stamps {
    particle_source: Stamp,
    particle_field: Stamp,
    large_attractor_source: Stamp,
    large_attractor_field: Stamp,
    small_attractor_source: Stamp,
    small_attractor_field: Stamp,
}

#[derive(Clone)]
struct SizedTexture {
    size: [usize; 2],
    texture: TextureHandle,
}

#[derive(Clone, Copy, Debug)]
struct DeterministicRng {
    state: u64,
}

#[derive(Clone, Copy, Debug)]
enum PanelKind {
    Source,
    Proportional,
    Monospace,
}

impl Default for VariableTypographicAsciiDemo {
    fn default() -> Self {
        let mut rng = DeterministicRng::new(0x5eed_5eed_cafe_babe);
        let stamps = Stamps::new();
        let mut demo = Self {
            open: false,
            particles: initial_particles(&mut rng),
            rng,
            last_time: None,
            frame_accumulator: 0.0,
            simulation_time_ms: 0.0,
            source_pixels: vec![0.0; CANVAS_W * CANVAS_H],
            brightness_field: vec![0.0; FIELD_COLS * FIELD_ROWS],
            stamps,
            palette: None,
            palette_builder: None,
            palette_engine_revision: None,
            rows: Vec::new(),
            source_texture: None,
        };
        demo.step_simulation();
        demo
    }
}

impl DemoWindow for VariableTypographicAsciiDemo {
    fn id(&self) -> &'static str {
        "variable_typographic_ascii"
    }

    fn title(&self) -> &str {
        TITLE
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.last_time = None;
        }
    }

    fn warmup_status(&self) -> DemoWarmupStatus {
        if self.palette.is_some() {
            return DemoWarmupStatus::ready();
        }

        if let Some(builder) = &self.palette_builder {
            return DemoWarmupStatus::pending(
                palette_warmup_stage_label(builder),
                palette_warmup_completed(builder),
                palette_warmup_total(builder),
            );
        }

        DemoWarmupStatus::pending("palette glyphs", 0, palette_warmup_total_from_static())
    }

    fn warmup_step(
        &mut self,
        _ctx: &egui::Context,
        engine: &PretextEngine,
        _assets: &mut EguiPretextRenderer,
        _budget: Duration,
    ) -> bool {
        self.invalidate_palette_if_needed(engine);
        if self.palette.is_some() {
            return true;
        }

        if self.palette_builder.is_none() {
            self.palette_builder = Some(PaletteWarmupState::new(engine));
        }

        let builder = self
            .palette_builder
            .as_mut()
            .expect("palette warmup builder should exist");
        builder.step(engine, PALETTE_WARMUP_BATCH);

        if builder.is_ready_to_finalize() {
            let builder = self
                .palette_builder
                .take()
                .expect("palette warmup builder should exist");
            self.palette = Some(builder.finalize());
            self.palette_engine_revision = Some(engine.revision());
            self.rows.clear();
        }

        self.palette.is_some()
    }

    fn show_loading(
        &mut self,
        ctx: &egui::Context,
        _engine: &PretextEngine,
        _assets: &mut EguiPretextRenderer,
    ) {
        let mut open = self.open;
        let window_frame = egui::Frame::window(ctx.style().as_ref())
            .fill(WINDOW_BG_TOP)
            .stroke(Stroke::new(1.0, WINDOW_STROKE))
            .corner_radius(CornerRadius::same(18));
        let status = self.warmup_status();
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT))
            .frame(window_frame)
            .show(ctx, |ui| {
                paint_window_background(ui.painter(), ui.max_rect());
                ui.vertical_centered(|ui| {
                    ui.add_space(18.0);
                    ui.heading(self.title());
                    ui.label("Preparing proportional and monospace glyph palettes.");
                    ui.add_space(6.0);
                    ui.monospace(format_warmup_status(status));
                    ui.add_space(12.0);
                    ui.spinner();
                });
            });
        self.open = open;
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let mut open = self.open;
        let window_frame = egui::Frame::window(ctx.style().as_ref())
            .fill(WINDOW_BG_TOP)
            .stroke(Stroke::new(1.0, WINDOW_STROKE))
            .corner_radius(CornerRadius::same(18));
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT))
            .frame(window_frame)
            .show(ctx, |ui| {
                paint_window_background(ui.painter(), ui.max_rect());
                self.invalidate_palette_if_needed(engine);
                #[cfg(test)]
                if self.palette.is_none() {
                    self.palette = Some(build_palette(engine));
                    self.palette_engine_revision = Some(engine.revision());
                }
                self.update(ctx.input(|input| input.time));

                let palette = self
                    .palette
                    .as_ref()
                    .expect("palette should exist before row generation");
                rebuild_rows(&mut self.rows, &self.brightness_field, palette);
                let source_texture = self.ensure_source_texture(ctx);
                let rows = &self.rows;

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(TITLE)
                                .size(24.0)
                                .strong()
                                .color(Color32::from_rgba_premultiplied(255, 255, 255, 230)),
                        );
                        ui.add_space(6.0);
                        ui.add_sized(
                            [ui.available_width().min(SUBTITLE_MAX_W), 0.0],
                            Label::new(
                                RichText::new(SUBTITLE)
                                    .size(13.0)
                                    .color(Color32::from_rgba_premultiplied(255, 255, 255, 77)),
                            )
                            .wrap(),
                        );
                    });
                    ui.add_space(28.0);

                    let available_width = ui.available_width();
                    let columns = panel_columns(available_width);
                    let panel_width = panel_width_for_columns(available_width, columns);
                    let panel_block_height = ART_BOX_HEIGHT + PANEL_LABEL_GAP + 16.0;
                    let panels = [
                        PanelKind::Source,
                        PanelKind::Proportional,
                        PanelKind::Monospace,
                    ];

                    for chunk in panels.chunks(columns) {
                        ui.horizontal(|ui| {
                            let used_width = chunk.len() as f32 * panel_width
                                + (chunk.len().saturating_sub(1) as f32) * PANEL_GAP;
                            ui.add_space(((ui.available_width() - used_width) * 0.5).max(0.0));

                            for (index, panel) in chunk.iter().enumerate() {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(panel_width, panel_block_height),
                                    egui::Layout::top_down(egui::Align::Center),
                                    |ui| {
                                        ui.label(
                                            RichText::new(panel_label(*panel))
                                                .size(10.0)
                                                .strong()
                                                .color(Color32::from_rgba_premultiplied(
                                                    255, 255, 255, 77,
                                                )),
                                        );
                                        ui.add_space(PANEL_LABEL_GAP);
                                        let (art_box_rect, _) = ui.allocate_exact_size(
                                            egui::vec2(panel_width, ART_BOX_HEIGHT),
                                            Sense::hover(),
                                        );
                                        let painter = ui.painter().clone();
                                        let palette = self
                                            .palette
                                            .as_ref()
                                            .expect("palette should exist while painting");
                                        match panel {
                                            PanelKind::Source => paint_source_panel(
                                                &painter,
                                                art_box_rect,
                                                &source_texture,
                                            ),
                                            PanelKind::Proportional => paint_prop_panel(
                                                &painter,
                                                art_box_rect,
                                                &rows,
                                                palette,
                                                ctx,
                                                engine,
                                                assets,
                                            ),
                                            PanelKind::Monospace => paint_mono_panel(
                                                &painter,
                                                art_box_rect,
                                                &rows,
                                                palette,
                                                ctx,
                                                engine,
                                                assets,
                                            ),
                                        }
                                    },
                                );

                                if index + 1 < chunk.len() {
                                    ui.add_space(PANEL_GAP);
                                }
                            }
                        });
                        ui.add_space(PANEL_GAP);
                    }

                    ui.add_space(10.0);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(CREDIT)
                                .size(11.0)
                                .color(Color32::from_rgba_premultiplied(255, 255, 255, 71)),
                        );
                    });
                });

                ctx.request_repaint_after(FRAME_INTERVAL);
            });
        self.open = open;
    }
}

impl VariableTypographicAsciiDemo {
    fn invalidate_palette_if_needed(&mut self, engine: &PretextEngine) {
        let revision = engine.revision();
        if self.palette_engine_revision == Some(revision)
            || self
                .palette_builder
                .as_ref()
                .is_some_and(|builder| builder.engine_revision == revision)
        {
            return;
        }

        self.palette = None;
        self.palette_builder = None;
        self.palette_engine_revision = None;
        self.rows.clear();
    }

    fn update(&mut self, now: f64) {
        let dt = match self.last_time {
            Some(last_time) => (now - last_time).clamp(0.0, 0.25) as f32,
            None => FRAME_DT_SECONDS,
        };
        self.last_time = Some(now);
        self.frame_accumulator = (self.frame_accumulator + dt).min(FRAME_DT_SECONDS * 8.0);

        while self.frame_accumulator >= FRAME_DT_SECONDS {
            self.step_simulation();
            self.frame_accumulator -= FRAME_DT_SECONDS;
        }
    }

    fn step_simulation(&mut self) {
        let attractor_1 = large_attractor(self.simulation_time_ms);
        let attractor_2 = small_attractor(self.simulation_time_ms);

        step_particles(&mut self.particles, &mut self.rng, attractor_1, attractor_2);

        fade_buffer(&mut self.source_pixels, FIELD_DECAY);
        let source_cols = CANVAS_W;
        let source_rows = CANVAS_H;
        for particle in &self.particles {
            splat_scaled_stamp(
                &mut self.source_pixels,
                source_cols,
                source_rows,
                particle.x,
                particle.y,
                1.0,
                1.0,
                &self.stamps.particle_source,
            );
        }
        splat_scaled_stamp(
            &mut self.source_pixels,
            source_cols,
            source_rows,
            attractor_1.x,
            attractor_1.y,
            1.0,
            1.0,
            &self.stamps.large_attractor_source,
        );
        splat_scaled_stamp(
            &mut self.source_pixels,
            source_cols,
            source_rows,
            attractor_2.x,
            attractor_2.y,
            1.0,
            1.0,
            &self.stamps.small_attractor_source,
        );

        fade_buffer(&mut self.brightness_field, FIELD_DECAY);
        let field_cols = FIELD_COLS;
        let field_rows = FIELD_ROWS;
        for particle in &self.particles {
            splat_scaled_stamp(
                &mut self.brightness_field,
                field_cols,
                field_rows,
                particle.x,
                particle.y,
                FIELD_SCALE_X,
                FIELD_SCALE_Y,
                &self.stamps.particle_field,
            );
        }
        splat_scaled_stamp(
            &mut self.brightness_field,
            field_cols,
            field_rows,
            attractor_1.x,
            attractor_1.y,
            FIELD_SCALE_X,
            FIELD_SCALE_Y,
            &self.stamps.large_attractor_field,
        );
        splat_scaled_stamp(
            &mut self.brightness_field,
            field_cols,
            field_rows,
            attractor_2.x,
            attractor_2.y,
            FIELD_SCALE_X,
            FIELD_SCALE_Y,
            &self.stamps.small_attractor_field,
        );

        self.simulation_time_ms += FRAME_DT_MILLIS;
    }

    fn ensure_source_texture(&mut self, ctx: &egui::Context) -> TextureHandle {
        let image = source_color_image(&self.source_pixels);
        if let Some(cached) = &mut self.source_texture {
            if cached.size == [CANVAS_W, CANVAS_H] {
                cached.texture.set(image, TextureOptions::LINEAR);
                return cached.texture.clone();
            }
        }

        let texture = ctx.load_texture(
            "variable-typographic-ascii/source-field",
            image,
            TextureOptions::LINEAR,
        );
        self.source_texture = Some(SizedTexture {
            size: [CANVAS_W, CANVAS_H],
            texture: texture.clone(),
        });
        texture
    }
}

impl PaletteWarmupState {
    fn new(engine: &PretextEngine) -> Self {
        let mono_style = mono_style();
        Self {
            engine_revision: engine.revision(),
            raster_cache: TextRenderCache::default(),
            variants: build_variants(),
            chars: charset_chars(),
            mono_chars: MONO_RAMP.chars().collect(),
            variant_index: 0,
            char_index: 0,
            mono_index: 0,
            entries: Vec::new(),
            mono_entries: Vec::new(),
            prop_space_width: measure_text_width(engine, " ", &prop_base_style()).max(1.0),
            mono_cell_width: measure_char_width(engine, '0', &mono_style).max(1.0),
            mono_style,
        }
    }

    fn step(&mut self, engine: &PretextEngine, budget: usize) {
        let mut processed = 0usize;
        while processed < budget && self.variant_index < self.variants.len() {
            let ch = self.chars[self.char_index];
            let variant = &self.variants[self.variant_index];
            let width = measure_char_width(engine, ch, &variant.style);
            if width > 0.0 {
                let brightness =
                    estimate_brightness(engine, &mut self.raster_cache, ch, &variant.style, width);
                let mut buffer = [0u8; 4];
                let glyph_runs =
                    single_line_glyph_runs(engine, ch.encode_utf8(&mut buffer), &variant.style);
                self.entries.push(PaletteEntry {
                    text: ch.to_string(),
                    variant_index: self.variant_index,
                    width,
                    brightness,
                    glyph_runs,
                });
            }

            self.char_index += 1;
            if self.char_index == self.chars.len() {
                self.char_index = 0;
                self.variant_index += 1;
            }
            processed += 1;
        }

        while processed < budget && self.variant_index >= self.variants.len() && self.mono_index < self.mono_chars.len() {
            let ch = self.mono_chars[self.mono_index];
            let mut buffer = [0u8; 4];
            self.mono_entries.push(MonoPaletteEntry {
                ch,
                text: ch.to_string(),
                glyph_runs: single_line_glyph_runs(
                    engine,
                    ch.encode_utf8(&mut buffer),
                    &self.mono_style,
                ),
            });
            self.mono_index += 1;
            processed += 1;
        }
    }

    fn is_ready_to_finalize(&self) -> bool {
        self.variant_index >= self.variants.len() && self.mono_index >= self.mono_chars.len()
    }

    fn finalize(mut self) -> PaletteCache {
        let max_brightness = self
            .entries
            .iter()
            .map(|entry| entry.brightness)
            .fold(0.0f32, f32::max);
        if max_brightness > 0.0 {
            for entry in &mut self.entries {
                entry.brightness /= max_brightness;
            }
        }
        self.entries
            .sort_by(|left, right| left.brightness.total_cmp(&right.brightness));

        let mono_chars = self
            .mono_entries
            .iter()
            .map(|entry| entry.ch)
            .collect::<Vec<_>>();
        let mut lookup = [BrightnessEntry::default(); 256];
        for brightness_byte in 0..=255u16 {
            let brightness = brightness_byte as f32 / 255.0;
            let mono_index =
                ((brightness * mono_chars.len() as f32).floor() as usize).min(mono_chars.len() - 1);
            let mono_char = mono_chars[mono_index];
            lookup[brightness_byte as usize] = if brightness < 0.03 {
                BrightnessEntry {
                    mono_char,
                    mono_palette_index: mono_index as u8,
                    prop: PropLookup::Blank,
                }
            } else {
                let palette_index = find_best_entry(&self.entries, brightness, TARGET_CELL_W);
                let alpha_step = (brightness * 10.0).round().clamp(1.0, 10.0) as u8;
                BrightnessEntry {
                    mono_char,
                    mono_palette_index: mono_index as u8,
                    prop: PropLookup::Glyph {
                        palette_index,
                        alpha_step,
                    },
                }
            };
        }

        PaletteCache {
            variants: self.variants,
            entries: self.entries,
            mono_entries: self.mono_entries,
            lookup,
            prop_space_width: self.prop_space_width,
            mono_style: self.mono_style,
            mono_cell_width: self.mono_cell_width,
            mono_row_width: self.mono_cell_width * COLS as f32,
        }
    }
}

fn charset_chars() -> Vec<char> {
    CHARSET.chars().filter(|ch| *ch != ' ').collect()
}

fn palette_warmup_total(builder: &PaletteWarmupState) -> usize {
    builder.variants.len() * builder.chars.len() + builder.mono_chars.len() + 1
}

fn palette_warmup_total_from_static() -> usize {
    build_variants().len() * charset_chars().len() + MONO_RAMP.chars().count() + 1
}

fn palette_warmup_completed(builder: &PaletteWarmupState) -> usize {
    let variant_work = builder.variant_index * builder.chars.len() + builder.char_index;
    let mono_work = builder.mono_index;
    let finalize_work = usize::from(builder.is_ready_to_finalize());
    variant_work + mono_work + finalize_work
}

fn palette_warmup_stage_label(builder: &PaletteWarmupState) -> &'static str {
    if builder.variant_index < builder.variants.len() {
        "palette glyphs"
    } else if builder.mono_index < builder.mono_chars.len() {
        "mono glyphs"
    } else {
        "lookup table"
    }
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_f32(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let bits = (self.state >> 40) as u32;
        bits as f32 / (1u32 << 24) as f32
    }
}

impl Stamps {
    fn new() -> Self {
        Self {
            particle_source: create_stamp(SPRITE_R, 1.0, 1.0),
            particle_field: create_stamp(SPRITE_R, FIELD_SCALE_X, FIELD_SCALE_Y),
            large_attractor_source: create_stamp(LARGE_ATTRACTOR_R, 1.0, 1.0),
            large_attractor_field: create_stamp(LARGE_ATTRACTOR_R, FIELD_SCALE_X, FIELD_SCALE_Y),
            small_attractor_source: create_stamp(ATTRACTOR_R, 1.0, 1.0),
            small_attractor_field: create_stamp(ATTRACTOR_R, FIELD_SCALE_X, FIELD_SCALE_Y),
        }
    }
}

#[cfg(test)]
fn build_palette(engine: &PretextEngine) -> PaletteCache {
    let mut raster_cache = TextRenderCache::default();
    let prop_space_width = measure_text_width(engine, " ", &prop_base_style()).max(1.0);
    let variants = build_variants();
    let mono_style = mono_style();
    let mut entries = Vec::new();

    for (variant_index, variant) in variants.iter().enumerate() {
        for ch in CHARSET.chars() {
            if ch == ' ' {
                continue;
            }

            let width = measure_char_width(engine, ch, &variant.style);
            if width <= 0.0 {
                continue;
            }

            let brightness =
                estimate_brightness(engine, &mut raster_cache, ch, &variant.style, width);
            let mut buffer = [0u8; 4];
            let glyph_runs =
                single_line_glyph_runs(engine, ch.encode_utf8(&mut buffer), &variant.style);
            entries.push(PaletteEntry {
                text: ch.to_string(),
                variant_index,
                width,
                brightness,
                glyph_runs,
            });
        }
    }

    let max_brightness = entries
        .iter()
        .map(|entry| entry.brightness)
        .fold(0.0f32, f32::max);
    if max_brightness > 0.0 {
        for entry in &mut entries {
            entry.brightness /= max_brightness;
        }
    }
    entries.sort_by(|left, right| left.brightness.total_cmp(&right.brightness));

    let mono_cell_width = measure_char_width(engine, '0', &mono_style).max(1.0);
    let mono_entries = MONO_RAMP
        .chars()
        .map(|ch| {
            let mut buffer = [0u8; 4];
            MonoPaletteEntry {
                ch,
                text: ch.to_string(),
                glyph_runs: single_line_glyph_runs(
                    engine,
                    ch.encode_utf8(&mut buffer),
                    &mono_style,
                ),
            }
        })
        .collect::<Vec<_>>();
    let mut lookup = [BrightnessEntry::default(); 256];
    let mono_chars = mono_entries
        .iter()
        .map(|entry| entry.ch)
        .collect::<Vec<_>>();

    for brightness_byte in 0..=255u16 {
        let brightness = brightness_byte as f32 / 255.0;
        let mono_index =
            ((brightness * mono_chars.len() as f32).floor() as usize).min(mono_chars.len() - 1);
        let mono_char = mono_chars[mono_index];

        lookup[brightness_byte as usize] = if brightness < 0.03 {
            BrightnessEntry {
                mono_char,
                mono_palette_index: mono_index as u8,
                prop: PropLookup::Blank,
            }
        } else {
            let palette_index = find_best_entry(&entries, brightness, TARGET_CELL_W);
            let alpha_step = (brightness * 10.0).round().clamp(1.0, 10.0) as u8;
            BrightnessEntry {
                mono_char,
                mono_palette_index: mono_index as u8,
                prop: PropLookup::Glyph {
                    palette_index,
                    alpha_step,
                },
            }
        };
    }

    PaletteCache {
        variants,
        entries,
        mono_entries,
        lookup,
        prop_space_width,
        mono_style,
        mono_cell_width,
        mono_row_width: mono_cell_width * COLS as f32,
    }
}

fn build_variants() -> Vec<VariantStyle> {
    let mut variants = Vec::with_capacity(STYLES.len() * WEIGHTS.len());
    for italic in STYLES {
        for weight in WEIGHTS {
            variants.push(VariantStyle {
                weight,
                italic,
                style: prop_style(weight, italic),
            });
        }
    }
    variants
}

fn rebuild_rows(rows: &mut Vec<RowRender>, field: &[f32], palette: &PaletteCache) {
    if rows.len() < ROWS {
        rows.resize_with(ROWS, RowRender::default);
    } else {
        rows.truncate(ROWS);
    }

    for (row, row_render) in rows.iter_mut().enumerate() {
        let field_row_start = row * FIELD_OVERSAMPLE * FIELD_COLS;
        let mut first_changed_col = if row_render.initialized {
            None
        } else {
            Some(0)
        };

        for col in 0..COLS {
            let brightness_byte = sample_cell_brightness_byte(field, field_row_start, col);
            if !row_render.initialized || brightness_byte != row_render.brightness_bytes[col] {
                row_render.brightness_bytes[col] = brightness_byte;
                let lookup = palette.lookup[brightness_byte as usize];
                row_render.cells[col] = lookup;
                row_render.mono_bytes[col] = lookup.mono_char as u8;
                if row_render.initialized && first_changed_col.is_none() {
                    first_changed_col = Some(col);
                }
            }
        }

        if let Some(start_col) = first_changed_col {
            rebuild_row_suffix(row_render, start_col, palette);
            row_render.initialized = true;
        }
    }
}

fn sample_cell_brightness_byte(field: &[f32], field_row_start: usize, col: usize) -> u8 {
    let field_col_start = col * FIELD_OVERSAMPLE;
    let mut brightness = 0.0;
    for sample_y in 0..FIELD_OVERSAMPLE {
        let sample_row_offset = field_row_start + sample_y * FIELD_COLS + field_col_start;
        for sample_x in 0..FIELD_OVERSAMPLE {
            brightness += field[sample_row_offset + sample_x];
        }
    }

    let average = brightness / (FIELD_OVERSAMPLE * FIELD_OVERSAMPLE) as f32;
    (average * 255.0).floor().clamp(0.0, 255.0) as u8
}

fn rebuild_row_suffix(row_render: &mut RowRender, start_col: usize, palette: &PaletteCache) {
    if start_col == 0 {
        row_render.mono_glyphs.clear();
        row_render.prop_glyphs.clear();
        row_render.prop_prefix_widths[0] = 0.0;
    } else {
        row_render
            .mono_glyphs
            .retain(|glyph| (glyph.column as usize) < start_col);
        row_render
            .prop_glyphs
            .retain(|glyph| (glyph.column as usize) < start_col);
    }

    let mut mono_x = start_col as f32 * palette.mono_cell_width;
    let mut prop_width = row_render.prop_prefix_widths[start_col];

    for col in start_col..COLS {
        let lookup = row_render.cells[col];
        let mono_palette_index = lookup.mono_palette_index as usize;
        if !palette.mono_entries[mono_palette_index]
            .glyph_runs
            .is_empty()
        {
            row_render.mono_glyphs.push(MonoGlyph {
                column: col as u8,
                x_offset: mono_x,
                palette_index: mono_palette_index,
            });
        }
        mono_x += palette.mono_cell_width;

        row_render.prop_prefix_widths[col] = prop_width;
        match lookup.prop {
            PropLookup::Blank => {
                prop_width += palette.prop_space_width;
            }
            PropLookup::Glyph {
                palette_index,
                alpha_step,
            } => {
                row_render.prop_glyphs.push(RowGlyph {
                    column: col as u8,
                    x_offset: prop_width,
                    palette_index,
                    alpha_step,
                });
                prop_width += palette.entries[palette_index].width;
            }
        }
        row_render.prop_prefix_widths[col + 1] = prop_width;
    }

    row_render.prop_width = prop_width;
}

#[cfg(test)]
fn build_rows(field: &[f32], palette: &PaletteCache) -> Vec<RowRender> {
    let mut rows = Vec::new();
    rebuild_rows(&mut rows, field, palette);
    rows
}

fn prop_style(weight: u16, italic: bool) -> TextStyleSpec {
    TextStyleSpec {
        families: prop_families(),
        size_px: FONT_SIZE,
        weight,
        italic,
    }
}

fn prop_base_style() -> TextStyleSpec {
    prop_style(400, false)
}

fn mono_style() -> TextStyleSpec {
    TextStyleSpec {
        families: mono_families(),
        size_px: FONT_SIZE,
        weight: 400,
        italic: false,
    }
}

fn prop_families() -> Vec<String> {
    vec![
        "Georgia".to_owned(),
        "Palatino".to_owned(),
        "Times New Roman".to_owned(),
        "Noto Serif".to_owned(),
        "Noto Sans".to_owned(),
    ]
}

fn mono_families() -> Vec<String> {
    vec![
        "Courier New".to_owned(),
        "Courier".to_owned(),
        "Noto Sans Mono".to_owned(),
        "Noto Sans".to_owned(),
    ]
}

fn measure_char_width(engine: &PretextEngine, ch: char, style: &TextStyleSpec) -> f32 {
    let mut buffer = [0u8; 4];
    measure_text_width(engine, ch.encode_utf8(&mut buffer), style)
}

fn measure_text_width(engine: &PretextEngine, text: &str, style: &TextStyleSpec) -> f32 {
    engine
        .shape_text_spans_shared(text, style, BidiDirection::Ltr)
        .iter()
        .map(|span| span.width)
        .sum()
}

fn single_line_glyph_runs(
    engine: &PretextEngine,
    text: &str,
    style: &TextStyleSpec,
) -> Vec<LayoutLineGlyphRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let prepared = engine.prepare_paragraph(text, style, &PrepareOptions::default());
    let mut cursor = LayoutCursor::default();
    engine
        .layout_next_line_with_glyph_runs(&prepared, &mut cursor, 100_000.0)
        .map(|line| line.glyph_runs)
        .unwrap_or_default()
}

fn estimate_brightness(
    engine: &PretextEngine,
    raster_cache: &mut TextRenderCache,
    ch: char,
    style: &TextStyleSpec,
    width: f32,
) -> f32 {
    let mut buffer = [0u8; 4];
    let text = ch.encode_utf8(&mut buffer);
    let request = TextRasterRequest {
        text,
        style,
        direction: BidiDirection::Ltr,
        slot_height: BRIGHTNESS_CANVAS_SIZE,
        padding_x: 1.0,
        padding_y: 0.0,
        slack_x: (BRIGHTNESS_CANVAS_SIZE - width.ceil() - 2.0).max(0.0),
        slack_y: 0.0,
        baseline_mode: BaselineMode::AutoFontMetrics,
    };
    let Some(rasterized) = raster_cache.rasterized_text(engine, request, 1.0) else {
        return 0.0;
    };

    let size = rasterized.pixel_size();
    let width_px = size[0].min(BRIGHTNESS_CANVAS_PIXELS);
    let height_px = size[1].min(BRIGHTNESS_CANVAS_PIXELS);
    let alpha_pixels = rasterized.alpha_pixels();
    let mut sum = 0u32;
    for y in 0..height_px {
        let row_offset = y * size[0];
        for x in 0..width_px {
            sum += alpha_pixels[row_offset + x] as u32;
        }
    }

    sum as f32 / (255.0 * BRIGHTNESS_CANVAS_SIZE * BRIGHTNESS_CANVAS_SIZE)
}

fn find_best_entry(entries: &[PaletteEntry], target_brightness: f32, target_width: f32) -> usize {
    let mut lo = 0usize;
    let mut hi = entries.len().saturating_sub(1);
    while lo < hi {
        let mid = (lo + hi) >> 1;
        if entries[mid].brightness < target_brightness {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    let start = lo.saturating_sub(15);
    let end = (lo + 15).min(entries.len());
    let mut best_index = lo;
    let mut best_score = f32::INFINITY;

    for index in start..end {
        let entry = &entries[index];
        let brightness_error = (entry.brightness - target_brightness).abs() * 2.5;
        let width_error = (entry.width - target_width).abs() / target_width;
        let score = brightness_error + width_error;
        if score < best_score {
            best_score = score;
            best_index = index;
        }
    }

    best_index
}

fn initial_particles(rng: &mut DeterministicRng) -> Vec<Particle> {
    let center_x = CANVAS_W as f32 * 0.5;
    let center_y = CANVAS_H as f32 * 0.5;
    (0..PARTICLE_N)
        .map(|_| {
            let angle = rng.next_f32() * std::f32::consts::TAU;
            let radius = rng.next_f32() * 40.0 + 20.0;
            Particle {
                x: center_x + angle.cos() * radius,
                y: center_y + angle.sin() * radius,
                vx: (rng.next_f32() - 0.5) * 0.8,
                vy: (rng.next_f32() - 0.5) * 0.8,
            }
        })
        .collect()
}

fn large_attractor(time_ms: f32) -> Attractor {
    Attractor {
        x: (time_ms * 0.0007).cos() * CANVAS_W as f32 * 0.25 + CANVAS_W as f32 * 0.5,
        y: (time_ms * 0.0011).sin() * CANVAS_H as f32 * 0.3 + CANVAS_H as f32 * 0.5,
    }
}

fn small_attractor(time_ms: f32) -> Attractor {
    Attractor {
        x: (time_ms * 0.0013 + std::f32::consts::PI).cos() * CANVAS_W as f32 * 0.2
            + CANVAS_W as f32 * 0.5,
        y: (time_ms * 0.0009 + std::f32::consts::PI).sin() * CANVAS_H as f32 * 0.25
            + CANVAS_H as f32 * 0.5,
    }
}

fn step_particles(
    particles: &mut [Particle],
    rng: &mut DeterministicRng,
    attractor_1: Attractor,
    attractor_2: Attractor,
) {
    for particle in particles {
        let d1x = attractor_1.x - particle.x;
        let d1y = attractor_1.y - particle.y;
        let d2x = attractor_2.x - particle.x;
        let d2y = attractor_2.y - particle.y;
        let dist1 = d1x * d1x + d1y * d1y;
        let dist2 = d2x * d2x + d2y * d2y;
        let (ax, ay, force, dist_sq) = if dist1 < dist2 {
            (d1x, d1y, ATTRACTOR_FORCE_1, dist1)
        } else {
            (d2x, d2y, ATTRACTOR_FORCE_2, dist2)
        };
        let dist = dist_sq.sqrt() + 1.0;

        particle.vx += ax / dist * force;
        particle.vy += ay / dist * force;
        particle.vx += (rng.next_f32() - 0.5) * 0.25;
        particle.vy += (rng.next_f32() - 0.5) * 0.25;
        particle.vx *= 0.97;
        particle.vy *= 0.97;
        particle.x += particle.vx;
        particle.y += particle.vy;

        if particle.x < -SPRITE_R {
            particle.x += CANVAS_W as f32 + SPRITE_R * 2.0;
        }
        if particle.x > CANVAS_W as f32 + SPRITE_R {
            particle.x -= CANVAS_W as f32 + SPRITE_R * 2.0;
        }
        if particle.y < -SPRITE_R {
            particle.y += CANVAS_H as f32 + SPRITE_R * 2.0;
        }
        if particle.y > CANVAS_H as f32 + SPRITE_R {
            particle.y -= CANVAS_H as f32 + SPRITE_R * 2.0;
        }
    }
}

fn fade_buffer(buffer: &mut [f32], factor: f32) {
    for value in buffer {
        *value *= factor;
    }
}

fn create_stamp(radius_px: f32, scale_x: f32, scale_y: f32) -> Stamp {
    let field_radius_x = radius_px * scale_x;
    let field_radius_y = radius_px * scale_y;
    let radius_x = field_radius_x.ceil() as i32;
    let radius_y = field_radius_y.ceil() as i32;
    let size_x = (radius_x * 2 + 1) as usize;
    let size_y = (radius_y * 2 + 1) as usize;
    let mut values = vec![0.0; size_x * size_y];

    for y in -radius_y..=radius_y {
        for x in -radius_x..=radius_x {
            let normalized_distance =
                ((x as f32 / field_radius_x).powi(2) + (y as f32 / field_radius_y).powi(2)).sqrt();
            values[(y + radius_y) as usize * size_x + (x + radius_x) as usize] =
                sprite_alpha_at(normalized_distance);
        }
    }

    Stamp {
        radius_x,
        radius_y,
        size_x,
        values,
    }
}

fn sprite_alpha_at(normalized_distance: f32) -> f32 {
    if normalized_distance >= 1.0 {
        return 0.0;
    }
    if normalized_distance <= 0.35 {
        return 0.45 + (0.15 - 0.45) * (normalized_distance / 0.35);
    }
    0.15 * (1.0 - (normalized_distance - 0.35) / 0.65)
}

fn splat_scaled_stamp(
    field: &mut [f32],
    cols: usize,
    rows: usize,
    center_x: f32,
    center_y: f32,
    scale_x: f32,
    scale_y: f32,
    stamp: &Stamp,
) {
    let grid_center_x = (center_x * scale_x).round() as i32;
    let grid_center_y = (center_y * scale_y).round() as i32;

    for y in -stamp.radius_y..=stamp.radius_y {
        let grid_y = grid_center_y + y;
        if !(0..rows as i32).contains(&grid_y) {
            continue;
        }
        let field_row_offset = grid_y as usize * cols;
        let stamp_row_offset = (y + stamp.radius_y) as usize * stamp.size_x;
        for x in -stamp.radius_x..=stamp.radius_x {
            let grid_x = grid_center_x + x;
            if !(0..cols as i32).contains(&grid_x) {
                continue;
            }
            let stamp_value = stamp.values[stamp_row_offset + (x + stamp.radius_x) as usize];
            if stamp_value == 0.0 {
                continue;
            }
            let field_index = field_row_offset + grid_x as usize;
            field[field_index] = (field[field_index] + stamp_value).min(1.0);
        }
    }
}

fn panel_columns(available_width: f32) -> usize {
    if available_width >= PANEL_WIDTH * 3.0 + PANEL_GAP * 2.0 {
        3
    } else if available_width >= PANEL_WIDTH * 2.0 + PANEL_GAP {
        2
    } else {
        1
    }
}

fn panel_width_for_columns(available_width: f32, columns: usize) -> f32 {
    let gaps = PANEL_GAP * columns.saturating_sub(1) as f32;
    PANEL_WIDTH.min((available_width - gaps).max(220.0) / columns as f32)
}

fn panel_label(kind: PanelKind) -> &'static str {
    match kind {
        PanelKind::Source => SOURCE_FIELD_LABEL,
        PanelKind::Proportional => PROPORTIONAL_LABEL,
        PanelKind::Monospace => MONOSPACE_LABEL,
    }
}

fn paint_source_panel(painter: &egui::Painter, rect: Rect, texture: &TextureHandle) {
    paint_panel_box(painter, rect);
    let content_rect = panel_content_rect(rect);
    let image_rect = contain_rect(content_rect, egui::vec2(CANVAS_W as f32, CANVAS_H as f32));
    painter.image(
        texture.id(),
        image_rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

fn paint_prop_panel(
    painter: &egui::Painter,
    rect: Rect,
    rows: &[RowRender],
    palette: &PaletteCache,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) {
    paint_panel_box(painter, rect);
    let content_rect = panel_content_rect(rect);
    let clipped = painter.with_clip_rect(rect);
    let _ = paint_styled_positioned_text_runs(
        &clipped,
        rows.iter().enumerate().flat_map(|(row_index, row)| {
            let row_y = content_rect.top() + row_index as f32 * LINE_HEIGHT;
            let start_x = content_rect.center().x - row.prop_width * 0.5;
            row.prop_glyphs.iter().map(move |glyph| {
                let entry = &palette.entries[glyph.palette_index];
                let variant = &palette.variants[entry.variant_index];
                StyledPositionedTextRunRef {
                    x: start_x + glyph.x_offset,
                    y: row_y,
                    text: &entry.text,
                    glyph_runs: &entry.glyph_runs,
                    emoji_overlays: &[],
                    options: EguiPretextPaintOptions::new(&variant.style, LINE_HEIGHT)
                        .color(prop_color(glyph.alpha_step))
                        .fallback_font(egui::FontId::new(
                            variant.style.size_px,
                            egui::FontFamily::Proportional,
                        ))
                        .fallback_align(egui::Align2::LEFT_TOP),
                }
            })
        }),
        ctx,
        engine,
        assets,
    );
}

fn paint_mono_panel(
    painter: &egui::Painter,
    rect: Rect,
    rows: &[RowRender],
    palette: &PaletteCache,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) {
    paint_panel_box(painter, rect);
    let content_rect = panel_content_rect(rect);
    let clipped = painter.with_clip_rect(rect);
    let start_x = content_rect.center().x - palette.mono_row_width * 0.5;
    let mono_color = Color32::from_rgba_premultiplied(130, 155, 210, 179);
    let _ = paint_styled_positioned_text_runs(
        &clipped,
        rows.iter().enumerate().flat_map(|(row_index, row)| {
            let row_y = content_rect.top() + row_index as f32 * LINE_HEIGHT;
            row.mono_glyphs.iter().map(move |glyph| {
                let entry = &palette.mono_entries[glyph.palette_index];
                StyledPositionedTextRunRef {
                    x: start_x + glyph.x_offset,
                    y: row_y,
                    text: &entry.text,
                    glyph_runs: &entry.glyph_runs,
                    emoji_overlays: &[],
                    options: EguiPretextPaintOptions::new(&palette.mono_style, LINE_HEIGHT)
                        .color(mono_color)
                        .fallback_font(egui::FontId::new(
                            palette.mono_style.size_px,
                            egui::FontFamily::Monospace,
                        ))
                        .fallback_align(egui::Align2::LEFT_TOP),
                }
            })
        }),
        ctx,
        engine,
        assets,
    );
}

fn paint_panel_box(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(
        rect,
        CornerRadius::same(10),
        Color32::from_rgba_premultiplied(0, 0, 0, 102),
    );
    painter.rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 13)),
        StrokeKind::Inside,
    );
}

fn paint_window_background(painter: &egui::Painter, rect: Rect) {
    let mut mesh = egui::epaint::Mesh::default();
    let base = mesh.vertices.len() as u32;
    mesh.colored_vertex(rect.left_top(), WINDOW_BG_TOP);
    mesh.colored_vertex(rect.right_top(), WINDOW_BG_TOP);
    mesh.colored_vertex(rect.right_bottom(), WINDOW_BG_BOTTOM);
    mesh.colored_vertex(rect.left_bottom(), WINDOW_BG_BOTTOM);
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base, base + 2, base + 3);
    painter.add(egui::Shape::mesh(mesh));
}

fn panel_content_rect(rect: Rect) -> Rect {
    rect.shrink2(egui::vec2(ART_BOX_PADDING, ART_BOX_PADDING))
}

fn contain_rect(bounds: Rect, content_size: egui::Vec2) -> Rect {
    let scale = (bounds.width() / content_size.x)
        .min(bounds.height() / content_size.y)
        .max(0.0);
    let size = content_size * scale;
    Rect::from_center_size(bounds.center(), size)
}

fn prop_color(alpha_step: u8) -> Color32 {
    let alpha = ((alpha_step as f32 / 10.0) * 255.0).round() as u8;
    Color32::from_rgba_premultiplied(
        PROP_COLOR_RGB[0],
        PROP_COLOR_RGB[1],
        PROP_COLOR_RGB[2],
        alpha,
    )
}

fn source_color_image(source_pixels: &[f32]) -> ColorImage {
    let pixels = source_pixels
        .iter()
        .map(|value| {
            let luminance = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
            Color32::from_rgb(luminance, luminance, luminance)
        })
        .collect();
    ColorImage::new([CANVAS_W, CANVAS_H], pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demos::DemoWindow;

    fn normalize_whitespace(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn engine() -> PretextEngine {
        PretextEngine::builder()
            .with_font_data(pretext_egui::experimental::demo_assets::bundled_font_data())
            .include_system_fonts(false)
            .build()
    }

    fn particles_close(left: &[Particle], right: &[Particle]) {
        assert_eq!(left.len(), right.len());
        for (left, right) in left.iter().zip(right.iter()) {
            assert!((left.x - right.x).abs() < 0.0001);
            assert!((left.y - right.y).abs() < 0.0001);
            assert!((left.vx - right.vx).abs() < 0.0001);
            assert!((left.vy - right.vy).abs() < 0.0001);
        }
    }

    #[test]
    fn particle_step_is_deterministic() {
        let mut rng_a = DeterministicRng::new(7);
        let mut rng_b = DeterministicRng::new(7);
        let mut particles_a = initial_particles(&mut rng_a);
        let mut particles_b = initial_particles(&mut rng_b);
        let attractor_1 = large_attractor(250.0);
        let attractor_2 = small_attractor(250.0);

        step_particles(&mut particles_a, &mut rng_a, attractor_1, attractor_2);
        step_particles(&mut particles_b, &mut rng_b, attractor_1, attractor_2);

        particles_close(&particles_a, &particles_b);
    }

    #[test]
    fn blank_field_renders_expected_grid_dimensions() {
        let palette = build_palette(&engine());
        let rows = build_rows(&vec![0.0; FIELD_COLS * FIELD_ROWS], &palette);

        assert_eq!(rows.len(), ROWS);
        assert!(matches!(palette.lookup[0].prop, PropLookup::Blank));
        assert!(matches!(palette.lookup[255].prop, PropLookup::Glyph { .. }));
        for row in &rows {
            assert_eq!(row.mono_text().chars().count(), COLS);
            assert!(row.mono_glyphs.is_empty());
            assert!(row.prop_glyphs.is_empty());
            assert!((row.prop_width - palette.prop_space_width * COLS as f32).abs() < 0.001);
        }
    }

    #[test]
    fn rebuild_rows_does_not_reenter_text_layout() {
        let engine = engine();
        let palette = build_palette(&engine);
        let before = engine.runtime_stats();
        let mut rows = Vec::new();

        rebuild_rows(&mut rows, &vec![0.5; FIELD_COLS * FIELD_ROWS], &palette);

        let after = engine.runtime_stats();
        assert_eq!(
            after.prepare_with_segments_calls,
            before.prepare_with_segments_calls
        );
        assert_eq!(
            after.layout_with_lines_calls,
            before.layout_with_lines_calls
        );
        assert_eq!(after.line_glyph_runs_calls, before.line_glyph_runs_calls);
    }

    #[test]
    fn variable_ascii_render_uses_atlas_without_shaped_text_textures() {
        let ctx = egui::Context::default();
        let engine = engine();
        let mut assets = EguiPretextRenderer::default();
        pretext_egui::experimental::demo_assets::install_demo_fonts(&ctx);
        let mut demo = VariableTypographicAsciiDemo::default();
        demo.set_open(true);

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            demo.show(ctx, &engine, &mut assets);
        });
        let stats = assets.stats();

        assert!(stats.atlas_entries > 0);
        assert_eq!(stats.shaped_text_textures, 0);
    }

    #[test]
    fn js_source_constants_match_rust() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../pretext_js/pages/demos/variable-typographic-ascii.ts");
        let source = std::fs::read_to_string(path).expect("variable-typographic-ascii.ts source");

        assert!(source.contains("const COLS = 50"));
        assert!(source.contains("const ROWS = 28"));
        assert!(source.contains("const FONT_SIZE = 14"));
        assert!(source.contains("const LINE_HEIGHT = 16"));
        assert!(source.contains("const TARGET_ROW_W = 440"));
        assert!(source.contains(r#"const CHARSET = ' .,:;!+-=*#@%&abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'"#));
        assert!(source.contains(r#"const MONO_RAMP = ' .`-_:,;^=+/|)\\!?0oOQ#%@'"#));
        assert!(source.contains("const WEIGHTS = [300, 500, 800] as const"));
        assert!(source.contains("const STYLES = ['normal', 'italic'] as const"));
    }

    #[test]
    fn js_html_copy_matches_rust() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../pretext_js/pages/demos/variable-typographic-ascii.html");
        let source = std::fs::read_to_string(path).expect("variable-typographic-ascii.html source");
        let normalized = normalize_whitespace(&source);

        assert!(normalized.contains(&format!("<h1>{TITLE}</h1>")));
        assert!(normalized.contains(&normalize_whitespace(SUBTITLE)));
        assert!(normalized.contains(&format!(
            r#"<div class="panel-label">{SOURCE_FIELD_LABEL}</div>"#
        )));
        assert!(normalized.contains(&format!(
            r#"<div class="panel-label">{PROPORTIONAL_LABEL}</div>"#
        )));
        assert!(normalized.contains(&format!(
            r#"<div class="panel-label">{MONOSPACE_LABEL}</div>"#
        )));
        assert!(normalized.contains(CREDIT));
    }

    #[test]
    fn palette_builds_all_weight_and_style_variants() {
        let palette = build_palette(&engine());

        assert_eq!(palette.variants.len(), WEIGHTS.len() * STYLES.len());
        assert!(palette
            .variants
            .iter()
            .any(|variant| variant.weight == 300 && !variant.italic));
        assert!(palette
            .variants
            .iter()
            .any(|variant| variant.weight == 800 && variant.italic));
        assert!(palette.entries.len() > CHARSET.chars().count());
        assert!(palette.mono_row_width > 0.0);
    }
}

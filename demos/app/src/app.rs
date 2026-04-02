use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;
use pretext::{
    EngineRuntimeStats, ParagraphDirection, PrepareOptions, PretextEngine, WhiteSpaceMode,
};
use pretext_egui::{AssetRegistry, AssetRegistryStats};

use crate::demos::{self, DemoWindow};

pub struct PretextDemoApp {
    engine: PretextEngine,
    assets: AssetRegistry,
    demos: Vec<Box<dyn DemoWindow>>,
    sample_line_count: usize,
    root_viewport_activation_pending: bool,
    system_engine_rx: Option<Receiver<PretextEngine>>,
    system_fonts_ready: bool,
    last_interaction_at: Instant,
    perf_hud_visible: bool,
    perf_hud: PerfHudState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AssetRegistryFrameStats {
    texture_cache_hits: u64,
    texture_cache_misses: u64,
    texture_uploads: u64,
    texture_upload_bytes: u64,
    render_cache_hits: u64,
    render_cache_misses: u64,
    rasterizations: u64,
    glyph_path_hits: u64,
    glyph_path_misses: u64,
    static_svg_textures: usize,
    shaped_text_textures: usize,
    raster_cache_entries: usize,
    glyph_path_entries: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct PerfHudState {
    frame_time_ms_ema: f32,
    last_engine_totals: EngineRuntimeStats,
    last_engine_frame: EngineRuntimeStats,
    last_asset_totals: AssetRegistryStats,
    last_asset_frame: AssetRegistryFrameStats,
}

impl PretextDemoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = Instant::now();
        let mut assets = AssetRegistry::default();
        assets.install_fonts(&cc.egui_ctx);
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        prime_startup_fonts(&engine, false);
        let sample_line_count = compute_sample_line_count(&engine);
        Self {
            engine,
            assets,
            demos: demos::default_demos(),
            sample_line_count,
            root_viewport_activation_pending: cfg!(target_os = "macos"),
            system_engine_rx: None,
            system_fonts_ready: false,
            last_interaction_at: now,
            perf_hud_visible: true,
            perf_hud: PerfHudState::default(),
        }
    }

    pub fn new_headless() -> Self {
        let now = Instant::now();
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let sample_line_count = compute_sample_line_count(&engine);
        Self {
            engine,
            assets: AssetRegistry::default(),
            demos: demos::default_demos(),
            sample_line_count,
            root_viewport_activation_pending: false,
            system_engine_rx: None,
            system_fonts_ready: true,
            last_interaction_at: now,
            perf_hud_visible: false,
            perf_hud: PerfHudState::default(),
        }
    }

    pub fn demos_mut(&mut self) -> &mut Vec<Box<dyn DemoWindow>> {
        &mut self.demos
    }

    pub fn update_headless(&mut self, ctx: &egui::Context) {
        self.render(ctx);
    }

    fn render(&mut self, ctx: &egui::Context) {
        let frame_start = Instant::now();
        self.note_interaction(ctx);
        self.try_swap_in_system_engine();
        self.maybe_start_system_font_scan(ctx);
        self.ensure_root_viewport_visible(ctx);

        egui::SidePanel::left("catalog")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Pretext");
                ui.label("Rust + egui baseline");
                ui.separator();
                demos::catalog::show_catalog(ui, &mut self.demos);
                ui.separator();
                ui.label(format!("Sample lines: {}", self.sample_line_count));
                ui.label(self.system_font_status_label());
                ui.checkbox(&mut self.perf_hud_visible, "Show perf HUD");
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Workspace Baseline");
            ui.label("Catalog lives in the left panel. Open demos there.");
        });

        for demo in &mut self.demos {
            if demo.is_open() {
                demo.show(ctx, &self.engine, &mut self.assets);
            }
        }

        self.perf_hud.record_frame(
            frame_start.elapsed().as_secs_f32() * 1000.0,
            self.engine.runtime_stats(),
            self.assets.stats_snapshot(),
        );
        if self.perf_hud_visible {
            self.show_perf_hud(ctx);
        }
    }

    fn ensure_root_viewport_visible(&mut self, ctx: &egui::Context) {
        if !self.root_viewport_activation_pending {
            return;
        }

        // eframe creates the root window hidden until the first frame is rendered.
        // On macOS that can leave the app running without bringing the restored
        // window to the front, so we explicitly show and focus it once.
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.root_viewport_activation_pending = false;
    }

    fn note_interaction(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| !input.events.is_empty()) {
            self.last_interaction_at = Instant::now();
        }
    }

    fn maybe_start_system_font_scan(&mut self, ctx: &egui::Context) {
        if self.system_fonts_ready {
            return;
        }

        ctx.request_repaint_after(SYSTEM_FONT_IDLE_POLL_INTERVAL);
        if self.system_engine_rx.is_some() {
            return;
        }

        if self.last_interaction_at.elapsed() < SYSTEM_FONT_IDLE_DELAY {
            return;
        }

        self.system_engine_rx = Some(spawn_system_font_engine());
    }

    fn try_swap_in_system_engine(&mut self) {
        let Some(rx) = &self.system_engine_rx else {
            return;
        };

        if let Ok(engine) = rx.try_recv() {
            self.engine = engine;
            self.sample_line_count = compute_sample_line_count(&self.engine);
            self.system_fonts_ready = true;
            self.system_engine_rx = None;
        }
    }

    fn system_font_status_label(&self) -> &'static str {
        if self.system_fonts_ready {
            "System fonts: ready"
        } else if self.system_engine_rx.is_some() {
            "System fonts: indexing in background..."
        } else {
            "System fonts: pending idle scan"
        }
    }

    fn show_perf_hud(&self, ctx: &egui::Context) {
        egui::Window::new("Perf HUD")
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.label(format!("Frame EMA: {:.2} ms", self.perf_hud.frame_time_ms_ema));
                ui.label(self.system_font_status_label());
                ui.separator();
                ui.label(format!(
                    "Engine/frame: prepare={} prepare+seg={} layout={} layout+lines={} next_line={} visual_runs={} glyph_runs={} prefix_widths={} shape_spans={}",
                    self.perf_hud.last_engine_frame.prepare_calls,
                    self.perf_hud.last_engine_frame.prepare_with_segments_calls,
                    self.perf_hud.last_engine_frame.layout_calls,
                    self.perf_hud.last_engine_frame.layout_with_lines_calls,
                    self.perf_hud.last_engine_frame.layout_next_line_calls,
                    self.perf_hud.last_engine_frame.line_visual_runs_calls,
                    self.perf_hud.last_engine_frame.line_glyph_runs_calls,
                    self.perf_hud.last_engine_frame.prefix_widths_calls,
                    self.perf_hud.last_engine_frame.shape_text_spans_calls,
                ));
                ui.label(format!(
                    "Prepare cache/frame: text h/m={} / {} atomic h/m={} / {}",
                    self.perf_hud.last_engine_frame.prepare_cache_hits,
                    self.perf_hud.last_engine_frame.prepare_cache_misses,
                    self.perf_hud.last_engine_frame.atomic_placeholder_cache_hits,
                    self.perf_hud.last_engine_frame.atomic_placeholder_cache_misses,
                ));
                ui.label(format!(
                    "Textures/frame: hits={} misses={} uploads={} ({:.1} KiB)",
                    self.perf_hud.last_asset_frame.texture_cache_hits,
                    self.perf_hud.last_asset_frame.texture_cache_misses,
                    self.perf_hud.last_asset_frame.texture_uploads,
                    self.perf_hud.last_asset_frame.texture_upload_bytes as f32 / 1024.0,
                ));
                ui.label(format!(
                    "Raster/frame: hits={} misses={} rasterize={} glyph hits={} misses={}",
                    self.perf_hud.last_asset_frame.render_cache_hits,
                    self.perf_hud.last_asset_frame.render_cache_misses,
                    self.perf_hud.last_asset_frame.rasterizations,
                    self.perf_hud.last_asset_frame.glyph_path_hits,
                    self.perf_hud.last_asset_frame.glyph_path_misses,
                ));
                ui.label(format!(
                    "Cache totals: svg={} text={} raster={} glyph_paths={}",
                    self.perf_hud.last_asset_frame.static_svg_textures,
                    self.perf_hud.last_asset_frame.shaped_text_textures,
                    self.perf_hud.last_asset_frame.raster_cache_entries,
                    self.perf_hud.last_asset_frame.glyph_path_entries,
                ));
            });
    }
}

impl eframe::App for PretextDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
    }
}

impl PerfHudState {
    fn record_frame(
        &mut self,
        frame_ms: f32,
        engine_totals: EngineRuntimeStats,
        asset_totals: AssetRegistryStats,
    ) {
        self.frame_time_ms_ema = if self.frame_time_ms_ema <= 0.0 {
            frame_ms
        } else {
            self.frame_time_ms_ema * 0.88 + frame_ms * 0.12
        };
        self.last_engine_frame = diff_engine_stats(engine_totals, self.last_engine_totals);
        self.last_engine_totals = engine_totals;
        self.last_asset_frame = diff_asset_stats(asset_totals, self.last_asset_totals);
        self.last_asset_totals = asset_totals;
    }
}

fn diff_engine_stats(
    current: EngineRuntimeStats,
    previous: EngineRuntimeStats,
) -> EngineRuntimeStats {
    EngineRuntimeStats {
        prepare_calls: current.prepare_calls.saturating_sub(previous.prepare_calls),
        prepare_with_segments_calls: current
            .prepare_with_segments_calls
            .saturating_sub(previous.prepare_with_segments_calls),
        prepare_atomic_placeholder_calls: current
            .prepare_atomic_placeholder_calls
            .saturating_sub(previous.prepare_atomic_placeholder_calls),
        prepare_cache_hits: current
            .prepare_cache_hits
            .saturating_sub(previous.prepare_cache_hits),
        prepare_cache_misses: current
            .prepare_cache_misses
            .saturating_sub(previous.prepare_cache_misses),
        atomic_placeholder_cache_hits: current
            .atomic_placeholder_cache_hits
            .saturating_sub(previous.atomic_placeholder_cache_hits),
        atomic_placeholder_cache_misses: current
            .atomic_placeholder_cache_misses
            .saturating_sub(previous.atomic_placeholder_cache_misses),
        layout_calls: current.layout_calls.saturating_sub(previous.layout_calls),
        layout_with_lines_calls: current
            .layout_with_lines_calls
            .saturating_sub(previous.layout_with_lines_calls),
        walk_line_ranges_calls: current
            .walk_line_ranges_calls
            .saturating_sub(previous.walk_line_ranges_calls),
        layout_next_line_calls: current
            .layout_next_line_calls
            .saturating_sub(previous.layout_next_line_calls),
        line_visual_runs_calls: current
            .line_visual_runs_calls
            .saturating_sub(previous.line_visual_runs_calls),
        line_glyph_runs_calls: current
            .line_glyph_runs_calls
            .saturating_sub(previous.line_glyph_runs_calls),
        glyph_advance_calls: current
            .glyph_advance_calls
            .saturating_sub(previous.glyph_advance_calls),
        prefix_widths_calls: current
            .prefix_widths_calls
            .saturating_sub(previous.prefix_widths_calls),
        shape_text_spans_calls: current
            .shape_text_spans_calls
            .saturating_sub(previous.shape_text_spans_calls),
    }
}

fn diff_asset_stats(
    current: AssetRegistryStats,
    previous: AssetRegistryStats,
) -> AssetRegistryFrameStats {
    AssetRegistryFrameStats {
        texture_cache_hits: current
            .texture_cache_hits
            .saturating_sub(previous.texture_cache_hits),
        texture_cache_misses: current
            .texture_cache_misses
            .saturating_sub(previous.texture_cache_misses),
        texture_uploads: current
            .texture_uploads
            .saturating_sub(previous.texture_uploads),
        texture_upload_bytes: current
            .texture_upload_bytes
            .saturating_sub(previous.texture_upload_bytes),
        render_cache_hits: current
            .render
            .raster_cache_hits
            .saturating_sub(previous.render.raster_cache_hits),
        render_cache_misses: current
            .render
            .raster_cache_misses
            .saturating_sub(previous.render.raster_cache_misses),
        rasterizations: current
            .render
            .rasterizations
            .saturating_sub(previous.render.rasterizations),
        glyph_path_hits: current
            .render
            .glyph_path_hits
            .saturating_sub(previous.render.glyph_path_hits),
        glyph_path_misses: current
            .render
            .glyph_path_misses
            .saturating_sub(previous.render.glyph_path_misses),
        static_svg_textures: current.static_svg_textures,
        shaped_text_textures: current.shaped_text_textures,
        raster_cache_entries: current.render.raster_cache_entries,
        glyph_path_entries: current.render.glyph_path_entries,
    }
}

const SAMPLE_TEXT: &str = "The engine API is wired into the demo shell.";
const SAMPLE_WIDTH: f32 = 180.0;
const SAMPLE_LINE_HEIGHT: f32 = 22.0;
const SYSTEM_FONT_IDLE_DELAY: Duration = Duration::from_millis(1_500);
const SYSTEM_FONT_IDLE_POLL_INTERVAL: Duration = Duration::from_millis(250);

fn compute_sample_line_count(engine: &PretextEngine) -> usize {
    let prepared = engine.prepare(SAMPLE_TEXT, &default_style(), &default_options());
    engine
        .layout(&prepared, SAMPLE_WIDTH, SAMPLE_LINE_HEIGHT)
        .line_count
}

fn spawn_system_font_engine() -> Receiver<PretextEngine> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            true,
        );
        prime_startup_fonts(&engine, true);
        let _ = tx.send(engine);
    });
    rx
}

fn prime_startup_fonts(engine: &PretextEngine, include_system_primes: bool) {
    let mut requests = vec![
        ("Hello layout", default_style()),
        ("مرحبا", bundled_arabic_style()),
        ("中文标签", bundled_cjk_style()),
        ("မြန်မာ", bundled_myanmar_style()),
        ("i1{}", bundled_mono_style()),
        ("✅", bundled_emoji_style()),
    ];

    if include_system_primes {
        requests.push(("Hello", system_sans_prime_style()));
        requests.push(("i1{}", system_mono_prime_style()));
    }

    for (text, style) in requests {
        let _ = engine.prefix_widths(text, &style);
    }
}

fn default_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
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

fn default_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
    }
}

fn bundled_arabic_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans Arabic".to_owned(),
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 16.0,
        weight: 400,
        italic: false,
    }
}

fn bundled_cjk_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans CJK".to_owned(),
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 16.0,
        weight: 400,
        italic: false,
    }
}

fn bundled_myanmar_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans Myanmar".to_owned(),
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 16.0,
        weight: 400,
        italic: false,
    }
}

fn bundled_mono_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans Mono".to_owned(),
            "Menlo".to_owned(),
            "Monaco".to_owned(),
        ],
        size_px: 14.0,
        weight: 600,
        italic: false,
    }
}

fn bundled_emoji_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Color Emoji".to_owned(),
            "Noto Emoji".to_owned(),
            "Noto Sans".to_owned(),
        ],
        size_px: 16.0,
        weight: 400,
        italic: false,
    }
}

fn system_sans_prime_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Helvetica".to_owned(),
            "Arial".to_owned(),
            "Noto Sans".to_owned(),
        ],
        size_px: 16.0,
        weight: 400,
        italic: false,
    }
}

fn system_mono_prime_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Menlo".to_owned(),
            "Monaco".to_owned(),
            "Noto Sans Mono".to_owned(),
        ],
        size_px: 14.0,
        weight: 600,
        italic: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_app_keeps_cached_sample_line_count_between_updates() {
        let ctx = egui::Context::default();
        let mut app = PretextDemoApp::new_headless();
        let cached = app.sample_line_count;

        assert_eq!(cached, compute_sample_line_count(&app.engine));

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.update_headless(ctx);
        });
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.update_headless(ctx);
        });

        assert_eq!(app.sample_line_count, cached);
    }
}

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use eframe::egui;
use pretext::{
    EngineRuntimeStats, ParagraphDirection, PretextEngine, PretextParagraphOptions, PretextStyle,
    WhiteSpaceMode,
};
use pretext_egui::{
    advanced::{enqueue_atlas_warmup, tick_atlas_warmup, AtlasWarmupBucket},
    EguiPretextRenderer, EguiPretextRendererStats,
};

use crate::demo_assets::{bundled_font_data, install_demo_fonts};
use crate::demos::catalog::CatalogInteraction;
use crate::demos::{self, DemoPerfStats, DemoWarmupStatus, DemoWindow};

pub struct PretextDemoApp {
    engine: PretextEngine,
    assets: EguiPretextRenderer,
    demos: Vec<Box<dyn DemoWindow>>,
    sample_line_count: usize,
    root_viewport_activation_pending: bool,
    system_engine_rx: Option<Receiver<PretextEngine>>,
    system_fonts_ready: bool,
    last_interaction_at: Instant,
    perf_hud_visible: bool,
    perf_hud: PerfHudState,
    demo_warmup_frame: DemoWarmupFrameStats,
    demo_lifecycle: HashMap<&'static str, DemoLifecycleRecord>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AssetRegistryFrameStats {
    texture_cache_hits: u64,
    texture_cache_misses: u64,
    texture_uploads: u64,
    texture_upload_bytes: u64,
    atlas_hits: u64,
    atlas_misses: u64,
    mesh_flushes: u64,
    glyph_quads: u64,
    render_cache_hits: u64,
    render_cache_misses: u64,
    rasterizations: u64,
    glyph_path_hits: u64,
    glyph_path_misses: u64,
    static_svg_textures: usize,
    shaped_text_textures: usize,
    atlas_pages: usize,
    atlas_entries: usize,
    warmup_queue_depth: usize,
    raster_cache_entries: usize,
    glyph_path_entries: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct PerfHudState {
    frame_time_ms_ema: f32,
    last_engine_totals: EngineRuntimeStats,
    last_engine_frame: EngineRuntimeStats,
    last_asset_totals: EguiPretextRendererStats,
    last_asset_frame: AssetRegistryFrameStats,
    last_demo_frame: DemoPerfStats,
}

#[derive(Clone, Copy, Debug, Default)]
struct DemoWarmupFrameStats {
    step_count: usize,
    pending_open: usize,
    pending_background: usize,
    active_demo: Option<&'static str>,
    active_stage: &'static str,
}

#[derive(Clone, Copy, Debug, Default)]
struct DemoLifecycleRecord {
    was_open: bool,
    ready_once: bool,
    awaiting_ready_since: Option<Instant>,
    first_open_ready_ms: Option<f32>,
    last_reopen_ready_ms: Option<f32>,
}

impl PretextDemoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let now = Instant::now();
        let assets = EguiPretextRenderer::default();
        install_demo_fonts(&cc.egui_ctx);
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
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
            demo_warmup_frame: DemoWarmupFrameStats::default(),
            demo_lifecycle: HashMap::new(),
        }
    }

    pub fn new_headless() -> Self {
        let now = Instant::now();
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let sample_line_count = compute_sample_line_count(&engine);
        Self {
            engine,
            assets: EguiPretextRenderer::default(),
            demos: demos::default_demos(),
            sample_line_count,
            root_viewport_activation_pending: false,
            system_engine_rx: None,
            system_fonts_ready: true,
            last_interaction_at: now,
            perf_hud_visible: false,
            perf_hud: PerfHudState::default(),
            demo_warmup_frame: DemoWarmupFrameStats::default(),
            demo_lifecycle: HashMap::new(),
        }
    }

    pub fn demos_mut(&mut self) -> &mut Vec<Box<dyn DemoWindow>> {
        &mut self.demos
    }

    pub fn update_headless(&mut self, ctx: &egui::Context) {
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::new((ctx.viewport_id(), "pretext_headless_root")),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(ctx.content_rect()),
        );
        self.render(&mut ui);
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let frame_start = Instant::now();
        self.note_interaction(&ctx);
        self.try_swap_in_system_engine();
        self.ensure_root_viewport_visible(&ctx);
        self.demo_warmup_frame = DemoWarmupFrameStats::default();

        let mut catalog_interaction = CatalogInteraction::default();
        egui::Panel::left("catalog")
            .resizable(true)
            .default_size(220.0)
            .show_inside(ui, |ui| {
                ui.heading("Pretext");
                ui.label("Rust + egui baseline");
                ui.separator();
                catalog_interaction = demos::catalog::show_catalog(ui, &mut self.demos);
                ui.separator();
                ui.label(format!("Sample lines: {}", self.sample_line_count));
                ui.label(self.system_font_status_label());
                if !self.system_fonts_ready && self.system_engine_rx.is_none() {
                    if ui.button("Load system fonts").clicked() {
                        self.system_engine_rx = Some(spawn_system_font_engine());
                    }
                }
                ui.checkbox(&mut self.perf_hud_visible, "Show perf HUD");
            });

        self.run_hover_warmup(&ctx, catalog_interaction);
        self.run_open_demo_warmups(&ctx, catalog_interaction);

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Workspace Baseline");
            ui.label("Catalog lives in the left panel. Open demos there.");
        });

        for demo in &mut self.demos {
            if demo.is_open() {
                if demo.warmup_status().ready {
                    demo.show(&ctx, &self.engine, &mut self.assets);
                } else {
                    demo.show_loading(&ctx, &self.engine, &mut self.assets);
                }
            }
        }

        self.update_demo_lifecycle();

        if self.demo_warmup_frame.pending_open > 0
            || self.demo_warmup_frame.pending_background > 0
            || self.system_engine_rx.is_some()
        {
            ctx.request_repaint();
        }
        self.maybe_tick_atlas_warmup(&ctx);
        let demo_perf = self.demos.iter().filter(|demo| demo.is_open()).fold(
            DemoPerfStats::default(),
            |acc, demo| {
                let stats = demo.perf_stats();
                DemoPerfStats {
                    dynamic_bucket_hits: acc.dynamic_bucket_hits + stats.dynamic_bucket_hits,
                    dynamic_dirty_bands: acc.dynamic_dirty_bands + stats.dynamic_dirty_bands,
                    dynamic_full_recomputes: acc.dynamic_full_recomputes
                        + stats.dynamic_full_recomputes,
                    editorial_bucket_hits: acc.editorial_bucket_hits + stats.editorial_bucket_hits,
                    editorial_dirty_bands: acc.editorial_dirty_bands + stats.editorial_dirty_bands,
                    editorial_full_recomputes: acc.editorial_full_recomputes
                        + stats.editorial_full_recomputes,
                }
            },
        );

        self.perf_hud.record_frame(
            frame_start.elapsed().as_secs_f32() * 1000.0,
            self.engine.runtime_stats(),
            self.assets.stats(),
            demo_perf,
        );
        if self.perf_hud_visible {
            self.show_perf_hud(&ctx);
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
            "System fonts: bundled-only"
        }
    }

    fn run_hover_warmup(&mut self, ctx: &egui::Context, interaction: CatalogInteraction) {
        let Some(hovered_demo_id) = interaction.hovered_demo_id else {
            return;
        };

        let Some(index) = self
            .demos
            .iter()
            .position(|demo| demo.id() == hovered_demo_id && !demo.is_open())
        else {
            return;
        };

        let status = self.demos[index].warmup_status();
        if status.ready {
            return;
        }

        self.demo_warmup_frame.pending_background += 1;
        let _ = self.warm_demo(index, ctx, BACKGROUND_DEMO_WARMUP_BUDGET);
    }

    fn run_open_demo_warmups(&mut self, ctx: &egui::Context, interaction: CatalogInteraction) {
        let frame_deadline = Instant::now() + OPEN_DEMO_WARMUP_BUDGET;
        let prioritized = interaction.opened_demo_id.and_then(|opened_demo_id| {
            self.demos
                .iter()
                .position(|demo| demo.id() == opened_demo_id)
        });
        let mut pending_indices = Vec::new();

        if let Some(index) = prioritized {
            if self.demos[index].is_open() && !self.demos[index].warmup_status().ready {
                pending_indices.push(index);
            }
        }

        for (index, demo) in self.demos.iter().enumerate() {
            if Some(index) == prioritized {
                continue;
            }
            if demo.is_open() && !demo.warmup_status().ready {
                pending_indices.push(index);
            }
        }

        self.demo_warmup_frame.pending_open = pending_indices.len();
        for index in pending_indices {
            let remaining = frame_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            let _ = self.warm_demo(index, ctx, remaining);
        }
    }

    fn warm_demo(
        &mut self,
        index: usize,
        ctx: &egui::Context,
        budget: Duration,
    ) -> DemoWarmupStatus {
        let id = self.demos[index].id();
        let status = self.demos[index].warmup_status();
        if self.demo_warmup_frame.active_demo.is_none() {
            self.demo_warmup_frame.active_demo = Some(id);
            self.demo_warmup_frame.active_stage = status.stage;
        }
        self.demo_warmup_frame.step_count += 1;
        let _ = self.demos[index].warmup_step(ctx, &self.engine, &mut self.assets, budget);
        self.demos[index].warmup_status()
    }

    fn update_demo_lifecycle(&mut self) {
        let now = Instant::now();
        for demo in &self.demos {
            let status = demo.warmup_status();
            let record = self.demo_lifecycle.entry(demo.id()).or_default();
            let is_open = demo.is_open();

            if !record.was_open && is_open {
                if status.ready {
                    if record.ready_once {
                        record.last_reopen_ready_ms = Some(0.0);
                    } else {
                        record.first_open_ready_ms = Some(0.0);
                        record.ready_once = true;
                    }
                    record.awaiting_ready_since = None;
                } else {
                    record.awaiting_ready_since = Some(now);
                }
            }

            if is_open && !status.ready && record.awaiting_ready_since.is_none() {
                record.awaiting_ready_since = Some(now);
            }

            if is_open && status.ready {
                if let Some(started) = record.awaiting_ready_since.take() {
                    let ready_ms = now.duration_since(started).as_secs_f32() * 1000.0;
                    if record.ready_once {
                        record.last_reopen_ready_ms = Some(ready_ms);
                    } else {
                        record.first_open_ready_ms = Some(ready_ms);
                        record.ready_once = true;
                    }
                }
            }

            if !is_open {
                record.awaiting_ready_since = None;
            }

            record.was_open = is_open;
        }
    }

    fn maybe_tick_atlas_warmup(&mut self, ctx: &egui::Context) {
        if self.last_interaction_at.elapsed() < SYSTEM_FONT_IDLE_DELAY {
            return;
        }

        schedule_default_atlas_warmup(&mut self.assets, &self.engine, ctx);
        let _ = tick_atlas_warmup(
            &mut self.assets,
            ctx,
            &self.engine,
            ATLAS_WARMUP_GLYPH_BUDGET,
            ATLAS_WARMUP_PAGE_BUDGET,
        );
    }

    fn show_perf_hud(&self, ctx: &egui::Context) {
        egui::Window::new("Perf HUD")
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
            .resizable(false)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.label(format!("Frame EMA: {:.2} ms", self.perf_hud.frame_time_ms_ema));
                ui.label(self.system_font_status_label());
                ui.label(format!(
                    "Demo warmup/frame: steps={} open_pending={} background_pending={}",
                    self.demo_warmup_frame.step_count,
                    self.demo_warmup_frame.pending_open,
                    self.demo_warmup_frame.pending_background,
                ));
                if let Some(active_demo) = self.demo_warmup_frame.active_demo {
                    ui.label(format!(
                        "Warmup target: {} ({})",
                        active_demo, self.demo_warmup_frame.active_stage
                    ));
                }
                if let Some((demo_id, record)) = self
                    .demo_lifecycle
                    .iter()
                    .find(|(_, record)| record.first_open_ready_ms.is_some() || record.last_reopen_ready_ms.is_some())
                {
                    let first_open = record
                        .first_open_ready_ms
                        .map(|value| format!("{value:.1} ms"))
                        .unwrap_or_else(|| "-".to_owned());
                    let reopen = record
                        .last_reopen_ready_ms
                        .map(|value| format!("{value:.1} ms"))
                        .unwrap_or_else(|| "-".to_owned());
                    ui.label(format!(
                        "Warmup telemetry: {} first={} reopen={}",
                        demo_id, first_open, reopen
                    ));
                }
                ui.separator();
                ui.label(format!(
                    "Engine/frame: prepare={} prepare+seg={} layout={} layout+lines={} layout+runs={} next_line={} next_line+glyph={} next_line+runs={} visual_runs={} glyph_runs={} line_runs={} prefix_widths={} shape_spans={}",
                    self.perf_hud.last_engine_frame.prepare_calls,
                    self.perf_hud.last_engine_frame.prepare_with_segments_calls,
                    self.perf_hud.last_engine_frame.layout_calls,
                    self.perf_hud.last_engine_frame.layout_with_lines_calls,
                    self.perf_hud.last_engine_frame.layout_with_runs_calls,
                    self.perf_hud.last_engine_frame.layout_next_line_calls,
                    self.perf_hud.last_engine_frame.layout_next_line_with_glyph_runs_calls,
                    self.perf_hud.last_engine_frame.layout_next_line_with_runs_calls,
                    self.perf_hud.last_engine_frame.line_visual_runs_calls,
                    self.perf_hud.last_engine_frame.line_glyph_runs_calls,
                    self.perf_hud.last_engine_frame.line_runs_calls,
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
                    "Atlas/frame: hits={} misses={} flushes={} quads={}",
                    self.perf_hud.last_asset_frame.atlas_hits,
                    self.perf_hud.last_asset_frame.atlas_misses,
                    self.perf_hud.last_asset_frame.mesh_flushes,
                    self.perf_hud.last_asset_frame.glyph_quads,
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
                    "Cache totals: svg={} text={} atlas={} pages={} warmup={} raster={} glyph_paths={}",
                    self.perf_hud.last_asset_frame.static_svg_textures,
                    self.perf_hud.last_asset_frame.shaped_text_textures,
                    self.perf_hud.last_asset_frame.atlas_entries,
                    self.perf_hud.last_asset_frame.atlas_pages,
                    self.perf_hud.last_asset_frame.warmup_queue_depth,
                    self.perf_hud.last_asset_frame.raster_cache_entries,
                    self.perf_hud.last_asset_frame.glyph_path_entries,
                ));
                ui.label(format!(
                    "Dynamic/frame: bucket_hits={} dirty_bands={} full_recomputes={}",
                    self.perf_hud.last_demo_frame.dynamic_bucket_hits,
                    self.perf_hud.last_demo_frame.dynamic_dirty_bands,
                    self.perf_hud.last_demo_frame.dynamic_full_recomputes,
                ));
                ui.label(format!(
                    "Editorial/frame: bucket_hits={} dirty_bands={} full_recomputes={}",
                    self.perf_hud.last_demo_frame.editorial_bucket_hits,
                    self.perf_hud.last_demo_frame.editorial_dirty_bands,
                    self.perf_hud.last_demo_frame.editorial_full_recomputes,
                ));
            });
    }
}

impl eframe::App for PretextDemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
}

impl PerfHudState {
    fn record_frame(
        &mut self,
        frame_ms: f32,
        engine_totals: EngineRuntimeStats,
        asset_totals: EguiPretextRendererStats,
        demo_frame: DemoPerfStats,
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
        self.last_demo_frame = demo_frame;
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
        layout_with_runs_calls: current
            .layout_with_runs_calls
            .saturating_sub(previous.layout_with_runs_calls),
        walk_line_ranges_calls: current
            .walk_line_ranges_calls
            .saturating_sub(previous.walk_line_ranges_calls),
        layout_next_line_calls: current
            .layout_next_line_calls
            .saturating_sub(previous.layout_next_line_calls),
        layout_next_line_with_glyph_runs_calls: current
            .layout_next_line_with_glyph_runs_calls
            .saturating_sub(previous.layout_next_line_with_glyph_runs_calls),
        layout_next_line_with_runs_calls: current
            .layout_next_line_with_runs_calls
            .saturating_sub(previous.layout_next_line_with_runs_calls),
        line_visual_runs_calls: current
            .line_visual_runs_calls
            .saturating_sub(previous.line_visual_runs_calls),
        line_glyph_runs_calls: current
            .line_glyph_runs_calls
            .saturating_sub(previous.line_glyph_runs_calls),
        line_runs_calls: current
            .line_runs_calls
            .saturating_sub(previous.line_runs_calls),
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
    current: EguiPretextRendererStats,
    previous: EguiPretextRendererStats,
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
        atlas_hits: current.atlas_hits.saturating_sub(previous.atlas_hits),
        atlas_misses: current.atlas_misses.saturating_sub(previous.atlas_misses),
        mesh_flushes: current.mesh_flushes.saturating_sub(previous.mesh_flushes),
        glyph_quads: current.glyph_quads.saturating_sub(previous.glyph_quads),
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
        atlas_pages: current.atlas_pages,
        atlas_entries: current.atlas_entries,
        warmup_queue_depth: current.warmup_queue_depth,
        raster_cache_entries: current.render.raster_cache_entries,
        glyph_path_entries: current.render.glyph_path_entries,
    }
}

const SAMPLE_TEXT: &str = "The engine API is wired into the demo shell.";
const SAMPLE_WIDTH: f32 = 180.0;
const SAMPLE_LINE_HEIGHT: f32 = 22.0;
const SYSTEM_FONT_IDLE_DELAY: Duration = Duration::from_millis(1_500);
const OPEN_DEMO_WARMUP_BUDGET: Duration = Duration::from_millis(8);
const BACKGROUND_DEMO_WARMUP_BUDGET: Duration = Duration::from_millis(3);
const ATLAS_WARMUP_GLYPH_BUDGET: usize = 24;
const ATLAS_WARMUP_PAGE_BUDGET: usize = 6;

const ASCII_VISIBLE_WARMUP: &str =
    " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~";
const MASONRY_WARMUP: &str =
    "Masonry cards keep scrolling smooth while cached atlas glyphs stay hot.";
const ACCORDION_WARMUP: &str =
    "Accordion panels mix English, العربية, and emoji ✅ without rebuilding textures.";
const BUBBLES_WARMUP: &str =
    "كل شيء! Mixed bidi, grapheme clusters, and emoji ✅ stay stable while the chat resizes.";
const EDITORIAL_BODY_WARMUP: &str =
    "Editorial body copy wraps around pull quotes while animated obstacles only dirty local bands.";
const EDITORIAL_HEADLINE_WARMUP: &str = "Editorial Engine Keeps Text Flow Stable";
const DRAGON_TEXT_WARMUP: &str =
    "A dragon parts serif copy like water while pretext recomputes every line.";
const CJK_WARMUP: &str = "中文标签与段落布局";
const MYANMAR_WARMUP: &str = "မြန်မာစာ စာပိုဒ်";

fn compute_sample_line_count(engine: &PretextEngine) -> usize {
    let prepared = engine.prepare_paragraph(SAMPLE_TEXT, &default_style(), &default_options());
    prepared
        .measure(engine, SAMPLE_WIDTH, SAMPLE_LINE_HEIGHT)
        .line_count
}

fn spawn_system_font_engine() -> Receiver<PretextEngine> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(true)
            .build();
        prime_startup_fonts(&engine, true);
        let _ = tx.send(engine);
    });
    rx
}

fn schedule_default_atlas_warmup(
    assets: &mut EguiPretextRenderer,
    engine: &PretextEngine,
    ctx: &egui::Context,
) {
    enqueue_atlas_warmup(
        assets,
        AtlasWarmupBucket::CommonSans,
        &default_style(),
        &[
            ASCII_VISIBLE_WARMUP,
            MASONRY_WARMUP,
            ACCORDION_WARMUP,
            BUBBLES_WARMUP,
        ],
        engine,
        ctx,
    );
    enqueue_atlas_warmup(
        assets,
        AtlasWarmupBucket::CommonSerif,
        &warmup_editorial_body_style(),
        &[
            ASCII_VISIBLE_WARMUP,
            EDITORIAL_BODY_WARMUP,
            DRAGON_TEXT_WARMUP,
        ],
        engine,
        ctx,
    );
    enqueue_atlas_warmup(
        assets,
        AtlasWarmupBucket::SerifDisplay,
        &warmup_editorial_headline_style(),
        &[EDITORIAL_HEADLINE_WARMUP],
        engine,
        ctx,
    );
    enqueue_atlas_warmup(
        assets,
        AtlasWarmupBucket::Mono,
        &bundled_mono_style(),
        &[ASCII_VISIBLE_WARMUP, "i1{} -> mono warmup"],
        engine,
        ctx,
    );
    enqueue_atlas_warmup(
        assets,
        AtlasWarmupBucket::Arabic,
        &bundled_arabic_style(),
        &["بدأت الرحلة بالعربية 123 ✅", BUBBLES_WARMUP],
        engine,
        ctx,
    );
    enqueue_atlas_warmup(
        assets,
        AtlasWarmupBucket::Cjk,
        &bundled_cjk_style(),
        &[CJK_WARMUP],
        engine,
        ctx,
    );
    enqueue_atlas_warmup(
        assets,
        AtlasWarmupBucket::Myanmar,
        &bundled_myanmar_style(),
        &[MYANMAR_WARMUP],
        engine,
        ctx,
    );
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

fn default_style() -> PretextStyle {
    PretextStyle {
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

fn default_options() -> PretextParagraphOptions {
    PretextParagraphOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: pretext::WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
        letter_spacing: 0.0,
    }
}

fn bundled_arabic_style() -> PretextStyle {
    PretextStyle {
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

fn bundled_cjk_style() -> PretextStyle {
    PretextStyle {
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

fn bundled_myanmar_style() -> PretextStyle {
    PretextStyle {
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

fn bundled_mono_style() -> PretextStyle {
    PretextStyle {
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

fn bundled_emoji_style() -> PretextStyle {
    PretextStyle {
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

fn warmup_editorial_body_style() -> PretextStyle {
    PretextStyle {
        families: vec![
            "Noto Serif".to_owned(),
            "Georgia".to_owned(),
            "Times New Roman".to_owned(),
            "Noto Sans".to_owned(),
        ],
        size_px: 18.0,
        weight: 400,
        italic: false,
    }
}

fn warmup_editorial_headline_style() -> PretextStyle {
    PretextStyle {
        families: vec![
            "Noto Serif".to_owned(),
            "Georgia".to_owned(),
            "Times New Roman".to_owned(),
            "Noto Sans".to_owned(),
        ],
        size_px: 64.0,
        weight: 700,
        italic: false,
    }
}

fn system_sans_prime_style() -> PretextStyle {
    PretextStyle {
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

fn system_mono_prime_style() -> PretextStyle {
    PretextStyle {
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

        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            app.update_headless(ctx);
        });
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            app.update_headless(ctx);
        });

        assert_eq!(app.sample_line_count, cached);
    }
}

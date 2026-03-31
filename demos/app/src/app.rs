use eframe::egui;
use pretext::{ParagraphDirection, PrepareOptions, PretextEngine, WhiteSpaceMode};

use crate::assets::AssetRegistry;
use crate::demos::{self, DemoWindow};

pub struct PretextDemoApp {
    engine: PretextEngine,
    assets: AssetRegistry,
    demos: Vec<Box<dyn DemoWindow>>,
    default_style: pretext::TextStyleSpec,
    default_options: PrepareOptions,
}

impl PretextDemoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut assets = AssetRegistry::default();
        assets.install_fonts(&cc.egui_ctx);
        Self {
            engine: PretextEngine::with_font_data_and_system_fonts(
                AssetRegistry::bundled_font_data(),
                false,
            ),
            assets,
            demos: demos::default_demos(),
            default_style: default_style(),
            default_options: PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        }
    }

    pub fn new_headless() -> Self {
        Self {
            engine: PretextEngine::with_font_data_and_system_fonts(
                AssetRegistry::bundled_font_data(),
                false,
            ),
            assets: AssetRegistry::default(),
            demos: demos::default_demos(),
            default_style: default_style(),
            default_options: PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: ParagraphDirection::Auto,
            },
        }
    }

    pub fn demos_mut(&mut self) -> &mut Vec<Box<dyn DemoWindow>> {
        &mut self.demos
    }

    pub fn update_headless(&mut self, ctx: &egui::Context) {
        self.render(ctx);
    }

    fn render(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("catalog")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Pretext");
                ui.label("Rust + egui baseline");
                ui.separator();
                demos::catalog::show_catalog(ui, &mut self.demos);
                ui.separator();

                let sample = "The engine API is wired into the demo shell.";
                let prepared =
                    self.engine
                        .prepare(sample, &self.default_style, &self.default_options);
                let layout = self.engine.layout(&prepared, 180.0, 22.0);
                ui.label(format!("Sample lines: {}", layout.line_count));
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
    }
}

impl eframe::App for PretextDemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
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

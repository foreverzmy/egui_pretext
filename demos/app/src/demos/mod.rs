pub mod accordion;
pub mod bubbles;
pub mod catalog;
pub mod dragon_through_text;
pub mod dynamic_layout;
pub mod editorial_engine;
pub mod justification_algorithms;
pub mod markdown_chat;
pub mod masonry;
pub mod rich_note;
pub mod variable_typographic_ascii;

use std::time::Duration;

use eframe::egui;
use pretext::PretextEngine;
use pretext_egui::EguiPretextRenderer;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DemoPerfStats {
    pub dynamic_bucket_hits: usize,
    pub dynamic_dirty_bands: usize,
    pub dynamic_full_recomputes: usize,
    pub editorial_bucket_hits: usize,
    pub editorial_dirty_bands: usize,
    pub editorial_full_recomputes: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DemoWarmupStatus {
    pub ready: bool,
    pub stage: &'static str,
    pub completed: usize,
    pub total: usize,
}

impl DemoWarmupStatus {
    pub const fn ready() -> Self {
        Self {
            ready: true,
            stage: "ready",
            completed: 1,
            total: 1,
        }
    }

    pub const fn pending(stage: &'static str, completed: usize, total: usize) -> Self {
        Self {
            ready: false,
            stage,
            completed,
            total,
        }
    }
}

pub fn format_warmup_status(status: DemoWarmupStatus) -> String {
    if status.ready {
        "Ready".to_owned()
    } else if status.total > 0 {
        format!("{} ({}/{})", status.stage, status.completed, status.total)
    } else {
        status.stage.to_owned()
    }
}

pub trait DemoWindow {
    fn id(&self) -> &'static str;
    fn title(&self) -> &str;
    fn is_open(&self) -> bool;
    fn set_open(&mut self, open: bool);
    fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    );
    fn warmup_status(&self) -> DemoWarmupStatus {
        DemoWarmupStatus::ready()
    }
    fn warmup_step(
        &mut self,
        _ctx: &egui::Context,
        _engine: &PretextEngine,
        _assets: &mut EguiPretextRenderer,
        _budget: Duration,
    ) -> bool {
        true
    }
    fn show_loading(
        &mut self,
        ctx: &egui::Context,
        _engine: &PretextEngine,
        _assets: &mut EguiPretextRenderer,
    ) {
        let mut open = self.is_open();
        let status = self.warmup_status();
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(12.0);
                    ui.spinner();
                    ui.add_space(12.0);
                    ui.heading(self.title());
                    ui.label("Preparing demo assets and layout caches.");
                    ui.add_space(4.0);
                    ui.monospace(format_warmup_status(status));
                });
            });
        self.set_open(open);
    }
    fn perf_stats(&self) -> DemoPerfStats {
        DemoPerfStats::default()
    }
}

pub fn default_demos() -> Vec<Box<dyn DemoWindow>> {
    vec![
        Box::new(accordion::AccordionDemo::default()),
        Box::new(bubbles::BubblesDemo::default()),
        Box::new(markdown_chat::MarkdownChatDemo::default()),
        Box::new(rich_note::RichNoteDemo::default()),
        Box::new(masonry::MasonryDemo::default()),
        Box::new(dynamic_layout::DynamicLayoutDemo::default()),
        Box::new(dragon_through_text::DragonThroughTextDemo::default()),
        Box::new(editorial_engine::EditorialEngineDemo::default()),
        Box::new(justification_algorithms::JustificationAlgorithmsDemo::default()),
        Box::new(variable_typographic_ascii::VariableTypographicAsciiDemo::default()),
    ]
}

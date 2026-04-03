pub mod accordion;
pub mod bubbles;
pub mod catalog;
pub mod dragon_through_text;
pub mod dynamic_layout;
pub mod editorial_engine;
pub mod justification_algorithms;
pub mod masonry;
pub mod rich_note;
pub mod variable_typographic_ascii;

use eframe::egui;
use pretext::PretextEngine;
use pretext_egui::AssetRegistry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DemoPerfStats {
    pub dynamic_bucket_hits: usize,
    pub dynamic_dirty_bands: usize,
    pub dynamic_full_recomputes: usize,
    pub editorial_bucket_hits: usize,
    pub editorial_dirty_bands: usize,
    pub editorial_full_recomputes: usize,
}

pub trait DemoWindow {
    fn title(&self) -> &str;
    fn is_open(&self) -> bool;
    fn set_open(&mut self, open: bool);
    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, assets: &mut AssetRegistry);
    fn perf_stats(&self) -> DemoPerfStats {
        DemoPerfStats::default()
    }
}

pub fn default_demos() -> Vec<Box<dyn DemoWindow>> {
    vec![
        Box::new(accordion::AccordionDemo::default()),
        Box::new(bubbles::BubblesDemo::default()),
        Box::new(rich_note::RichNoteDemo::default()),
        Box::new(masonry::MasonryDemo::default()),
        Box::new(dynamic_layout::DynamicLayoutDemo::default()),
        Box::new(dragon_through_text::DragonThroughTextDemo::default()),
        Box::new(editorial_engine::EditorialEngineDemo::default()),
        Box::new(justification_algorithms::JustificationAlgorithmsDemo::default()),
        Box::new(variable_typographic_ascii::VariableTypographicAsciiDemo::default()),
    ]
}

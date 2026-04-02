pub mod accordion;
pub mod bubbles;
pub mod catalog;
pub mod dynamic_layout;
pub mod editorial_engine;
pub mod masonry;
pub mod rich_note;
pub(crate) mod text_runs;
pub mod variable_typographic_ascii;

use eframe::egui;
use pretext::PretextEngine;
use pretext_egui::AssetRegistry;

pub trait DemoWindow {
    fn title(&self) -> &str;
    fn is_open(&self) -> bool;
    fn set_open(&mut self, open: bool);
    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, assets: &mut AssetRegistry);
}

pub fn default_demos() -> Vec<Box<dyn DemoWindow>> {
    vec![
        Box::new(accordion::AccordionDemo::default()),
        Box::new(bubbles::BubblesDemo::default()),
        Box::new(rich_note::RichNoteDemo::default()),
        Box::new(masonry::MasonryDemo::default()),
        Box::new(dynamic_layout::DynamicLayoutDemo::default()),
        Box::new(editorial_engine::EditorialEngineDemo::default()),
        Box::new(variable_typographic_ascii::VariableTypographicAsciiDemo::default()),
    ]
}

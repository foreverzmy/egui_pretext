use eframe::{egui, NativeOptions};
use pretext_demo_app::PretextDemoApp;

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1600.0, 1200.0]),
        ..NativeOptions::default()
    };
    eframe::run_native(
        "Pretext Demo",
        options,
        Box::new(|cc| Ok(Box::new(PretextDemoApp::new(cc)))),
    )
}

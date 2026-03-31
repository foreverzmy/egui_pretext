use eframe::NativeOptions;
use pretext_demo_app::PretextDemoApp;

fn main() -> eframe::Result<()> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Pretext Demo",
        options,
        Box::new(|cc| Ok(Box::new(PretextDemoApp::new(cc)))),
    )
}

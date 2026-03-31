use eframe::egui;
use pretext_demo_app::PretextDemoApp;

#[test]
fn all_demos_open_without_panic() {
    let ctx = egui::Context::default();
    let mut app = PretextDemoApp::new_headless();
    for demo in app.demos_mut() {
        demo.set_open(true);
    }

    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        app.update_headless(ctx);
    });
}

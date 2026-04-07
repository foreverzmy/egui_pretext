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

#[test]
fn heavy_demo_warmups_reach_ready_state() {
    let ctx = egui::Context::default();
    let mut app = PretextDemoApp::new_headless();
    for demo in app.demos_mut() {
        demo.set_open(true);
    }

    for _ in 0..96 {
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.update_headless(ctx);
        });
        if app.demos_mut().iter().all(|demo| demo.warmup_status().ready) {
            return;
        }
    }

    let statuses = app
        .demos_mut()
        .iter()
        .map(|demo| format!("{}: {:?}", demo.id(), demo.warmup_status()))
        .collect::<Vec<_>>();
    panic!("expected all demos to reach ready state, got {statuses:?}");
}

#[test]
fn heavy_demo_reopen_keeps_warm_caches() {
    let ctx = egui::Context::default();
    let mut app = PretextDemoApp::new_headless();
    for demo in app.demos_mut() {
        if matches!(
            demo.id(),
            "markdown_chat" | "dynamic_layout" | "editorial_engine" | "variable_typographic_ascii"
        ) {
            demo.set_open(true);
        }
    }

    for _ in 0..96 {
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            app.update_headless(ctx);
        });
        if app
            .demos_mut()
            .iter()
            .filter(|demo| {
                matches!(
                    demo.id(),
                    "markdown_chat"
                        | "dynamic_layout"
                        | "editorial_engine"
                        | "variable_typographic_ascii"
                )
            })
            .all(|demo| demo.warmup_status().ready)
        {
            break;
        }
    }

    for demo in app.demos_mut() {
        if matches!(
            demo.id(),
            "markdown_chat" | "dynamic_layout" | "editorial_engine" | "variable_typographic_ascii"
        ) {
            assert!(demo.warmup_status().ready, "{} should be warm before close", demo.id());
            demo.set_open(false);
            demo.set_open(true);
            assert!(
                demo.warmup_status().ready,
                "{} should stay warm across close/open",
                demo.id()
            );
        }
    }
}

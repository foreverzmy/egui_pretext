use eframe::egui;

use crate::demos::DemoWindow;

pub fn show_catalog(ui: &mut egui::Ui, demos: &mut [Box<dyn DemoWindow>]) {
    for demo in demos {
        let mut open = demo.is_open();
        ui.horizontal(|ui| {
            ui.checkbox(&mut open, "");
            if ui.button(demo.title()).clicked() {
                open = true;
            }
        });
        demo.set_open(open);
    }
}

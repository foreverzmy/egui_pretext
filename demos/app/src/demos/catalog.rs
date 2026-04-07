use eframe::egui;

use crate::demos::DemoWindow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CatalogInteraction {
    pub hovered_demo_id: Option<&'static str>,
    pub opened_demo_id: Option<&'static str>,
}

pub fn show_catalog(ui: &mut egui::Ui, demos: &mut [Box<dyn DemoWindow>]) -> CatalogInteraction {
    let mut interaction = CatalogInteraction::default();
    for demo in demos {
        let was_open = demo.is_open();
        let mut open = was_open;
        let demo_id = demo.id();
        ui.horizontal(|ui| {
            let checkbox = ui.checkbox(&mut open, "");
            let button = ui.button(demo.title());
            if button.clicked() {
                open = true;
            }
            if checkbox.hovered() || button.hovered() {
                interaction.hovered_demo_id = Some(demo_id);
            }
        });
        demo.set_open(open);
        if !was_open && open {
            interaction.opened_demo_id = Some(demo_id);
        }
    }
    interaction
}

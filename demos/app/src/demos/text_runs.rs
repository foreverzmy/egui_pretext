use eframe::egui;
use egui::{Align2, Color32, FontId};
use pretext::{BidiDirection, LayoutLineVisualRun};

pub(crate) fn paint_visual_runs(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    fallback_text: &str,
    runs: &[LayoutLineVisualRun],
    font: &FontId,
    color: Color32,
) {
    if runs.is_empty() {
        painter.text(
            egui::pos2(x, y),
            Align2::LEFT_TOP,
            fallback_text,
            font.clone(),
            color,
        );
        return;
    }

    let mut offset = 0.0f32;
    for run in runs {
        if run.text.is_empty() {
            continue;
        }

        let (anchor, pos_x) = match run.direction {
            BidiDirection::Ltr => (Align2::LEFT_TOP, x + offset),
            BidiDirection::Rtl => (Align2::RIGHT_TOP, x + offset + run.width),
        };
        painter.text(egui::pos2(pos_x, y), anchor, &run.text, font.clone(), color);
        offset += run.width;
    }
}

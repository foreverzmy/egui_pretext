use eframe::egui;
use egui::{Align2, Color32, FontFamily, FontId};
use pretext::{LayoutLineGlyphRun, TextStyleSpec};
use pretext_egui::AssetRegistry;

pub(crate) fn paint_glyph_runs(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    fallback_text: &str,
    glyph_runs: &[LayoutLineGlyphRun],
    style: &TextStyleSpec,
    line_height: f32,
    color: Color32,
    ctx: &egui::Context,
    engine: &pretext::PretextEngine,
    assets: &mut AssetRegistry,
) {
    paint_glyph_runs_with_fallback(
        painter,
        x,
        y,
        fallback_text,
        glyph_runs,
        style,
        line_height,
        color,
        ctx,
        engine,
        assets,
    );
}

pub(crate) fn paint_glyph_runs_with_fallback(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    fallback_text: &str,
    glyph_runs: &[LayoutLineGlyphRun],
    style: &TextStyleSpec,
    line_height: f32,
    color: Color32,
    ctx: &egui::Context,
    engine: &pretext::PretextEngine,
    assets: &mut AssetRegistry,
) {
    if assets.paint_line_glyph_runs(
        painter,
        x,
        y,
        glyph_runs,
        style,
        line_height,
        color,
        ctx,
        engine,
    ) {
        return;
    }

    if fallback_text.is_empty() {
        return;
    }

    painter.text(
        egui::pos2(x, y),
        Align2::LEFT_TOP,
        fallback_text,
        FontId::new(style.size_px, FontFamily::Proportional),
        color,
    );
}

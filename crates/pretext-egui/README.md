# pretext-egui

`egui` painting helpers for layouts produced by `pretext`.

`pretext-egui` provides an `EguiPretextRenderer`, paragraph paint options, a paragraph widget, glyph
atlas-backed painting, and texture rasterization utilities. It is designed for apps that want to
measure text with `pretext` and paint the resulting glyph runs inside `egui`.

## Install

```toml
[dependencies]
pretext = "0.1.0"
pretext-egui = "0.1.0"
```

## Paint a paragraph in an `egui::Ui`

```rust,no_run
use pretext::{PretextEngine, PretextParagraphOptions, PretextStyle};
use pretext_egui::{EguiPretextPaintOptions, EguiPretextRenderer};

fn draw_pretext(ui: &mut egui::Ui, renderer: &mut EguiPretextRenderer) {
    let engine = PretextEngine::builder().build();
    let style = PretextStyle {
        families: vec!["Arial".to_owned()],
        size_px: 18.0,
        weight: 400,
        italic: false,
    };
    let prepared = engine.prepare_paragraph(
        "Hello from pretext-egui.",
        &style,
        &PretextParagraphOptions::default(),
    );
    let layout = engine.layout_paragraph(&prepared, ui.available_width(), 24.0);
    let paint = EguiPretextPaintOptions::new(&style, 24.0).color(egui::Color32::WHITE);

    renderer.paint_paragraph(ui.painter(), ui.min_rect().min, &layout, &paint, ui.ctx(), &engine);
}
```

## Use the widget adapter

```rust,no_run
# use pretext::{PretextEngine, PretextParagraphOptions, PretextStyle};
# use pretext_egui::EguiPretextRenderer;
# fn show(ui: &mut egui::Ui, renderer: &mut EguiPretextRenderer) {
# let engine = PretextEngine::builder().build();
# let style = PretextStyle { families: vec!["Arial".to_owned()], size_px: 18.0, weight: 400, italic: false };
# let prepared = engine.prepare_paragraph("Widget text", &style, &PretextParagraphOptions::default());
# let layout = engine.layout_paragraph(&prepared, ui.available_width(), 24.0);
ui.add(
    renderer
        .paragraph(&layout, &style, 24.0, &engine)
        .desired_width(ui.available_width())
        .color(egui::Color32::WHITE),
);
# }
```

## Fonts and assets

`pretext-egui` includes a small sample font/SVG asset set under `experimental::demo_assets` for
examples and tests. Production apps should normally provide their own fonts to `PretextEngine` and
configure their own `egui` font stack.

## Advanced rendering

Use `pretext_egui::advanced` only when you need glyph-scene batching, atlas warmup, positioned run
painting, or SVG emoji overlay helpers. The stable path is `EguiPretextRenderer` plus
`EguiPretextPaintOptions`.

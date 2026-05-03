# pretext-render

Shared rasterization helpers for Pretext text layouts.

`pretext-render` converts shaped text from `pretext` into alpha-mask rasters and exposes baseline
metrics that renderer crates can reuse. Most apps should use `pretext` for layout and a renderer
crate such as `pretext-egui`; use this crate when you are building your own rendering backend.

## Install

```toml
[dependencies]
pretext = "0.1.0"
pretext-render = "0.1.0"
```

## Rasterize a shaped text run

```rust
use pretext::{BidiDirection, PretextEngine, PretextStyle};
use pretext_render::{BaselineMode, TextRasterRequest, TextRenderCache};

let engine = PretextEngine::builder().build();
let style = PretextStyle {
    families: vec!["Arial".to_owned()],
    size_px: 18.0,
    weight: 400,
    italic: false,
};
let request = TextRasterRequest {
    text: "Hello, raster cache.",
    style: &style,
    direction: BidiDirection::Ltr,
    slot_height: 24.0,
    padding_x: 2.0,
    padding_y: 2.0,
    slack_x: 0.0,
    slack_y: 0.0,
    baseline_mode: BaselineMode::AutoFontMetrics,
};

let cache = TextRenderCache::default();
let rasterized = cache
    .rasterized_text(&engine, request, 1.0)
    .expect("text should rasterize with an available font");

let size = rasterized.pixel_size();
let alpha = rasterized.alpha_pixels();
assert_eq!(alpha.len(), size[0] * size[1]);
```

## Baseline metrics

```rust
# use pretext::{BidiDirection, PretextEngine, PretextStyle};
# use pretext_render::{BaselineMode, TextRasterRequest, text_baseline_metrics};
# let engine = PretextEngine::builder().build();
# let style = PretextStyle { families: vec!["Arial".to_owned()], size_px: 18.0, weight: 400, italic: false };
let request = TextRasterRequest {
    text: "Align me",
    style: &style,
    direction: BidiDirection::Ltr,
    slot_height: 24.0,
    padding_x: 0.0,
    padding_y: 0.0,
    slack_x: 0.0,
    slack_y: 0.0,
    baseline_mode: BaselineMode::AutoFontMetrics,
};
let metrics = text_baseline_metrics(&engine, request);
assert!(metrics.content_height_px() > 0.0);
```

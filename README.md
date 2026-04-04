# pretext-rs

Rust + `egui` native desktop implementation of the Pretext text-layout demos, using `egui = 0.33.3`.

This workspace follows [plan.md](/Users/bytedance/Workspace/demos/pretext/plan.md) as the implementation source of truth.

## Workspace

- `crates/pretext`
  Stable layout SDK: paragraph preparation, measurement, layout, bidi, shaping, and runtime stats.
- `crates/pretext-egui`
  Stable `egui` renderer SDK: paragraph painting, glyph atlas, and texture rasterization. `advanced` contains low-level rendering helpers; `experimental` contains demo-only bundled assets.
- `demos/app`
  `eframe` desktop demo shell with the catalog, accordion, bubbles, rich note, masonry, variable typographic ASCII, dynamic layout, and editorial engine demos.

## Stable SDK

Use the root exports for the standard path:

- `pretext::PretextEngine::builder()`
- `pretext::PretextStyle`
- `pretext::PretextParagraphOptions`
- `pretext::PretextPreparedParagraph`
- `pretext_egui::EguiPretextRenderer`
- `pretext_egui::EguiPretextPaintOptions`

Use `pretext::advanced` or `pretext_egui::advanced` only for cursor-driven layout, obstacle flow, glyph-scene painting, or custom shaping/rendering. For lower-level `pretext` flows, `pretext::advanced::{prepare_text, measure_text, layout_lines}` keeps those entry points out of the stable root path. For renderer internals, `pretext_egui::advanced` exposes helpers such as `paint_line_glyph_runs`, `enqueue_atlas_warmup`, `tick_atlas_warmup`, and the glyph-scene builders. Demo fonts, SVG logos, and bundled emoji assets live under `pretext_egui::experimental::demo_assets`.

Minimal engine example:

```rust
use pretext::{
    PretextEngine, PretextParagraphOptions, PretextStyle, WhiteSpaceMode,
};

let engine = PretextEngine::builder().build();
let style = PretextStyle {
    families: vec!["Times New Roman".to_owned(), "Arial".to_owned()],
    size_px: 18.0,
    weight: 400,
    italic: false,
};
let options = PretextParagraphOptions {
    white_space: WhiteSpaceMode::Normal,
    ..Default::default()
};

let prepared = engine.prepare_paragraph("Hello, pretext.", &style, &options);
let metrics = prepared.measure(&engine, 320.0, 24.0);
let layout = prepared.layout(&engine, 320.0, 24.0);

assert!(metrics.line_count >= 1);
assert_eq!(layout.line_count, metrics.line_count);
```

Minimal `egui` example:

```rust
use pretext_egui::{EguiPretextPaintOptions, EguiPretextRenderer};

let mut renderer = EguiPretextRenderer::default();
let paint = EguiPretextPaintOptions::new(&style, 24.0);

renderer.paint_paragraph(ui.painter(), ui.min_rect().min, &layout, &paint, ui.ctx(), &engine);
```

## Run

```bash
cargo run -p pretext-demo-app
```

## Test

```bash
cargo test --all
```

The workspace includes:

- engine unit tests for shaping, fallback, whitespace, segmentation, line breaking, bidi, and layout parity
- demo logic tests for accordion, bubbles, masonry, rich note, variable ASCII, dynamic layout, and editorial obstacle reflow
- headless smoke coverage for opening every demo window
- 12 JSON golden fixtures under `crates/pretext/tests/goldens`, including the editorial obstacle scene

## Update Goldens

Refresh all 12 goldens:

```bash
UPDATE_GOLDENS=1 cargo test -p pretext --test goldens
```

## Bundled Assets

Bundled fonts live in `demos/app/assets/fonts`.

- `NotoSans-Regular.ttf`
- `NotoSansArabic-Regular.ttf`
- `NotoSansCJK-Regular.ttc`
- `NotoSansMyanmar-Regular.ttf`
- `NotoSansMono-Regular.ttf`
- `Noto-COLRv1.ttf`

Bundled emoji SVG assets also live under `demos/app/assets`, currently including `emoji_u1f680.svg` for the accordion demo and `emoji_u1f389.svg` for the bubbles demo.

Emoji rendering uses `Noto-COLRv1.ttf` from `googlefonts/noto-emoji` in both the engine font bundle and the `egui` UI font stack. Source notes live in `demos/app/assets/fonts/SOURCES.md`.

SVG logo assets live in `demos/app/assets/logos` and are used both for texture upload and alpha-hull extraction.

# pretext-rs

Rust + `egui` native desktop implementation of the Pretext text-layout demos, using `egui = 0.33.3`.

Current public names in [`crates/pretext/src/lib.rs`](crates/pretext/src/lib.rs), [`crates/pretext-egui/src/lib.rs`](crates/pretext-egui/src/lib.rs), and [`skills/egui-pretext-layout/references/`](skills/egui-pretext-layout/references/) are the implementation authority. [`plan.md`](plan.md) is the architecture record and should be refreshed when it drifts.

When diffing against the upstream TypeScript reference under `pretext_js/`, use [`docs/upstream-map.md`](docs/upstream-map.md) as the module/API correspondence guide.

## Workspace

- `crates/pretext`
  Stable layout SDK: paragraph preparation, measurement, layout, bidi, shaping, runtime stats, and the `rich_inline` helper for inline-only mixed-style flow.
- `crates/pretext-render`
  Shared shaping-backed rasterization helpers used for text textures and other non-egui rendering paths.
- `crates/pretext-egui`
  Stable `egui` renderer SDK: paragraph painting, glyph atlas, and texture rasterization. `advanced` contains low-level rendering helpers; `experimental` contains demo-only bundled assets.
- `demos/app`
  `eframe` desktop demo shell with the catalog, accordion, bubbles, markdown chat, rich note, masonry, dynamic layout, dragon-through-text, editorial engine, justification algorithms, and variable typographic ASCII demos.

## Stable SDK

Use the root exports for the standard path:

- `pretext::PretextEngine::builder()`
- `pretext::PretextStyle`
- `pretext::PretextParagraphOptions`
- `pretext::PretextPreparedParagraph`
- `pretext::WordBreakMode`
- `pretext::rich_inline`
- `pretext_egui::EguiPretextRenderer`
- `pretext_egui::EguiPretextPaintOptions`

Use `pretext::advanced` or `pretext_egui::advanced` only for cursor-driven layout, obstacle flow, glyph-scene painting, or custom shaping/rendering. For inline-only rich text with mixed styles, boundary whitespace collapse, or atomic embedded items, start with `pretext::rich_inline::*`. For lower-level `pretext` flows, `pretext::advanced::{prepare_text, measure_text, layout_lines, measure_line_geometry, measure_natural_width, layout_next_line_range}` keeps those entry points out of the stable root path while staying close to the upstream JS geometry helpers. For renderer internals, `pretext_egui::advanced` exposes helpers such as `paint_line_glyph_runs`, `enqueue_atlas_warmup`, `tick_atlas_warmup`, and the glyph-scene builders. Demo fonts, SVG logos, and bundled emoji assets live under `pretext_egui::experimental::demo_assets`.

Prefer `..Default::default()` when constructing `PretextParagraphOptions`; that keeps examples resilient as fields such as `word_break` evolve. Reach for `WordBreakMode::KeepAll` only when no-space CJK-led text should remain cohesive.

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
let geometry = prepared.measure_line_geometry(&engine, 320.0);
let natural_width = prepared.measure_natural_width(&engine);

assert!(metrics.line_count >= 1);
assert_eq!(layout.line_count, metrics.line_count);
assert_eq!(geometry.line_count, layout.line_count);
assert!(natural_width >= geometry.max_line_width);
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
- rich-inline helper tests inside `crates/pretext`
- demo logic tests for accordion, bubbles, markdown chat, rich note, masonry, dynamic layout, dragon-through-text, editorial engine, justification algorithms, and variable ASCII
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
- `NotoEmoji-Regular.ttf`
- `NotoColorEmoji.ttf`
- `NotoSansMono-Regular.ttf`
- `Noto-COLRv1.ttf`

Bundled emoji SVG assets also live under `demos/app/assets`, currently including `emoji_u1f680.svg`, `emoji_u1f389.svg`, and `emoji_u2705.svg`.

Emoji rendering uses a layered local stack of `NotoEmoji-Regular.ttf`, `NotoColorEmoji.ttf`, and `Noto-COLRv1.ttf` in both the engine font bundle and the `egui` UI font stack. Source notes live in `demos/app/assets/fonts/SOURCES.md`.

SVG logo assets live in `demos/app/assets/logos` and are used both for texture upload and alpha-hull extraction.

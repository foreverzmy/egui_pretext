# pretext

Native Unicode paragraph preparation and layout for the Pretext Rust workspace.

`pretext` owns font discovery, shaping, bidi resolution, whitespace handling, line breaking,
paragraph measurement, and reusable layout caches. It is renderer-agnostic: use it directly for
measurement or pair it with `pretext-egui` for painting in `egui`.

## Install

```toml
[dependencies]
pretext = "0.1.0"
```

## Basic paragraph layout

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

## Font setup

By default, `PretextEngine::builder().build()` includes system fonts. For deterministic output,
provide font bytes and disable system font fallback:

```rust,no_run
use pretext::PretextEngine;

let font_data = vec![std::fs::read("NotoSans-Regular.ttf")?];
let engine = PretextEngine::builder()
    .with_font_data(font_data)
    .include_system_fonts(false)
    .build();
# Ok::<(), std::io::Error>(())
```

## Lower-level APIs

- Use root exports such as `PretextEngine`, `PretextStyle`, and `PretextParagraphOptions` for the stable path.
- Use `pretext::rich_inline` for inline-only mixed-style flow with atomic placeholders.
- Use `pretext::advanced` for cursor-driven line continuation, glyph runs, and custom renderers.

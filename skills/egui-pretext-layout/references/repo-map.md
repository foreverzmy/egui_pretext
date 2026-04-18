# Repo Map

Use this file to find the nearest working pattern before changing layout code.

Start here before relying on `plan.md`. If the plan disagrees with current code, follow the current code and refresh the docs.

## Public API boundaries

### `crates/pretext/src/lib.rs`

Read this first when you need the stable SDK names:

- `PretextEngine::builder`
- `PretextStyle`
- `PretextParagraphOptions`
- `PretextPreparedParagraph`
- `PretextParagraphMetrics`
- `PretextParagraphLayout`

Use this file when deciding whether the code should stay on the stable root path or move into `pretext::advanced`.

### `crates/pretext/src/advanced.rs`

Read this when lower-level text flow is actually required:

- `LayoutCursor`
- `PreparedText`
- `prepare_text`
- `measure_text`
- `layout_lines`
- `measure_line_geometry`
- `measure_natural_width`
- `layout_next_line_range`
- shaping and font exports

Use this file for cursor-driven continuation, range-only streaming, line-only geometry helpers, low-level line payloads, custom shaping inspection, or debugging.

### `crates/pretext/src/rich_inline.rs`

Read this when the job is inline-only rich text rather than full block layout:

- boundary whitespace collapse
- atomic inline items and inline chrome
- width-only stats for virtualized UIs
- line-range walking without DOM reads

Use this file for markdown-ish chat messages, inline code pills, or mixed-style flowing fragments that should still behave like one line box stream.

### `crates/pretext/src/engine.rs`

Read this when behavior depends on cache keys, cursor advancement, paragraph convenience methods, or advanced engine entry points.

Use this file to understand:

- `prepare_paragraph`, `measure_paragraph`, `layout_paragraph`
- `measure_line_geometry`, `measure_natural_width`
- `walk_line_ranges`
- `layout_next_line`, `layout_next_line_with_glyph_runs`, `layout_next_line_with_runs`
- `line_visual_runs`, `line_glyph_runs`, `line_runs`
- `prepare_atomic_placeholder`
- `prefix_widths`
- paragraph cache bucketing and runtime stats

### `crates/pretext/src/layout.rs`

Read this when behavior depends on cache keys, line fitting, cursor advancement, or layout determinism.

Use this file to understand:

- paragraph cache bucketing
- line-fit behavior
- cursor advancement and EOF handling
- where repeated layout calls are amortized

### `crates/pretext-egui/src/lib.rs`

Read this first when deciding how stable `pretext` output becomes `egui` paint commands.

Important entry points:

- `EguiPretextRenderer`
- `EguiPretextParagraph`
- `EguiPretextPaintOptions`
- `paint_paragraph`
- `paragraph`
- `rasterize_text_texture`
- `stats`

Use this file when the stable renderer path may be enough.

### `crates/pretext-egui/src/advanced.rs`

Read this when the task is really about lower-level painting:

- `paint_positioned_text_runs`
- `paint_styled_positioned_text_runs`
- `PretextFragmentPainter`
- `paint_line_glyph_runs`
- `enqueue_atlas_warmup`
- `tick_atlas_warmup`
- glyph-scene and emoji-overlay helpers

### `crates/pretext-egui/src/experimental.rs`

Read this when the task is demo bootstrapping or bundled assets:

- `demo_assets::bundled_font_data`
- `demo_assets::install_demo_fonts`
- bundled SVG and emoji asset helpers

Use this file only for demo and test asset plumbing.

### `crates/pretext-egui/src/glyph_atlas.rs`

Read this when the problem is really about atlas lifetime, glyph upload behavior, baseline calculation, mesh batching, or color glyph rasterization.

### `crates/pretext-render/src/lib.rs`

Read this when the change is really about shaped text textures, rasterized vertical extents, or color-glyph baseline behavior outside the egui paragraph path.

## App shell

### `demos/app/src/app.rs`

Read this when the change touches global engine setup, bundled fonts, atlas warmup, perf counters, or app-level caching strategy.

Use this file as the pattern for:

- constructing `PretextEngine`
- constructing and retaining `EguiPretextRenderer`
- bootstrapping bundled demo fonts
- priming fonts
- scheduling demo warmup steps and loading shells
- scheduling atlas warmup
- controlling system-font loading without surprise cache invalidation
- showing perf HUD counters

## Demo patterns

### `demos/app/src/demos/accordion.rs`

Read this for:

- standard paragraph widgets and explicit paragraph painting
- pairing paragraph measurement with paragraph layout
- width-quantized disclosure caches
- shaped-text texture regression tests on mixed-script content

### `demos/app/src/demos/bubbles.rs`

Read this for:

- shrinkwrap width search
- `walk_line_ranges` usage
- paragraph-level rendering through `paint_paragraph`
- bubble sizing driven by text geometry

### `demos/app/src/demos/rich_note.rs`

Read this for:

- `pretext::rich_inline` in a real demo
- inline item modeling
- `prepare_atomic_placeholder`
- mixed inline flow plus chip placeholders
- emoji overlay splitting
- mixed fragment painting
- styled positioned runs
- chip-like non-breaking content in flowing text

### `demos/app/src/demos/markdown_chat.rs`

Read this for:

- `pulldown-cmark` driven markdown-ish content
- `pretext::rich_inline` for inline code, links, and image chips
- height measurement before paint
- viewport virtualization without DOM reads
- ordered visible-range materialization
- staged cold-start warmup for template prep and the default conversation frame

### `demos/app/src/demos/masonry.rs`

Read this for:

- pre-measuring card heights with `prepared.measure(...)`
- layout decisions driven by text height
- positioned text runs inside card layouts

### `demos/app/src/demos/justification_algorithms.rs`

Read this for:

- glyph-run aware layout analysis
- positioned text painting with explicit metrics
- comparing different line-breaking or spacing strategies

### `demos/app/src/demos/variable_typographic_ascii.rs`

Read this for:

- mixed styles on positioned runs
- glyph-aware visual treatments where text is part of a custom scene
- incremental palette warmup instead of synchronous first-open palette builds

### `demos/app/src/demos/dynamic_layout.rs`

Read this for:

- obstacle-aware layout
- `layout_next_line_with_runs`
- `PretextFragmentPainter` inside a custom paint pipeline
- mixing SVG-driven geometry, demo assets, and text reflow
- staged warmup of text prep, hulls, layout, geometry, and projection

### `demos/app/src/demos/editorial_engine.rs`

Read this for the most advanced flow pattern:

- multi-column continuation using cursors
- incremental reflow
- obstacle bucketing
- separating animation updates from relayout triggers
- explicit body-column planning
- staged warmup of static text/layout state, textures, and dynamic body projection

Use this file whenever a task sounds like "text should continue around or between things."

### `demos/app/src/demos/dragon_through_text.rs`

Read this when text must interact with a moving visual object and glyph runs must still remain explicit.

## How to choose a reference quickly

- Need a standard paragraph widget or measured disclosure content: start with `accordion.rs`
- Need plain paragraph geometry or shrinkwrap: start with `bubbles.rs`; use `crates/pretext/src/engine.rs` first if aggregate geometry helpers may be enough
- Need height-only card measurement: start with `masonry.rs`
- Need inline-only rich flow primitives: start with `crates/pretext/src/rich_inline.rs`
- Need markdown-like chat bubbles or virtualized rich messages: start with `markdown_chat.rs`
- Need inline chips or mixed text fragments: start with `rich_note.rs`
- Need explicit positioned runs or per-line run extraction: start with `justification_algorithms.rs` and `crates/pretext/src/engine.rs`
- Need obstacle-aware or animated wrap: start with `dynamic_layout.rs`
- Need cursor-driven multi-column flow: start with `editorial_engine.rs`
- Need text reacting to a moving visual object: start with `dragon_through_text.rs`

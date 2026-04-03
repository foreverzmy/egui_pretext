# Repo Map

Use this file to find the nearest working pattern before changing layout code.

## Core crates

### `crates/pretext/src/engine.rs`

Read this first when you need the public text API surface:

- `prepare` and `prepare_with_segments`
- `layout`, `layout_with_lines`, `layout_with_runs`
- `walk_line_ranges`
- `layout_next_line`, `layout_next_line_with_glyph_runs`, `layout_next_line_with_runs`
- `prepare_atomic_placeholder`
- `prefix_widths`

Use this file when choosing the minimal layout API for a task.

### `crates/pretext/src/layout.rs`

Read this when behavior depends on cache keys, line fitting, cursor advancement, or layout determinism.

Use this file to understand:

- paragraph cache bucketing
- line-fit behavior
- cursor advancement and EOF handling
- where repeated layout calls are amortized

### `crates/pretext-egui/src/lib.rs`

Read this when deciding how `pretext` output becomes `egui` paint commands.

Important entry points:

- `AssetRegistry`
- `PretextParagraphLayout`
- `PretextParagraph`
- `PretextFragmentPainter`
- `paint_pretext_paragraph`
- `paint_positioned_text_runs`
- `paint_styled_positioned_text_runs`
- `shaped_text_texture`
- atlas warmup helpers

### `crates/pretext-egui/src/glyph_atlas.rs`

Read this when the problem is really about atlas lifetime, glyph upload behavior, baseline calculation, mesh batching, or color glyph rasterization.

## App shell

### `demos/app/src/app.rs`

Read this when the change touches global engine setup, bundled fonts, atlas warmup, perf counters, or app-level caching strategy.

Use this file as the pattern for:

- constructing `PretextEngine`
- constructing and retaining `AssetRegistry`
- priming fonts
- scheduling atlas warmup
- showing perf HUD counters

## Demo patterns

### `demos/app/src/demos/bubbles.rs`

Read this for:

- shrinkwrap width search
- `walk_line_ranges` usage
- paragraph-level rendering through `PretextParagraphLayout`
- bubble sizing driven by text geometry

### `demos/app/src/demos/rich_note.rs`

Read this for:

- inline item modeling
- `prepare_atomic_placeholder`
- mixed fragment painting
- styled positioned runs
- chip-like non-breaking content in flowing text

### `demos/app/src/demos/masonry.rs`

Read this for:

- pre-measuring card heights before placement
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

### `demos/app/src/demos/dynamic_layout.rs`

Read this for:

- obstacle-aware layout
- `layout_next_line_with_runs`
- `PretextFragmentPainter` inside a custom paint pipeline
- mixing SVG-driven geometry and text reflow

### `demos/app/src/demos/editorial_engine.rs`

Read this for the most advanced flow pattern:

- multi-column continuation using cursors
- incremental reflow
- obstacle bucketing
- separating animation updates from relayout triggers
- explicit body-column planning

Use this file whenever a task sounds like "text should continue around or between things."

### `demos/app/src/demos/dragon_through_text.rs`

Read this when text must interact with a moving visual object and glyph runs must still remain explicit.

## How to choose a reference quickly

- Need plain paragraph geometry or shrinkwrap: start with `bubbles.rs`
- Need inline chips or mixed text fragments: start with `rich_note.rs`
- Need measured card heights: start with `masonry.rs`
- Need explicit positioned runs: start with `justification_algorithms.rs`
- Need obstacle-aware or animated wrap: start with `dynamic_layout.rs`
- Need cursor-driven multi-column flow: start with `editorial_engine.rs`

# Patterns

Prefer current public names in new code:

- `pretext::*` root for standard paragraph prep, measurement, and layout
- `pretext::rich_inline` for inline-only mixed-style flow
- `pretext::advanced` for cursor-driven or lower-level text flow
- `pretext_egui::*` root for stable paragraph rendering and text textures
- `pretext_egui::advanced` for positioned runs, fragment painters, glyph scenes, warmup, and emoji overlays
- `pretext_egui::experimental::demo_assets` only for demos and tests

## API Choice Matrix

| Need | Use | Avoid |
| --- | --- | --- |
| Only total height or line count | `prepared.measure(...)` or `engine.measure_paragraph(...)` | hidden root `layout(...)` or full run layouts |
| Need full paragraph layout with visual or glyph runs | `prepared.layout(...)` or `engine.layout_paragraph(...)` | reconstructing runs later |
| Need inline-only rich text with mixed styles, atomic items, boundary whitespace collapse, or extra chrome | `pretext::rich_inline::*` | hand-rolled inline flow in `egui` or repeated `layout_next_line*` glue code |
| Need stable lines as strings and widths without full run payloads | `pretext::advanced::layout_lines(...)` | manual splitting in `egui` |
| Need aggregate line geometry for one known width | `prepared.measure_line_geometry(...)` or `pretext::advanced::measure_line_geometry(...)` | walking every line just to recover `line_count` or `max_line_width` |
| Need natural unwrapped width | `prepared.measure_natural_width(...)` or `pretext::advanced::measure_natural_width(...)` | fake infinite-width layout probes unless you truly need per-line callbacks |
| Need width search or inspect every line width | `walk_line_ranges` | repeatedly calling full render paths |
| Need exact line ranges without line text or run payloads | `prepared.layout_next_line_range(...)` or `pretext::advanced::layout_next_line_range(...)` | full line or run materialization |
| Need continuation across slots or columns | `layout_next_line*` with `pretext::advanced::LayoutCursor` | slicing rendered text afterward |
| Need visual or glyph runs for one known line | `engine.line_visual_runs(...)`, `engine.line_glyph_runs(...)`, or `engine.line_runs(...)` | recomputing the whole paragraph layout when you already have `LayoutLine` |
| Need an unbreakable inline element | `prepare_atomic_placeholder` | embedding fake spaces or punctuation |
| Need per-grapheme offsets | `prefix_widths` | guessing from byte indices |
| Need no-space CJK-led text to stay cohesive | `WordBreakMode::KeepAll` | hard-coded punctuation merges in app code |
| Need bundled demo fonts or SVG assets | `pretext_egui::experimental::demo_assets::*` | baking demo-only helpers into generic library code |

## Rendering Choice Matrix

| Output shape | Use |
| --- | --- |
| One paragraph widget in layout flow | `EguiPretextRenderer::paragraph(...)` + `ui.add(...)` or `EguiPretextParagraph::new(...)` |
| One paragraph with an explicit paint origin | `EguiPretextRenderer::paint_paragraph(...)` |
| Many positioned lines with one style | `pretext_egui::advanced::paint_positioned_text_runs(...)` |
| Many positioned lines with mixed styles | `pretext_egui::advanced::paint_styled_positioned_text_runs(...)` |
| Streaming fragments mixed with other custom paint ops | `pretext_egui::advanced::PretextFragmentPainter` |
| Explicit texture object for later reuse or composition | `EguiPretextRenderer::rasterize_text_texture(...)` |
| Atlas warmup, direct line painting, or glyph-scene control | `pretext_egui::advanced::*` |

Default to atlas-backed paragraph or positioned-run painting. Treat `rasterize_text_texture(...)` as a special-purpose path, not the baseline paragraph renderer.

## Recipe: Standard Paragraph in egui

1. Build or reuse a `PretextPreparedParagraph` with `prepare_paragraph`.
2. Compute a `PretextParagraphLayout` from the current wrap width with `prepared.layout(...)` or `engine.layout_paragraph(...)`.
3. Allocate space in `egui`.
4. Paint with `assets.paragraph(...)` or `assets.paint_paragraph(...)`.

Use this for widgets that still behave like paragraphs but require `pretext` line breaking or fallback behavior.

## Recipe: Measured Disclosure or Accordion Section

1. Prepare section copy once.
2. Use `prepared.measure(...)` when only height is needed, or pair it with `prepared.layout(...)` when paint data is also required.
3. Cache width-quantized paragraph results.
4. Paint with `assets.paint_paragraph(...)`.

Read `accordion.rs` before implementing this pattern.

## Recipe: Shrinkwrap Bubble or Caption

1. Prepare text once.
2. If the width is fixed and you only need aggregate geometry, start with `prepared.measure_line_geometry(...)`.
3. Use `walk_line_ranges` only when you are searching widths or need every line width.
4. Search widths until line count changes, or compute the max line width directly for the current wrap result.
5. Size the bubble from text geometry, then paint the paragraph.

Read `bubbles.rs` before implementing this pattern.

## Recipe: Reuse Existing Lines but Add Runs

1. Generate or cache `LayoutLine` values with `prepared.layout(...)`, `pretext::advanced::layout_lines(...)`, or `layout_next_line*`.
2. Materialize `engine.line_visual_runs(...)`, `engine.line_glyph_runs(...)`, or `engine.line_runs(...)` only for the lines you are about to inspect or paint.
3. Feed those runs into positioned-run painting, overlay placement, or custom metrics logic.

Use this when line geometry is already cached and a second full paragraph layout would be redundant.

## Recipe: Inline Chip or Non-Breaking Embedded Item

1. Measure the inline item text or chrome width.
2. Replace the item with `prepare_atomic_placeholder`.
3. Keep the rendered chip data in a parallel structure.
4. Drive line flow with `layout_next_line*`.
5. Paint the chip separately where the placeholder lands.

Read `rich_note.rs` before implementing this pattern.

## Recipe: Rich Inline Flow

1. Model the source as `RichInlineItemSpec` entries with one style per item.
2. Prepare once with `prepare_rich_inline(...)`.
3. Use `measure_rich_inline_stats(...)` or `walk_rich_inline_line_ranges(...)` when you need deterministic heights before paint.
4. Materialize only the visible line or block ranges you actually need.
5. Paint the resulting fragments with positioned-run helpers or a fragment painter.

Read `crates/pretext/src/rich_inline.rs` first.
Read `markdown_chat.rs` for virtualization and `rich_note.rs` for chip-heavy fragments.

## Recipe: Markdown-ish Chat Surface

1. Parse markdown into block and inline structures.
2. Convert inline content into `pretext::rich_inline` items.
3. Cache block metrics and visible ranges separately from paint state.
4. Measure before paint so virtualization decisions do not depend on DOM reads.
5. Fall back to plain code blocks or simplified renderers for unsupported markdown structures.

Read `markdown_chat.rs` before implementing this pattern.

## Recipe: Obstacle-Aware Flow

1. Keep `egui` in charge of the obstacle interaction state.
2. Quantize obstacle movement if full-frame relayout would be noisy.
3. For each band, column, or slot, compute the available horizontal intervals.
4. Use `layout_next_line_with_runs` to continue from the previous cursor.
5. Paint positioned results with `PretextFragmentPainter` or positioned-run helpers.

Read `dynamic_layout.rs` for single-region obstacles.
Read `editorial_engine.rs` for multi-column continuation and dirty-band recomputation.

## Recipe: Positioned Text in a Custom Scene

1. Compute explicit `(x, y)` origins from your scene geometry.
2. Use `prepared.layout(...)`, `pretext::advanced::layout_lines(...)`, or `layout_next_line*` to generate the line payload.
3. Convert to positioned run refs.
4. Paint with `pretext_egui::advanced::paint_positioned_text_runs(...)` or `pretext_egui::advanced::paint_styled_positioned_text_runs(...)`.

Use this when text is one element in a larger drawing, not the dominant widget tree element.

## Recipe: Warm the Glyph Atlas

1. Keep one long-lived `EguiPretextRenderer`.
2. Enqueue warmup buckets with `pretext_egui::advanced::enqueue_atlas_warmup(...)` from app startup or after a font revision change.
3. Tick warmup with `pretext_egui::advanced::tick_atlas_warmup(...)` during idle or startup frames with budgets.
4. Inspect `EguiPretextRenderer::stats()` before assuming rendering is slow for other reasons.

Use app-level warmup only when the same glyph families will clearly be used soon.

## Recipe: Demo Asset Bootstrap

1. Build the engine with `PretextEngine::builder().with_font_data(pretext_egui::experimental::demo_assets::bundled_font_data())`.
2. Call `pretext_egui::experimental::demo_assets::install_demo_fonts(ctx)` so egui fallback matches the bundled engine fonts.
3. Keep this pattern inside demos, fixtures, or tests unless the task is explicitly about bundled assets.

## Cache Boundaries

Keep these layers separate:

- Text preparation cache: invalidated by text, style, whitespace mode, locale, or direction changes
- Rich-inline flow cache: invalidated by text items, item styles, break modes, or extra inline chrome
- Layout cache: invalidated by width bucket or obstacle signature changes
- Asset cache: invalidated by engine/font revision, glyph size, or pixels-per-point changes

If a bug feels random, first check whether one of these boundaries is too coarse or too fine.

## Common Failure Modes

- Recreating `PretextPreparedParagraph` every frame even when text is static
- Re-implementing inline whitespace collapse or atomic-item flow instead of using `pretext::rich_inline`
- Recreating `EguiPretextRenderer` or paint scene state every update
- Pulling outdated names or hidden APIs from older commits into new code
- Hand-writing `PretextParagraphOptions` field lists instead of using `..Default::default()`, then drifting when fields such as `word_break` change
- Using `egui` text widgets when later code needs line widths or cursor continuation
- Choosing `prepared.layout(...)` when only total height is needed
- Using `walk_line_ranges` when `measure_line_geometry(...)` or `measure_natural_width(...)` already answers the question
- Recomputing full paragraph layout when `line_runs(...)` on an existing `LayoutLine` is enough
- Forgetting to quantize widths or obstacle positions, causing relayout churn
- Hiding fallback behavior and then wondering why some text disappears on atlas misses
- Using `rasterize_text_texture(...)` as the default paragraph renderer
- Coupling animation ticks directly to expensive reflow without dirty checks

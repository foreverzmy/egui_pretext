# Patterns

## API Choice Matrix

| Need | Use | Avoid |
| --- | --- | --- |
| Only total height or line count | `layout` | `layout_with_runs` |
| Need stable lines as strings and widths | `layout_with_lines` | manual splitting in `egui` |
| Need visual runs or glyph runs for paint | `layout_with_runs` | reconstructing runs later |
| Need width search or inspect every line width | `walk_line_ranges` | repeatedly calling full render paths |
| Need continuation across slots or columns | `layout_next_line*` | slicing rendered text afterward |
| Need an unbreakable inline element | `prepare_atomic_placeholder` | embedding fake spaces or punctuation |
| Need per-grapheme offsets | `prefix_widths` | guessing from byte indices |

## Rendering Choice Matrix

| Output shape | Use |
| --- | --- |
| One paragraph widget in layout flow | `PretextParagraphLayout::from_prepared` + `PretextParagraph::new` |
| Many positioned lines with one style | `paint_positioned_text_runs` |
| Many positioned lines with mixed styles | `paint_styled_positioned_text_runs` |
| Streaming fragments mixed with other custom paint ops | `PretextFragmentPainter` |
| Explicit texture object for later reuse or composition | `AssetRegistry::shaped_text_texture` |

Default to atlas-backed glyph-run painting. Treat `shaped_text_texture` as a special-purpose path, not the baseline paragraph renderer.

## Recipe: Standard Paragraph in egui

1. Build or reuse a `PreparedTextWithSegments`.
2. Compute a `PretextParagraphLayout` from the current wrap width.
3. Allocate space in `egui`.
4. Paint with `PretextParagraph::new`.

Use this for widgets that still behave like paragraphs but require `pretext` line breaking or fallback behavior.

## Recipe: Shrinkwrap Bubble or Caption

1. Prepare text once.
2. Use `walk_line_ranges` to inspect line widths.
3. Search widths until line count changes, or compute the max line width directly for the current wrap result.
4. Size the bubble from text geometry, then paint the paragraph.

Read `bubbles.rs` before implementing this pattern.

## Recipe: Inline Chip or Non-Breaking Embedded Item

1. Measure the inline item text or chrome width.
2. Replace the item with `prepare_atomic_placeholder`.
3. Keep the rendered chip data in a parallel structure.
4. Drive line flow with `layout_next_line*`.
5. Paint the chip separately where the placeholder lands.

Read `rich_note.rs` before implementing this pattern.

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
2. Use `layout_with_lines` or `layout_next_line*` to generate the line payload.
3. Convert to positioned run refs.
4. Paint with `paint_positioned_text_runs` or `paint_styled_positioned_text_runs`.

Use this when text is one element in a larger drawing, not the dominant widget tree element.

## Recipe: Warm the Glyph Atlas

1. Keep one long-lived `AssetRegistry`.
2. Enqueue warmup buckets from app startup or after a font revision change.
3. Tick warmup during idle or startup frames with budgets.
4. Inspect `AssetRegistryStats` before assuming rendering is slow for other reasons.

Use app-level warmup only when the same glyph families will clearly be used soon.

## Cache Boundaries

Keep these layers separate:

- Text preparation cache: invalidated by text, style, whitespace mode, locale, or direction changes
- Layout cache: invalidated by width bucket or obstacle signature changes
- Asset cache: invalidated by engine/font revision, glyph size, or pixels-per-point changes

If a bug feels random, first check whether one of these boundaries is too coarse or too fine.

## Common Failure Modes

- Recreating `PreparedTextWithSegments` every frame even when text is static
- Recreating `AssetRegistry` or paint scene state every update
- Using `egui` text widgets when later code needs line widths or cursor continuation
- Choosing `layout_with_runs` when only total height is needed
- Forgetting to quantize widths or obstacle positions, causing relayout churn
- Hiding fallback behavior and then wondering why some text disappears on atlas misses
- Coupling animation ticks directly to expensive reflow without dirty checks

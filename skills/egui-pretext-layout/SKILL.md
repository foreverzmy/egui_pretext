---
name: egui-pretext-layout
description: Implement, review, and refactor Rust UI code that combines egui containers, interaction, and painting with pretext text measurement, shaping, line breaking, or custom text rendering. Use when a task involves deciding whether layout belongs in egui or pretext; building custom paragraphs, shrinkwrap bubbles, inline atomic placeholders, obstacle-aware reflow, multi-column text flow, glyph-run painting, atlas warmup, or performance-sensitive text UI in this pretext workspace.
---

# egui + pretext Layout

Use this skill to separate responsibilities cleanly:

- Use `egui` for windows, panels, input, animation timing, scroll areas, texture placement, and generic widgets.
- Use `pretext` for text geometry: shaping, bidi, line breaking, cursor-driven flow, shrinkwrap measurement, inline atomic items, and stable line/runs data.
- Use `pretext-egui` to paint prepared `pretext` output into `egui` through paragraph widgets, positioned runs, fragment painters, or glyph atlas helpers.

If the task is mostly chrome or standard widgets, stay in `egui`. If the task needs exact line structure, cursor progression, wrap decisions, or obstacle-aware text flow, move the text logic into `pretext` early.

## Workflow

1. Read `references/repo-map.md` to find the closest existing pattern in this workspace before designing a new one.
2. Decide ownership first.
   - Keep container sizing, scrolling, drag state, animation clocks, and texture lifetime in `egui`.
   - Keep text preparation, line fitting, wrap decisions, bidi, and inline placeholder behavior in `pretext`.
3. Cache aggressively.
   - Reuse `PreparedText` or `PreparedTextWithSegments` across frames.
   - Reuse `AssetRegistry` across frames.
   - Treat text, style, width bucket, obstacle signature, and locale as reflow keys.
4. Choose the smallest `pretext` API that matches the job.
   - Height only: `layout`
   - Stable lines: `layout_with_lines`
   - Visual and glyph runs: `layout_with_runs`
   - Width search or bounds inspection: `walk_line_ranges`
   - Incremental flow, obstacles, or multi-column continuation: `layout_next_line*`
   - Non-breaking inline box: `prepare_atomic_placeholder`
   - Per-grapheme geometry or overlay alignment: `prefix_widths`
5. Choose the rendering path intentionally.
   - Whole paragraph widget: `PretextParagraphLayout` + `PretextParagraph`
   - Positioned lines with one style: `paint_positioned_text_runs`
   - Positioned lines with mixed styles: `paint_styled_positioned_text_runs`
   - Streaming fragments inside a larger paint pass: `PretextFragmentPainter`
   - Rasterized fragment texture: `shaped_text_texture` only when a texture is the actual output you need
6. Keep reflow and paint separate.
   - Update interactive state every frame if needed.
   - Recompute text layout only when width, text, style, or obstacles materially change.
   - Bucket noisy float inputs when repeated 1px changes would cause useless relayout.
7. Validate with deterministic tests and a visual pass.
   - Add or update unit tests when line counts, cursor progression, inline atomics, or obstacle wrapping change.
   - Run the demo app when the change affects actual rendering or interaction.

## Default Rules

- Prefer existing demo patterns over inventing a new architecture.
- Do not recreate `AssetRegistry` per frame.
- Do not default to `egui::Label` or `Painter::text` if later logic needs line geometry.
- Do not default to `shaped_text_texture` for paragraph rendering when atlas-backed glyph runs already fit the case.
- Do not mix interaction state and text layout state in one opaque cache; keep them separable.
- Do not recompute `prepare_with_segments` inside hot inner loops unless the text actually changed.
- Preserve fallback behavior through `fallback_font` and `fallback_align` whenever atlas painting may legitimately fail.

## Read References Selectively

- Read `references/repo-map.md` when you need file-level examples from this workspace.
- Read `references/patterns.md` when you need API selection guidance or implementation recipes.
- Read `references/validation.md` when you need test targets, visual checks, or golden update commands.

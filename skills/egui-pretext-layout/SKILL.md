---
name: egui-pretext-layout
description: Implement, review, and refactor Rust UI code that combines egui containers, interaction, and painting with pretext paragraph preparation, measurement, layout, rich inline flow, or custom text rendering. Use when a task involves choosing between the stable root APIs (`prepare_paragraph`, `PretextPreparedParagraph::measure/layout`, `EguiPretextRenderer`), the inline-only helper (`pretext::rich_inline`), and the advanced modules (`pretext::advanced`, `pretext_egui::advanced`); building custom paragraphs, shrinkwrap bubbles, markdown-ish chat surfaces, inline atomic placeholders, obstacle-aware reflow, multi-column text flow, glyph-run painting, atlas warmup, shaped-text textures, or demo-asset bootstrapping in this pretext workspace.
---

# egui + pretext Layout

Use this skill to separate responsibilities cleanly:

- Use `egui` for windows, panels, input, animation timing, scroll areas, texture placement, and generic widgets.
- Use `pretext` root exports for standard paragraph preparation, measurement, and layout.
- Use `pretext::rich_inline` for inline-only mixed-style flow, boundary whitespace collapse, atomic inline items, or extra inline chrome.
- Use `pretext::advanced` only when the task needs cursor-driven continuation, low-level line payloads, shaping details, or font internals.
- Use `pretext_egui` root exports for stable paragraph rendering, text textures, and renderer stats.
- Use `pretext_egui::advanced` for positioned runs, fragment painters, glyph-scene helpers, emoji overlays, or atlas warmup.
- Use `pretext_egui::experimental::demo_assets` only for demo and test asset bootstrapping in this workspace.

If the task is mostly chrome or standard widgets, stay in `egui`. If the task needs exact line structure, cursor progression, wrap decisions, or obstacle-aware text flow, move the text logic into `pretext` early.

## Workflow

1. Read `references/repo-map.md` to find the closest existing pattern in this workspace before designing a new one.
2. Decide ownership first.
   - Keep container sizing, scrolling, drag state, animation clocks, and texture lifetime in `egui`.
   - Keep text preparation, line fitting, wrap decisions, bidi, and inline placeholder behavior in `pretext`.
3. Cache aggressively.
   - Reuse `PretextPreparedParagraph` across frames.
   - Reuse one long-lived `EguiPretextRenderer` across frames.
   - Treat text, style, width bucket, obstacle signature, and locale as reflow keys.
4. Choose the smallest `pretext` API that matches the job.
   - Engine/bootstrap: `PretextEngine::builder()`; use `pretext_egui::experimental::demo_assets::{bundled_font_data, install_demo_fonts}` only in demos/tests.
   - Standard paragraph prep: `prepare_paragraph`
   - Height only: `prepared.measure(...)` or `measure_paragraph(...)`
   - Standard paragraph layout with visual and glyph runs: `prepared.layout(...)` or `layout_paragraph(...)`
   - Inline-only mixed-style flow with atomic spans or extra chrome: `pretext::rich_inline::*`
   - Stable lines without full run payloads: `pretext::advanced::layout_lines(...)`
   - Width search or bounds inspection: `walk_line_ranges`
   - Incremental flow, obstacles, or multi-column continuation: `layout_next_line*` with `pretext::advanced::LayoutCursor`
   - Non-breaking inline box: `prepare_atomic_placeholder`
   - Per-grapheme geometry or overlay alignment: `prefix_widths`
   - When constructing `PretextParagraphOptions`, prefer `..Default::default()` unless the task explicitly needs non-default `white_space`, `word_break`, or `paragraph_direction`
5. Choose the rendering path intentionally.
   - Whole paragraph widget in layout flow: `EguiPretextRenderer::paragraph(...)` plus `ui.add(...)`, or `EguiPretextParagraph::new(...)`
   - Whole paragraph with an explicit paint origin: `EguiPretextRenderer::paint_paragraph(...)`
   - Positioned lines with one style: `pretext_egui::advanced::paint_positioned_text_runs(...)`
   - Positioned lines with mixed styles: `pretext_egui::advanced::paint_styled_positioned_text_runs(...)`
   - Streaming fragments inside a larger paint pass: `pretext_egui::advanced::PretextFragmentPainter`
   - Rasterized text texture: `EguiPretextRenderer::rasterize_text_texture(...)` only when the texture itself is the output you need
   - Glyph-atlas warmup, emoji overlays, direct line painting, or glyph scenes: `pretext_egui::advanced::*`
6. Keep reflow and paint separate.
   - Update interactive state every frame if needed.
   - Recompute text layout only when width, text, style, or obstacles materially change.
   - Bucket noisy float inputs when repeated 1px changes would cause useless relayout.
7. Validate with deterministic tests and a visual pass.
   - Add or update unit tests when line counts, cursor progression, inline atomics, or obstacle wrapping change.
   - Run the demo app when the change affects actual rendering or interaction.

## Default Rules

- Prefer stable root exports and current public names in new code.
- Reach for `pretext::advanced` or `pretext_egui::advanced` only when the stable path no longer expresses the problem cleanly.
- Treat `pretext_egui::experimental` as demo-only. Do not leak it into generic library code unless the task is explicitly about bundled assets or fixtures.
- Prefer existing demo patterns over inventing a new architecture.
- If the task sounds like markdown chat, inline code chips, rich message bubbles, or virtualization without DOM reads, inspect `crates/pretext/src/rich_inline.rs`, `demos/app/src/demos/rich_note.rs`, and `demos/app/src/demos/markdown_chat.rs` before designing a custom flow.
- Do not import outdated names from older commits such as `AssetRegistry`, `PretextParagraph`, or hidden root `layout_with_*` entry points in new code.
- Do not recreate `EguiPretextRenderer` per frame.
- Do not default to `egui::Label` or `Painter::text` if later logic needs line geometry.
- Do not default to `rasterize_text_texture` for paragraph rendering when atlas-backed glyph runs already fit the case.
- Do not mix interaction state and text layout state in one opaque cache; keep them separable.
- Do not hand-write `PretextParagraphOptions` field lists unless you really need all of them; prefer `..Default::default()` so new fields such as `word_break` do not make examples stale.
- Do not recompute `prepare_paragraph` inside hot inner loops unless the text actually changed.
- Preserve fallback behavior through `fallback_font` and `fallback_align` whenever atlas painting may legitimately fail.
- Set `emoji_size` and `emoji_slot_height` intentionally whenever emoji or color glyph alignment is part of the output.

## Read References Selectively

- Read `references/repo-map.md` when you need file-level examples from this workspace.
- Read `references/patterns.md` when you need API selection guidance or implementation recipes.
- Read `references/validation.md` when you need test targets, visual checks, or golden update commands.

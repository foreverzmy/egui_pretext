# Validation

Use this checklist after changing layout code that mixes `egui` and `pretext`.

## Fast checks

1. Run targeted tests closest to the changed area.
2. Run `cargo test -p pretext` when paragraph prep, measurement, layout, or cache behavior changed.
3. Run `cargo test -p pretext-egui` when renderer root APIs, `advanced` helpers, glyph atlas, or text textures changed.
4. Run `cargo test -p pretext-demo-app` when demo logic, bundled asset bootstrapping, or egui integration changed.
5. Run `cargo test --all` if the change crosses crates or multiple demos.
6. Run `cargo run -p pretext-demo-app` for visual confirmation when interaction or paint output changed.

## Targeted test map

- Stable engine behavior: `cargo test -p pretext`
- Renderer APIs, glyph atlas, text texture rasterization, and warmup: `cargo test -p pretext-egui`
- Demo logic and integration: `cargo test -p pretext-demo-app`
- All workspace tests: `cargo test --all`
- Golden line fixtures: `cargo test -p pretext --test goldens`
- Intentional golden refresh: `UPDATE_GOLDENS=1 cargo test -p pretext --test goldens`

## What to assert

- `prepared.measure(...)` and `prepared.layout(...)` agree on height or line count when both are used for the same paragraph
- `measure_rich_inline_stats(...)` agrees with the materialized `pretext::rich_inline` layout when both are used for the same content
- Line count stays deterministic for the same input
- Cursor progression reaches EOF without hidden state
- Atomic placeholders never split across lines
- Obstacle changes alter only the intended regions
- `word_break` changes only the intended scripts or segments
- Atlas-backed paint paths still emit textured shapes when expected
- Texture or atlas caches reuse handles or entries on the second pass
- Virtualized visible ranges remain ordered and rebuild the correct padding when the viewport changes
- `EguiPretextRenderer::stats()` reflects the expected cache path instead of silent fallback-only rendering

## Visual checks

- Resize the window and confirm layout changes only when width meaningfully changes
- Exercise mixed-script text if the change touches shaping, fallback, or bidi
- Exercise CJK-led no-space text if the change touches `word_break` or segmentation
- Exercise emoji or color glyphs if the change touches atlas or fallback behavior
- Exercise drag or animation if the change touches obstacle-aware reflow
- Exercise virtualization or occlusion if the change touches markdown/chat-style scrolling surfaces
- If bundled demo assets are involved, install demo fonts first so `egui` fallback matches engine fonts

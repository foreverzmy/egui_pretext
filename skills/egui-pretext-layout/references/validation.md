# Validation

Use this checklist after changing layout code that mixes `egui` and `pretext`.

## Fast checks

1. Run targeted tests closest to the changed area.
2. Run `cargo test --all` if the change crosses crates or multiple demos.
3. Run `cargo run -p pretext-demo-app` for visual confirmation when interaction or paint output changed.

## Targeted test map

- Engine behavior: `cargo test -p pretext`
- All workspace tests: `cargo test --all`
- Golden line fixtures: `cargo test -p pretext --test goldens`
- Intentional golden refresh: `UPDATE_GOLDENS=1 cargo test -p pretext --test goldens`

## What to assert

- Line count stays deterministic for the same input
- Cursor progression reaches EOF without hidden state
- Atomic placeholders never split across lines
- Obstacle changes alter only the intended regions
- Atlas-backed paint paths still emit textured shapes when expected
- Texture or atlas caches reuse entries on the second pass

## Visual checks

- Resize the window and confirm layout changes only when width meaningfully changes
- Exercise mixed-script text if the change touches shaping, fallback, or bidi
- Exercise emoji or color glyphs if the change touches atlas or fallback behavior
- Exercise drag or animation if the change touches obstacle-aware reflow

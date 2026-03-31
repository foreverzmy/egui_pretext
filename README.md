# pretext-rs

Rust + `egui` native desktop implementation of the Pretext text-layout demos, using `egui = 0.33.3`.

This workspace follows [plan.md](/Users/bytedance/Workspace/demos/pretext/plan.md) as the implementation source of truth.

## Workspace

- `crates/pretext`
  Pure layout engine: whitespace analysis, bidi, font fallback, shaping, line breaking, and the four layout APIs.
- `demos/app`
  `eframe` desktop demo shell with the catalog, accordion, bubbles, rich note, masonry, variable typographic ASCII, dynamic layout, and editorial engine demos.

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
- demo logic tests for accordion, bubbles, masonry, rich note, variable ASCII, dynamic layout, and editorial obstacle reflow
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
- `NotoSansMono-Regular.ttf`
- `Noto-COLRv1.ttf`

Bundled emoji SVG assets also live under `demos/app/assets`, currently including `emoji_u1f680.svg` for the accordion demo and `emoji_u1f389.svg` for the bubbles demo.

Emoji rendering uses `Noto-COLRv1.ttf` from `googlefonts/noto-emoji` in both the engine font bundle and the `egui` UI font stack. Source notes live in `demos/app/assets/fonts/SOURCES.md`.

SVG logo assets live in `demos/app/assets/logos` and are used both for texture upload and alpha-hull extraction.

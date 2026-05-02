# AGENTS.md

## Project Overview

This repository is `pretext-rs`, a Rust workspace that ports the upstream TypeScript Pretext text-layout demos to a native desktop implementation using `egui = 0.33.3` / `eframe = 0.33.3`.

The current implementation authority is, in order:

1. Public names exported from `crates/pretext/src/lib.rs` and `crates/pretext-egui/src/lib.rs`.
2. Reference notes under `skills/egui-pretext-layout/references/`.
3. `plan.md` as the architecture record and migration background.

If documentation examples conflict with code, follow the current code and update the relevant docs in the same change.

## Repository Layout

- `crates/pretext` contains the stable layout SDK: paragraph preparation, measurement, shaping, bidi, line breaking, layout, caches, and the `rich_inline` helper.
- `crates/pretext-render` contains shared shaping-backed rasterization and text texture helpers.
- `crates/pretext-egui` contains the stable `egui` renderer SDK, glyph atlas, advanced painting helpers, and experimental demo assets.
- `demos/app` contains the `eframe` desktop demo shell and all demo windows.
- `tests_support` contains shared test fixtures/helpers used by integration tests.
- `pretext_js` is a Git submodule pointing at the upstream TypeScript reference. It has its own `pretext_js/AGENTS.md`; obey that file when editing inside the submodule.
- `docs/upstream-map.md` maps upstream JS modules and APIs to Rust modules.

## Setup Commands

- Install the stable Rust toolchain if needed: `rustup toolchain install stable`.
- Fetch submodules after cloning: `git submodule update --init --recursive`.
- Check the workspace quickly: `cargo check --all`.
- Run the desktop demo: `cargo run -p pretext-demo-app`.
- On Linux, install GUI build dependencies matching `.github/workflows/ci.yml` before building the demo or running CI-equivalent checks.

## Development Workflow

- Work from the workspace root unless a command explicitly targets a crate.
- Keep dependency versions centralized in the root `Cargo.toml`; do not bump `egui`/`eframe` from `0.33.3` unless the task explicitly asks for that migration.
- Prefer the stable root exports for normal SDK usage: `pretext::PretextEngine`, `pretext::PretextStyle`, `pretext::PretextParagraphOptions`, `pretext::PretextPreparedParagraph`, `pretext::WordBreakMode`, `pretext::rich_inline`, `pretext_egui::EguiPretextRenderer`, and `pretext_egui::EguiPretextPaintOptions`.
- Use `pretext::advanced` and `pretext_egui::advanced` only for low-level cursor-driven layout, obstacle flow, glyph-scene painting, custom shaping, or renderer internals.
- Prefer `..Default::default()` when constructing `PretextParagraphOptions` so examples remain resilient as option fields evolve.
- Update `README.md`, `plan.md`, or `docs/upstream-map.md` when public APIs, architecture, or JS/Rust parity assumptions change.

## Testing Instructions

- Run all tests: `cargo test --all`.
- Run formatting exactly like CI: `cargo fmt --all --check`.
- Run a focused crate test: `cargo test -p pretext`.
- Run a focused integration test: `cargo test -p pretext --test goldens`.
- Run a single test by name: `cargo test -p pretext <test_name>`.
- Refresh golden fixtures only when the output change is intentional: `UPDATE_GOLDENS=1 cargo test -p pretext --test goldens`, then review every JSON diff under `crates/pretext/tests/goldens`.
- The CI workflow currently runs `cargo fmt --all --check` and `cargo test --all` on stable Rust.

## Code Style

- Follow existing Rust style and let `rustfmt` own formatting.
- Keep changes small and crate-local when possible; avoid unrelated cleanups in files touched for a specific fix.
- Preserve the separation between layout (`pretext`), rasterization (`pretext-render`), and UI painting (`pretext-egui`).
- Keep public SDK additions intentional and re-exported from the appropriate crate root only when they are meant to be stable.
- Add or update tests adjacent to the behavior being changed, especially for shaping, whitespace, segmentation, bidi, line breaking, layout parity, rich inline flow, and goldens.
- Do not add inline comments unless they clarify non-obvious layout, shaping, Unicode, or renderer behavior.

## Assets And Fixtures

- Bundled demo fonts and emoji/SVG assets live under `demos/app/assets` and are included by the demo build.
- Keep font/source attribution files in sync when assets change, especially `demos/app/assets/fonts/SOURCES.md`.
- Do not regenerate golden JSON fixtures casually; golden updates should be explicit and reviewed as behavioral changes.

## Upstream Reference Workflow

- Treat `pretext_js` as the upstream TypeScript reference, not the Rust source of truth.
- Before porting upstream behavior, consult `docs/upstream-map.md` to find the Rust counterpart.
- Do not edit or advance the `pretext_js` submodule pointer unless the user explicitly requests an upstream sync or submodule update.
- When JS behavior and Rust architecture differ, prefer the Rust public API shape unless the task is specifically about parity.

## Pull Request Checklist

- Run `cargo fmt --all --check` before handoff.
- Run the narrowest relevant `cargo test` command first, then `cargo test --all` when practical.
- Document any skipped validation and why.
- Do not create commits, tags, or branches unless explicitly requested.

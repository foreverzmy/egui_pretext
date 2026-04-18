# Upstream Map

This repo tracks the TypeScript reference implementation under `pretext_js/`.

Use this file when an upstream `pretext_js` commit lands and you need to decide
 where the Rust follow-up belongs.

## Public API Map

JS root API:

- `prepare()` -> `PretextEngine::prepare()`
- `layout()` -> `PretextEngine::layout()` / `PreparedTextWithSegments::measure()`
- `prepareWithSegments()` -> `PretextEngine::prepare_with_segments()` / `prepare_paragraph()`
- `layoutWithLines()` -> `PretextEngine::layout_with_lines()`
- `walkLineRanges()` -> `PretextEngine::walk_line_ranges()`
- `layoutNextLine()` -> `PretextEngine::layout_next_line()`
- `layoutNextLineRange()` -> `PreparedTextWithSegments::layout_next_line_range()` or `pretext::advanced::layout_next_line_range()`
- `measureLineGeometry()` -> `PreparedTextWithSegments::measure_line_geometry()` or `pretext::advanced::measure_line_geometry()`
- `measureNaturalWidth()` -> `PreparedTextWithSegments::measure_natural_width()` or `pretext::advanced::measure_natural_width()`
- rich inline / inline flow helpers -> `pretext::rich_inline::*`

## Internal Module Map

Upstream `pretext_js/src/*` to Rust:

- `analysis.ts` -> `crates/pretext/src/analysis.rs`
- `bidi.ts` -> `crates/pretext/src/bidi.rs`
- `measurement.ts` -> `crates/pretext/src/measure.rs` and `crates/pretext/src/font_catalog.rs`
- `line-break.ts` -> `crates/pretext/src/layout.rs` and `crates/pretext/src/line_break.rs`
- `layout.ts` -> `crates/pretext/src/engine.rs`, `crates/pretext/src/layout.rs`, `crates/pretext/src/advanced.rs`
- `rich-inline.ts` / inline-flow helpers -> `crates/pretext/src/rich_inline.rs`

## Tracking Rules

- If upstream changes segmentation, punctuation glue, keep-all, or script policy, start in `analysis.rs`.
- If upstream changes break legality rules, start in `line_break.rs`.
- If upstream changes walker alignment, soft-hyphen handling, or line-range behavior, start in `layout.rs`.
- If upstream changes width measurement, grapheme prefix widths, or shaping caches, start in `measure.rs`.
- If upstream changes browser-specific heuristics, first decide whether the behavior is browser-only. Rust should only copy the semantic result when it still applies in a native shaping pipeline.
- If upstream adds a new public geometry helper, prefer adding it to `PreparedTextWithSegments` and mirroring it in `pretext::advanced` rather than exposing more raw internals.

## Intentional Divergences

- Rust keeps an explicit `PretextEngine` because fonts, shaping caches, and locale state are engine-owned.
- Rust uses `unicode_bidi` for real bidi runs; JS keeps a lighter rich-path metadata helper.
- Rust splits shaping and font fallback into dedicated modules instead of folding them into one browser-facing layout file.

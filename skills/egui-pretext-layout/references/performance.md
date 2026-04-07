# Performance

Use this reference when a task is about reducing cold-open jank, reflow churn, or cache invalidation in the `egui` + `pretext` demo shell.

The current implementation treats performance as a layering problem:

- `egui` owns interaction timing, window shells, hover/open intent, and frame budgets.
- `pretext` owns reusable paragraph prep, reflow, cursor continuation, and width-sensitive layout caches.
- `pretext_egui` owns atlas warmup, texture reuse, and paint-path stats.

## Primary Rule

Do not put unbounded cold-start work directly inside `DemoWindow::show(...)`.

If a demo needs to build heavy text state, geometry, or textures for the first time, move that work into `warmup_step(...)` and let `show_loading(...)` render the window shell until the expensive state is ready.

This is now the default pattern in the app shell.

## Current App-Level Strategy

Read `demos/app/src/app.rs` before changing performance behavior.

The app shell now does all of the following:

- keeps one long-lived `PretextEngine`
- keeps one long-lived `EguiPretextRenderer`
- tracks per-demo warmup state through `DemoWindow::warmup_status()` and `warmup_step(...)`
- runs small background warmup steps for hovered demos
- prioritizes larger warmup budgets for newly opened demos
- shows loading shells for demos that are not ready yet
- keeps simple warmup telemetry in the perf HUD

### Frame-budget model

Use app-level budgets instead of one-off ad hoc timers inside a demo:

- hover warmup: small background budget
- open demo warmup: larger foreground budget
- idle atlas warmup: separate from demo warmup

Prefer a few deterministic warmup stages over a single opaque "build everything" pass.

## Cache Policy

Treat cache lifetime as an explicit product decision.

For demo UX, the current default is:

- keep expensive caches alive across close/open
- invalidate only when the `engine.revision()` changes or an actual reflow key changes
- avoid rebuilding the same heavy state every time a user reopens a demo

This matters more than micro-optimizing individual layout calls.

### Good cache boundaries

- prepared text cache
  invalidated by engine revision, text, style, paragraph options
- width/layout cache
  invalidated by quantized width or layout key changes
- obstacle/reflow cache
  invalidated by quantized obstacle signatures, not raw float noise
- texture cache
  invalidated by actual texture key changes, not window visibility
- parser/AST cache
  invalidated by source content changes, not engine revision

### Bad cache boundaries

- clearing layout caches in `set_open(false)` just because a window closed
- tying pure markdown parsing to engine lifetime
- using raw per-frame float inputs as cache keys without bucketing
- rebuilding texture-producing images on every frame when the source parameters did not change

## Heavy Demo Patterns

These demos are the main reference implementations for cold-start optimization.

### `markdown_chat.rs`

Use this file for:

- staged warmup of markdown templates before building a large virtualized frame
- separating parser cache from engine-dependent prepared blocks
- warming a default conversation frame before first full paint

Pattern:

1. Parse markdown once into block AST.
2. Warm prepared templates incrementally.
3. Warm the default 10k-message frame separately.
4. Only enter the real render path after those caches are ready.

### `dynamic_layout.rs`

Use this file for:

- staged prep of body text, credit text, SVG hulls, page layout, transformed logo geometry, and final projection
- preserving reusable layout/projection caches across close/open
- quantized obstacle geometry buckets

Pattern:

1. Prepare paragraph state.
2. Build or reuse static SVG-derived geometry.
3. Build the default layout key.
4. Materialize the projection cache.
5. Paint only after the projection is ready.

### `editorial_engine.rs`

Use this file for:

- separating static editorial projection from dynamic body projection
- precomputing background and orb textures outside the first interactive paint
- keeping multi-column body caches warm across close/open

Pattern:

1. Warm static text and layout inputs.
2. Warm static projection.
3. Warm background and orb textures.
4. Warm dynamic body projection.
5. Reuse body projection when obstacle buckets do not change.

### `variable_typographic_ascii.rs`

Use this file for:

- incremental construction of a large glyph palette
- separating palette build from the per-frame animation loop
- keeping the fast render path atlas-backed after warmup

Pattern:

1. Build palette entries in batches.
2. Build mono entries in batches.
3. Finalize the lookup table once the entries are ready.
4. Only then rebuild rows and paint panels.

## System Font Policy

Do not automatically swap in a new system-font engine during an unrelated user interaction if that swap would invalidate large amounts of cached demo state.

The current app shell prefers:

- bundled-font engine by default
- explicit user-triggered system-font loading
- one deliberate engine revision change instead of a surprise background swap

This avoids stacking "demo first open" work on top of "global engine revision changed" work.

## Loading Shell Guidance

Loading shells are acceptable when they improve responsiveness.

For demo UX, prefer:

- immediate window open
- concise loading status
- stable default window size
- warmup progress based on named stages

Avoid blank windows or blocking the UI until all cold caches are ready.

## Quantization Guidance

Quantize noisy inputs before using them as reflow keys.

Good candidates:

- width buckets
- height buckets
- obstacle positions
- rotation angles
- drop-cap widths

The goal is to avoid relayout churn from visually insignificant changes while still invalidating correctly when the geometry materially changes.

## Validation Targets

When performance work lands, verify both user-visible behavior and cache behavior:

- the demo window opens immediately, even before heavy caches are ready
- warmup converges to `ready`
- reopening a heavy demo stays warm
- engine revision changes still invalidate the right state
- renderer stats still show the intended atlas or texture reuse path
- virtualization or obstacle caches still produce deterministic results

Use `demos/app/tests/smoke.rs` as the app-level place to lock these behaviors.

## Anti-Patterns

- doing full-frame prep or texture generation synchronously in `show(...)`
- hiding performance work behind a single `ensure_*` that may do arbitrarily much work
- clearing caches because a window is no longer visible
- mixing animation time updates with expensive cold-start prep
- tying parser caches to engine revision when the parse result is engine-independent
- assuming atlas warmup alone will fix first-open jank when the real cost is CPU-side setup

## Decision Order

When a demo feels slow to open, check in this order:

1. Is heavy work happening in `show(...)`?
2. Can the work be split into warmup stages?
3. Is the cache being thrown away on close?
4. Is an engine revision change invalidating more state than necessary?
5. Are raw floats causing avoidable relayout churn?
6. Only then inspect lower-level layout or rasterization micro-costs.

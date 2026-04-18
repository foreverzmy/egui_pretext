---
name: pretext-upstream-sync
description: Compare upstream `pretext_js` changes with this Rust workspace, update the local `pretext_js` submodule to a chosen remote branch, tag, or commit when explicitly requested, record that aligned JS commit in the parent repository's submodule pointer, map changed JS files and public helpers to Rust counterparts, decide what Rust should borrow, port, mirror, or intentionally ignore, and assess whether Rust architecture or public APIs should align with existing JS capabilities to make future upstream tracking easier. Use when the task mentions upstream sync, updating the `pretext_js` submodule, parity review, JS-to-Rust mapping, borrowable changes, architecture alignment, or follow-up recommendations for `pretext_js`.
---

# Pretext Upstream Sync

Use this skill to turn `pretext_js` upstream deltas into concrete Rust follow-up and to keep the parent repo's `pretext_js` submodule pointer aligned with the JS commit that Rust is targeting, without mechanically copying browser-specific implementation details.

## Use This Skill When

- You need to compare recent `pretext_js` upstream changes against the current Rust branch.
- You need to update the local `pretext_js` submodule to the corresponding remote branch, tag, or commit before reviewing deltas.
- You need Rust and JS to converge on the same baseline and want the parent repo to record that JS commit via the submodule pointer.
- You need to decide whether Rust already has an equivalent capability.
- You need to judge whether a JS helper should become a stable Rust API, a `pretext::advanced` helper, an internal-only change, or no port at all.
- You need to check whether Rust architecture should align with JS semantics so future upstream tracking stays cheaper.

## Grounding

1. Work from the repo root and treat `pretext_js/` as a git submodule plus the local upstream reference.
2. Read `docs/upstream-map.md` first. It is the source of truth for JS-to-Rust module and API correspondence.
3. Read `README.md` when you need the current stable Rust exports or test commands.
4. Read `plan.md` only when architecture intent or intentional divergence matters.
5. Read `skills/egui-pretext-layout/references/repo-map.md` only for egui or demo-level follow-up.

## Workflow

1. Resolve the local submodule state and target remote.
   - Record `git submodule status -- pretext_js`, `git -C pretext_js rev-parse HEAD`, and `git -C pretext_js status --short` before doing anything else.
   - If the user gave an explicit branch, tag, or SHA, use that as the target.
   - Otherwise treat `origin/main` as the upstream target for review.
   - If the user explicitly asked to update `pretext_js`, fetch first and only move the submodule worktree when the `pretext_js` worktree is clean.
   - Preferred commands:
     - `git submodule status -- pretext_js`
     - `git -C pretext_js fetch origin`
     - `git -C pretext_js status --short`
     - `git -C pretext_js rev-parse HEAD`
     - `git -C pretext_js rev-parse origin/main`
   - Preferred update modes:
     - Exact review target: `git -C pretext_js checkout --detach <target>`
     - Fast-forward tracked branch: `git -C pretext_js checkout <branch>` then `git -C pretext_js pull --ff-only origin <branch>`
   - If the task is to persist the alignment in this repo, stage the gitlink change from the parent repo with `git add pretext_js` after the submodule reaches the chosen commit.
2. Establish the comparison range.
   - If the user gave explicit SHAs, tags, or branches, use them.
   - Otherwise use the current local `pretext_js` HEAD as the base and `origin/main` as the upstream target.
   - Typical commands:
     - `git -C pretext_js fetch origin`
     - `git -C pretext_js log --oneline <base>..origin/main`
     - `git -C pretext_js diff --stat <base>..origin/main`
3. Classify upstream changes before mapping them.
   - `analysis / segmentation / keep-all / punctuation / annotations`
   - `line-break / walker / soft-hyphen / range behavior`
   - `layout / cursor / geometry helpers / public measurement helpers`
   - `rich-inline / inline-flow`
   - `renderer / demo / browser-only`
   - `docs / infra`
4. Map each changed JS file or exported helper to Rust.
   - `analysis.ts` -> `crates/pretext/src/analysis.rs`
   - `line-break.ts` -> `crates/pretext/src/layout.rs` and `crates/pretext/src/line_break.rs`
   - `layout.ts` -> `crates/pretext/src/engine.rs`, `crates/pretext/src/layout.rs`, and `crates/pretext/src/advanced.rs`
   - `rich-inline.ts` -> `crates/pretext/src/rich_inline.rs`
   - For other files, keep using `docs/upstream-map.md` instead of guessing.
5. Classify the Rust action for each item.
   - `already present`
   - `direct port candidate`
   - `Rust-native analogue needed`
   - `browser-only or demo-only`
   - `defer`
6. Check architecture alignment, not just feature parity.
   - Ask whether matching the same conceptual layer would reduce future diff noise.
   - Prefer stable root exports for durable public geometry or measurement helpers.
   - Prefer `pretext::advanced` for reusable but niche cursor, range, or low-level layout helpers.
   - Keep implementation details internal when JS changed internals without creating a durable semantic surface.
   - Do not mirror browser-centric heuristics when Rust already has a better engine-owned equivalent.
7. Define validation before recommending implementation.
   - Engine semantics: targeted `cargo test -p pretext`
   - Layout or geometry parity: relevant layout or golden coverage
   - Renderer or demo borrow: targeted demo tests or smoke coverage
8. If alignment is complete, record the submodule pointer in the parent repo.
   - After Rust and JS are aligned on the chosen baseline, ensure `pretext_js` is at that commit.
   - Confirm the parent repo shows the gitlink change with `git status --short pretext_js`.
   - If the task includes committing, include the staged `pretext_js` gitlink update in the parent repo commit that lands the alignment or in a dedicated sync commit.

## Decision Rules

- Optimize for semantic parity, not file-by-file parity.
- Never discard local `pretext_js` changes just to match remote. If `git -C pretext_js status --short` is non-empty, stop and report the dirty state instead of resetting or force-checking out.
- When updating the `pretext_js` submodule, prefer exact target checkouts or `--ff-only` pulls so the resulting base SHA is unambiguous.
- Do not stop at moving the submodule worktree if the goal is persistent repo alignment. The parent repo must record the new gitlink for `pretext_js`.
- Detached HEAD inside the submodule is acceptable for a pinned baseline as long as the parent repo records the intended submodule SHA.
- If Rust already covers the behavior under a different API, call that out instead of cloning JS naming.
- If upstream adds a broadly useful public geometry helper, consider placing it on `PreparedTextWithSegments` and mirroring it in `pretext::advanced` rather than exposing raw internals.
- When a JS change exists only to compensate for browser measurement quirks, copy the user-visible behavior only if it still matters in Rust.
- When recommending architecture alignment, state the concrete payoff: easier future diffing, clearer module ownership, or less duplicated logic.

## Output Format

Return results in this order:

1. Submodule state: old recorded SHA, current submodule HEAD, target remote ref, new SHA if updated, and whether the parent repo now has a `pretext_js` gitlink change.
2. Compared range: local base SHA, upstream target SHA, and changed JS files.
3. Priority recommendations: what Rust should borrow first and why.
4. Already covered in Rust: existing APIs or modules that already satisfy the upstream intent.
5. Intentional divergences: browser-only, demo-only, or not worth mirroring.
6. Architecture alignment notes: where Rust should realign module or API placement to make future upstream sync cheaper.
7. Validation: exact tests or checks to run if follow-up implementation happens.
8. Landing notes: whether `git add pretext_js` and a parent repo commit are still pending.

## Read Selectively

- `docs/upstream-map.md`: always first.
- `README.md`: stable public API and current test commands.
- `plan.md`: architecture intent and long-lived divergences.
- `skills/egui-pretext-layout/references/repo-map.md`: egui and demo patterns only.

## Example Prompts

- `Use $pretext-upstream-sync to compare recent pretext_js upstream changes with this Rust workspace and recommend what Rust should borrow.`
- `Use $pretext-upstream-sync to update the pretext_js submodule to origin/main, then compare the new upstream delta and tell me what Rust should borrow.`
- `Use $pretext-upstream-sync to align Rust with a chosen pretext_js commit and make sure the parent repo records that submodule SHA.`
- `Use $pretext-upstream-sync to review analysis.ts and line-break.ts changes since <sha> and map them to Rust follow-ups.`
- `Use $pretext-upstream-sync to decide whether a new JS geometry helper should be a stable Rust API, an advanced helper, or remain internal.`

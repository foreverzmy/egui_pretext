# Spec System

- `current/` 是 LLM 默认读取的唯一 Current State Snapshot。
- `drafts/` 是非权威 Draft Workspace，用于调研、候选 Patch、测试计划和验证证据。
- `guides/` 是弱语义解释层，只辅助理解，不覆盖 `current/`。
- `log/` 是 Spec Patch 或 bootstrap 记录，默认不进入上下文。
- `archive/` 是历史 Snapshot，默认不参与当前决策。

## Current Index

- `system.workspace`: Rust workspace、crate 边界、上游子模块边界。
- `system.runtime`: Rust/egui 运行时和依赖版本约束。
- `system.validation`: 测试、格式化、golden 与视觉验证入口。
- `system.upstream-alignment`: `pretext_js` 到 Rust 的同步和取舍规则。
- `capability.text-layout`: `pretext` 段落准备、测量、排版、shaping、bidi 与 line breaking。
- `capability.rich-inline-flow`: inline-only mixed-style flow、atomic item 与 line-range walking。
- `capability.text-rasterization`: `pretext-render` shaping-backed text texture rasterization。
- `capability.egui-rendering`: `pretext-egui` paragraph painting、glyph atlas、texture cache 与 warmup。
- `capability.demo-app`: `eframe` demo shell 与 demo windows。
- `contract.pretext-public-api`: `pretext` 稳定 root API 与 advanced API 边界。
- `contract.egui-renderer-api`: `pretext-egui` 稳定 renderer API 与 advanced renderer API 边界。
- `contract.upstream-map`: TypeScript reference API/module 到 Rust counterpart 的映射。

## Validation

```bash
python3 .agents/skills/writing-spec/scripts/spec_store.py validate ./omni-coding/specs
```

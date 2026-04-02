# FACET v2.1.3 Examples

This directory contains runnable FACET examples aligned with the v2.1.3 production spec.

Primary suite: `examples/spec/` (ordered from simple to complex).

## Quick Start

```bash
# Build release binary once
cargo build --release --bin facet-fct

# Run full suite smoke checks
./scripts/smoke_examples_spec.sh
```

## Ordered Spec Suite

### 01. `examples/spec/01_minimal.facet`
- Minimal canonical document: `@meta`, `@context`, `@system`, `@user`.

### 02. `examples/spec/02_vars_types_pipelines.facet`
- `@vars` + `@var_types` + string pipelines (`trim`, `lowercase`) + list/map literals.

### 03. `examples/spec/03_input_runtime.facet`
- Runtime `@input(...)` materialization with types/defaults.
- Runtime values file: `examples/spec/03_input_runtime.input.json`.

### 04. `examples/spec/04_when_gating.facet`
- Boolean `when` gating on message facets.
- Runtime values file: `examples/spec/04_when_gating.input.json`.

### 05. `examples/spec/05_imports_merge.facet`
- Deterministic `@import` resolution and merge.
- Imported modules: `examples/spec/imports/05_base.facet`, `examples/spec/imports/05_override.facet`.

### 06. `examples/spec/06_interfaces_policy.facet`
- `@interface`, `@system.tools`, and `@policy` allow rules for `tool_expose`/`tool_call`.

### 07. `examples/spec/07_policy_conditions.facet`
- `PolicyCond` with `all/any/not` and runtime-controlled policy behavior.
- Runtime values file: `examples/spec/07_policy_conditions.input.json`.

### 08. `examples/spec/08_test_suite.facet`
- Spec-style tests: `@test "name"`, `mock`, `assert`.
- Includes assertion over execution provenance (`execution.provenance.events[...]`).

### 09. `examples/spec/09_multimodal_content.facet`
- Canonical multimodal content items (`text`, `image`, `audio`) and canonical asset shape.

### 10. `examples/spec/10_layout_budget.facet`
- Token Box controls: `priority`, `min`, `shrink`, deterministic layout under budget.

### 11. `examples/spec/11_pure_mode_expected_f803.facet`
- Expected failure case in Pure mode: Level-1 lens cache miss -> `F803`.

### 12. `examples/spec/12_exec_mode_expected_f454.facet`
- Expected failure case in Exec mode: policy deny on guarded `lens_call` -> `F454`.

## Manual Commands

```bash
BIN=./target/release/facet-fct

# Build any example
$BIN build --input examples/spec/01_minimal.facet

# Run with runtime input
$BIN run --input examples/spec/03_input_runtime.facet \
  --runtime-input examples/spec/03_input_runtime.input.json \
  --format json

# Run tests
$BIN test --input examples/spec/08_test_suite.facet --output summary

# Inspect AST/DAG/Layout/Policy views
$BIN inspect --input examples/spec/02_vars_types_pipelines.facet \
  --ast /tmp/ast.json --dag /tmp/dag.json --layout /tmp/layout.json --policy /tmp/policy.json
```

## Legacy Top-Level Examples

These files are kept for compatibility and quick demos:
- `examples/basic_prompt.facet`
- `examples/rag_pipeline.facet`
- `examples/advanced_features.facet`
- `examples/policy_guard_test.facet`

New feature-complete coverage should be added under `examples/spec/`.

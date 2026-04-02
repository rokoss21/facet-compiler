# FACET v2.1.3 Migration Summary

Updated: 2026-04-02

## Status

- Migration status: **Complete**
- Conformance target: **FACET v2.1.3 REC-PROD** (2026-02-19)
- Delivery effort: **133 implementation iterations**
- Result: **100% checklist completion** against the repository migration plan

## Scope

This repository was migrated from the earlier FACET v2.0-oriented implementation to a production-aligned FACET v2.1.3 compiler/runtime behavior.

The migration covers parser, resolver, validator, execution engine, renderer, policy guard system, CLI tooling, examples, and CI quality gates.

## What Was Delivered

### 1) Parsing and normalization alignment

- Enforced UTF-8 input, Unicode NFC normalization, and LF line normalization.
- Enforced strict lexical and indentation rules (including tab rejection and 2-space indentation behavior).
- Implemented full FACET syntax parsing across profiles, with profile restriction errors mapped to the expected codes.
- Added/expanded specialized parsing support for `@interface` and `@test` forms.

### 2) Deterministic Phase-1 resolution and merge

- Implemented sandboxed `@import` resolution with deterministic expansion order.
- Added import-cycle and sandbox violation handling with FACET-aligned diagnostics.
- Implemented ordered singleton-map merge behavior with stable first-insertion key position.
- Preserved deterministic order for repeatable block facets.

### 3) FTS and semantic validation

- Implemented/validated FACET Type System (FTS) coverage for primitives, composites, unions, and multimodal forms.
- Added `@var_types` checks, lens pipeline step assignability checks, and runtime input validation semantics.
- Implemented interface schema mappability checks aligned with Appendix D expectations.

### 4) Runtime execution semantics (R-DAG)

- Implemented deterministic reactive compute with dependency graph evaluation.
- Enforced unknown variable/path/cycle behavior with spec-aligned errors.
- Added gas accounting/enforcement behavior.
- Implemented Pure vs Exec trust-level behavior for lenses, including cache-only Level-1 rules in Pure mode.

### 5) Policy and runtime guard model

- Implemented `@policy` parsing/validation/merge semantics and effective policy materialization.
- Implemented `policy_hash` generation bound to `policy_version` envelope semantics.
- Implemented conjunctive rule matching, canonical name matching, and short-circuit `PolicyCond` evaluation.
- Enforced fail-closed runtime guard decisions with explicit `F454` vs `F455` separation.
- Added execution artifact emission with `GuardDecision` events and provenance hash-chain support.

### 6) Token Box and canonical render behavior

- Implemented FACET Units-based layout budgeting and deterministic packing behavior.
- Added deterministic compression/truncation/drop semantics, including UTF-8-safe truncation handling.
- Aligned canonical message/tool ordering requirements.
- Aligned canonical metadata output (`facet_version`, `policy_version`, `document_hash`, etc.).

### 7) CLI, examples, and compliance automation

- Completed operational CLI flows for build/run/test/inspect with phase-appropriate checks.
- Added spec-oriented examples and smoke/matrix validation scripts.
- Added automated compliance report generation.
- Integrated compliance reporting and release-quality gates in CI workflows.

## Verification Gates

The migration was validated repeatedly with repository gates, including:

- `cargo test -q --workspace`
- `./scripts/smoke_examples_spec.sh ./target/release/facet-fct`
- `./scripts/spec_matrix_examples.sh ./target/release/facet-fct`
- `python3 scripts/generate_compliance_report.py --checklist docs/14-v2.1.3-migration-checklist.md --output compliance-report.md`

## Where To Find Full Evidence

- Full final migration checklist:
  - [`docs/FACET-v2.1.3-Migration-Checklist-FINAL.md`](./docs/FACET-v2.1.3-Migration-Checklist-FINAL.md)
- Compatibility path (same finalized checklist status):
  - [`docs/14-v2.1.3-migration-checklist.md`](./docs/14-v2.1.3-migration-checklist.md)
- Language specification used as target:
  - [`FACET-v2.1.3-Production-Language-Specification.md`](./FACET-v2.1.3-Production-Language-Specification.md)

## Versioning Note

Compiler release versioning (`fct` release tags) and FACET language specification versioning are intentionally separate:

- Language target: **FACET v2.1.3**
- Compiler release line: **`fct` 0.x**

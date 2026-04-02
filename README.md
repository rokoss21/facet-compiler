# FACET Compiler (Rust)

[![CI](https://github.com/rokoss21/facet-compiler/actions/workflows/ci.yml/badge.svg)](https://github.com/rokoss21/facet-compiler/actions/workflows/ci.yml)
[![Release](https://github.com/rokoss21/facet-compiler/actions/workflows/release.yml/badge.svg)](https://github.com/rokoss21/facet-compiler/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/rokoss21/facet-compiler?sort=semver)](https://github.com/rokoss21/facet-compiler/releases)
[![FACET Spec](https://img.shields.io/badge/FACET-v2.1.3-0A66C2)](./FACET-v2.1.3-Production-Language-Specification.md)
[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./LICENSE-MIT)

Deterministic compiler/runtime for **FACET v2.1.3** — a contract-first language for reliable AI request construction.

FACET Compiler turns `.facet` programs into validated, reproducible canonical payloads with strict typing, deterministic execution, policy guard enforcement, and provenance artifacts.

## Why FACET Compiler

Most AI stacks fail at the contract boundary: schemas drift, tool calls are weakly enforced, context packing is ad hoc, and behavior differs across runs/providers.

FACET Compiler addresses this by enforcing the contract **before generation**:

- deterministic phases (resolution -> type check -> compute -> layout -> render)
- strict FTS typing and placement rules
- fail-closed policy/guard model (`F454`, `F455`)
- canonical JSON with stable hashes for replay/audit
- deterministic context budget behavior (Token Box Model)

## What This Repository Is

This repository is the **Rust reference compiler implementation** for FACET v2.1.3.

- Language target: FACET v2.1.3 REC-PROD
- Binary: `facet-fct` (`fct --version`)
- Conformance tracker: [`docs/14-v2.1.3-migration-checklist.md`](./docs/14-v2.1.3-migration-checklist.md)
- Canonical spec in this repo: [`FACET-v2.1.3-Production-Language-Specification.md`](./FACET-v2.1.3-Production-Language-Specification.md)

## Quick Start

### 1) Build

```bash
cargo build --release --bin facet-fct
./target/release/facet-fct --version
```

### 2) Validate a `.facet` file (Phases 1-2)

```bash
./target/release/facet-fct build --input examples/spec/01_minimal.facet
```

### 3) Run full pipeline (Phases 1-5)

```bash
./target/release/facet-fct run --input examples/spec/01_minimal.facet --exec --format pretty
```

### 4) Run deterministic suites used in CI/release gates

```bash
./scripts/smoke_examples_spec.sh ./target/release/facet-fct
./scripts/spec_matrix_examples.sh ./target/release/facet-fct
```

## CLI Overview

```bash
# Global help
./target/release/facet-fct --help

# Build: parse + resolve + validate
./target/release/facet-fct build --input file.facet

# Run: full execution pipeline
./target/release/facet-fct run --input file.facet --exec
./target/release/facet-fct run --input file.facet --pure

# Test: execute @test blocks
./target/release/facet-fct test --input file.facet --exec

# Inspect: dump compiler internals
./target/release/facet-fct inspect --input file.facet \
  --ast ast.json --dag dag.json --layout layout.json --policy policy.json
```

Runtime inputs for `@input(...)`:

```bash
./target/release/facet-fct run \
  --input examples/spec/03_input_runtime.facet \
  --runtime-input examples/spec/03_input_runtime.input.json \
  --exec
```

## Architecture (Compiler Pipeline)

| Phase | Purpose |
| --- | --- |
| Phase 1 | Normalize, parse, resolve `@import`, deterministic merge |
| Phase 2 | Type checking, semantic validation, policy schema validation |
| Phase 3 | Reactive compute (R-DAG), lens execution, input materialization |
| Phase 4 | Deterministic context packing (Token Box Model) |
| Phase 5 | Canonical JSON render + execution provenance emission |

## Ecosystem and Methodology

FACET Compiler is part of a broader engineering model around deterministic AI systems and measurable delivery:

- **FACET Standard**: language/specification source of truth  
  [github.com/rokoss21/facet-standard](https://github.com/rokoss21/facet-standard)
- **IOSM Specification**: methodology for controlled improvement cycles  
  [github.com/rokoss21/IOSM](https://github.com/rokoss21/IOSM)
- **IOSM CLI Runtime**: terminal execution runtime implementing IOSM  
  [github.com/rokoss21/iosm-cli](https://github.com/rokoss21/iosm-cli)

How they connect conceptually:

- FACET defines the **deterministic contract layer** for AI behavior.
- IOSM defines the **engineering methodology** (Improve -> Optimize -> Shrink -> Modularize) for iterative system quality improvement with artifacts/metrics.
- iosm-cli operationalizes IOSM as a runtime for real repositories.

Together they align language contracts, execution control, and process governance.

## Quality Gates

Recommended local gate:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -q --workspace
```

Release tag gate (`v*`) includes:

- workspace tests
- spec smoke/matrix runs
- compliance report artifact generation

## Project Layout

- `src/commands/*` — CLI commands (`build`, `run`, `test`, `inspect`, `codegen`)
- `crates/fct-parser` — parser + normalization
- `crates/fct-resolver` — import resolution + deterministic merge
- `crates/fct-validator` — Phase 2 semantic/type/policy validation
- `crates/fct-engine` — Phase 3/4 runtime + guard-aware compute
- `crates/fct-render` — canonical payload/provenance render
- `crates/fct-std` — standard lens registry/library
- `docs/` — documentation and migration evidence
- `examples/spec/` — ordered spec-conformance examples

## Author

**Emil Rokossovskiy**

## License

Dual-licensed under [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE).

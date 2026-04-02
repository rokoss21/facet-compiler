# FACET Compiler (Rust)

Deterministic compiler/runtime for **FACET v2.1.3** (Neural Architecture Description Language).

FACET turns `.facet` documents into validated, reproducible execution artifacts and canonical request payloads.

## Philosophy

FACET keeps the original ideology: AI behavior must be engineered like software.

- Deterministic phases instead of ad-hoc prompt string handling
- Static validation (types, policy schema, placement rules) before execution
- Fail-closed runtime guard (`F454/F455`) for policy-sensitive operations
- Canonical JSON + hashes for replayability and auditability

## Current Status

- Language target: **FACET v2.1.3 REC-PROD**
- Implementation: **Rust workspace** (`fct-*` crates)
- Canonical spec: `FACET-v2.1.3-Production-Language-Specification.md`
- Migration evidence/checklist: `docs/14-v2.1.3-migration-checklist.md`
- Main local gate: `cargo test -q --workspace`

## Install / Build

Build from source:

```bash
cargo build --release
```

Release binary:

```bash
target/release/facet-fct
```

Check version:

```bash
./target/release/facet-fct --version
```

## CLI Usage

Global help:

```bash
cargo run -q -- --help
```

Build (parse + resolve + validate):

```bash
cargo run -q -- build --input examples/quickstart.facet
```

Run full pipeline (phases 1-5):

```bash
cargo run -q -- run --input examples/quickstart.facet --exec --format pretty
```

Run in pure mode:

```bash
cargo run -q -- run --input examples/quickstart.facet --pure
```

Run tests from `@test` blocks:

```bash
cargo run -q -- test --input examples/quickstart.facet --exec
```

Inspect internals (AST/DAG/layout/policy):

```bash
cargo run -q -- inspect --input examples/quickstart.facet \
  --ast ast.json --dag dag.json --layout layout.json --policy policy.json
```

## Runtime Inputs (`@input`)

Provide runtime values as JSON object:

```bash
cargo run -q -- run --input examples/with-input.facet \
  --runtime-input runtime-input.json --exec
```

## Project Layout

- `src/commands/*` — CLI commands (`build/run/test/inspect/codegen`)
- `crates/fct-parser` — parser + normalization
- `crates/fct-resolver` — import resolution + deterministic merge
- `crates/fct-validator` — Phase 2 semantic/type/policy checks
- `crates/fct-engine` — R-DAG compute + guard-aware execution
- `crates/fct-render` — canonical payload rendering
- `crates/fct-std` — standard lenses
- `docs/` — documentation and migration/conformance artifacts

## Quality Gates

Recommended local release gate:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -q --workspace
```

## Release

GitHub release workflow is tag-driven (`v*`).

Example:

```bash
git tag v0.1.2
git push origin v0.1.2
```

## License

Dual-licensed under MIT or Apache-2.0.

# FACET Compiler (`facet-fct`)

[![CI](https://github.com/rokoss21/facet-compiler/actions/workflows/ci.yml/badge.svg)](https://github.com/rokoss21/facet-compiler/actions/workflows/ci.yml)
[![Release](https://github.com/rokoss21/facet-compiler/actions/workflows/release.yml/badge.svg)](https://github.com/rokoss21/facet-compiler/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/rokoss21/facet-compiler?sort=semver)](https://github.com/rokoss21/facet-compiler/releases)
[![FACET Spec](https://img.shields.io/badge/spec-FACET%20v2.1.3-0A66C2)](./FACET-v2.1.3-Production-Language-Specification.md)
[![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](./LICENSE-MIT)

Deterministic compiler and execution layer for **FACET v2.1.3**.

`facet-fct` compiles `.facet` contracts into canonical, policy-aware AI request context with explicit failure semantics (`F*` codes), deterministic ordering, and bounded execution behavior.

## What You Get

- Deterministic pipeline: normalization -> resolution -> type check -> compute -> layout -> render
- Strict type and placement checks (FTS)
- Policy and guard fail-closed behavior (`F454`, `F455`)
- Canonical JSON output + stable document and policy hashes
- Spec-oriented examples and conformance suites in CI

## Install in One Command (Recommended)

```bash
cargo install --git https://github.com/rokoss21/facet-compiler --bin facet-fct
facet-fct --version
```

Optional short alias:

```bash
alias fct=facet-fct
```

## Quick Start (2 Minutes)

### 1) Ensure compiler is installed

```bash
facet-fct --version
# fct 0.1.2
```

### 2) Create a minimal contract

```facet
@meta
  name: "hello"

@context
  budget: 32000

@vars
  query: @input(type="string")

@system
  content: "You are a helpful assistant."

@user
  content: $query
```

Save as `hello.facet` and create runtime input:

```json
{ "query": "Hello FACET" }
```

Save as `hello.input.json`.

### 3) Validate and run

```bash
facet-fct build --input hello.facet
facet-fct run --input hello.facet --runtime-input hello.input.json --exec --format pretty
```

## Installation

### Option A: Fast install from Git (Cargo)

```bash
cargo install --git https://github.com/rokoss21/facet-compiler --bin facet-fct
facet-fct --version
```

### Option B: Download release binary

From [Releases](https://github.com/rokoss21/facet-compiler/releases/latest):

- Linux: `facet-fct-linux-x86_64.tar.gz`
- macOS (Intel): `facet-fct-macos-x86_64.tar.gz`
- Windows: `facet-fct-windows-x86_64.zip`

Example (Linux/macOS):

```bash
tar -xzf facet-fct-*.tar.gz
chmod +x facet-fct
./facet-fct --version
```

### Option C: Build from source (compile locally)

```bash
git clone https://github.com/rokoss21/facet-compiler
cd facet-compiler
cargo build --release --bin facet-fct
./target/release/facet-fct --version
```

### Option D: Install from local source with Cargo

```bash
cargo install --path . --bin facet-fct
facet-fct --version
```

### Homebrew

Official Homebrew formula is not published yet. Use `cargo install` or release binaries.

## CLI Cheatsheet

```bash
# Parse + resolve + type check
facet-fct build --input file.facet

# Full execution (default mode: --exec)
facet-fct run --input file.facet --exec
facet-fct run --input file.facet --pure

# Provide runtime @input values
facet-fct run --input file.facet --runtime-input input.json --exec

# Run @test blocks
facet-fct test --input file.facet --exec

# Dump internals
facet-fct inspect --input file.facet \
  --ast ast.json --dag dag.json --layout layout.json --policy policy.json

# Help
facet-fct --help
facet-fct run --help
```

## Docs Map

- Documentation portal: [rokoss21.github.io/facet-compiler](https://rokoss21.github.io/facet-compiler/)
- Quick Start: [docs/01-quickstart.md](./docs/01-quickstart.md)
- Execution Model: [docs/15-execution-model.md](./docs/15-execution-model.md)
- Production Failure Scenario: [docs/16-production-scenario.md](./docs/16-production-scenario.md)
- Integration Guide: [docs/18-integration-guide.md](./docs/18-integration-guide.md)
- Spec-conformance checklist: [docs/14-v2.1.3-migration-checklist.md](./docs/14-v2.1.3-migration-checklist.md)
- Language specification: [FACET-v2.1.3-Production-Language-Specification.md](./FACET-v2.1.3-Production-Language-Specification.md)

## Common Workflows

### Validate contracts in CI

```bash
facet-fct build --input examples/spec/01_minimal.facet
```

### Run deterministic example suites

```bash
./scripts/smoke_examples_spec.sh ./target/release/facet-fct
./scripts/spec_matrix_examples.sh ./target/release/facet-fct
```

### Local quality gate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -q --workspace
```

## Integration Pattern (Brownfield)

Minimal-risk adoption path:

1. Keep your current provider SDK and business logic.
2. Replace prompt assembly with `facet-fct run`.
3. Keep strict response schema validation in host code.
4. Add bounded retry/fallback policy in host runtime.

Detailed guide: [docs/18-integration-guide.md](./docs/18-integration-guide.md)

## Project Scope

FACET **does** guarantee deterministic contract compilation/execution boundaries.

FACET **does not** guarantee deterministic model generation output or correctness of external APIs.

## Repository Structure

- `src/commands/*` - CLI commands (`build`, `run`, `test`, `inspect`, `codegen`)
- `crates/fct-parser` - parser and normalization
- `crates/fct-resolver` - import resolution and deterministic merge
- `crates/fct-validator` - type/semantic/policy checks
- `crates/fct-engine` - compute/layout and guard-aware runtime behavior
- `crates/fct-render` - canonical JSON and provenance output
- `crates/fct-std` - standard lens registry
- `docs/` - documentation and conformance evidence
- `examples/spec/` - ordered specification scenarios

## Ecosystem

- FACET Standard: [github.com/rokoss21/facet-standard](https://github.com/rokoss21/facet-standard)
- IOSM Methodology: [github.com/rokoss21/IOSM](https://github.com/rokoss21/IOSM)
- IOSM CLI: [github.com/rokoss21/iosm-cli](https://github.com/rokoss21/iosm-cli)

## Contributing

- Contribution guide: [CONTRIBUTING.md](./CONTRIBUTING.md)
- Issues: [github.com/rokoss21/facet-compiler/issues](https://github.com/rokoss21/facet-compiler/issues)

## Author

**Emil Rokossovskiy**

- GitHub: [github.com/rokoss21](https://github.com/rokoss21)
- Email: [ecsiar@gmail.com](mailto:ecsiar@gmail.com)
- Site: [facetcore.dev](https://facetcore.dev/)

## License

Dual-licensed under [MIT](./LICENSE-MIT) or [Apache-2.0](./LICENSE-APACHE).

---
permalink: /06-cli.html
title: CLI Reference
---

# 06. CLI Reference (`facet-fct`)

Compiler version in this repo: **fct 0.1.2**
Language target: **FACET v2.1.3**

## Install

```bash
cargo install --git https://github.com/rokoss21/facet-compiler --bin facet-fct
facet-fct --version
```

Optional alias:

```bash
alias fct=facet-fct
```

## Top-level help

```bash
facet-fct --help
```

Commands:

- `build` — parse/resolve/validate
- `inspect` — export AST/DAG/layout/policy views
- `run` — full pipeline
- `test` — run `@test` blocks
- `codegen` — generate SDK from interfaces

## `build`

```bash
facet-fct build --input file.facet
```

Use when you need Phase 1 + Phase 2 validation only.

## `run`

```bash
facet-fct run --input file.facet --format pretty
```

Important options:

- `--runtime-input <json-file>`: values for `@input(...)`
- `--budget <int>`: layout budget
- `--context-budget <int>`: execution context budget
- `--pure` / `--exec`: execution mode

Example:

```bash
facet-fct run \
  --input examples/basic_prompt.facet \
  --runtime-input examples/basic_prompt.input.json \
  --format pretty \
  --pure
```

## `inspect`

```bash
facet-fct inspect --input file.facet --ast ast.json --dag dag.json --layout layout.json --policy policy.json
```

Useful for deterministic debugging and CI artifacts.

## `test`

```bash
facet-fct test --input file.facet
```

Options:

- `--filter <name-pattern>`
- `--output summary|verbose|json`
- `--budget <int>`
- `--gas-limit <int>`
- `--pure` / `--exec`

## Common workflows

### Validate before run

```bash
facet-fct build --input contract.facet
facet-fct run --input contract.facet --format pretty
```

### Run with runtime input

```bash
facet-fct run --input contract.facet --runtime-input runtime.json --format pretty
```

### CI check

```bash
facet-fct build --input contract.facet
facet-fct test --input contract.facet --output summary --pure
```

## Notes on command naming

The spec names the reference CLI as `fct`; this implementation ships binary `facet-fct`.
Using `alias fct=facet-fct` gives spec-style command names locally.

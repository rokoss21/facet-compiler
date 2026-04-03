---
permalink: /05-examples-guide.html
title: Examples
---

# 05. Examples Guide (Spec-Aligned)

This page documents repository examples that follow FACET v2.1.3.

## Run any example

```bash
facet-fct build --input examples/basic_prompt.facet
facet-fct run --input examples/basic_prompt.facet --runtime-input examples/basic_prompt.input.json --format pretty
```

## 1) `examples/basic_prompt.facet`

Purpose:

- minimal `@meta/@context/@vars/@system/@user`
- runtime input via `@input(type="string")`
- Level-0 lenses: `trim`, `uppercase`

Key points:

- valid `@system` uses only `content`
- canonical output contains `metadata`, `tools`, `messages`

## 2) `examples/rag_pipeline.facet`

Purpose:

- deterministic pre-processing of query and corpus
- list/map handling in `@vars`
- no non-spec `@context.documents` pattern

RAG in FACET v2.1.3 is represented by variables + messages, while `@context` remains layout configuration.

## 3) `examples/advanced_features.facet`

Purpose:

- typed variables via `@var_types`
- inline list/map literals
- transformations via pipeline operator `|>`

## 4) `examples/policy_guard_test.facet`

Purpose:

- interface declaration with mandatory `effect`
- policy allow rules for `tool_expose` and `tool_call`
- `@test` with mocked tool behavior

## 5) `examples/spec/*`

The `examples/spec` directory contains focused conformance snippets for parser/validator/runtime behavior.

## Valid syntax checklist for examples

- Message facets (`@system/@user/@assistant`) use `content` and optional layout fields/`when`.
- `@system.tools` references interfaces as `$InterfaceName`.
- `@context` includes only `budget` and optional `defaults` map.
- `@input(...)` appears only as base expression in `@vars` values (optionally piped).
- String-keyed maps are used only in `@meta`.

## Quick matrix

| File | Build | Run | Test |
|---|---|---|---|
| `examples/basic_prompt.facet` | ✅ | ✅ | n/a |
| `examples/rag_pipeline.facet` | ✅ | ✅ | n/a |
| `examples/advanced_features.facet` | ✅ | ✅ | n/a |
| `examples/policy_guard_test.facet` | ✅ | n/a | ✅ |

## Notes on legacy syntax

The following patterns are intentionally **not** used as valid examples:

- `@system.role`, `@system.model`, `@system.instructions`, `@system.temperature`
- `@context.documents`
- custom conditional directives not defined by v2.1.3

---
permalink: /08-lenses.html
title: Lenses
---

# 08. Lenses (FACET v2.1.3)

This page lists **normative standard Level-0 lenses** from Appendix A and key runtime rules from §9.

## Lens model

Each registered lens has:

- `name`, `version`
- `input_type`, `output_type` (FTS)
- `trust_level` (`0|1|2`)
- gas function
- determinism class (`pure|bounded|volatile`)
- `effect_class` (required for trust levels 1/2)

Unknown lens → `F802`.
Missing/invalid `effect_class` for Level-1/2 → `F456`.

## Trust levels

- Level 0: deterministic, no I/O
- Level 1: bounded external behavior with cache contract
- Level 2: volatile external/nondeterministic behavior

Mode rules:

- Pure mode: Level 2 forbidden (`F801`); Level 1 cache-only (`F803` on miss)
- Exec mode: Level 1/2 allowed only if host + policy/guard allow

## Standard Level-0 text lenses (Appendix A.1)

- `trim() -> string`
- `lowercase() -> string` (locale-independent)
- `uppercase() -> string` (locale-independent)
- `split(separator: string) -> list<string>`
- `replace(pattern: string, replacement: string) -> string` (safe regex subset)
- `indent(level: int) -> string` (2 spaces × level)

## Standard Level-0 data lenses (Appendix A.2)

- `json(indent: int = 0) -> string`
- `keys() -> list<string>`
- `values() -> list<any>`
- `map(field: string) -> list<any>`
- `sort_by(field: string, desc: bool = false) -> list<any>`
- `default(value: any) -> any`
- `ensure_list() -> list<any>`

## Pipeline typing

Each pipeline step must accept previous output type.
Type mismatch → `F451`.

Valid:

```facet
@vars
  clean: "  FACET  " |> trim() |> lowercase()
  arr: { a: 1, b: 2 } |> keys()
```

Invalid:

```facet
@vars
  bad: 42 |> trim()
```

## Layout strategy constraints

When a lens pipeline is used as a section `strategy`:

- Pure mode requires Level-0 only (else `F801`)
- strategy must be deterministic, idempotent, total for valid NFC+LF strings
- strategy must not depend on locale/time/env/filesystem/network

## Regex safety

Any regex-capable lens must use linear-time safe regex behavior (RE2-class or proven subset) to qualify as Level-0.

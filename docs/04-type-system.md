---
permalink: /04-type-system.html
title: Type System
---

# 04. FACET Type System (FTS)

This page summarizes normative FTS behavior from FACET v2.1.3.

## Primitive types

- `string`
- `int`
- `float`
- `bool`
- `null`
- `any`

Constraint violations:

- type mismatch → `F451`
- invalid constraint/placement/unsupported construct → `F452`

## Composite types

- `struct { field: T, ... }`
- `list<T>`
- `map<string, T>`
- union: `T1 | T2 | ...`

Optional fields are expressed as `T | null`.

## Multimodal types

- `image` with constraints: `format ∈ {png,jpeg,webp}`, `max_dim`
- `audio` with constraints: `format ∈ {mp3,wav,ogg}`, `max_duration`
- `embedding<size=N>` where `N > 0`

## Assignability rules

`T1` assignable to `T2` iff one of:

1. `T2 == any`
2. same primitive + constraints satisfied
3. union target contains assignable member
4. list/map element assignability
5. struct target required fields exist and are assignable

## `@var_types` and `@vars`

`@var_types` is an ordered singleton map from variable name to FTS expression.

```facet
@var_types
  username: "string"
  age: "int"
  score: "float"
  status: "string | null"

@vars
  username: "alice"
  age: 30
  score: 97.5
  status: null
```

If a declared type exists, computed value must satisfy it.

## Input typing with `@input`

```facet
@vars
  query: @input(type="string") |> trim()
  n: @input(type="int", default=3)
```

- `type` is required and must parse as FTS type.
- supplied/defaulted runtime value must satisfy `type`, else `F453`.

## Pipeline typing

Each pipeline step must accept previous step output type.

```facet
@vars
  raw: "  HELLO  "
  clean: $raw |> trim() |> lowercase()
```

Invalid:

```facet
@vars
  bad: 42 |> trim()
```

This must fail with `F451` (wrong input type for lens).

## JSON Schema mapping

`@interface` signatures must map to JSON Schema (Appendix D):

- primitive mappings (`string/int/float/bool/null/any`)
- struct → object with `required`
- list/map/union mapping
- `embedding<size=N>` mapped to fixed-length numeric array

Unmappable type usage must raise `F452`.

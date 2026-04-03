---
permalink: /13-import-system.html
title: Import System
---

# 13. `@import` Resolution and Merge

This page covers FACET v2.1.3 import and merge semantics.

## Basic import

```facet
@import "relative/path/module.facet"
```

Rules:

1. resolved relative to importing file
2. constrained by allowlisted roots
3. deterministic source-order expansion (in-place)

## Security constraints (`F601`)

The following must be rejected:

- absolute paths
- `..` traversal
- URL imports
- paths outside allowlisted roots

Missing file also raises `F601`.
Import cycle raises `F602`.

## Standard facet cardinality

Singleton-map facets (deep-merged):

- `@meta`, `@context`, `@vars`, `@var_types`, `@policy`

Repeatable-block facets (concatenated):

- `@interface`, `@system`, `@user`, `@assistant`, `@test`

## Singleton merge behavior

- first appearance inserts key position
- later override changes value but **keeps first position**
- nested map values deep-merge recursively
- lists are not deep-merged unless keyed-list merge is explicitly enabled or facet-specific rule applies (`@policy.allow/deny`)

## Repeatable merge behavior

Repeatable blocks are appended in resolved encounter order.

## Example: override with stable key order

`a.facet`:

```facet
@vars
  x: "a"
  y: "a"
```

`b.facet`:

```facet
@vars
  y: "b"
  z: "b"
```

`main.facet`:

```facet
@import "a.facet"
@import "b.facet"
```

Effective `@vars` values:

- `x="a"`, `y="b"`, `z="b"`

Effective key order remains first-insertion order: `x, y, z`.

## Policy list merge (`allow`/`deny`)

Inside `@policy`:

- rules with string `id` merge by keyed id
- rules without `id` append in encounter order
- invalid non-string `id` raises `F452`

## Determinism implications

Phase 1 output must be deterministic:

- Resolved Source Form
- Resolved AST ordering

These order guarantees are later used by Phase 3 tie-break and canonical rendering.

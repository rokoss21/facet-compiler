---
permalink: /15-execution-model.html
title: Execution Model
---

# 15. Execution Model (Phases 1–5)

Hypervisor execution order is strict:

1. **Resolution**
2. **Type checking**
3. **Reactive compute (R-DAG)**
4. **Layout (Token Box Model)**
5. **Render (Canonical JSON)**

## Phase 1: Resolution

- UTF-8 + NFC + LF normalization
- parse to AST
- import resolution
- deterministic merge

## Phase 2: Type checking

- validate facets and schemas
- pipeline typing
- interface schema mappability
- `@input` placement/type constraints
- policy schema and condition typing

AST is immutable after successful Phase 2.

## Phase 3: R-DAG

- build dependency graph from `$var` references
- detect unknown refs (`F401`) and cycles (`F505`)
- evaluate topologically
- tie-break independent nodes by merged ordered-map insertion order
- materialize runtime `@input` values
- enforce gas/mode/cache/policy/guard rules

## Phase 4: Layout

- compute section sizes in FACET Units
- enforce critical load
- apply deterministic compression/truncation/drop for flexible sections
- preserve canonical message order

## Phase 5: Render

- emit canonical JSON
- emit provider payload preserving canonical semantics
- optionally emit execution artifact (Hypervisor run/test)

## Modes

- **pure**: no volatile/external behavior; strict cache-only for Level-1 lenses
- **exec**: runtime effects possible if allowed by policy + guard

---
permalink: /03-architecture.html
title: Architecture
---

# 03. Architecture (FACET v2.1.3)

FACET architecture is phase-driven and deterministic for fixed normalized source, profile, mode, runtime inputs, and host configuration.

## Compiler/Runtime Pipeline

1. **Phase 1: Resolution**
   - UTF-8/NFC/LF normalization
   - parse to AST
   - `@import` resolution
   - deterministic merge
2. **Phase 2: Type Checking**
   - `@vars` / `@var_types`
   - pipeline type checks
   - facet schema checks
   - `@input` placement checks
   - policy schema/type checks
3. **Phase 3: Reactive Compute (R-DAG)**
   - dependency graph on `$var` references
   - topological evaluation with stable insertion-order tie-break
   - input materialization for `@input`
   - lens execution subject to mode/profile/policy/guard/gas/cache
4. **Phase 4: Layout (Token Box Model)**
   - budget in FACET Units (UTF-8 byte length)
   - deterministic packing, truncation, and drop rules
5. **Phase 5: Render**
   - Canonical JSON output
   - provider payload preserving canonical semantics

## Profiles

- **Core profile**: parse+validate+render subset, strict `F801` on Hypervisor-only constructs.
- **Hypervisor profile**: full phases 1–5, `@interface`, `@input`, `@test`, guard, policy enforcement, execution artifact.

## Determinism Boundaries

Deterministic by spec:

- source normalization
- merge ordering
- AST ordering
- type checks and error codes
- R-DAG evaluation order
- layout and canonical ordering

Not standardized by FACET:

- model generation internals
- provider-specific inference stochasticity

## Core Data Objects

- **Resolved Source Form**: normalized, imports expanded.
- **Resolved AST**: ordered maps/lists with spans.
- **Effective Policy Object**: merged + validated `@policy`.
- **Canonical JSON**: provider-agnostic request context.
- **Execution Artifact** (Hypervisor): guard decisions + hash-chain provenance.

## Canonical JSON contract

The canonical object includes:

- `metadata` (`facet_version`, `profile`, `mode`, `host_profile_id`, `document_hash`, `policy_hash`, `policy_version`, `budget_units`, `target_provider_id`)
- `tools` (ordered interface schemas)
- `messages` (ordered `system -> user -> assistant`)

## Failure Surfaces

- Parse/lex/normalization: `F001/F002/F003/F402`
- Semantic/type: `F401/F405/F451/F452/F453/F456`
- Graph/import/runtime/layout: `F505/F601/F602/F801/F802/F803/F901/F902`
- Guard policy fail-closed: `F454/F455`

## Practical Guidance

- Keep message bodies in `content` fields only.
- Keep `@context` for layout (`budget` and `defaults`) only.
- Keep business/tool access decisions in `@policy`, not in prompt prose.
- Treat Canonical JSON as the stable contract boundary for integrations.

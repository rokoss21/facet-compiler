---
permalink: /15-execution-model.html
title: Execution Model
---

# 15. FACET v2.1.3 Execution Model
**Reading Time:** 10-15 minutes | **Difficulty:** Advanced | **Previous:** [03-architecture.md](03-architecture.html)
**Compiler Version:** 0.1.2 | **Spec Version:** 2.1.3

This page defines the execution model in engineering terms: execution unit, state, transitions, failure behavior, and reproducibility limits.

## 1. Execution Unit

An execution unit is the tuple:

`U = (resolved_source, profile, mode, host_profile_id, target_provider_id, runtime_inputs, policy_version)`

Where:
- `resolved_source` is NFC+LF normalized and import-expanded.
- `profile` is `core` or `hypervisor`.
- `mode` is `pure` or `exec`.
- `runtime_inputs` are values supplied for `@input(...)`.

`document_hash = sha256(resolved_source)` identifies the compiled contract.

## 2. State Model

A run is a sequence of states:

`S0 -> S1 -> S2 -> S3 -> S4 -> S5`

- `S0`: normalized source loaded
- `S1`: resolved AST (imports + merge applied)
- `S2`: typed AST (Phase 2 checks passed)
- `S3`: computed vars (R-DAG evaluated)
- `S4`: layout-packed sections within budget
- `S5`: canonical JSON rendered

In Hypervisor runs/tests, an optional execution artifact state is also produced (guard decisions + hash chain).

## 3. Transition Semantics

Transitions are pure over `U` for compile/eval steps, except where mode/profile allow guarded external effects.

- `T1 (Resolution)`: `S0 -> S1`
- `T2 (Type Check)`: `S1 -> S2`
- `T3 (Reactive Compute)`: `S2 -> S3`
- `T4 (Layout)`: `S3 -> S4`
- `T5 (Render)`: `S4 -> S5`

If any transition fails, execution stops with the corresponding `F*` error.

## 4. Compile-Time vs Runtime Boundary

### Compile-Time (Hermetic Contract Checks)

- Parse, normalize, resolve imports, apply deterministic merge.
- Validate types, placements, policies, and interface/lens signatures.
- No network I/O. Import file access restricted to sandbox rules.

### Runtime (Bounded Evaluation)

- Materialize `@input` values.
- Evaluate R-DAG, lens pipelines, gas accounting.
- Apply layout budget logic.
- Enforce guard decisions for guarded operations (Hypervisor).

### Out of FACET Scope

- Provider-side generation randomness.
- External API truthfulness.
- Business-domain correctness of model output.

## 5. Failure Semantics

FACET failure behavior is fail-fast and fail-closed.

- Compile-time failure: no runtime side effects start.
- Runtime policy deny: `F454` (deterministic deny).
- Guard undecidable: `F455` (fail-closed).
- Pure mode cache miss for Level-1 lens: `F803`.
- Critical budget overflow: `F901`.

No silent downgrade: errors are surfaced as contract-visible outcomes.

## 6. Failure Scenario: Invalid Model Output

Scenario:
1. FACET compiles and emits canonical request context successfully.
2. Provider returns syntactically valid text but semantically invalid JSON for your app schema.

Interpretation:
- FACET layer: success (request contract execution completed).
- Application layer: schema validation failure (outside FACET language core).

Recommended handling:
1. Validate model output against an explicit schema in host code.
2. Apply deterministic retry policy (bounded attempts, logged reason).
3. Route to fallback/degraded path when retries are exhausted.

This boundary is intentional: FACET constrains request construction and execution behavior; response acceptance policy remains an application concern.

## 7. Replay and Reproducibility

What can be replayed deterministically:
- Resolved source and `document_hash`.
- Canonical JSON for identical execution tuple `U`.
- Guard decisions/hash-chain artifact (Hypervisor, when enabled).

What is not guaranteed byte-for-byte:
- Provider model textual output.

Correct phrasing:
- FACET is a deterministic contract/execution layer.
- End-to-end model inference remains probabilistic unless constrained by provider-side controls.

## 8. What FACET Does Not Solve

- It does not make LLM inference mathematically deterministic.
- It does not verify correctness of external APIs.
- It does not replace domain business logic or product policy.
- It does not provide legal certification by itself.

## References

- [FACET v2.1.3 Production Language Specification](https://github.com/rokoss21/facet-compiler/blob/master/FACET-v2.1.3-Production-Language-Specification.md)
- [Architecture](03-architecture.html)
- [Security](11-security.html)
- [Error Codes](12-errors.html)

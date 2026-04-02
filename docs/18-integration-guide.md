---
permalink: /18-integration-guide.html
title: Integration Guide
---

# 18. Integration Guide (Brownfield)
**Reading Time:** 10-12 minutes | **Difficulty:** Intermediate
**Compiler Version:** 0.1.2 | **Spec Version:** 2.1.3

This guide focuses on integrating FACET into existing systems without a full rewrite.

## 1. Minimal Insertion Points

### Pattern A: Sidecar Compiler (lowest risk)

- Keep current application flow.
- Replace prompt assembly function with `fct run` call.
- Continue using your existing provider client and response validator.

### Pattern B: CI Contract Gate

- Keep runtime unchanged initially.
- Add `fct build` in CI for all `.facet` contracts.
- Block merges on contract/type/policy failures.

### Pattern C: Runtime Guard Rollout

- Start in `core` profile for static validation.
- Move selected flows to `hypervisor` with policy/guard events.
- Enable strict fail-closed behavior for high-risk operations.

## 2. Migration Plan (Incremental)

1. Inventory 3 high-value prompt flows.
2. Port them to `.facet` contracts.
3. Add CI gate: `fct build --input <file>`.
4. Add runtime call path: `fct run` -> provider.
5. Enforce schema validation on provider response.
6. Add bounded retry/fallback policy.

## 3. Cost of Adoption

### Engineering Cost

- Initial contract modeling effort.
- Host-side response validation and retry policy wiring.
- Policy/guard setup for Hypervisor flows.

### Runtime Cost

- Additional per-request compile/run overhead.
- Artifact/telemetry storage if guard provenance is enabled.

### Risk Reduction Value

- Contract errors move left (compile-time).
- Runtime failures become explicit and classifiable.
- Guarded operations become auditable.

## 4. Compatibility Strategy

- Keep provider SDKs unchanged.
- Keep business logic unchanged.
- Replace only request construction and policy gating layer first.

## 5. Anti-Patterns

- Treating FACET as response truth validator by itself.
- Skipping host response schema checks.
- Enabling retries without strict bounds.
- Using nondeterministic host defaults for policy decisions.

## 6. Recommended “First Week” Checklist

- [ ] Add `fct build` in CI on changed contracts.
- [ ] Add one production flow via sidecar integration.
- [ ] Log `document_hash` and `policy_hash` per request.
- [ ] Add deterministic retry/fallback policy.
- [ ] Validate post-model response schema in host code.

## 7. Related

- [Execution Model](15-execution-model.html)
- [Production Scenario](16-production-scenario.html)
- [Mini Benchmark Report](17-benchmark-report.html)

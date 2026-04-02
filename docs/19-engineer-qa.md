---
permalink: /19-engineer-qa.html
title: Engineer Q&A
---

# 19. Engineer Q&A (Hard Questions)
**Reading Time:** 6-8 minutes

## Q: Is FACET deterministic end-to-end?
No. FACET guarantees deterministic contract compilation/execution boundaries; model generation remains probabilistic.

## Q: What exactly is deterministic then?
Normalization, import/merge order, type/policy checks, R-DAG traversal, layout ordering, canonical JSON, and guard decision evaluation rules.

## Q: Where is compile-time boundary?
Parsing/resolution/type/policy validation are compile-time checks; they run before runtime side effects.

## Q: What happens on contract violation?
Execution stops immediately with explicit `F*` error; execution never continues after contract violation.

## Q: What about malformed model output?
That is application-layer response validation; FACET controls request-side correctness, host controls response acceptance policy.

## Q: Is this a framework replacement?
No. FACET is a control layer for request contracts and bounded execution behavior.

## Q: Can I adopt it without rewriting everything?
Yes. Insert FACET as sidecar request compiler and keep existing provider client/business logic.

## Q: Where is auditability?
Use `document_hash`, `policy_hash`, and (Hypervisor) guard decision artifact/hash chain.

## Related

- [Execution Model](15-execution-model.html)
- [Integration Guide](18-integration-guide.html)
- [Production Scenario](16-production-scenario.html)

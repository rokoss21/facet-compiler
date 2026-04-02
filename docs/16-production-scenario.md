---
permalink: /16-production-scenario.html
title: Production Scenario
---

# 16. Production Failure Scenario
**Reading Time:** 10-12 minutes | **Difficulty:** Advanced
**Compiler Version:** 0.1.2 | **Spec Version:** 2.1.3

This page shows a realistic multi-step path with failure, retry, and fallback behavior.

## 1. Contract

**Example contract:** [production_support_flow.facet](examples/production_support_flow.facet)

```facet
@meta
  name: "Support Escalation Contract"
  version: "1.0"

@context
  budget: 32000

@vars
  tenant: @input(type="string", default="default")
  query: @input(type="string")
  normalized_query: $query |> trim() |> lowercase()

@system
  content: "Classify support intent. Return JSON with keys: action, confidence, reason."

@user
  content: $normalized_query
```

## 2. Runtime Chain

1. Host calls `fct run` and gets canonical JSON.
2. Host sends canonical JSON to model provider.
3. Host validates model response against strict app schema.
4. If schema fails, host applies bounded retry policy.
5. If retries fail, host routes to deterministic fallback.

## 3. Failure Semantics

### Case A: Contract violation (FACET layer)

Examples:
- missing required `@input` value (`F453`)
- denied operation by policy (`F454`)
- guard undecidable (`F455`)

Behavior:
- Execution stops immediately.
- No next transition is executed.
- Error code is emitted as the contract outcome.

**Rule:** execution never continues after contract violation.

### Case B: Invalid model payload (application layer)

Examples:
- model returns malformed JSON
- JSON shape does not match expected app schema

Behavior:
- FACET run itself is considered successful (request contract executed).
- Host response validator fails the response path.
- Host decides retry/fallback (outside FACET core).

## 4. Deterministic Retry Policy (Host)

A practical policy:

- max attempts: `2`
- retry only for schema/parse failure
- fixed backoff: `250ms`
- no unbounded loops

Pseudo-flow:

```text
compile+run -> provider call -> schema validate
  success -> continue
  fail    -> retry(1)
  fail    -> retry(2)
  fail    -> fallback("manual-review")
```

## 5. Why This Is Production-Safe

- FACET constrains request construction and guarded operations.
- Host constrains response acceptance and recovery policy.
- Both sides expose explicit failure surfaces.

## 6. Related References

- [Execution Model](15-execution-model.html)
- [Error Codes](12-errors.html)
- [Security](11-security.html)

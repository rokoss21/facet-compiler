---
permalink: /10-performance.html
title: Performance
---

# 10. Performance & Determinism Notes

FACET performance is constrained by deterministic semantics in the spec.

## Main cost centers

1. Parse + validation (Phases 1–2)
2. R-DAG compute and lens execution (Phase 3)
3. Layout packing/truncation (Phase 4)
4. Canonical serialization (Phase 5)

## Deterministic constraints that affect performance

- ordered-map merge with stable first insertion positions
- deterministic topological traversal tie-break in `@vars`
- deterministic layout ordering and truncation
- canonical JSON serialization (JCS)

## Gas and budget controls

- lens calls consume gas; over limit must fail with `F902`
- layout budget overflow on critical sections must fail with `F901`
- pure mode Level-1 cache miss must fail with `F803`

## Practical optimization checklist

- keep `@vars` DAG shallow and explicit
- use Level-0 deterministic lenses where possible
- avoid unnecessary large string payloads in messages
- tune `--budget` for realistic context sizes
- separate heavy documents into inputs/vars and compact before message emission

## Useful commands

```bash
facet-fct build --input contract.facet
facet-fct inspect --input contract.facet --dag dag.json --layout layout.json --policy policy.json
facet-fct run --input contract.facet --format pretty --budget 8192
facet-fct test --input contract.facet --output summary --gas-limit 10000
```

## Reproducibility guidance

For stable runs compare:

- normalized source + imports
- profile/mode
- host profile id
- runtime input payload
- policy hash
- budget and gas settings

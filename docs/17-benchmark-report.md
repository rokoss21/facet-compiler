---
permalink: /17-benchmark-report.html
title: Benchmark Notes
---

# 17. Benchmark Notes

This page documents benchmark methodology goals for FACET compiler/runtime behavior.

## What to measure

- Phase 1 parse/resolve time
- Phase 2 validation time
- Phase 3 R-DAG compute time
- Phase 4 layout time
- Phase 5 render time
- memory usage by phase

## Required benchmark controls

- fixed normalized source corpus
- fixed runtime input payloads
- fixed profile/mode
- fixed host profile id and budget/gas limits
- warm/cold separation for cache-sensitive paths

## Determinism check

For identical benchmark inputs, verify:

- same `document_hash`
- same `policy_hash`
- stable canonical message ordering
- stable guard decision ordering (if artifact emitted)

## Suggested command baseline

```bash
facet-fct build --input bench.facet
facet-fct inspect --input bench.facet --dag dag.json --layout layout.json --policy policy.json
facet-fct run --input bench.facet --format json --budget 8192 --exec
```

---
permalink: /17-benchmark-report.html
title: Benchmark Report
---

# 17. Mini Benchmark Report
**Reading Time:** 8-10 minutes | **Difficulty:** Advanced
**Compiler Version:** 0.1.2 | **Spec Version:** 2.1.3

This benchmark is a quick engineering signal, not a formal performance paper.

## 1. Goal

Show operational overhead of FACET execution compared with naive JSON assembly.

- **Baseline:** direct JSON construction with `jq` (no type checks, no policy checks, no R-DAG).
- **FACET:** full `facet-fct run` pipeline.

## 2. Environment

- Host: Apple M2 Pro
- OS: macOS
- Binary: `target/release/facet-fct` (`fct 0.1.2`)

## 3. Commands Used

### Baseline (basic payload, 1000 iterations)

```bash
/usr/bin/time -p /tmp/bench_baseline_basic.sh
```

Script body:

```bash
jq -c . examples/basic_prompt.input.json >/dev/null
jq -n '{"messages":[{"role":"system","content":"You are a helpful assistant."},{"role":"user","content":"Hello"}]}' >/dev/null
```

### FACET run (basic contract, 200 iterations)

```bash
/usr/bin/time -p sh -c 'for i in $(seq 1 200); do target/release/facet-fct run --input examples/basic_prompt.facet --format json >/dev/null; done'
```

### Baseline (RAG-like payload, 500 iterations)

```bash
/usr/bin/time -p /tmp/bench_baseline_rag.sh
```

### FACET run (RAG contract, 100 iterations)

```bash
/usr/bin/time -p sh -c 'for i in $(seq 1 100); do target/release/facet-fct run --input examples/rag_pipeline.facet --format json >/dev/null; done'
```

### FACET build-only (300 iterations each)

```bash
/usr/bin/time -p sh -c 'for i in $(seq 1 300); do target/release/facet-fct build --input examples/basic_prompt.facet >/dev/null; done'
/usr/bin/time -p sh -c 'for i in $(seq 1 300); do target/release/facet-fct build --input examples/rag_pipeline.facet >/dev/null; done'
```

## 4. Results

| Scenario | Total Time | Iterations | Avg per Iteration |
|---|---:|---:|---:|
| Baseline basic JSON assembly | 5.57s | 1000 | 5.57ms |
| FACET run (`basic_prompt.facet`) | 40.30s | 200 | 201.50ms |
| Baseline RAG-like JSON assembly | 2.51s | 500 | 5.02ms |
| FACET run (`rag_pipeline.facet`) | 20.37s | 100 | 203.70ms |
| FACET build (`basic_prompt.facet`) | 50.07s | 300 | 166.90ms |
| FACET build (`rag_pipeline.facet`) | 49.46s | 300 | 164.87ms |

## 5. Interpretation

- Naive JSON assembly is faster, but it does not provide contract/type/policy guarantees.
- FACET overhead is the cost of deterministic validation and bounded execution semantics.
- In exchange, failures are explicit (`F*`), and system behavior is controlled rather than best-effort.

## 6. Limits of This Benchmark

- Synthetic setup, single machine.
- No provider network latency included.
- Not a throughput benchmark under concurrent load.

For release gating, use CI matrix + workload-specific perf tests.

## 7. Related

- [Performance Guide](10-performance.html)
- [Execution Model](15-execution-model.html)
- [Production Scenario](16-production-scenario.html)

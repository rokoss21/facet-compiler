---
permalink: /
---

# FACET v2.1.3 Documentation

FACET is a deterministic execution layer for LLM systems.
It compiles structured contracts into reproducible, resource-bounded runtime behavior.

## What Problem FACET Solves

- LLM pipelines are often hard to reproduce, hard to validate, and hard to audit.
- Prompt/runtime glue code tends to mix schema, policy, and execution concerns.
- Production systems need explicit failure surfaces, not best-effort behavior.

## What FACET Guarantees

- Deterministic compilation pipeline and canonical request assembly (for fixed normalized source, inputs, profile, and mode).
- Type-checked contracts with explicit error codes.
- Bounded execution via budget/gas/policy controls.

## What FACET Does Not Solve

- FACET does not eliminate model-level nondeterminism in generation.
- FACET does not guarantee external API correctness.
- FACET does not replace domain business logic.
- FACET constrains model interaction so system-level correctness remains controlled by contracts, policy, and validation.

## Quick Path

1. [Quick Start](01-quickstart.html)
2. [Tutorial](02-tutorial.html)
3. [Examples Guide](05-examples-guide.html)
4. [CLI Reference](06-cli.html)

## Evidence And References

- [FACET v2.1.3 Production Language Specification](https://github.com/rokoss21/facet-compiler/blob/master/FACET-v2.1.3-Production-Language-Specification.md)
- [v2.1.3 Migration Checklist / Conformance](14-v2.1.3-migration-checklist.html)
- [MIGRATION Summary](https://github.com/rokoss21/facet-compiler/blob/master/MIGRATION.md)
- [Project README](https://github.com/rokoss21/facet-compiler/blob/master/README.md)

## Deep Dives

- [Architecture](03-architecture.html)
- [Execution Model](15-execution-model.html)
- [Production Scenario](16-production-scenario.html)
- [Mini Benchmark Report](17-benchmark-report.html)
- [Integration Guide](18-integration-guide.html)
- [Engineer Q&A](19-engineer-qa.html)
- [Type System](04-type-system.html)
- [Lenses](08-lenses.html)
- [Testing](09-testing.html)
- [Performance](10-performance.html)
- [Security](11-security.html)
- [Error Codes](12-errors.html)
- [Import System](13-import-system.html)
- [API Reference](07-api-reference.html)
- [FAQ](faq.html)

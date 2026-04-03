---
permalink: /18-integration-guide.html
title: Integration Guide
---

# 18. Integration Guide

## Goal

Integrate FACET as a deterministic request-construction layer in an existing AI stack.

## Recommended flow

1. store `.facet` contracts in repo
2. run `facet-fct build` in CI
3. run `facet-fct test` for contract tests
4. run `facet-fct run` at execution time with runtime input JSON
5. pass canonical JSON to provider adapter

## Minimal runtime contract

- input: facet file + runtime input JSON
- output: canonical JSON (`metadata/tools/messages`)
- optional output: execution artifact for audit

## Integration rules

- treat canonical JSON as the boundary object
- keep provider-specific settings outside FACET syntax unless represented as host extensions
- keep policy in `@policy` and enforce via guard path
- log `document_hash` + `policy_hash` with downstream request ids

## CI template

```bash
facet-fct build --input contracts/main.facet
facet-fct test --input contracts/main.facet --output summary --pure
```

## Runtime template

```bash
facet-fct run --input contracts/main.facet --runtime-input runtime.json --format json --exec
```

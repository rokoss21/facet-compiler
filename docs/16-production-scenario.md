---
permalink: /16-production-scenario.html
title: Production Scenario
---

# 16. Production Scenario (Spec-Conformant)

Example: support-query contract with typed runtime input and deterministic normalization.

```facet
@meta
  version: "1.0"
  "x.acme.service": "support"

@context
  budget: 12000
  defaults:
    priority: 500
    min: 0
    grow: 0
    shrink: 0

@var_types
  tenant: "string"
  query: "string"
  normalized_query: "string"

@vars
  tenant: @input(type="string", default="default")
  query: @input(type="string")
  normalized_query: $query |> trim() |> lowercase()

@system
  content: "You are a support assistant. Keep answers concise and policy-compliant."

@user
  content: "[$tenant] $normalized_query"
```

Run:

```bash
facet-fct run --input scenario.facet --runtime-input runtime.json --format pretty --exec
```

Why this is production-safe by spec:

- explicit runtime inputs (`@input`)
- type declarations (`@var_types`)
- deterministic normalization pipeline
- bounded context budget
- canonical JSON output for downstream systems

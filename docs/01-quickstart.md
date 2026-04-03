---
permalink: /01-quickstart.html
title: Quick Start
---

# 01. FACET v2.1.3 Quick Start

This quick start uses only **normative FACET v2.1.3 syntax**.

## 1) Install

```bash
cargo install --git https://github.com/rokoss21/facet-compiler --bin facet-fct
facet-fct --version
```

Optional alias:

```bash
alias fct=facet-fct
```

## 2) Create a minimal file

Create `hello.facet`:

```facet
@meta
  version: "1.0"
  author: "Quick Start"

@context
  budget: 32000

@vars
  name: "World"
  greeting: "Hello"

@system
  content: "You are a helpful assistant."

@user
  content: "$greeting, $name!"
```

## 3) Validate and run

```bash
facet-fct build --input hello.facet
facet-fct run --input hello.facet --format pretty
```

Expected canonical JSON shape:

```json
{
  "metadata": {
    "facet_version": "2.1.3",
    "profile": "hypervisor",
    "mode": "exec",
    "host_profile_id": "...",
    "document_hash": "sha256:...",
    "policy_hash": null,
    "policy_version": "1",
    "budget_units": 32000,
    "target_provider_id": "..."
  },
  "tools": [],
  "messages": [
    { "role": "system", "content": "You are a helpful assistant." },
    { "role": "user", "content": "Hello, World!" }
  ]
}
```

## 4) Add pipeline transforms

```facet
@vars
  raw_query: "  What is FACET?  "
  query: $raw_query |> trim() |> lowercase()

@system
  content: "Answer briefly and clearly."

@user
  content: $query
```

`|>` is the FACET pipeline operator. In v2.1.3, standard Level-0 lenses are listed in Appendix A.

## 5) Add runtime input (`@input`)

```facet
@vars
  query: @input(type="string") |> trim()

@system
  content: "You are a helpful assistant."

@user
  content: $query
```

Create `runtime.json`:

```json
{ "query": "  Explain R-DAG in FACET  " }
```

Run:

```bash
facet-fct run --input input.facet --runtime-input runtime.json --format pretty
```

## 6) Common invalid patterns (spec violations)

The following are **not valid FACET v2.1.3 message fields** and should fail with `F452`:

```facet
@system
  role: "assistant"
  model: "vendor-model"
  instructions: "..."
```

`@context` is for layout budget/defaults, not RAG documents. This is invalid:

```facet
@context
  documents: ["..."]
```

## 7) Next pages

1. [Tutorial](02-tutorial.html)
2. [Examples Guide](05-examples-guide.html)
3. [CLI Reference](06-cli.html)
4. [Error Codes](12-errors.html)

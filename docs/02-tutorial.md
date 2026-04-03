---
permalink: /02-tutorial.html
title: Tutorial
---

# 02. FACET v2.1.3 Tutorial

This tutorial is intentionally strict: every valid snippet below follows FACET v2.1.3 normative syntax.

## 1) Minimal contract

```facet
@context
  budget: 32000

@vars
  prompt: "Explain deterministic compilation in one paragraph."

@system
  content: "You are a technical assistant."

@user
  content: $prompt
```

Run:

```bash
facet-fct build --input tutorial-01.facet
facet-fct run --input tutorial-01.facet --format pretty
```

## 2) Variables, references, and R-DAG

Forward references are allowed; unknown references are not.

```facet
@vars
  final_prompt: "$prefix: $query"
  prefix: "Question"
  query: @input(type="string") |> trim()

@system
  content: "Answer clearly."

@user
  content: $final_prompt
```

- `query` comes from runtime input.
- `final_prompt` depends on `prefix` and `query`.
- Evaluation order is resolved by dependency graph (Phase 3), with tie-break by ordered-map insertion rules.

## 3) `@input(...)` placement rules

Valid:

```facet
@vars
  q: @input(type="string")
  clean_q: @input(type="string") |> trim() |> lowercase()
```

Invalid (must raise `F452`):

```facet
@vars
  bad: { q: @input(type="string") }
```

## 4) Type declarations (`@var_types`)

```facet
@var_types
  age: "int"
  name: "string"
  status: "string | null"

@vars
  age: 30
  name: "Alice"
  status: null
```

Constraint examples:

```facet
@var_types
  score: "float"

@vars
  score: 98.5
```

Type mismatch must raise `F451`; constraint/placement violations must raise `F452`.

## 5) Message blocks and `when`

Valid message fields are limited to `content`, layout fields (`id|priority|min|grow|shrink|strategy`), and `when`.

```facet
@vars
  include_note: true

@system
  content: "You are concise."

@assistant(when=$include_note)
  content: "Acknowledged."

@user
  content: "Summarize FACET in 3 bullets."
```

Non-boolean `when` must raise `F451`.

## 6) Imports and deterministic merge

`base.facet`:

```facet
@vars
  a: "base"

@system
  content: "Base system message."
```

`main.facet`:

```facet
@import "base.facet"

@vars
  a: "override"
  b: "new"

@user
  content: "$a / $b"
```

Rules:

- Imports are expanded in source order.
- Singleton facets are deep-merged with stable first-insertion key positions.
- Repeatable facets (`@system`, `@user`, etc.) are concatenated in encounter order.

## 7) Interfaces and tool exposure

```facet
@interface WeatherAPI
  fn get_current(city: string) -> struct {
    temp: float
    condition: string
  } (effect="read")

@system
  tools: [$WeatherAPI]
  content: "Use weather tools when needed."

@user
  content: "Weather in Minsk?"
```

Missing function `effect` must raise `F456`.

## 8) Policy basics

```facet
@policy
  deny:
    - id: "deny-write-tools"
      op: "tool_call"
      effect: "write"
  allow:
    - id: "allow-weather-read"
      op: "tool_call"
      name: "WeatherAPI.get_current"
      effect: "read"

@system
  tools: [$WeatherAPI]
  content: "You can read weather only."
```

Key properties:

- Rule fields are conjunctive (AND).
- `deny` is evaluated before `allow`.
- Deterministic deny is `F454`; undecidable guard state is `F455`.

## 9) Testing (`@test`)

```facet
@vars
  query: @input(type="string")

@system
  content: "You are helpful."

@user
  content: $query

@test "basic"
  input:
    query: "hello"
  assert:
    - canonical.messages[0].role == "system"
    - canonical contains "hello"
```

Run tests:

```bash
facet-fct test --input tutorial-test.facet
```

## 10) What to avoid

These are **legacy/non-spec** and should not be used as valid FACET syntax:

- `@system` fields like `role`, `model`, `instructions`, `temperature`
- `@context.documents` or using `@context` as data storage
- non-standard directives such as `@if(...)` in expressions

## Next

1. [Architecture](03-architecture.html)
2. [Type System](04-type-system.html)
3. [Examples Guide](05-examples-guide.html)
4. [Error Codes](12-errors.html)

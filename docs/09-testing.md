---
permalink: /09-testing.html
title: Testing
---

# 09. Testing with `@test` (Hypervisor)

`@test` is required for Hypervisor profile conformance.

## Minimal syntax

```facet
@test "basic"
  vars:
    username: "TestUser"
  input:
    query: "hello"
  mock:
    WeatherAPI.get_current: { temp: 10, condition: "Rain" }
  assert:
    - canonical.messages[0].role == "system"
    - canonical contains "hello"
    - telemetry.gas_used < 5000
```

## Semantics

- each test runs in isolation
- `vars` overrides variables in test environment (still type-checked)
- `input` supplies runtime values for `@input`
- `mock` intercepts interface calls by fully-qualified name
- assertions run against:
  - `canonical`
  - `telemetry`
  - `execution` (recommended, if artifact exposed)

## Example file

```facet
@vars
  query: @input(type="string")

@system
  content: "You are helpful."

@user
  content: $query

@test "query-roundtrip"
  input:
    query: "hello"
  assert:
    - canonical.messages[1].role == "user"
    - canonical contains "hello"
```

Run:

```bash
facet-fct test --input test_example.facet
```

## Pure mode expectations

In pure mode tests:

- Level-1 cache miss → `F803`
- disallowed I/O → `F801`
- policy/guard deny/undecidable → `F454` / `F455`

## Practical tips

- keep tests deterministic (fixed inputs/mocks)
- assert on canonical structure and key values
- add policy tests for allow/deny behavior
- use `facet-fct inspect` when debugging failing assertions

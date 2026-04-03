---
permalink: /12-errors.html
title: Error Codes
---

# 12. Error Codes (Normative)

FACET reserves `F000–F999` for standard diagnostics.
Host extensions must use `X.<host>.<code>`.

## Parse / syntax

- `F001` invalid indentation (must be 2 spaces)
- `F002` tabs forbidden
- `F003` malformed syntax / invalid token / invalid escape / unclosed structure
- `F402` attribute interpolation forbidden (`{{` or `}}`)

## Semantic / type

- `F401` variable not found
- `F405` invalid variable path (missing field)
- `F451` type mismatch
- `F452` constraint violation / unsupported construct / invalid placement / invalid signature
- `F453` runtime input validation failed (`@input`)
- `F454` policy deny (deterministic)
- `F455` guard undecidable (fail-closed)
- `F456` missing/invalid effect declaration

## Graph / imports

- `F505` cyclic dependency in R-DAG
- `F601` import not found / disallowed path
- `F602` import cycle

## Runtime / mode / layout / gas

- `F801` prohibited I/O or construct disallowed by profile/mode
- `F802` unknown lens
- `F803` pure-mode cache miss for Level-1 lens
- `F901` critical overflow in Token Box Model
- `F902` gas exhausted

## Fast troubleshooting map

- Parse issues first (`F001/F002/F003`)
- Then placement/schema (`F452`)
- Then types (`F451/F453`)
- Then dependency graph (`F401/F405/F505`)
- Then runtime mode/policy (`F801/F803/F454/F455/F902`)

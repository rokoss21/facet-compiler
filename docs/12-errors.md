---
permalink: /12-errors.html
---

# 12. FACET v2.1.3 Error Codes Reference
**Reading Time:** 15-20 minutes | **Difficulty:** Intermediate | **Previous:** [11-security.md](11-security.html) | **Next:** [13-import-system.md](13-import-system.html)

This reference follows the normative error catalog in FACET v2.1.3.

## Table of Contents

- [Error Model](#error-model)
- [Syntax & Parse Errors (F001-F003, F402)](#syntax--parse-errors-f001-f003-f402)
- [Semantic & Type Errors (F401-F456)](#semantic--type-errors-f401-f456)
- [Graph Errors (F505)](#graph-errors-f505)
- [Import Errors (F601-F602)](#import-errors-f601-f602)
- [Runtime, Security, and Layout Errors (F801-F902)](#runtime-security-and-layout-errors-f801-f902)
- [Quick Reference](#quick-reference)

---

## Error Model

FACET treats errors as part of the contract surface, not incidental side effects.

- **Compile-time errors:** Parsing, resolution, type and policy validation failures (`build` / Phase 1-2).
- **Runtime errors:** Deterministic execution failures in reactive compute, guard checks, and layout (`run` / Phase 3-5).
- **Contract violations:** Invalid placement, unsupported constructs, or denied operations with explicit `F*` code.

Operational rule: if FACET cannot prove a guarded operation is safe/allowed, execution fails closed.

---

## Syntax & Parse Errors (F001-F003, F402)

### F001: Invalid indentation

**Description:** Indentation is not exactly 2 spaces per level.

**Example**
```facet
@system
   content: "bad"   # 3 spaces
```

**Fix:** Use exact 2-space indentation.

### F002: Tabs forbidden

**Description:** TAB (`\t`) appears in source.

**Fix:** Replace tabs with spaces.

### F003: Malformed syntax / invalid token / invalid escape

**Examples**
```facet
@vars
  text: "unclosed
```

```facet
@vars
  items: ["a", "b",]   # trailing comma
```

### F402: Attribute interpolation forbidden

**Description:** `{{` or `}}` used inside facet attributes.

**Example**
```facet
@system(model="{{gpt}}")
  content: "..."
```

---

## Semantic & Type Errors (F401-F456)

### F401: Variable not found

**Description:** Reference to missing variable.

```facet
@vars
  greeting: $missing
```

### F405: Invalid variable path

**Description:** Path segment missing in `$var.path` traversal.

```facet
@vars
  profile: { name: "Alice" }
  city: $profile.address.city
```

### F451: Type mismatch

**Description:** Value is not assignable to declared/required type.

### F452: Constraint violation / invalid placement / unsupported construct

Common cases:
- `@input(...)` placed outside allowed positions.
- String-keyed map entry outside `@meta`.
- Invalid policy rule shape.
- Numeric list indexing in var paths (not standardized in v2.1.3).

### F453: Runtime input validation failed

**Description:** `@input` value (supplied or defaulted) fails declared type/constraints.

### F454: Policy deny (deterministic)

**Description:** Policy evaluation succeeded and outcome is a deterministic DENY.

### F455: Guard undecidable (fail-closed)

**Description:** Guard cannot complete deterministic decision due to evaluation failure (missing data/type failure/internal guard failure).

### F456: Missing/invalid effect class

**Description:** Required effect declaration is missing or invalid.

Common cases:
- `@interface fn ...` missing `effect="..."`
- Level-1/2 lens registry entry missing `effect_class`

---

## Graph Errors (F505)

### F505: Cyclic dependency detected

**Description:** R-DAG contains direct or indirect cycle.

```facet
@vars
  a: $b
  b: $a
```

Forward references are allowed in FACET; cycles are not.

---

## Import Errors (F601-F602)

### F601: Import not found / disallowed import path

Raised for:
- File not found
- Absolute import path
- `..` traversal
- URL import
- Path outside allowlisted roots

### F602: Import cycle

Raised when imports form a cycle.

---

## Runtime, Security, and Layout Errors (F801-F902)

### F801: I/O prohibited / lens disallowed by profile or mode

Examples:
- Hypervisor-only construct used in Core profile.
- Level-2 lens in Pure mode.
- Runtime I/O outside allowed interfaces/lenses.

### F802: Unknown lens

Lens name not present in registry.

### F803: Pure cache miss

In Pure mode, Level-1 lens attempted without cache hit.

### F901: Critical overflow (Token Box Model)

Critical sections alone exceed budget.

### F902: Compute gas exhausted

Total gas for lens/runtime compute exceeded configured limit.

---

## Quick Reference

| Code | Category | Meaning |
|------|----------|---------|
| F001 | Syntax | Invalid indentation (must be 2 spaces) |
| F002 | Syntax | Tabs forbidden |
| F003 | Syntax | Malformed syntax / invalid token |
| F402 | Syntax | Attribute interpolation forbidden |
| F401 | Semantic | Variable not found |
| F405 | Semantic | Invalid variable path |
| F451 | Type | Type mismatch |
| F452 | Type/Semantic | Constraint violation / invalid construct |
| F453 | Runtime Input | Input validation failed |
| F454 | Policy | Deterministic policy deny |
| F455 | Guard | Undecidable guard (fail-closed) |
| F456 | Policy/Registry | Missing/invalid effect class |
| F505 | Graph | Cyclic dependency |
| F601 | Import | Import not found / disallowed path |
| F602 | Import | Import cycle |
| F801 | Runtime/Security | I/O prohibited or disallowed execution |
| F802 | Runtime | Unknown lens |
| F803 | Runtime/Pure | Cache-only miss |
| F901 | Layout | Critical overflow |
| F902 | Runtime | Gas exhausted |

## Next Steps

- [06-cli.md](06-cli.html) - CLI error handling and exit behavior
- [09-testing.md](09-testing.html) - Assertions for expected error paths
- [11-security.md](11-security.html) - Guard and policy fail-closed model
- [13-import-system.md](13-import-system.html) - Import sandbox and merge behavior
- [FACET v2.1.3 specification](https://github.com/rokoss21/facet-compiler/blob/master/FACET-v2.1.3-Production-Language-Specification.md) - Normative source

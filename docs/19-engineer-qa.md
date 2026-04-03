---
permalink: /19-engineer-qa.html
title: Engineer Q&A
---

# 19. Engineer Q&A

## Is FACET model output deterministic?

No. FACET standardizes deterministic contract compilation/execution boundaries, not model internals.

## What is deterministic then?

Normalization, parse/merge ordering, type checks, R-DAG evaluation order, layout packing rules, canonical JSON ordering, and policy/guard decision logic.

## Where is the integration boundary?

Canonical JSON (`metadata/tools/messages`) is the provider-agnostic boundary object.

## Why both `F454` and `F455`?

- `F454`: deterministic policy deny.
- `F455`: guard undecidable/evaluation failure; fail-closed deny.

## Why does `@system role/model` fail?

Because v2.1.3 message schema allows `content`, layout fields, optional `when`, and `@system.tools` only.

---
permalink: /faq.html
title: FAQ
---

# FAQ (FACET v2.1.3)

## 1) Why does `@system role/model/instructions` fail?

Because those fields are not in v2.1.3 message schema. Valid message fields are:

- `content` (required)
- `id|priority|min|grow|shrink|strategy` (optional layout)
- `when` (optional bool gate)
- `tools` (only in `@system`)

Invalid fields must raise `F452`.

## 2) Can I put documents into `@context`?

No. `@context` is layout configuration (`budget` and optional `defaults`).
RAG context should be represented via `@vars` + message `content`.

## 3) Is `@input(...)` supported?

Yes, in Hypervisor profile.
It must appear only as base expression of a `@vars` value (optionally followed by pipeline).
Invalid placement must raise `F452`.

## 4) Does FACET make LLM output deterministic?

No. FACET makes contract processing and request construction deterministic.
Model generation remains provider/model-dependent.

## 5) What are the most common errors?

- `F001/F002/F003`: indentation/tabs/syntax
- `F451/F452`: type or schema violations
- `F453`: runtime input fails declared type
- `F601/F602`: import sandbox/cycle
- `F801/F803`: disallowed mode/runtime behavior
- `F454/F455`: policy deny vs guard undecidable

## 6) Why does a valid-looking file fail in Core profile?

Core parses full syntax but rejects disallowed constructs with `F801`.
Examples: `@input`, interfaces, runtime-only behavior.

## 7) How are messages ordered in canonical JSON?

Always:

1. all `@system` blocks
2. then all `@user`
3. then all `@assistant`

Within each role, resolved source order is preserved.

## 8) Can layout reorder messages?

No. Layout may compress/truncate/drop flexible sections but must preserve canonical ordering.

## 9) How is `document_hash` computed?

`sha256` over Resolved Source Form (imports expanded, NFC+LF normalized).

## 10) What is `policy_hash`?

If policy exists:

`sha256(JCS({ policy_version, policy: EffectivePolicyObject }))`

Else `null`.

## 11) What is the difference between `F454` and `F455`?

- `F454`: deterministic deny (policy evaluated successfully)
- `F455`: guard could not deterministically evaluate (fail-closed)

## 12) Why did a Level-1 lens fail in pure mode?

Pure mode requires Level-1 cache-only execution.
Cache miss must raise `F803`.

## 13) Can I use string keys in any map?

String keys are allowed only in `@meta`.
Outside `@meta`, string-keyed map entries must raise `F452`.

## 14) Can I index lists with `$x.0`?

Numeric list indexing is not standardized in v2.1.3.
It must raise `F452` unless a namespaced host extension explicitly enables it.

## 15) What should I treat as the external API boundary?

Canonical JSON from Phase 5.
It is provider-agnostic and deterministic by spec constraints.

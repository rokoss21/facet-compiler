---
permalink: /07-api-reference.html
title: API Reference
---

# 07. Canonical API Surface (FACET v2.1.3)

FACET standardizes a language/runtime contract, not a provider-specific inference API.
The practical integration boundary is **Canonical JSON**.

## Canonical JSON object

Phase 5 must produce:

```json
{
  "metadata": {
    "facet_version": "2.1.3",
    "profile": "hypervisor",
    "mode": "exec",
    "host_profile_id": "...",
    "document_hash": "sha256:...",
    "policy_hash": "sha256:...",
    "policy_version": "1",
    "budget_units": 32000,
    "target_provider_id": "generic-llm"
  },
  "tools": [],
  "messages": [
    { "role": "system", "content": "..." },
    { "role": "user", "content": "..." }
  ]
}
```

## Metadata fields

- `facet_version`: fixed to `2.1.3`
- `profile`: `core | hypervisor`
- `mode`: `pure | exec`
- `host_profile_id`: stable host profile/version id
- `document_hash`: sha256 of Resolved Source Form
- `policy_hash`: `null` if no `@policy`, else policy hash per §16.2.4
- `policy_version`: policy DSL/guard semantics version (`"1"` for v2.1.3)
- `budget_units`: effective layout budget in FACET Units
- `target_provider_id`: provider routing id

## Ordering requirements

- `tools`: order follows resolved `@interface` declaration order.
- `messages`: all `@system` then `@user` then `@assistant`; each group preserves resolved source order.
- Layout may truncate/drop sections within budget, but must not reorder canonical message sequence.

## Tool exposure

When `@system.tools` exists:

- `canonical.tools` is union of referenced interfaces.
- In Hypervisor, `tool_expose` policy filtering is mandatory before emission.
- denied tools are omitted (not marked), and guard decisions are recorded in Execution Artifact.

## Content forms

Message `content` is either:

- `string`, or
- list of content items:
  - `{ type: "text", text: string }`
  - `{ type: "image", asset: <canonical asset> }`
  - `{ type: "audio", asset: <canonical asset> }`

## Canonical assets

Image asset shape (example):

```json
{
  "kind": "image",
  "format": "jpeg",
  "digest": { "algo": "sha256", "value": "..." },
  "shape": { "width": 1024, "height": 768 }
}
```

Audio asset shape (example):

```json
{
  "kind": "audio",
  "format": "wav",
  "digest": { "algo": "sha256", "value": "..." },
  "shape": { "duration": 3.2 }
}
```

## Serialization

Canonical JSON must use RFC 8785 (JCS) or equivalent canonicalization:

- UTF-8
- stable object key ordering
- stable number formatting

## Execution Artifact (Hypervisor)

When emitted, the artifact is separate from canonical JSON and includes:

- metadata linkage (`document_hash`, `policy_hash`, `policy_version`)
- guard decision events (`tool_call/lens_call/tool_expose/message_emit`)
- hash-chain head
- optional attestation envelope

See Appendix F for the normative schema.

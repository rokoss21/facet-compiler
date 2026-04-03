---
permalink: /11-security.html
title: Security
---

# 11. Security Model (Spec v2.1.3)

This page mirrors normative security behavior from §§16–17.

## 1) Hermetic compilation boundary

Phases 1–2 must be hermetic:

- no network access
- no filesystem access outside import roots
- no environment leakage
- deterministic behavior

## 2) Import sandbox

`@import` must be constrained to allowlisted roots.
Disallowed import patterns must raise `F601`:

- absolute paths
- `..` traversal
- URL imports
- outside allowlisted roots

Import cycle must raise `F602`.

## 3) Runtime I/O restrictions

Runtime I/O is allowed only through:

- `@input`
- registered Level-1/2 lenses under mode/policy/guard rules

Anything else must raise `F801`.

## 4) Policy and runtime guard

Hypervisor must enforce fail-closed runtime guard for guarded operations:

- `tool_call`
- `lens_call` (Level-2 always; Level-1 in exec mode)
- optionally `tool_expose` / `message_emit` (if host enables)

Guard decision must happen **before** prohibited external initiation.

## 5) Fail-closed error split

- `F454`: deterministic policy deny
- `F455`: guard undecidable/evaluation failure

`F455` must not replace `F454` when deterministic deny is available.

## 6) Effect classes

Standard effects: `read`, `write`, `external`, `payment`, `filesystem`, `network`.

Key rules:

- `read` is non-mutating by contract
- classes are not interchangeable
- Level-1/2 lenses require declared `effect_class` (`F456` if missing/invalid)

## 7) Provenance artifact

Hypervisor run/test may emit Execution Artifact with:

- guard decisions sequence
- per-event input hash
- hash-chain head seeded with document/policy/profile/mode/policy_version
- optional attestation envelope

Use artifact data for auditability and reproducible policy enforcement analysis.

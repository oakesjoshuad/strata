---
id: "EDR-0001"
title: "Identifier scheme — (kind, number) uniqueness, transactional per-kind allocation"
record-type: edr
status: accepted
revision: 2
date: 2026-08-19
slug: identifier-scheme-kind-number-uniqueness-transactional-per-kind-
tags: []
relationships: {}
---

# EDR-0001: Identifier scheme — (kind, number) uniqueness, transactional per-kind allocation

## Context

Every record needs an identifier that is stable, human-readable, and safe to reference
from other records (`record_relation.source_id` / `target_id`, ADR-0001's schema). The
specification proposes kind-prefixed, monotonically numbered identifiers — `RFC-0007`,
`PDR-0012`, `ADR-0029`, `EDR-0017` — with uniqueness enforced per kind, not globally
(`docs/specification.md`, section 6). This EDR fixes the concrete allocation mechanics:
where the next number comes from and how a race between two allocations in the same
kind is prevented.

Because SQLite serializes writers (ADR-0001), the actual mechanism is simpler than it
would be under concurrent multi-writer access: allocation just needs to happen inside
the same transaction as the record insert, so no two records in the same kind can ever
be assigned the same number, even if that were somehow attempted twice in quick
succession.

## Decision

We will allocate identifier numbers in the application core, not accept them from the
caller. On `strata new <kind> <title>`, the core computes the next number for that kind
as one more than the current maximum `number` for that kind (starting at 1 if none
exist), and inserts the new `engineering_record` row with that `(kind, number)` pair in
the same transaction — enforced additionally by the `UNIQUE(kind, number)` constraint
from ADR-0001's schema, so even a mistake in the allocation logic itself cannot produce
a duplicate.

## Considered Options

- **Numbers picked by the caller** (a human or agent specifies `RFC-0007` explicitly
  when creating a record).
- **Numbers allocated by the application core**, transactionally, as `MAX(number) + 1`
  within the target kind, inside the same transaction that inserts the new record.

## Consequences

- Callers never choose or guess an identifier; `strata new` always returns the
  identifier it assigned. This removes an entire class of possible mistake (two
  different records both claiming `ADR-0029`) at the cost of callers not being able to
  pre-reserve a specific number for a record they haven't created yet.
- Numbering is per-kind, not global — `RFC-0001` and `ADR-0001` can coexist; there is
  no shared counter across kinds. This matches the specification's stated identifier
  model directly (section 6) and means renumbering one kind never affects another.
- Deleting a record (if ever supported — not currently planned) would leave a gap in
  that kind's numbering rather than being reused, the same way most such schemes work;
  this EDR does not need to decide record deletion policy to hold, since no deletion
  path exists yet.

## Evidence

- `docs/specification.md`, section 6 (the originating identifier scheme this decision
  adopts).
- ADR-0001 (SQLite as canonical store): this decision's transactional-safety guarantee
  depends directly on SQLite's single-writer, transactional model — it would need
  reconsideration under any storage engine without that property.

---
id: "EDR-0006"
title: "Evidence identifiers are allocated per record, not globally"
record-type: edr
status: accepted
revision: 2
date: 2026-08-19
slug: evidence-identifiers-are-allocated-per-record-not-globally
tags: []
relationships:
  derived-from:
    - "EDR-0001"
  relates-to:
    - "PDR-0001"
---

# EDR-0006: Evidence identifiers are allocated per record, not globally

## Context

store/migrations/V1__initial.sql fixed `evidence.id TEXT PRIMARY KEY` from the specification's representative schema (docs/specification.md section 8.4) without fixing how a value for that column is generated -- the spec shows the column, not an allocation scheme. That gap sat unimplemented alongside the rest of the evidence and code_reference tables: both existed in the schema from the first migration, and records::Evidence/CodeReference existed in the domain model from the start, but nothing in store or cli ever read or wrote either table until store/src/evidence.rs and store/src/code_reference.rs landed. EDR-0001 already settled the analogous question for RecordId -- transactional MAX(number)+1 within a `(kind)` namespace, chosen specifically because SQLite serializes writers (ADR-0001) so allocating inside the same transaction as the insert is sufficient to prevent a duplicate, no additional locking needed. Evidence needed the same category of decision: a stable, unique, human-legible identifier, generated without adding a dependency this workspace doesn't otherwise need. That decision was made and shipped in store/src/evidence.rs without a record documenting it first, which is a smaller version of the same process gap RFC-0002 exists to prevent -- this EDR closes it after the fact rather than before, the same way RFC-0001 documents Strata's own founding retroactively. code_reference has no primary key column at all in V1__initial.sql (just `record_id, relation, path, symbol, line_start, line_end`), so it has no identifier to allocate and is out of scope for this decision entirely.

## Decision

We will allocate evidence ids per record rather than from a single global namespace: `format!("{record_id}-EV-{ordinal:03}")`, for example `ADR-0029-EV-001`, where `ordinal` is `COUNT(*) + 1` over existing evidence rows for that `record_id`, computed inside the same transaction as the `INSERT` (store::add_evidence, store/src/evidence.rs). This is EDR-0001's transactional MAX+1-in-namespace pattern applied again, with the namespace narrowed from a `RecordKind` to a single record. The full id string is globally unique because it is prefixed by `record_id`, which is itself already globally unique (ADR-0001's schema, `UNIQUE(kind, number)` on engineering_record).

## Considered Options

A single global monotonic counter across all evidence rows (e.g. `EV-00001`) -- rejected: evidence is only ever looked up scoped to its owning record (store::list_evidence takes a record_id), so a global counter buys no real query benefit and is less legible than a record-prefixed id at the one place ids are actually read by a human or agent, `strata graph <id>` output. A UUID or other random identifier -- rejected: this workspace has no UUID dependency in [workspace.dependencies], and adding one for a single ID field is disproportionate; a random id is also strictly less legible than a record-prefixed one in exactly the read path (graph output, traceability queries) this data exists to serve. Dropping the synthetic `evidence.id` column entirely in favor of a composite `(record_id, ordinal)` primary key -- rejected: `evidence.id TEXT PRIMARY KEY` is already fixed by V1__initial.sql under ADR-0001's schema, and records::model::Evidence already carries `id: String` as an independent field referenced on its own (not always alongside its owning record_id); revisiting the column shape itself is out of scope for this EDR.

## Consequences

Evidence ids are unique only by construction of the full string, not because each record's local `-EV-NNN` suffix is globally distinct -- two different records will each have their own `-EV-001` -- but this matches how RecordId's own `(kind, number)` pairs already work (ADR-0001's `UNIQUE(kind, number)`, not a single global number), so it is consistent with an existing precedent rather than a new kind of non-uniqueness. Deleting evidence (not currently supported, same caveat EDR-0001 states for RecordId) would leave a gap in that record's ordinal sequence rather than reusing it. No new dependency was added to the workspace to support this. This decision is scoped to `evidence.id` only; `code_reference` has no identifier of its own and this EDR does not extend to it.

## Evidence

store/migrations/V1__initial.sql: `evidence.id TEXT PRIMARY KEY` fixes the column this EDR allocates values for, without itself specifying a scheme. EDR-0001 (identifier scheme -- kind, number uniqueness, transactional per-kind allocation): the precedent this decision extends, narrowing the allocation namespace from a RecordKind to a single record. ADR-0001 (SQLite as Strata's canonical persistent store): the single-writer transactional guarantee both this decision and EDR-0001 depend on for allocation safety. store/src/evidence.rs (implemented ahead of this record, the specific gap this EDR closes retroactively): `Store::add_evidence` computes `ordinal` via `SELECT COUNT(*) + 1 FROM evidence WHERE record_id = :record_id` inside the same transaction as the row insert. Workspace Cargo.toml's `[workspace.dependencies]`: no uuid or similar crate is present, confirming the alternative considered above would have been new dependency surface with no other consumer in the workspace.

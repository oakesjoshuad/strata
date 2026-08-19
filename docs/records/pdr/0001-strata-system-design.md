---
id: "PDR-0001"
title: "Strata system design"
record-type: pdr
status: approved
revision: 3
date: 2026-08-19
slug: strata-system-design
tags: []
relationships:
  produces:
    - "ADR-0001"
    - "ADR-0002"
    - "ADR-0003"
    - "ADR-0004"
    - "ADR-0005"
    - "ADR-0006"
    - "ADR-0007"
    - "EDR-0001"
    - "EDR-0002"
---

# PDR-0001: Strata system design

## Problem

RFC-0001 decided that Strata should exist: a Rust CLI over a SQLite knowledge base for
RFC/PDR/ADR/EDR records, replacing Markdown-file-and-convention tooling. This document
covers how that system is shaped for its first working version — the record model, the
storage schema, the CLI/core split, and search — and draws on
`docs/specification.md` (the originating draft specification) without restating it
verbatim.

This document assumes a single local user and a single local SQLite file. It does not
cover multi-user access, network exposure, or hosted deployment — none of those are in
scope for the first version, and nothing here should be read as ruling them out later.

## Requirements

Carried directly from `docs/specification.md` section 5 (record semantics) and section
30 (security and trust boundary):

- Each record kind (RFC, PDR, ADR, EDR) has its own required document fields and its
  own lifecycle states; infrastructure (identity, storage, revisioning, search) is
  shared across kinds.
- Identifiers are `(kind, number)` pairs, unique per kind, allocated transactionally
  (EDR-0001).
- Relationships between records are explicit graph edges, not prose conventions —
  `derived-from`, `produces`, `constrains`, `supersedes`, and similar (spec section 7).
- An LLM agent interacting with Strata gets the same validated CLI surface a human
  gets. It is not trusted merely because it is using a documented skill (spec section
  30) — every mutation goes through the same validation regardless of caller.

## Constraints

**Goals:**
- A record can be created, revised, linked to other records, searched, and inspected
  entirely through the CLI, with no other write path into the database.
- Every mutation is transactional: a record's current state, its revision history, its
  relationships, and its search index stay consistent with each other at all times —
  there is no window where one of those four is stale relative to the others.
- A bad request (a link to a record that does not exist, a relationship kind that
  is not defined, a document missing a required field) fails without partially
  mutating the database.
- The tool is useful without Pandoc, without a static site, and without an MCP server
  — those are additive later, not prerequisites for a working v0.1.

**Non-goals** (things that could reasonably have been goals here, but are deliberately
excluded from this document and from v0.1):
- Rendering records to Markdown, HTML, PDF, or any other output format. Storage
  correctness and the CLI's own read commands (`show`, `search`, `--json`) are enough
  to use the tool; rendering is a separate, later concern (see "v0.1 scope cut" below).
- Vector search or embeddings. FTS5 is the retrieval mechanism for v0.1 in full; this
  is not a placeholder pending a better search backend, it is the actual plan unless a
  real retrieval failure shows FTS5 is insufficient.
- An MCP adapter (ADR-0005).

## Proposed Design

**Storage.** SQLite is the canonical store (ADR-0001). The core tables, adapted from
the specification's representative schema (section 8):

- `engineering_record` — `id`, `kind`, `number`, `title`, `status`, `document` (JSON,
  `CHECK(json_valid(document))`), `revision`, `created_at`, `updated_at`,
  `UNIQUE(kind, number)`.
- `record_relation` — `source_id`, `relation`, `target_id`, with foreign keys into
  `engineering_record` on both ends, so a relationship can never point at a record
  that does not exist.
- `record_revision` — `record_id`, `revision`, `document`, `changed_at`,
  `changed_by`, `change_summary`. Every meaningful mutation preserves the prior
  document state here before it is overwritten.
- `evidence` and `code_reference` — first-class tables so a decision can cite a
  benchmark, an experiment, an issue, or a specific file/line range, rather than
  referencing them only in prose.

**Record graph.** Relationships are edges with a typed `relation` column, not implicit
cross-references inside document text. The initial relationship vocabulary is the
spec's (section 7): `relates-to`, `derived-from`, `explored-by`, `produces`,
`resolves`, `constrains`, `implements`, `implemented-by`, `supported-by`,
`supersedes`. The graph stays flexible — an RFC is not required to produce a PDR, a
PDR is not required to produce both an ADR and an EDR (spec section 7) — because
forcing every idea through every stage would recreate the rigid pipeline this system
is explicitly trying to avoid (spec section 2).

**Application core and CLI.** Record lifecycle rules, identifier allocation,
relationship validation, and revision semantics live in a Rust application core, not
in the CLI parsing layer and not in SQL triggers. The CLI is one adapter over that
core; nothing about the core assumes it is the only adapter, but no second adapter
(MCP or otherwise) exists yet (ADR-0005). This split exists so that "what makes a
mutation valid" has exactly one implementation, matching the spec's stated principle
(section 3.4) that the CLI and any future adapter call the same application services
rather than each re-implementing validation.

**Search.** FTS5 over record title, body, and tags, using the `porter unicode61`
tokenizer (EDR-0002). The searchable body is deterministically derived from the
current structured document — search results reflect the same state `show` and
`--json` output would, never a separately-maintained copy that can drift.

## Components

**Storage.** SQLite is the canonical store (ADR-0001). The core tables, adapted from
the specification's representative schema (section 8):

- `engineering_record` — `id`, `kind`, `number`, `title`, `status`, `document` (JSON,
  `CHECK(json_valid(document))`), `revision`, `created_at`, `updated_at`,
  `UNIQUE(kind, number)`.
- `record_relation` — `source_id`, `relation`, `target_id`, with foreign keys into
  `engineering_record` on both ends, so a relationship can never point at a record
  that does not exist.
- `record_revision` — `record_id`, `revision`, `document`, `changed_at`,
  `changed_by`, `change_summary`. Every meaningful mutation preserves the prior
  document state here before it is overwritten.
- `evidence` and `code_reference` — first-class tables so a decision can cite a
  benchmark, an experiment, an issue, or a specific file/line range, rather than
  referencing them only in prose.

**Record graph.** Relationships are edges with a typed `relation` column, not implicit
cross-references inside document text. The initial relationship vocabulary is the
spec's (section 7): `relates-to`, `derived-from`, `explored-by`, `produces`,
`resolves`, `constrains`, `implements`, `implemented-by`, `supported-by`,
`supersedes`. The graph stays flexible — an RFC is not required to produce a PDR, a
PDR is not required to produce both an ADR and an EDR (spec section 7) — because
forcing every idea through every stage would recreate the rigid pipeline this system
is explicitly trying to avoid (spec section 2).

**Application core and CLI.** Record lifecycle rules, identifier allocation,
relationship validation, and revision semantics live in a Rust application core, not
in the CLI parsing layer and not in SQL triggers. The CLI is one adapter over that
core; nothing about the core assumes it is the only adapter, but no second adapter
(MCP or otherwise) exists yet (ADR-0005). This split exists so that "what makes a
mutation valid" has exactly one implementation, matching the spec's stated principle
(section 3.4) that the CLI and any future adapter call the same application services
rather than each re-implementing validation.

**Search.** FTS5 over record title, body, and tags, using the `porter unicode61`
tokenizer (EDR-0002). The searchable body is deterministically derived from the
current structured document — search results reflect the same state `show` and
`--json` output would, never a separately-maintained copy that can drift.

## Interfaces

**Application core and CLI.** Record lifecycle rules, identifier allocation,
relationship validation, and revision semantics live in a Rust application core, not
in the CLI parsing layer and not in SQL triggers. The CLI is one adapter over that
core; nothing about the core assumes it is the only adapter, but no second adapter
(MCP or otherwise) exists yet (ADR-0005). This split exists so that "what makes a
mutation valid" has exactly one implementation, matching the spec's stated principle
(section 3.4) that the CLI and any future adapter call the same application services
rather than each re-implementing validation.

## Data Model

A record's identity (`kind` + `number`) and lifecycle-relevant fields (`status`,
`revision`, timestamps) are relational columns, queryable directly. A record's
kind-specific content (an RFC's motivation, a PDR's requirements, an ADR's decision
text) lives in the `document` JSON column, validated against a per-kind schema at
write time. This split exists because the fields every record shares need to be
queried and joined efficiently (find every `accepted` ADR that constrains a given
EDR), while kind-specific content changes shape as each record kind's template
evolves — forcing every field into its own relational column would mean a schema
migration every time an RFC template gains a field.

## Failure Modes

- **A mutation partially applies.** Prevented by requiring `engineering_record`,
  `record_revision`, `record_relation`, and the FTS index update to commit inside one
  SQLite transaction. If any step fails, the whole mutation rolls back — this is a
  requirement, not yet a verified property; it needs a test that actually forces a
  failure mid-transaction, not just an assumption that "it's one transaction" implies
  correctness.
- **A relationship targets a record that doesn't exist, or that existed and was later
  deleted.** Prevented structurally by the foreign key constraints on
  `record_relation`, not by application-layer double-checking alone.
- **A record's document no longer matches its kind's current schema**, because the
  schema changed after the record was written. This is a real open question — see
  Risks below.

## Alternatives Considered

Alternatives to the *tool's existence* are covered in RFC-0001. This section covers
alternatives to how the tool is *built*, specifically the storage engine and mutation
boundary, which are covered in full in ADR-0001 and ADR-0002 respectively — this
section does not repeat their content, only notes that files-only storage,
CLI-plus-direct-SQL-access, and an embedded key-value store were all considered and
rejected during this design pass, for the reasons those ADRs give.

## Evidence

Before this design counts as validated rather than merely proposed: a real end-to-end
test that creates a record, revises it, links it to another record, and confirms all
four tables (`engineering_record`, `record_revision`, `record_relation`, FTS index)
reflect the change consistently — and a second test that forces a failure partway
through a mutation and confirms nothing partially committed. Neither test exists yet;
writing them is part of implementing this design, not a separate later concern.

## Experiments

Before this design counts as validated rather than merely proposed: a real end-to-end
test that creates a record, revises it, links it to another record, and confirms all
four tables (`engineering_record`, `record_revision`, `record_relation`, FTS index)
reflect the change consistently — and a second test that forces a failure partway
through a mutation and confirms nothing partially committed. Neither test exists yet;
writing them is part of implementing this design, not a separate later concern.

## Risks

- **Schema evolution.** Nothing in this design yet specifies what happens to an
  existing record's `document` JSON when its kind's schema gains or changes a
  required field. This is an open question, not a decided policy — resolving it
  (versioned document schemas inside the JSON itself, e.g. the spec's `"schema":
  "pdr/v1"` convention in section 10.1, is the leading candidate) is deferred to
  implementation and may need its own EDR once the first schema change actually
  happens.
- **Single-writer assumption.** SQLite handles one writer at a time natively; this
  design leans on that rather than building anything to fake concurrent writers. If
  Strata is ever used by more than one process against the same database file
  concurrently, this needs revisiting — it is explicitly out of scope for now (see
  Non-Goals), not solved.
- **FTS5 relevance at scale.** Untested against real record volume. The non-goal on
  embeddings (spec section 9) is a bet that FTS5 is good enough; that bet is unverified
  until there's enough real content to test it against.

## Open Questions

- Schema evolution policy for existing records (see Risks).
- Exact Rust crate boundaries — carried over from RFC-0001, still unresolved here.
- Whether `record_revision.changed_by` is populated at all for a single-user local
  tool, or whether that column is premature until Strata has more than one author.

## Resulting Decisions

- ADR-0001: SQLite as Strata's canonical persistent store.
- ADR-0002: CLI-only mutation boundary.
- ADR-0003: Markdown/Pandoc output is a generated projection, never canonical.
- ADR-0004: Strata owns its own static-site information architecture.
- ADR-0005: MCP adapter is deferred, not part of the initial build.
- EDR-0001: Identifier scheme — `(kind, number)` uniqueness, transactional allocation.
- EDR-0002: FTS5 configuration — `porter unicode61` tokenizer, embeddings deferred.

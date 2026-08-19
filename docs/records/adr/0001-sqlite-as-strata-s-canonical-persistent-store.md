---
id: "ADR-0001"
title: "SQLite as Strata's canonical persistent store"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: sqlite-as-strata-s-canonical-persistent-store
tags: []
relationships: {}
---

# ADR-0001: SQLite as Strata's canonical persistent store

## Context

Strata needs one place where a record's current state, its revision history, its
relationships to other records, and its search index all live consistently with each
other. The specification (`docs/specification.md`, section 8) proposes SQLite. Two
other shapes were realistic candidates: a directory of files (one per record, likely
Markdown or JSON, git-tracked directly) with no separate database at all, and an
embedded key-value store (e.g. `redb`, which `substrate`'s event store already uses
and this project's author has direct operational experience with).

Strata is a single local process, used by a single user at a time, with no network
service and no requirement to scale beyond one machine's worth of records. It needs
relational queries across records (find every accepted ADR that constrains a given
EDR; find every PDR with no resulting decision), full-text search, and referential
integrity between records (a relationship cannot point at a record that doesn't
exist) — all as first-class, not hand-rolled, capabilities.

## Decision

We will use SQLite as Strata's canonical persistent store, via a single local database
file, with the schema outlined in PDR-0001.

Files-only storage was rejected because it has no query engine — "find every ADR that
constrains this EDR" would mean parsing every file on every query, and referential
integrity (a relationship pointing at a record that doesn't exist) would have to be
checked by application code with no structural backstop, not enforced by the store
itself.

An embedded key-value store was rejected because Strata's actual access pattern is
relational and text-searchable from day one — join across records by kind, status,
and relationship, and free-text search over document content — not key-based lookup
by a single identifier. A key-value store would mean re-implementing a query layer,
a relationship graph, and search indexing on top of it by hand; SQLite already
provides SQL joins, foreign key constraints, and FTS5 as built-in facilities that
directly match what Strata needs, without Strata having to build them.

## Considered Options

\- **Files only** (one Markdown or JSON file per record, git as the only store).
\- **Embedded key-value store** (e.g. `redb`).
\- **SQLite** (the specification's proposal).

## Consequences

\- Strata gets ACID transactions across the record, revision, relationship, and search
  tables for free from SQLite's transaction model, rather than having to build that
  guarantee itself (see PDR-0001's failure-mode discussion on partial mutations).
\- Strata gets referential integrity (foreign keys on `record_relation`) and full-text
  search (FTS5) as built-in engine features rather than hand-rolled application code.
\- Strata inherits SQLite's single-writer-at-a-time model. This is a good fit for a
  single local user and was treated as acceptable rather than as a limitation to work
  around (see PDR-0001's "single-writer assumption" risk) — if Strata is ever used by
  multiple concurrent writers against the same file, this decision needs revisiting.
\- The database file becomes the one thing that must exist for Strata to function;
  losing or corrupting it loses canonical state, not just a cache. Backup/export
  strategy (`eng export`, spec section 28) exists specifically to mitigate this, but
  is a separate concern from this decision.

## Evidence

\- `docs/specification.md`, section 8 (the originating schema proposal this decision
  adopts).
\- Direct prior experience: the `substrate` repository operates a `redb`-backed event
  store (`store/event/`) and a separate SQLite-backed projection store
  (`store/projection/`), reconciled asynchronously across two engines. A 2026-08-17
  persistence evaluation in that repository concluded the two-engine split exists to
  satisfy a distributed-systems assumption (independent write/read scaling across
  machines) that a single-process system does not need, and that a single SQLite
  engine gets the same transactional consistency for free. Strata, being a genuinely
  single-process, single-writer tool with no distributed deployment target at all,
  is a direct application of that same conclusion — one engine, not two.

---
id: "ADR-0007"
title: "Domain value objects implement ToSql/FromSql directly"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: domain-value-objects-implement-tosql-fromsql-directly
tags: []
relationships: {}
---

# ADR-0007: Domain value objects implement ToSql/FromSql directly

## Context

Substrate's `store-projection` crate persists domain types (e.g. `PersonId`) it does
not itself define — those types live in `shared/primitives`, a separate crate.
Rust's orphan rule forbids implementing a foreign trait (`rusqlite::ToSql`/`FromSql`)
for a foreign type, so `store-projection` wraps each value in a local newtype,
`SqlCode<T>`, defined in `store-projection` itself, and implements `ToSql`/`FromSql`
on the wrapper instead of the value. That bridge type exists solely to satisfy the
orphan rule across that crate boundary.

Strata's records crate defines `RecordKind`, `Status`, and `RecordId` itself — the
domain crate owns these types outright. Because the type is local even though the
trait is foreign, records can implement `ToSql`/`FromSql` on them directly; the
orphan rule only blocks foreign-trait-for-foreign-type, not
foreign-trait-for-local-type. A `SqlCode<T>`-style wrapper would add a layer of
indirection Strata does not need.

## Decision

We will implement `ToSql` and `FromSql` directly on `RecordKind`, `Status`, and
`RecordId` in the records crate. The records crate depends on `rusqlite::types` for
these trait definitions, but never on `rusqlite::Connection` or any query or
transaction API — that remains the boundary that matters (coupling to a
serialization format is acceptable; coupling to live I/O is not), not "no
dependency on rusqlite at all."

This decision applies at the single-column value-object level only. The `Record`
aggregate as a whole does not get a `ToSql`/`FromSql` impl — it spans multiple
tables (the record row, revision rows, relation rows, the FTS index) and has no
single SQL value to convert to or from. Persisting it remains explicit
multi-statement code in the store crate (`Store::create`, `Store::revise`), built
from these value-level conversions.

## Considered Options

\- **Bridge wrapper** — define a local wrapper type in the store crate, mirroring
  substrate's `SqlCode<T>`, and convert each domain value through it at the storage
  boundary.
\- **Direct impl** — implement `rusqlite::types::ToSql` and `FromSql` directly on
  `RecordKind`, `Status`, and `RecordId` in the records crate.

## Consequences

\- No bridge or wrapper type exists anywhere in the codebase for `RecordKind`,
  `Status`, or `RecordId`; the store crate binds and reads these types directly
  through rusqlite's named-parameter API.
\- The records crate is no longer "no dependencies beyond std and serde" in the
  strictest sense — it takes a real dependency on `rusqlite::types`. This is a
  deliberate, narrow exception, not a general license to add other infrastructure
  dependencies to records; anything that touches a live connection, a query, or a
  transaction still belongs in the store crate only.
\- If Strata ever needs a second storage engine (see ADR-0006's reversal condition),
  these impls become dead weight specific to SQLite's value model and would need to
  move behind a boundary at that point — accepted now because there is no second
  engine to design for yet.
\- A future contributor reading `records/src/lib.rs` might reasonably ask why a
  domain crate imports rusqlite; this ADR is the answer.

## Evidence

\- `records/src/lib.rs`: `impl ToSql for RecordKind`, `impl FromSql for RecordKind`,
  `impl ToSql for Status`, `impl FromSql for Status` — direct impls, no wrapper type
  in between.
\- `store/projection/src/sql_code.rs` (substrate repository): the `SqlCode<T>`
  pattern this decision deliberately avoids, and why — that crate does not own the
  types it wraps; records does.

---
id: ADR-0006
title: No repository port trait for record persistence
status: accepted
date: 2026-08-19
derived-from: PDR-0001
---

# ADR-0006: No repository port trait for record persistence

## Context

PDR-0001 sketched storage behind a port interface in the hexagonal style substrate
uses: a domain-defined trait, implemented by an adapter, called through dynamic
dispatch or a generic parameter. Substrate's own dependency-rule enforcement (its
`domain/*`/`app/*` importing only ports, never concrete `runtime/*`/`store/*` types
outside `bin/*`) exists because many bounded contexts and, in principle, multiple
implementations depend on those seams — the trait boundary earns its cost there.

Strata has one storage engine (SQLite, ADR-0001) and one write path (the CLI,
ADR-0002). A `RecordRepository` trait with exactly one implementation adds an
indirection — a vtable or generic parameter threaded through every call site — with
no corresponding benefit: nothing is ever substituted behind it, and test isolation
is already available for free through SQLite's `:memory:` connections without a
second implementation to provide it.

## Considered Options

- **Repository port trait** — a `RecordRepository` trait implemented once by the
  store crate, called through `&dyn RecordRepository` or a generic type parameter
  from the CLI.
- **Concrete functions** — a plain `Store` type owned by the store crate, with
  inherent methods called directly.

## Decision

We will not introduce a repository trait. The store crate exposes a concrete `Store`
type with plain methods (`create`, `get`, `revise`, `set_status`, `link`, `history`,
`search`, `graph`, `validate`); the cli crate constructs one `Store` at startup and
calls its methods directly. No `dyn Trait` and no generic type parameter exists
anywhere in the codebase to abstract over "the storage implementation," because
there is only one.

## Consequences

- Every call site is concrete and monomorphic; there is no vtable indirection
  anywhere in the read or write path.
- If a second storage engine or a second real consumer of the store crate (e.g. the
  deferred MCP adapter, ADR-0005) is ever built, this decision needs revisiting —
  the trait should be introduced once there are two real implementations to shape
  it against, not speculatively now. This follows the same rule-of-three reasoning
  applied elsewhere in this project's design work.
- Testing the store crate uses a real SQLite `:memory:` connection, not a mock or a
  fake trait implementation — there is no trait to fake, and the real engine is
  cheap enough locally that mocking it would trade correctness for no real benefit.
- The records crate stays fully decoupled from the store crate's existence; the
  store crate is the only place that knows a `Store` exists.

## Evidence

- `store/src/lib.rs`: `Store` is a concrete struct with inherent methods; `Store::open`
  and `Store::open_memory` are its only construction sites.
- ADR-0002 (CLI-only mutation boundary): already establishes that the CLI is the
  sole caller of these methods, which is what makes a substitutable-implementation
  abstraction unnecessary in the first place.

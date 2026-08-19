---
id: ADR-0005
title: MCP adapter is deferred, not part of the initial build
status: accepted
date: 2026-08-18
derived-from: PDR-0001
---

# ADR-0005: MCP adapter is deferred, not part of the initial build

## Context

The specification (`docs/specification.md`, section 17) describes MCP as "an optional
future adapter over the same application services, not a required architectural
component," and lists what it would add (standardized tool discovery, typed
invocation, native integration with MCP-capable hosts) versus what it must not
duplicate (record schemas, lifecycle rules, identifier allocation, revision semantics,
FTS behavior, graph semantics, rendering, transaction logic — all of which stay in the
application core regardless of how many adapters exist over it).

Building an MCP server before the CLI and application core are proven adds a second
adapter surface to maintain and keep in sync with the first, before there is evidence
the core itself is correct. It also risks the MCP adapter becoming a second write path
implemented independently of the CLI's validation, which would undermine ADR-0002's
CLI-only mutation boundary if not built carefully — easier to avoid by not building it
yet than by building it carefully under time pressure.

## Considered Options

- **Build an MCP adapter alongside the CLI from the start.**
- **Defer the MCP adapter until the CLI and application core are proven under real
  use, and a concrete integration need justifies it.**

## Decision

We will not build an MCP adapter as part of Strata's initial build. The CLI is the
only interface for v0.1 (PDR-0001's scope cut). An MCP adapter may be added later, but
only once it can be built as a thin layer calling the same application-core services
the CLI calls (per the specification's own constraint, section 17) — not as an
independently-validated second write path.

## Consequences

- LLM agents (including the ones used to build Strata itself) interact with Strata
  through the CLI, the same as a human would, for the foreseeable future. This is
  consistent with ADR-0002 and does not need a second decision to justify it.
- If and when an MCP adapter is built, ADR-0002's constraint applies to it directly:
  it must not gain any write capability the CLI's application core doesn't already
  validate. That constraint doesn't need to be re-decided at that point — it already
  follows from this record and ADR-0002 together.
- This decision can be revisited without conflict once there's a concrete reason to
  (e.g. an MCP-capable host Strata's author actually wants to use it from). It is a
  deferral, not a rejection.

## Evidence

- `docs/specification.md`, section 17 (the originating "MCP is explicitly optional"
  principle and the list of what MCP must not duplicate — the direct basis for this
  decision's consequences).

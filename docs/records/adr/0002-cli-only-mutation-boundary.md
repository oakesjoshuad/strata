---
id: "ADR-0002"
title: "CLI-only mutation boundary"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: cli-only-mutation-boundary
tags: []
relationships: {}
---

# ADR-0002: CLI-only mutation boundary

## Context

The `substrate` repository's `adrs` MCP server exposed an `update_content` operation
that let an LLM agent (or a human, through the agent) rewrite parts of an ADR document
directly. In practice this corrupted required document sections and injected an
illegal `## Status` heading that the repository's own validator rejects — the tool
that answered "edit this record" had no model of what a valid record looks like, so it
could not refuse an edit that broke one. The corruption was only ever caught by
manually diffing the result after each call, never prevented at the point of mutation.

Strata's core holds the same kind of structured, validated documents. Anything that
can write to the SQLite database directly — a script, an agent with raw file or SQL
access, a human editing the `.db` file with a generic SQLite client — can reproduce the
same failure: a mutation that leaves the record, its revision history, its
relationships, or its search index inconsistent with each other, with nothing to
refuse it.

## Decision

We will require every mutation — record creation, revision, status transition, and
relationship linking — to go through the `strata` CLI's validated commands. No caller,
human or LLM agent, gets direct SQLite write access. This matches the specification's
stated trust boundary (`docs/specification.md`, section 3.3 and section 30): an LLM
expresses intent through validated CLI operations, and is not considered trusted
merely because it is using a documented skill or workflow.

Read access may be more permissive in practice (nothing stops a human from opening the
database file read-only to inspect it), but the write path has exactly one door, and
that door validates schema, lifecycle, and relationships before anything commits.

## Considered Options

- **Unrestricted access** — any caller (human, script, LLM agent) may open the SQLite
  database directly and read or write it with ordinary SQL.
- **CLI-only mutation** — all writes go through the `strata` binary's validated
  commands; nothing else is permitted to write to the database.

## Consequences

- The specific failure mode that motivated this decision — a mutation tool with no
  model of record validity silently corrupting a document — becomes structurally
  impossible for Strata's own records, because the only way to mutate a record is
  through code that has to construct and validate a full, well-formed change before it
  can commit one.
- An MCP adapter, if one is ever built (ADR-0005), must call the same application-core
  validation the CLI calls; it may not become a second, independently-implemented
  write path with its own (potentially divergent) notion of what a valid mutation is.
- This puts real weight on the CLI's own commands being correct and complete — a bug
  in a CLI command's validation logic is no longer just a bug, it is the entire trust
  boundary. This is an argument for the verification work described in PDR-0001
  (end-to-end tests of transactional consistency and forced-failure rollback), not
  just a stated intention.
- Bulk or scripted operations (e.g. importing many records at once) must go through
  the CLI's own batch-capable commands, not a hand-written script against the database
  file, even when that would be faster to write.

## Evidence

- `feedback_adr_mcp_update_content_gotcha` (session memory, substrate repository): the
  concrete incident this decision is a direct structural fix for.
- `docs/specification.md`, sections 3.2, 3.3, and 30 (the originating trust-boundary
  design this decision adopts).

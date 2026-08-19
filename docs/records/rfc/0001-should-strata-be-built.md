---
id: "RFC-0001"
title: "Should Strata be built?"
record-type: rfc
status: accepted
revision: 4
date: 2026-08-19
slug: should-strata-be-built
tags: []
relationships:
  produces:
    - "PDR-0001"
---

# RFC-0001: Should Strata be built?

## Motivation

Two concrete failures in the current Markdown-and-tooling approach (used in the
`substrate` repository) motivated this:

1. **A write path with no domain validation corrupted real records.** The `adrs` MCP
   server's `update_content` operation, used to edit existing ADRs, was found to
   silently corrupt required document sections and inject an illegal `## Status`
   heading that the repository's own validator (`scripts/check_adrs.sh`) rejects. The
   corruption was caught only by manually diffing after every call — there was no
   structural guarantee against it, because nothing in the write path understood what
   a valid ADR document looks like. A tool that treats "edit an ADR" as "replace some
   text in a Markdown file" has no way to prevent this class of bug.

2. **A flat document list stopped scaling.** `substrate`'s `docs/book/src/working/`
   directory holds working documents in a single chronologically-named list, rendered
   through mdBook's fixed book/chapter hierarchy. mdBook has no notion of a document
   graph — it cannot show "everything this ADR constrains," "everything superseding
   ADR-0016," or "every open PDR with no resulting decision" — because it has no model
   of relationships between documents at all, only a table of contents.

Underneath both failures is the same root cause: treating decision records as prose
files edited by convention, with no application layer that understands their structure,
lifecycle, or relationships. Records accumulate, but nothing enforces that they stay
internally consistent — a 2026-08-18 audit of `substrate`'s 102 existing ADRs found 29
that bundle more than one independently decidable choice into a single record, with the
worst case bundling nine. Nothing about the Markdown-file approach caught this as it
happened; it took a dedicated audit pass to find it after the fact.

Strata exists to make the write path itself the thing that prevents these failures,
rather than relying on a human or a separate audit to catch them afterward.

## Problem

Underneath both failures is the same root cause: treating decision records as prose
files edited by convention, with no application layer that understands their structure,
lifecycle, or relationships. Records accumulate, but nothing enforces that they stay
internally consistent — a 2026-08-18 audit of `substrate`'s 102 existing ADRs found 29
that bundle more than one independently decidable choice into a single record, with the
worst case bundling nine. Nothing about the Markdown-file approach caught this as it
happened; it took a dedicated audit pass to find it after the fact.

Strata exists to make the write path itself the thing that prevents these failures,
rather than relying on a human or a separate audit to catch them afterward.

## Scope

\- A Rust CLI binary (`strata`) as the only interface for creating, revising, linking,
  and querying records.
\- SQLite as the canonical persistent store: record identity, lifecycle status,
  structured document content (as validated JSON), revision history, and an explicit
  relationship graph between records.
\- Four record kinds at launch: RFC, PDR, ADR, EDR, matching the granularity Strata's
  own author wants (not Oxide's single flattened "RFD" kind — see Alternatives below).
\- FTS5 full-text search over record content.
\- Deterministic Markdown export, so records remain human-readable and reviewable in
  git without requiring the CLI to be installed to read them.

## Non-Goals

Carried from the originating specification (`docs/specification.md`, section 31) and
still accurate at the point of writing this RFC:

\- No server, no SaaS component, no multi-user concurrent-write story beyond what a
  single local SQLite file already provides.
\- No vector database, no embeddings, until a measured retrieval failure justifies the
  added complexity.
\- No MCP adapter at launch (see ADR-0005).
\- No web application framework, no JavaScript runtime.
\- No automatic enforcement of *other* projects' source-code architecture rules —
  Strata records engineering knowledge, it does not lint the codebases that knowledge
  is about.

## Constraints

The scope is bounded by the non-goals stated in this RFC:

Carried from the originating specification (`docs/specification.md`, section 31) and
still accurate at the point of writing this RFC:

\- No server, no SaaS component, no multi-user concurrent-write story beyond what a
  single local SQLite file already provides.
\- No vector database, no embeddings, until a measured retrieval failure justifies the
  added complexity.
\- No MCP adapter at launch (see ADR-0005).
\- No web application framework, no JavaScript runtime.
\- No automatic enforcement of *other* projects' source-code architecture rules —
  Strata records engineering knowledge, it does not lint the codebases that knowledge
  is about.

## Proposal

Build the system described in `docs/specification.md`, cut down to the scope in
PDR-0001 for the first working version: record CRUD, the relationship graph, revision
history, FTS5 search, and a JSON-capable CLI with self-description commands. Defer the
deterministic Pandoc renderer, the publication pipeline, static-site generation, and
the optional MCP adapter to later phases, once the core record store is proven with
real use.

## Alternatives Considered

**Keep using the existing `adrs` MCP server and Markdown-only convention.**
Rejected. This is the status quo that produced the `update_content` corruption bug and
gives no structural way to prevent bundled decisions or broken relationship references.

**Adopt MADR (Markdown Architectural Decision Records) as a pure-convention approach.**
Rejected as the whole solution, though its template shape is used as input to
Strata's own ADR format (see `docs/research/template-research.md`). MADR is a
documentation convention, not a system — it has no queryable graph, no transactional
identifier allocation, and nothing that structurally prevents a record from bundling
multiple decisions. It solves the "what should one ADR contain" question well; it does
not solve "how do I know my 102 ADRs are internally consistent."

**Adopt Oxide's RFD (Request for Discussion) as a single flattened record kind.**
Rejected. Oxide deliberately collapses RFC/PDR/ADR/EDR-shaped work into one kind with a
discussion-first lifecycle (`prediscussion -> ideation -> discussion -> published ->
committed -> abandoned`), because their process is oriented around pull-request
discussion as the review mechanism. Strata's author wants the RFC/PDR/ADR/EDR
granularity kept distinct — an RFC (should we do this) and an ADR (what did we decide)
answer different questions and should be able to have different lifecycles and
different downstream consumers (an EDR can cite an ADR as a constraint; that
relationship doesn't make sense if there's only one kind of record).

## Open Questions

\- The exact Rust crate layout (single crate vs. domain/adapter split at the crate
  boundary) is not decided. PDR-0001 sketches the shape; the actual `Cargo.toml`
  layout is deferred to implementation.
\- The exact `clippy.toml` gated-type rules Strata will hold itself to (analogous to
  `substrate`'s `disallowed-types`/`disallowed-methods` pattern) are not yet written.
\- Whether Strata ever needs multi-user concurrent access (and therefore something
  beyond a single local SQLite file) is explicitly unresolved — nothing in current use
  requires it, but it is not ruled out permanently, only deferred.

## Outcome

Accepted. This RFC is being recorded after the decision was already made in
conversation and the repository already created — it documents the reasoning
retroactively rather than gating a decision that hasn't happened yet. Future RFCs in
this repository should precede the decision they cover, not follow it; this one is the
exception because it is the record of Strata's own founding.

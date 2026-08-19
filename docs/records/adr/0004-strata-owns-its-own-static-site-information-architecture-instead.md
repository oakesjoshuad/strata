---
id: "ADR-0004"
title: "Strata owns its own static-site information architecture instead of mdBook"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: strata-owns-its-own-static-site-information-architecture-instead
tags: []
relationships: {}
---

# ADR-0004: Strata owns its own static-site information architecture instead of mdBook

## Context

`substrate`'s working documents live under `docs/book/src/working/`, rendered through
mdBook. mdBook's navigation is a fixed book/chapter tree defined in a `SUMMARY.md`-
style file — in practice, that repository's working set is a flat, chronologically
named list (`docs/book/src/working/README.md`), because there is no other structure
mdBook's model can express for a growing, cross-referencing set of documents. mdBook
has no concept of a document graph: it cannot generate "every ADR this one supersedes,"
"every record tagged `persistence`," or "every open PDR with no resulting decision,"
because it has no model of relationships between documents, only a table of contents
someone maintains by hand.

Strata's canonical data already includes exactly the graph mdBook lacks (ADR-0001,
PDR-0001's relationship model) — record kind, status, tags, and typed relationship
edges between records, all queryable. Choosing a site generator is really a choice
about who gets to decide navigation: a hand-maintained table of contents, or a query
over the graph Strata already has.

## Decision

We will have Strata's own application generate site navigation, indexes, and views
directly from the record graph, rather than adopting mdBook's fixed hierarchy. Pandoc
remains the document-level renderer (individual pages), but page *selection* and
*navigation structure* are computed from Strata's own data — record kind, status, tags,
and relationships — not authored by hand in a separate table-of-contents file.

## Considered Options

- **mdBook**, as already used elsewhere in this lineage of projects. Static-site
  generation is essentially free (it's already integrated in the surrounding
  ecosystem), at the cost of a fixed book/chapter hierarchy.
- **Strata owns site information architecture directly**, generating navigation,
  indexes, and multiple graph-derived views (by chronology, subsystem, record kind,
  RFC lineage, tag, status) itself, with Pandoc doing document-level rendering only
  (`docs/specification.md`, section 27).

## Consequences

- Multiple views become possible from one canonical graph without maintaining them as
  separate hand-written documents: a chronological view, a per-record-kind view, an
  RFC-lineage view (follow `derived-from`/`produces` edges from one RFC to its
  descendants), a tag view, a status view (every currently accepted ADR). None of
  these require a human to remember to update a table of contents when a new record
  is added — they update as a consequence of the record existing.
- This is genuinely more work than adopting mdBook, which already exists and is
  already wired into the surrounding tooling. This decision accepts that cost
  deliberately, because the flat-list strain already visible in `substrate`'s working
  set is the predictable outcome of *not* paying it, and Strata's whole reason for
  existing is to not repeat that outcome.
- Like ADR-0003's renderer, this is explicitly deferred past v0.1 (PDR-0001's scope
  cut: static-site generation is step 16 of the specification's sequence, not part of
  the first working version). It is recorded now, ahead of implementation, because it
  rules out quietly reaching for mdBook later as the path of least resistance once the
  first records exist and a "just render something" temptation shows up.

## Evidence

- `substrate` repository, `docs/book/src/working/README.md`: the flat, chronologically
  ordered document list this decision is a direct response to — twenty-five entries at
  last count, added in date order because mdBook's book/chapter model gives no better
  option for a growing, cross-referencing set of working documents.
- `docs/specification.md`, section 27 (the originating "own the site IA, Pandoc renders
  pages" proposal this decision adopts, including its explicit statement that "Pandoc
  alone is not a static-site generator and SHALL not be expected to replace all mdBook
  behavior" — read here as an argument for building navigation deliberately, not for
  keeping mdBook by default).

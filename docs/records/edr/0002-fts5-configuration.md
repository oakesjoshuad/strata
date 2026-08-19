---
id: EDR-0002
title: FTS5 configuration — porter unicode61 tokenizer, embeddings deferred
status: accepted
date: 2026-08-18
derived-from: PDR-0001
constrained-by: ADR-0001
---

# EDR-0002: FTS5 configuration — porter unicode61 tokenizer, embeddings deferred

## Context

Strata needs full-text search over record titles, bodies, and tags
(`docs/specification.md`, section 9). SQLite's FTS5 extension is the retrieval
mechanism (a direct consequence of ADR-0001 — SQLite already provides it, so building
or bolting on a separate search engine would duplicate a capability the chosen store
already has). FTS5 needs a tokenizer choice, and the specification is explicit that
vector embeddings are a non-goal for the initial version (section 9, restated as a
workspace-wide non-goal in section 31): "Vector embeddings SHALL NOT be required
initially. Embeddings MAY be added only if measured retrieval failures justify their
complexity." This EDR fixes the tokenizer choice and records the reasoning for the
embeddings deferral in one place, since both are implementation-level engineering
choices about the same feature rather than architectural ones.

## Considered Options

**Tokenizer:**
- `unicode61` alone — Unicode-aware tokenization, no stemming (a search for "decide"
  would not match a document containing "deciding").
- `porter unicode61` — Unicode-aware tokenization plus Porter stemming, so related word
  forms match each other.
- `trigram` — substring/character-trigram matching, better for partial-word or
  fuzzy matching, weaker for whole-word relevance ranking.

**Semantic retrieval:**
- FTS5 alone for the initial version.
- FTS5 plus vector embeddings from the start.

## Decision

We will configure the `engineering_record_fts` virtual table with
`tokenize='porter unicode61'`, matching the specification's representative schema
(section 9). We will not add vector embeddings for the initial version; FTS5 combined
with structured filtering (record kind, status, tags, relationships — PDR-0001's data
model) is the retrieval mechanism until a real, measured retrieval failure shows it is
insufficient.

Porter stemming was chosen over plain `unicode61` because engineering records
naturally use varied word forms for the same concept ("decide," "deciding,"
"decision," "decided") and matching only the exact token would make search
noticeably worse for the kind of prose these records actually contain. A trigram
tokenizer was considered and set aside for the initial version — it solves a
different problem (fuzzy or partial matching) than the one Strata has at launch,
which is "match related word forms of a known term," not "match despite typos or
partial input."

## Consequences

- Searches for one word form of a term will surface documents using a related form of
  the same term, without the caller needing to guess every variant.
- No embeddings means no embedding model dependency, no vector index to keep in sync
  with canonical state, and no additional non-determinism in search results — directly
  in line with the specification's stated bar (section 9) that embeddings need a
  measured failure to justify their complexity, not a hypothetical one.
- If real use later shows FTS5 misses queries a human would expect to match (synonyms
  it doesn't stem, conceptual matches with no shared vocabulary at all), that is the
  trigger to revisit this decision — not a general sense that embeddings would be
  nice to have. This EDR should be superseded, not silently worked around, if that
  happens.

## Evidence

- `docs/specification.md`, section 9 (the originating FTS5 schema and the explicit
  embeddings non-goal this decision adopts).

---
id: "EDR-0009"
title: "strata context composes search and graph into a bounded, one-hop retrieval envelope"
record-type: edr
status: draft
revision: 1
date: 2026-08-20
slug: strata-context-composes-search-and-graph-into-a-bounded-one-hop-
tags: []
relationships:
  relates-to:
    - "ADR-0014"
    - "ADR-0015"
    - "ADR-0016"
---

# EDR-0009: strata context composes search and graph into a bounded, one-hop retrieval envelope

## Context

Specification section 16 asks the CLI to eventually provide an agent-oriented `context` retrieval command combining FTS hits, graph neighbors, supersession chains, tags, evidence, and code references into "a bounded context package suitable for LLM reasoning." No such command exists today. `strata capabilities --json` lists `search` and `graph` but nothing that combines them, and none of the twenty accepted records in this database's own history proposed one -- this is new scope, not a gap left by an existing record.

The design surface turns out to be small enough not to need an RFC. `Store::search` (`store/src/queries.rs:47-70`) already returns ranked FTS hits. `Store::graph` (`store/src/queries.rs:72-88`) already returns a record's relationships (including `supersedes` edges in either direction, since it selects on `source_id = :id OR target_id = :id`), evidence, and code references in one call. Tags are not a separate store concept at all -- they live inline in each record's `document.tags` array and are already indexed into `engineering_record_fts` (`store/src/lib.rs:97`). Acceptance criterion 8 ("retrieve a record and its graph context as JSON") is already met by `strata graph <id> --json` alone; `context` is not closing that gap, it is the genuine spec-16 enhancement of driving that same retrieval from a search query across multiple records at once, which criterion 8 does not require.

Because every underlying primitive already exists and is already accepted, the only real decision is how `context` composes them and where it draws the "bounded" line the specification asks for. That is an implementation-mechanics choice within an already-established boundary (search and graph as separate, accepted primitives), which is exactly EDR territory under ADR-0016's classification test, not an ADR: nothing here changes ownership between components or introduces a new architectural boundary.

## Decision

Add `strata context <query> --json [--limit N] [--offset N]`, implemented entirely in the CLI composition root as a thin fold over the two existing store calls: run `Store::search(query, limit, offset)` to get ranked hits, then call `Store::graph(&hit.id)` once per hit to expand each into its relationships, evidence, and code references. No new `store` function, no new SQL, no new abstraction -- `search` and `graph` are called exactly as they are called by the existing `search` and `graph` CLI commands today.

The response envelope is:

```json
{
  "query": "changing the persistence writer",
  "results": [
    {
      "record": { "id": "ADR-0002", "title": "...", "status": "accepted", "document": { "...": "...", "tags": ["..."] } },
      "relationships": [ { "source_id": "...", "relation": "...", "target_id": "..." } ],
      "evidence": [ ... ],
      "code_references": [ ... ]
    }
  ]
}
```

`tags` are not hoisted into a separate top-level field; they are already present inside each result's `record.document.tags` (per ADR-0009's common frontmatter shape), so duplicating them into the envelope would be redundant state that could drift from the record itself. "Supersession chains" means the one-hop `supersedes`/`supersedes-target` edges already present in each result's `relationships` array -- the same information `graph` already exposes for a single record, now attached per search hit. `context` does not walk the graph transitively past that first hop.

`--limit` bounds the number of *search results* expanded into full graphs (default 10, distinct from `search`'s own default of 50 -- each context result costs one extra `graph` call, so the default is deliberately smaller). It does not bound the number of relationships, evidence, or code references returned within a single result's graph; those are returned in full, exactly as `strata graph` already returns them in full today. `--offset` behaves identically to `search`'s existing `--offset`.

## Considered Options

A dedicated `store`-layer `context()` method issuing one joined SQL query across `engineering_record_fts`, `record_relation`, `evidence`, and `code_reference` -- rejected. This repository's scale (dozens, not millions, of records) gives no measurable benefit from collapsing an N+1 pattern into one query, and it would duplicate logic that `search` and `graph` already own correctly, creating two code paths that can drift out of sync with each other. CLAUDE.md rule 6 already rejects new abstractions without a second real implementation to justify them; the same reasoning applies to a second retrieval code path without a concrete performance problem to justify it.

A transitive walk of the full supersession chain (A supersedes B supersedes C...) -- rejected. The specification explicitly asks for a *bounded* package, and a transitive walk has no natural bound short of an arbitrary hop limit or cycle-detection machinery that nothing else in this codebase needs yet. `validate` (via ADR-0015's neighborhood) is the place that reasons about the relationship graph's global consistency; `context` stays a bounded, per-hit expansion, consistent with `graph`'s own existing one-hop behavior.

An XML envelope alongside JSON, per specification section 16's "MAY use an XML envelope" -- deferred, not rejected outright, but out of scope for this slice. No command in the CLI emits XML today (ADR-0003/ADR-0008 keep Markdown as the one generated projection), and adding a second output format is a separable decision from the retrieval logic itself.

Hoisting tags to a top-level envelope field, deduplicated across results -- rejected. It reads convenient but introduces a second, derived representation of data that already lives canonically in each record's document, with no consumer need identified to justify the duplication.

## Consequences

`cli/src/args.rs` gains a `Context` command variant (query, `--json`, `--limit`, `--offset`); `cli/src/main.rs` dispatches it by calling `store.search` then `store.graph` per hit; `cli/src/output.rs` gains the JSON/text rendering. `store` and `records` are untouched -- this is the same "CLI composition, no store change" shape ADR-0014's resolution logic already established as the template for cross-cutting CLI features. `strata capabilities --json` and `--help` should list `context` alongside `search` and `graph` once it exists, closing the self-description gap for this command the same way EDR-0008 closed it for configuration.

Because `--limit` bounds result count rather than per-result graph size, a query matching a small number of highly-connected records can still return a large payload; this EDR accepts that tradeoff rather than adding a second bounding parameter with no identified consumer need yet. A future record can add one if that turns out to matter in practice.

## Evidence

`docs/specification.md:647-670` (section 16, the context-retrieval requirement this command implements) and `docs/specification.md:1106-1123` (section 33, acceptance criterion 8, already satisfied by `graph` alone -- cited to show what this command is not closing). `store/src/queries.rs:47-70` (`Store::search`) and `store/src/queries.rs:72-88` (`Store::graph`), the two primitives this command composes without modification. `store/src/lib.rs:97`, where tags are already indexed into FTS from `document.tags`. ADR-0016, the positive ADR/EDR classification test this EDR is scoped under. ADR-0015, the precedent for where relationship-graph reasoning belongs (`store` validation) versus where per-record graph expansion belongs (already `store::graph`, unchanged here). ADR-0014, the precedent for CLI-composition-root features that touch no store abstraction.

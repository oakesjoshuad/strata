---
id: "EDR-0003"
title: "Deterministic identifier-prefixed Markdown filename slugs"
record-type: edr
status: accepted
revision: 3
date: 2026-08-19
slug: deterministic-identifier-prefixed-markdown-filename-slugs
tags: []
relationships: {}
---

# EDR-0003: Deterministic identifier-prefixed Markdown filename slugs

## Context

strata export needs stable paths under docs/records/{kind}/NNNN-slug.md. Existing files use hand-chosen lowercase hyphenated slugs, but a renderer needs a reproducible rule for punctuation, Unicode, length, empty titles, and collisions. ADR-0012 additionally requires the slug to be immutable after a title change, which means it cannot be recomputed from the current title at export time -- it must be captured once and stored.

## Decision

We will derive the slug by Unicode NFKD normalization, removal of combining marks, ASCII lowercasing, replacing each run of non-ASCII-alphanumeric characters with one hyphen, trimming hyphens, and limiting the result to 64 ASCII characters. If normalization produces an empty slug, use untitled. The exported path is {kind-slug}/{number as four digits}-{slug}.md. The numeric identifier makes two different titles collision-free within a kind; no title-based suffix or nondeterministic disambiguator is added. The slug is computed exactly once, by Store::create at record-insertion time, and persisted in a new slug TEXT NOT NULL column on engineering_record. No later operation -- revise, set_status, or a future title update -- ever recomputes or overwrites it. Records that predate this column are backfilled once, computing the same algorithm from each record's current title.

## Considered Options

Preserve arbitrary Unicode and punctuation; use a hash or title suffix for collisions; allow unlimited slug length; use the title alone without the record number; or recompute the slug from the current title at every export instead of persisting it. Recomputing at export time is what ADR-0012 explicitly rejects -- it would make a title update silently move the exported file. The remaining alternatives produce less portable paths or unstable collision behavior.

## Consequences

The same title always produces the same readable portable slug, and two records with the same title still have distinct paths because their identifiers differ. Transliteration beyond NFKD and removal of non-ASCII characters is intentionally not guessed; unsupported characters become separators, and an entirely unsupported title becomes untitled. Adding the slug column requires a schema migration and a one-time backfill for every record created before this decision -- there is no default value that is correct for an existing row, since the slug must match that row's actual title.

## Evidence

Issue #6 requires docs/records/{kind}/NNNN-slug.md and explicitly asks for punctuation, length, and collision rules. ADR-0012 requires the slug to survive a later title change unchanged, which is the source of the persistence requirement added here. The existing founding paths demonstrate the desired lowercase hyphenated style. ADR-0003 and docs/specification.md section 28 require deterministic committed projections.

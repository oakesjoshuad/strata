---
id: "ADR-0009"
title: "Use one common frontmatter shape for all record kinds"
record-type: adr
status: accepted
revision: 3
date: 2026-08-19
slug: use-one-common-frontmatter-shape-for-all-record-kinds
tags: []
relationships: {}
---

# ADR-0009: Use one common frontmatter shape for all record kinds

## Context

The specification section 19 gives an ADR-shaped YAML example with title, identifier, record-type, status, revision, date, tags, and relationships. The existing hand-written records use a smaller frontmatter vocabulary and vary in optional relationship fields. Rendering and parsing need one explicit contract rather than kind-specific guesses.

## Decision

We will use one frontmatter shape for RFC, PDR, ADR, and EDR. The fields are id (for example RFC-0001), title, record-type (rfc, pdr, adr, or edr), status, revision (integer), date (ISO calendar date), slug, tags (a sequence, possibly empty), and relationships (a mapping from relation name to a sequence of record IDs, possibly empty). No frontmatter field is kind-specific; kind-specific content remains in the ordered body sections. relationships reflects outgoing edges only -- relations where this record is the source, exactly as stored in record_relation. It does not include incoming edges. The existing relation vocabulary (relates-to, derived-from, explored-by, produces, resolves, constrains, implements, implemented-by, supported-by, supersedes) declares an explicit inverse pair for only one relation (implements / implemented-by); inventing inverse names for the rest so incoming edges could be shown too is deliberately out of scope here, deferred until a real need for it appears. strata graph <id> remains the way to discover what points at a record, not its own frontmatter.

## Considered Options

Give RFC/PDR/ADR/EDR separate frontmatter schemas; preserve arbitrary top-level relationship keys; copy the specification example exactly with identifier instead of the repository's established id field; or show both outgoing and incoming relationships in frontmatter. Separate schemas increase parser branching, and arbitrary keys make validation and round trips ambiguous. Showing incoming relationships was considered and rejected for this decision specifically because it requires an inverse-relation vocabulary that does not exist yet for most of the ten defined relations -- building that now would be speculative, not motivated by an actual need.

## Consequences

Every kind has the same metadata validation and parser surface, and a rendered file declares enough identity and projection information to be checked independently. The renderer must serialize empty tags and relationships deterministically, and the parser must reject unknown or incorrectly typed frontmatter fields rather than silently discard them. A record's own file does not show what produced it or what points at it -- an ADR's frontmatter will not list its originating PDR. That information is one strata graph call away and is not duplicated into every downstream file's frontmatter.

## Evidence

docs/specification.md sections 19 and 23 identify title, identifier, record type, status, revision, date, tags, and relationships as publication metadata. Existing records establish id, title, status, date, and derived-from conventions; this decision normalizes those relationship facts under one relationships mapping, scoped to outgoing edges only. records/src/validation.rs's RELATIONSHIPS constant is the authoritative relation vocabulary and shows only one declared inverse pair.

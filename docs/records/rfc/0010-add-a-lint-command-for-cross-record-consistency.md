---
id: "RFC-0010"
title: "Add a lint command for cross-record consistency"
record-type: rfc
status: draft
revision: 1
date: 2026-08-21
slug: add-a-lint-command-for-cross-record-consistency
tags: []
relationships: {}
---

# RFC-0010: Add a lint command for cross-record consistency

## Motivation

`validate` already checks structural and lifecycle issues within a single
record (e.g. the existing warning that an accepted decision has no
qualifying lineage from an RFC or PDR). `record_relation` rows are
foreign-keyed to `engineering_record` (`store/migrations/V1__initial.sql`),
so a relationship can never point at a record that does not exist -- that
class of dangling reference is already impossible by construction. But a
record's document fields are free-text scalars (`questions_for_review`,
`alternatives`, `context`, and so on), and free text can reference another
record's id or describe another record's state in prose that drifts the
moment that other record moves on: an RFC's `questions_for_review` field
might say "resolved by ADR-0010", written when ADR-0010 was accepted, but say
nothing if ADR-0010 is later superseded. Nothing today re-checks that kind of
cross-record claim after it is written. This is exactly the sweep done by
hand this session, reading every record and cross-referencing relationships
and status against prose by eye, which is mechanical work a tool should do.

## Problem

A record's correctness is not fully local: its relationships and its prose
both make claims about the state of other records, and those claims can go
stale through changes made entirely on the other record, with no mutation to
the record making the claim. There is no sweep that re-derives whether those
claims still hold.

## Scope

Define a `lint` pass over the whole database that checks relationship-level
consistency beyond what `validate` already covers (for example: a
`relates-to` or `derived-from` target whose status has since moved in a way
the source record's own status arguably should follow, such as a record
`supported-by` or `derived-from` a target that has since been `superseded`),
and checks whether a record id mentioned in another record's free-text
document fields corresponds to a record whose current status matches what
the mentioning text asserts (best-effort: this is a text-pattern match
against known id formats like `ADR-0010`, not a semantic parse of prose).

## Non-Goals

This RFC does not duplicate `validate`'s existing single-record structural
and lifecycle checks. It does not propose automatically fixing an
inconsistency it finds -- only reporting it, consistent with Rule 7 against
silently resolving a validation failure. It does not attempt full natural-
language understanding of document prose; text-pattern matching for known id
formats is the extent of the free-text check, and it may under- or
over-report given prose is not structured data.

## Constraints

Because relationships are already foreign-key-enforced, this feature's value
is entirely in checks that span *derived* consistency (status implications
across a real relationship, or free-text claims against real current state),
not existence -- it must not reinvent a check `validate` or the schema's own
constraints already guarantee. Any free-text id-matching must be conservative
about false positives: report a candidate mismatch rather than assert a
contradiction, since prose is not guaranteed to be making a factual claim
about status at all.

## Proposal

Add `strata lint` as a new top-level command alongside `validate`, since it
operates across records rather than within one and depends on relationship
graph traversal `validate`'s current single-record checks do not need. Cover
at minimum: an accepted/superseded status mismatch across a `derived-from` or
`supported-by` edge (source status implies a target transition that has not
happened, or vice versa); a free-text mention of a known record id within
`questions_for_review`, `alternatives`, `context`, or other document fields,
cross-checked against that record's actual current status, reported as
`INFO` (a candidate to review) rather than `ERROR` given the false-positive
risk. Exit non-zero only on the relationship-status class of finding, which
is derived from structured data and not prose-matching, so it can compose
with CI the same way `validate` does; keep prose-derived findings advisory.

## Alternatives Considered

Fold these checks directly into `validate` -- rejected, since `validate`'s
existing checks are single-record and structural, while these are graph-wide
and involve free-text heuristics with a real false-positive rate; conflating
the two would make `validate`'s otherwise CI-safe pass/fail contract fuzzy.
Do nothing and continue doing this sweep by hand per review -- the status
quo, and what this RFC exists to remove given it does not scale past a
handful of records. Attempt full semantic parsing of document prose to
understand every claim it makes about other records -- rejected as
disproportionate; a conservative id-pattern match that under-reports is
preferable to a parser that invents structure prose was never written to
have.

## Open Questions

Which specific status-implication rules belong in the graph-wide check
first -- is it only `derived-from`/`supported-by` targets going stale, or
does `relates-to` (the least specific relationship kind) carry any
status-consistency expectation at all? Should the free-text id-matching
check be opt-in (given its false-positive rate) via a flag, or on by default
and simply labeled advisory? Does `lint` need its own exit-code contract
distinct from `validate`'s existing one, given this RFC proposes mixing a
CI-safe class of finding with an advisory one in the same command?

## Outcome

TODO: outcome

---
id: "RFC-0007"
title: "Detect stale code references via automated verification"
record-type: rfc
status: draft
revision: 1
date: 2026-08-21
slug: detect-stale-code-references-via-automated-verification
tags: []
relationships:
  produces:
    - "PDR-0003"
---

# RFC-0007: Detect stale code references via automated verification

## Motivation

Code references (`code-ref-add`) attach a `path`/`symbol`/`line_start`/`line_end`
claim to a record as durable evidence, but nothing verifies that claim stays true
as the referenced codebase changes. A concrete case surfaced while working against
graphlite's own Strata database (`.strata/strata.db` in that repo, tracking
graphlite's engineering journey, per this project's stated purpose of recording
the journey of other projects): EDR-0001 there carries two code references into
`src/rustdoc_enricher.rs` -- one to `extract_rustdoc_signature` at lines 600-614,
and one to `compute_crate_enrichment` at lines 190-198. As of the current graphlite
tree, `extract_rustdoc_signature` no longer exists at all (removed, matching that
record's own EDR-0001-EV-002 evidence entry describing the removal), and
`compute_crate_enrichment` now begins at line 209, not 190. Both code references
drifted silently -- nothing in Strata flagged the mismatch, unlike `export --check`,
which already exists to catch exactly this class of drift for the Markdown
projection.

## Problem

A code reference is a claim of the form "this record's reasoning applies to this
location in the code." That claim degrades the moment the referenced code moves,
is renamed, or is deleted, and today nothing distinguishes a code reference that
still matches the tree from one that has silently drifted or now points at nothing.
Reviewers and agents reading a record have no signal that a code reference is stale
short of manually opening the referenced file and checking by hand.

## Scope

Define a verification pass that, given a working tree checked out at a known path,
reads a record's (or the whole database's) `code_reference` rows and reports, per
reference: the file no longer exists, the symbol can no longer be found in the
file, or the symbol was found but at a different line range than recorded. Cover
both a single-record check and a whole-database sweep.

## Non-Goals

This RFC does not propose automatically repairing a stale code reference -- only
detecting and reporting one. It does not propose parsing or understanding source
code beyond locating a named symbol's declaration. It does not define behavior for
verifying against a repository other than the one working tree passed in for a
given run (multi-repo batch verification, if wanted later, is a separate design
question).

## Constraints

Strata's records may reference code in a project distinct from the Strata
repository itself -- graphlite's own EDR-0001 is exactly this case -- so the
check needs an explicit target checkout path rather than assuming the referenced
code lives alongside the database. Verification must be read-only: it must not
mutate `code_reference` rows itself. Any correction remains a human or agent
decision made through the normal `revise`/`code-ref-add` path, consistent with the
project rule against turning a validation failure into a silently-applied default.
Symbol location needs a per-language strategy; a naive text search is acceptable
as long as it is honest about ambiguity (multiple candidate matches) rather than
guessing one.

## Proposal

Add a code-reference verification pass, modeled on `export --check`'s existing
pass/fail contract: given a `--code-root <PATH>`, for every `code_reference` row
in scope, locate `path` relative to that root, confirm the file exists, search for
`symbol`, and compare its found line range against `line_start`/`line_end`. Report
one of `OK`, `FILE-MISSING`, `SYMBOL-MISSING`, or `LINE-DRIFT` (with the old and
found ranges), and exit non-zero if anything but `OK` is found, so it composes
with CI the same way `export --check` already does. Support both a single-record
filter (`strata graph <ID>` already prints code references for one record) and a
whole-database sweep.

## Alternatives Considered

Do nothing and rely on manual review -- the status quo, and the thing that let
both drifts in graphlite's EDR-0001 go unnoticed. Auto-update line ranges
opportunistically whenever verification runs -- rejected, since it violates the
read-only/no-silent-correction constraint above and risks masking an actually
significant drift (a deleted function) as a trivial one (a shifted line number).
Require re-attaching code references on every `revise` -- too disruptive to the
revision workflow, and would not catch drift introduced by commits unrelated to
the record itself. A dedicated read-only check command mirroring `export --check`'s
already-accepted pattern is the smallest addition consistent with existing
conventions.

## Open Questions

What symbol-location strategy is accurate enough without pulling in a full
per-language parser -- a regex heuristic, tree-sitter, or shelling out to something
like graphlite itself, which already solves exactly this problem for its own
indexing purposes? Should a small line-drift tolerance (symbol found, range
shifted by only a few lines) be a warning rather than a hard failure, or should any
drift at all be treated the same as a missing symbol? Does this verification
belong as a sub-check under `strata validate`, or as its own top-level command,
given it requires an external filesystem path that `validate`'s other checks do
not need?

## Outcome

TODO: outcome

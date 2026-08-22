---
id: "RFC-0008"
title: "Add a patch mode to strata revise"
record-type: rfc
status: accepted
revision: 5
date: 2026-08-21
slug: add-a-patch-mode-to-strata-revise
tags: []
relationships:
  produces:
    - "EDR-0013"
---

# RFC-0008: Add a patch mode to strata revise

## Motivation

`strata revise` takes `--document` as a full JSON object and replaces the
record's document wholesale; it also errors if any required field for that
record kind is missing from the payload. Editing a single field today means
`show`-ing the record, reconstructing the entire document object with every
field including the ones that did not change, and passing that whole object
back through `--document` (or writing a full replacement Markdown file for
`--file`). For a record with nine required fields (an RFC, say), a one-field
correction requires carrying eight unrelated fields along for the ride.

## Problem

Wholesale replacement is the right default for a full rewrite, but it makes
every small, targeted correction as expensive and as risky as a full rewrite:
a slip in reconstructing an unrelated field silently overwrites content that
was never meant to change, and there is no way to express "change only this
field" directly.

## Scope

Define a merge-based revision mode: the caller supplies a partial document
(one or more fields), and the stored document is updated by merging those
fields onto the current document rather than replacing it outright. Cover
both the `--document` (JSON) and `--file` (Markdown) input paths for
`revise`.

## Non-Goals

This RFC does not change `revise`'s existing wholesale-replacement behavior,
which remains the default and remains available. It does not add field
deletion semantics (removing a field entirely, as opposed to setting it to a
new value) -- every document field is a required scalar per `schema`, so
there is no meaningful "delete this field" operation to define yet. It does
not change how `retitle`, `status`, `link`, `evidence-add`, or `code-ref-add`
work.

## Constraints

`records`' required-fields validation (Rule 7 -- validation failures are
explicit errors, never silently swallowed) must still apply to the resulting
merged document, not be bypassed because the input was partial. The merge
must be unambiguous: a field named in the patch always wins outright over the
corresponding field in the current document, with no field-level sub-merging
(no per-kind knowledge of, say, appending to a list within a scalar field).
This keeps the feature a generic document-merge, not per-kind logic that
would violate the closed `RecordKind` match discipline (Rule 2) by growing
special cases per kind.

## Proposal

Add a `--patch` mode to `strata revise` alongside the existing
`--document`/`--file` inputs: `strata revise <ID> --patch --document
'{"field": "new value"}'` (and the equivalent `--patch --file
<partial.md>`) merges the given fields onto the current document's fields,
validates the merged result against the record kind's required fields exactly
as `revise` does today, and writes the merged document as the new revision.
Fields not named in the patch are left untouched. `--patch` without
`--document`/`--file` is a usage error, and combining `--patch` with a
payload that omits every field is also an error (nothing to change).

## Alternatives Considered

Keep the current wholesale-only `--document`/`--file` behavior and rely on
`show` plus external tooling (`jq`, a script) to reconstruct the full
document before calling `revise` -- the status quo, and the actual
round-tripping cost this RFC exists to remove. Add per-field flags (e.g.
`--set-motivation <TEXT>`) instead of a generic merge -- rejected, since it
would require one flag per document field per kind, growing without bound as
kinds gain fields, whereas a single `--patch` flag stays generic across all
four kinds. Make merge the *default* behavior of `--document`/`--file` rather
than a separate flag -- rejected, since existing callers (and any documented
behavior/tests) already depend on `--document` being a full replacement, and
changing that default silently would be exactly the kind of drift this
project's own conventions exist to prevent.

## Open Questions

Should the merged, post-patch document be printed back to the caller (as
`revise` already does for a normal call) so a partial edit's full resulting
state is visible without a separate `show`? Does `--patch --file` merge at
the whole-frontmatter level or does the Markdown parser need a distinct
partial-document parse path, given the existing parser currently expects a
complete, schema-valid frontmatter block? Should `--patch` be rejected
outright for `revise --file` initially, and shipped for `--document` only in
a first pass, given `--document` is the simpler merge target?

## Outcome

Accepted. The first implementation slice adds JSON-only --patch support to strata revise. A non-empty JSON object is merged onto the current document, the merged document is validated and recorded as one normal revision, and the full resulting record is returned. Partial Markdown patches remain deferred until the parser has an explicit partial-document shape.

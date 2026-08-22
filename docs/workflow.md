# RFC -> PDR -> ADR/EDR workflow, with graphlite

This is the process reference for greenfield work (starting with `substruct`):
how to take a problem from an open question to an implemented, verified
decision using strata for record-keeping and graphlite for structural ground
truth. It describes an already-working combination of the two tools, not a
proposal — every mechanism named below is implemented and, where noted, has
already caught real bugs.

Each tracked project (strata itself, graphlite itself, and eventually
substruct) keeps its own separate `.strata/strata.db`. This document describes
the *process*, not a shared database — substruct's decisions live in
substruct's own database, verified against substruct's own working tree.

## Stage 1 — RFC: explore the problem space

`strata new rfc` starts a record in `draft`. Before writing `motivation` and
`problem`, ground them in real structure rather than assumption: query the
target codebase with graphlite directly (`graphlite map`, `symbols`, `graph`,
`context`, `blast-radius`, `trace-path`) instead of describing the system from
memory. graphlite's own PDR-0001 evidence section did exactly this against a
second real workspace and got zero manual-configuration surprises out of it —
treat that as the expected baseline, not a lucky result.

Attach what you found as evidence on the RFC itself, while it's still a draft:

```
strata evidence-add RFC-XXXX measurement "call graph shows N fan-in on X" --content "..."
strata code-ref-add RFC-XXXX relates-to path/to/file.rs --symbol Foo::bar
```

Evidence and code references attach to *any* record kind — an RFC does not
need to wait for a PDR to start accumulating a real evidentiary trail.

## Stage 2 — back the RFC with evidence and reference implementations

A prototype or spike that tests a proposal's feasibility is evidence, not a
side note. Record it as such:

```
strata evidence-add RFC-XXXX prototype "concurrent-lock spike" --uri <branch-or-path> --content "what was measured"
strata code-ref-add RFC-XXXX supported-by path/to/spike.rs --symbol run_spike
```

Then verify the cited code still exists and matches before anyone reviews the
RFC, rather than after:

```
strata code-ref-check --code-root <path> --id RFC-XXXX
```

By default this shells out to `graphlite` (`CODE_REF_LOCATOR_DEFAULT`, ADR-0019).
This step is not optional ceremony — EDR-0012 found two real defects in the
strata/graphlite subprocess boundary (multi-node XML responses, multi-line
attribute values) that fixture-based unit tests had missed entirely. Any
reference-implementation citation crossing that same boundary carries the same
risk; check it against the real binary, not a mock.

Do not flip an RFC to `accepted` on the strength of a *planned* experiment.
graphlite's PDR-0002 explicitly withheld approval pending two named, unrun
experiments rather than approving on projected numbers — require a real run
before a record's `evidence`/`experiments` fields get credited toward a status
change. This is self-enforced discipline, not a tooling guarantee: `status`
only validates lifecycle legality (e.g. `draft -> proposed` is a legal move
for an RFC), not whether evidence or experiments actually exist behind it —
see the second known gap below.

## Stage 3 — PDR: commit to a design

Use a PDR when the RFC didn't already settle the design (see
`derived-from` linking the PDR back to its RFC). Carry forward the RFC's
evidence rather than re-deriving it; add new evidence only for what the PDR
itself needs to settle (architecture-level tradeoffs, not the problem
statement again).

A PDR's `Proposed Design` should not describe unverified behavior as fact.
Where a design decision depends on how the codebase or a dependency actually
behaves (locking semantics, format versions, timing), that must be a named,
run experiment attached as evidence before the PDR moves to `approved` — not
an assumption stated in prose.

## Stage 4 — ADR / EDR: decision and implementation, bidirectionally linked

An ADR records the decision; an EDR records how it was actually built. Link
them both directions with two explicit `link` calls — one per direction, since
`link` inserts a single directed edge per call, never a reciprocal pair
automatically:

```
strata link EDR-XXXX implements ADR-YYYY
strata link ADR-YYYY implemented-by EDR-XXXX
```

This is a deliberate change from strata's own historical practice, not a
description of it: `implemented-by` has zero uses in strata's own database
today, and where an EDR does reference an ADR it has so far used the generic
`relates-to` rather than `implements` (e.g. `EDR-0008 relates-to ADR-0013`).
`implements`/`implemented-by` is used here as the intended convention for
substruct going forward — a genuine bidirectional pair that says, from either
side, exactly what the relationship is — not a retroactive claim about how
strata got here.

Draft the EDR only once implementation actually starts, not speculatively
alongside the PDR that precedes it, even when the PDR's `Resulting Decisions`
section already names the EDR that will eventually exist. An EDR written
before any code exists documents a decision that hasn't really been made yet.
When implementation starts, write an explicit task prompt (scope, files,
constraints, what's out of scope) before writing code.

Before an EDR flips to `accepted`, its `Evidence` field should report a real
verification pass: test suite, lint, and — if it touches a cross-record or
cross-process boundary — an integration check against the real binary/service
involved, not only a mocked one.

## Ongoing: keep the graph honest

Re-run `code-ref-check` whenever referenced code moves, not only at proposal
time — a citation that was true when written can go stale silently otherwise.

**Known gap 1:** RFC-0010 (cross-record lint) is not yet implemented. `validate`
catches single-record structural/lifecycle issues, and `code-ref-check` catches
code-reference drift, but nothing yet catches graph-wide consistency issues —
e.g. an `accepted` EDR whose ADR was later `superseded`, or free-text prose
that makes a claim about another record's state that has since changed. Until
RFC-0010 lands, treat that class of drift as a manual review item at
`export`/`publish` time, not an automated guarantee.

**Known gap 2:** nothing gates a status transition on evidence actually
existing. `status` validates lifecycle legality only (`validate_transition`
checks `from -> to` against the kind's lifecycle table); it does not check
whether `evidence`/`experiments` fields are populated, or whether a cited
prototype has been verified, before allowing `accepted`/`approved`. Every
"do not flip to accepted without a real run" instruction in this document is
process discipline the author has to hold themselves to — not something
strata will refuse to let you skip.

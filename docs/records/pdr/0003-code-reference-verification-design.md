---
id: "PDR-0003"
title: "Code-reference verification design"
record-type: pdr
status: approved
revision: 3
date: 2026-08-21
slug: code-reference-verification-design
tags: []
relationships:
  produces:
    - "EDR-0012"
  relates-to:
    - "ADR-0006"
    - "ADR-0014"
    - "ADR-0019"
    - "PDR-0002"
  resolves:
    - "RFC-0007"
---

# PDR-0003: Code-reference verification design

## Problem

RFC-0007 decided Strata should detect when a `code_reference` row's claim about a
file/symbol/line-range has drifted from the actual working tree, motivated by a live
example: graphlite's own tracked EDR-0001 carries two code references into
`src/rustdoc_enricher.rs` that had silently drifted -- one symbol deleted, one line
range shifted from 190 to 209 -- with nothing flagging either. It left the symbol-
location strategy, the exit-code/tolerance contract, and the command's placement
(sub-check of `validate` vs. a standalone command) as open questions. This document
settles those questions for the first implementation.

## Requirements

Carried from RFC-0007's Scope and Proposal: given a working tree checked out at a
known path, verify every `code_reference` row (for one record or the whole database)
and report, per reference, one of `OK`, `FILE-MISSING`, `SYMBOL-MISSING`, or
`LINE-DRIFT`. Exit non-zero on anything but `OK`, composing with CI the same way
`export --check`/`dump --check` already do. Verification is read-only: it must never
mutate a `code_reference` row itself.

## Constraints

No `dyn` dispatch and no repository-port-style trait abstraction for symbol
location (CLAUDE.md rule 2; ADR-0006) -- there is exactly one real symbol-location
mechanism in scope for this design (an external `graphlite` invocation), so a
pluggable-backend trait would be exactly the speculative abstraction ADR-0006 and
ADR-0019 already rejected for storage and for the publish renderer. Strata's own
source is Rust; graphlite's own indexing is what already solves multi-language
symbol location correctly and is explicitly the tool this design must reuse rather
than reimplement (RFC-0007's stated motivation, restated directly by the project
owner: "we don't want to re-implement its core purpose"). Verification must degrade
honestly, not silently, when its symbol-location dependency is unavailable or
unindexed -- a missing `graphlite` binary or a stale `graphlite` index must be a
reported condition, never treated as "no drift found."

## Proposed Design

**Symbol location is an externally configured subprocess command, exactly ADR-0019's
shape.** `strata code-ref-check` resolves a symbol-locator command through the same
CLI > environment > config file > default precedence ADR-0014 already established
(`--symbol-locator-command`, `STRATA_CODE_REF_LOCATOR`, `code_ref_locator` in
`strata.config.json`), defaulting to a bundled `graphlite` invocation. This is the
identical resolution mechanism `--renderer-command`/`STRATA_PUBLISH_RENDERER` already
uses for Pandoc (ADR-0019), applied to a second external tool rather than inventing a
new configuration shape. No `SymbolLocator` trait is introduced; the swap point is
the subprocess boundary, matching ADR-0019's reasoning exactly -- one real
implementation today does not justify a compiled abstraction.

**Confirmed against graphlite's real CLI**, run directly during this design's
research (see Experiments): a two-call sequence is required per symbol.
`graphlite symbols <name>` resolves a name to a `stable_id` (its output includes
`file` and `signature` but no line range). `graphlite graph sym:<stable_id>` then
returns the neighborhood-focus node with a `range="Lstart-Lend"` attribute, the
value actually needed for `LINE-DRIFT` comparison. Both are invoked via
`std::process::Command`, and their XML output is parsed for `file`/`range` (a
narrow, purpose-built parse -- not a general XML dependency, matching rule 5's
I/O-boundary spirit of not pulling in machinery beyond what the concrete need
requires).

**Per code_reference row, the check is:**

1. Resolve `path` relative to the configured code root; if the file does not exist,
   report `FILE-MISSING` and stop -- no locator invocation needed.
2. Invoke the symbol-locator's `symbols` step with `symbol`, filtered to `path` where
   the locator's own flags support it (graphlite's `--file <PATTERN>`). Zero matches
   is `SYMBOL-MISSING`.
3. On a match, invoke the locator's `graph`-equivalent step for that stable id and
   compare its reported line range against `line_start`/`line_end`. Any difference
   is `LINE-DRIFT`, reported with both the recorded and the found range. An exact
   match is `OK`.
4. More than one match for a bare (non-qualified) symbol name is reported as its own
   condition, `SYMBOL-AMBIGUOUS`, rather than silently picking the first result --
   consistent with rule 7's stance against turning an unclear case into a guessed
   default.

**Locator unavailability is a distinct, loud condition, never a silent `OK`.** If the
configured locator command is not found on `PATH`, or exits non-zero on invocation
(distinct from "zero symbol matches," which is a successful invocation with an empty
result), the check reports `LOCATOR-UNAVAILABLE` for every reference it could not
evaluate and still exits non-zero overall -- an unavailable locator must never be
silently treated as "nothing to report."

**Locator index freshness is an accepted, documented limitation, not something Strata
re-verifies.** graphlite's own `codegraph.db` is not automatically kept in sync with
its target source tree -- confirmed directly in the Experiments below, where a stale
index would have reported a symbol as present at its old location even after
deletion, had the index not already been rebuilt after the edit. Strata does not
force a re-`discover` before every check (a ~3s cost on graphlite's own moderate
codebase, and not Strata's tool to manage the lifecycle of). This mirrors exactly how
`strata publish` treats Pandoc: an external, independently-installed and
independently-maintained tool that Strata invokes and trusts was kept current by
whoever configured it, verifying only that its *output* is sane (PDR-0002's staging
check that the expected file was actually produced), not that its internal state is
fresh. The corresponding risk here is a false `OK` from a stale locator index; this
is recorded under Risks rather than solved by this design.

## Components

`cli::code_ref_check` (new module, sibling to `cli::export`/`cli::dump`/`cli::publish`):
owns the per-reference verification pipeline above and the `Command::CodeRefCheck`
dispatch arm. Reuses `ResolvedConfig`'s existing CLI/env/config-file resolution
machinery (ADR-0014) for the new `--symbol-locator-command`/`--code-root` values,
adding no second resolution mechanism. No changes to `store` -- verification reads
already-exposed `Store` query surfaces for `code_reference` rows and adds no new I/O
boundary there, per rule 5.

## Interfaces

`strata code-ref-check [--id <RECORD_ID>] --code-root <PATH> [--symbol-locator-command
<CMD>] [--database <PATH>] [--json]`. Omitting `--id` checks every `code_reference`
row in the database. `--code-root` has no default -- unlike `--database`, there is no
sound repository-relative guess for where an arbitrary record's referenced code
actually lives (RFC-0007's own constraint that referenced code may belong to a
project distinct from the Strata repository itself), so it must always be supplied
explicitly.

## Data Model

No schema change. This reads existing `code_reference` rows (`path`, `symbol`,
`line_start`, `line_end`, `record_id`) exactly as `graph` already exposes them; the
verification result is a transient report, not a persisted table -- persisting
verification history is not in RFC-0007's scope.

## Failure Modes

Per-reference conditions: `OK`, `FILE-MISSING`, `SYMBOL-MISSING`, `SYMBOL-AMBIGUOUS`,
`LINE-DRIFT`, `LOCATOR-UNAVAILABLE`. The command exits non-zero if any reference in
scope resolves to anything but `OK`. A misconfigured locator command that exits zero
but produces unparseable output is treated the same as `LOCATOR-UNAVAILABLE` -- an
empty or malformed result is never interpreted as "zero matches found," mirroring
ADR-0019's own caution that a misbehaving external command must not be mistaken for
a successful negative result.

## Alternatives Considered

Reimplement symbol location directly in Strata (a regex heuristic, or a hand-rolled
per-language parser) -- rejected per direct project-owner direction: graphlite
already solves this correctly and is the tool whose core purpose this design should
reuse, not duplicate. A compiled `SymbolLocator` trait with a `Graphlite`
implementation, open to others -- rejected for the same reason ADR-0006 and ADR-0019
already reject this shape: one real implementation today does not justify a
dispatch abstraction, and CLAUDE.md rule 2 bans the `dyn` dispatch such a trait would
eventually need. Force a `graphlite discover` re-index before every check to
guarantee freshness -- rejected as scope creep: Strata does not own graphlite's
index lifecycle any more than it owns Pandoc's installation, and a ~3s-per-check
tax for a guarantee this design does not otherwise provide for any of its other
external dependencies is not a proportionate response; documented as a known risk
instead.

## Evidence

RFC-0007 (the RFC this PDR resolves), specifically its Proposal and Open Questions 1
and 3. ADR-0006 (no repository port trait without a second real implementation) and
ADR-0019 (static-site backend as an externally configured command, not a compiled
trait) -- the direct precedent this PDR extends from the Pandoc/publish boundary to
the graphlite/code-ref-check boundary. ADR-0014 (CLI > environment > config file >
default precedence, reused for the two new resolved values). PDR-0002's staging
validation step (verifying an external command's *output* rather than trusting its
exit code alone), the precedent this design's `LOCATOR-UNAVAILABLE` handling and
stale-index risk framing both reuse. CLAUDE.md rule 2 and rule 6 (no `dyn` dispatch;
no repository port trait without justification); rule 7 (no silent defaulting on an
unclear or failed case, informing `SYMBOL-AMBIGUOUS` and `LOCATOR-UNAVAILABLE`).

## Experiments

Run directly against `/home/joshua/repos/graphlite` (2026-08-21) to establish
feasibility before committing to this design:

\- `graphlite symbols compute_crate_enrichment` returned exactly one match, with
  `file="./src/rustdoc_enricher.rs"` and `stable_id="src/rustdoc_enricher.rs::fn::
  compute_crate_enrichment"`, but no line range in this output.
\- `graphlite graph sym:src/rustdoc_enricher.rs::fn::compute_crate_enrichment`
  returned `range="L209-L217"` for the same symbol -- matching the actual current
  line (209), confirming the two-call sequence above is necessary and sufficient to
  get both file/symbol match and a comparable line range.
\- `graphlite symbols extract_rustdoc_signature` returned zero matches, independently
  confirming (via a wholly different mechanism than the manual `grep` used earlier)
  that this symbol no longer exists in the tree -- direct evidence the drift RFC-0007
  is built around is real and that graphlite's search correctly reflects it.
\- `.graphlite/codegraph.db`'s mtime (15:17) was newer than `src/rustdoc_enricher.rs`'s
  mtime (13:47) at the time of this test -- i.e., graphlite's index had already been
  rebuilt after the file was last edited, which is *why* the above results were
  correct. `graphlite discover` has no staleness flag of its own (the `--stale`
  flags that exist in `graphlite policy lint`'s surface are for annotation
  suppressions, unrelated) and took ~3s to re-run on graphlite's own codebase --
  confirming the freshness risk and cost described in Proposed Design and Risks are
  real, not hypothetical.

## Risks

A stale `graphlite` index (source edited since the last `discover`/`watch` run)
would report a deleted or moved symbol at its old location, producing a false `OK`
for a reference that has actually drifted -- the exact inverse of what this feature
exists to catch. This design accepts that risk rather than solving it, mirroring how
`strata publish` accepts that a misconfigured Pandoc invocation could theoretically
misbehave in ways exit-code checking alone would miss (PDR-0002's Risks), catching
only the checkable subset (the output actually landing) rather than every possible
failure of an external tool Strata does not own. The two-call-per-symbol locator
sequence means whole-database verification cost scales with `2 * reference count`
external process invocations; this is acceptable for Strata's and graphlite's own
current record/symbol counts but would need batching if either grows by orders of
magnitude.

## Open Questions

The exact XML-parsing approach for graphlite's `symbols`/`graph` output (a minimal
purpose-built extractor vs. a general XML crate dependency) is implementation detail
left to a follow-on EDR, matching how EDR-0001 through EDR-0011 progressively locked
down PDR-0001's design during real implementation rather than upfront. Whether
`SYMBOL-AMBIGUOUS` should attempt graphlite's own `resolve` subcommand (described in
its `--help` as resolving an ambiguous query to a deterministic top candidate) before
giving up, rather than always failing loud on ambiguity, is deferred to
implementation once real ambiguous cases are observed. Whether `code-ref-check`
should be a `validate` sub-check or its own top-level command (RFC-0007's Open
Question 3) is resolved below as its own top-level command, since it requires
`--code-root`, a parameter none of `validate`'s other checks need.

## Resulting Decisions

ADR: `strata code-ref-check` locates symbols via an externally configured subprocess
command (default `graphlite`), resolved through the same CLI > environment > config
file > default precedence ADR-0014 established, with no compiled `SymbolLocator`
trait -- extending ADR-0019's Pandoc/publish precedent to a second external-tool
boundary. ADR: `code-ref-check` is its own top-level command, not a `validate`
sub-check, because it requires an external code-root path none of `validate`'s
existing checks need. EDR: the per-reference verification pipeline (file existence,
symbol lookup, line-range comparison, ambiguity and locator-unavailability handling)
as implemented against graphlite's `symbols`/`graph` output.

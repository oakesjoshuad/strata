---
id: "EDR-0012"
title: "Code-ref-check parses graphlite's XML with a quote-aware scanner keyed to the queried stable_id"
record-type: edr
status: accepted
revision: 3
date: 2026-08-21
slug: code-ref-check-parses-graphlite-s-xml-with-a-quote-aware-scanner
tags: []
relationships:
  implements:
    - "PDR-0003"
---

# EDR-0012: Code-ref-check parses graphlite's XML with a quote-aware scanner keyed to the queried stable_id

## Context

PDR-0003 left the exact symbol-locator XML-parsing approach as an open question for a
follow-on EDR once `strata code-ref-check` was actually implemented, rather than settled
upfront. Implementation added `cli/src/symbol_locator.rs` (a subprocess wrapper resolving a
`stable_id` via `graphlite symbols`, then a line range via `graphlite graph sym:<id>`) and
`cli/src/code_ref_check.rs` (the per-reference pipeline). The first working version passed
its own fake-locator unit tests (fixtures with single, simple `<symbol .../>` tags) but two
real defects only surfaced when verified against a real `graphlite` binary (built and
installed from graphlite's own `development` HEAD, commit `1de40f8`) against strata's own
indexed codebase:

1. `graphlite graph sym:<stable_id>` prints the focus node *and* its one-hop neighborhood,
   and every neighbor with a known location also carries its own `range` attribute --
   confirmed directly: `graphlite graph "sym:cli/src/code_ref_check.rs::fn::run"` returned
   seven distinct `range="..."` occurrences in one document, not one. A parser that
   collected "any tag with a `range` attribute" therefore reported `SYMBOL-AMBIGUOUS` for
   essentially every real symbol with call-graph neighbors, not just genuinely ambiguous
   queries.
2. The original attribute scanner tokenized the tag text with `str::split_whitespace()` and
   looked for `=` in each token. Every real `<symbol>` tag graphlite emits carries a
   `signature` attribute whose value is itself multi-line and contains internal spaces
   (e.g. `signature="fn compute_crate_enrichment(\n    root: &str,\n    ...\n)"`).
   Whitespace-splitting such a value produces tokens with no `=` in them at all, which the
   original parser's `?`-propagating loop turned into a silent `None` for the *entire* tag
   -- confirmed directly: a query for a real, unambiguous, existing symbol (`substitute` in
   `cli/src/render_backend.rs`) returned zero stable ids from real `graphlite` output that
   plainly contained a matching `<symbol stable_id="...substitute" .../>` tag, while the
   identical fake-locator fixture (whose attribute values never contained whitespace) had
   passed. This is a more fundamental failure than the ambiguity bug above: it produces a
   false `SYMBOL-MISSING` on nearly every real symbol, not just ones with neighbors.

Both defects were invisible to the unit-test suite because its fake-locator fixtures never
reproduced either shape (multiple range-bearing tags in one document; a whitespace-bearing
attribute value) -- they were only found by actually shelling out to a real, currently
installed `graphlite` binary rather than trusting the fake fixtures alone.

## Decision

`symbol_locator::parse_attributes` is a hand-rolled, quote-aware scanner: after the tag
name, it repeatedly reads a `name=`, then a quote character (`"` or `'`), then everything up
to the matching close quote as that attribute's value, regardless of whitespace inside it.
It does not tokenize on whitespace at any point. `symbol_locator::locate` additionally
matches the range-bearing tag back to the specific `stable_id` resolved by the `symbols`
step (`attributes.get("stable_id") == Some(stable_id)`), rather than accepting any
range-bearing tag in the `graph` output -- so a symbol's own neighbors, which are always
present for any non-leaf symbol, never register as a competing match. A regression test
(`resolves_symbol_when_attribute_values_contain_whitespace`) and an un-ignored fake-locator
fixture reproducing the multi-node-with-ranges shape are both added so this exact class of
defect cannot regress silently again; the pre-existing `#[ignore = "requires graphlite
installed on PATH"]` real-`graphlite` smoke test is retargeted at an unambiguous real symbol
whose signature genuinely spans multiple lines, so it continues to exercise the real shape
this decision fixes.

## Considered Options

Depend on a general XML-parsing crate (`quick-xml`, `roxmltree`) instead of a hand-rolled
scanner -- rejected for this narrow need: `graphlite`'s output is a small, fixed attribute
vocabulary on flat, self-closing-or-simple tags, not general XML with namespaces, CDATA, or
nesting-sensitive text content, and CLAUDE.md's rule 5 spirit (I/O-boundary code should not
carry machinery beyond the concrete need) argues against a new dependency for a single,
narrow extraction. Accept the first range-bearing tag found (rather than filtering to the
matched `stable_id`) and treat that as good enough -- rejected: this is exactly the bug
found in Context above, and "first tag" is not equivalent to "the focus node" in graphlite's
own output ordering, which is not contractually guaranteed to put the focus node first
(verified empirically it currently does, but relying on incidental ordering is the same
class of fragile assumption CLAUDE.md rule 7 warns against for validation logic).

## Consequences

`strata code-ref-check` now correctly resolves real symbols with real call-graph neighbors
and real multi-line signatures -- confirmed end to end against a scratch copy of strata's
own database and a real code reference into `cli/src/render_backend.rs::substitute`,
producing `OK`, `LINE-DRIFT`, `FILE-MISSING`, and `SYMBOL-MISSING` correctly for real working-
tree state. The parser remains coupled to graphlite's current attribute vocabulary and quote
conventions; if graphlite's own output format changes (a new escaping convention, attribute
values genuinely containing the same quote character unescaped), this scanner would need a
corresponding update, since it is not a general XML parser and does not claim to be.

## Evidence

PDR-0003 (the design this EDR implements, specifically its deferred XML-parsing open
question). Direct verification against `graphlite` 0.7.0, built and installed from
graphlite's `development` HEAD (commit `1de40f8`) via `cargo install --path .`, run against
strata's own `graphlite init`-built index: `graphlite graph
"sym:cli/src/code_ref_check.rs::fn::run"` showing seven `range=` occurrences in one
response; `graphlite symbols substitute --file cli/src/render_backend.rs` showing exactly
one match whose `signature` attribute spans multiple lines, which the original parser
dropped and the corrected parser retains. `cargo test --package strata -- --include-ignored`
passing all 49 unit tests plus the two real-`graphlite`-backed tests
(`resolves_symbol_with_real_graphlite`, and the render-backend real-Pandoc pair already
established as this crate's pattern for tests requiring an external binary). A manual
end-to-end run of `strata code-ref-check --code-root .` against a scratch copy of strata's
own database, producing `OK`, `LINE-DRIFT`, `FILE-MISSING`, and `SYMBOL-MISSING` correctly
for a real code reference under real working-tree conditions.

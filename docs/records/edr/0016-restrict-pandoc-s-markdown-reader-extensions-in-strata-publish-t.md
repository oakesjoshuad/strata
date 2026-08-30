---
id: "EDR-0016"
title: "Restrict Pandoc's markdown reader extensions in strata publish to stop silent content corruption"
record-type: edr
status: accepted
revision: 4
date: 2026-08-30
slug: restrict-pandoc-s-markdown-reader-extensions-in-strata-publish-t
tags: []
relationships:
  implements:
    - "PDR-0002"
  relates-to:
    - "PDR-0002"
---

# EDR-0016: Restrict Pandoc's markdown reader extensions in strata publish to stop silent content corruption

## Context

PDR-0002/ADR-0019 fixed strata publish's default rendering command as an externally configured Pandoc invocation, but left Pandoc's markdown reader running with its full default extension set. A dedicated research pass (2026-08-30, see PDR-0002-EV-001) tested that default configuration against Strata's real 49-record corpus and found three live, silent content-corruption defects, all independently re-verified before this record was written: Pandoc's `smart` extension converts literal `--` to an en-dash (U+2013) in prose context, affecting CLI flag text like `--check`/`--patch` (confirmed present in real rendered output); Pandoc's `raw_html` extension treats angle-bracket placeholder text such as `<id>` or `<kind>` as an unrecognized raw HTML tag rather than literal text, so it is emitted unescaped and a browser will not display it (confirmed directly on ADR-0009's rendered output); and Pandoc's `raw_tex` extension silently deletes text it interprets as LaTeX, confirmed dramatically on PDR-0004's own validation regex `^REQ-[A-Z0-9]+(-[A-Z0-9]+)*-\d{3}$`, which published as `^REQ-[A-Z0-9]+(-[A-Z0-9]+)*-$` -- the `\d{3}` fragment silently gone, with no error or warning at any point in the pipeline.

## Decision

Restrict PUBLISH_RENDERER_DEFAULT's Pandoc invocation to disable exactly the three offending extensions, via Pandoc's extension-disabling syntax on the reader format: `pandoc {input} -f markdown-smart-raw_html-raw_tex -o {output} ...`. Every other default extension (including `native_spans`, `bracketed_spans`, `header_attributes`, and `link_attributes`, all of which Strata's own content relies on or may rely on for future glossary-term styling) remains enabled. Implemented in cli/src/config.rs's PUBLISH_RENDERER_DEFAULT constant.

## Considered Options

Switch readers entirely (e.g. to `gfm` or `commonmark`) instead of subtracting extensions from `markdown` -- rejected because it changes more behavior than necessary and was not the configuration PDR-0002-EV-001 verified end-to-end against the real template/CSS/Lua-filter pipeline. Post-process Pandoc's HTML output to reverse the corruption (e.g. replace en-dashes back to `--`) -- rejected as exactly the kind of silent, fragile repair-after-the-fact rule 7 warns against; disabling the extension at the source is precise and the underlying cause is understood, not worked around. Leave the defaults as they are and document the corruption as a known limitation -- rejected outright: one of the three defects is silent data loss (a validation regex publishing with different, incorrect meaning than its source), which is not an acceptable limitation to simply document. Add a selective Lua filter to restore the en-dash-to-`--`-for-flags conversion while keeping `smart` otherwise on -- considered as a way to preserve the house-style `--` becomes an en-dash effect for ordinary prose separators; not implemented in this change since it is a refinement on top of this fix, not a substitute for it, and can be added later without revisiting this decision.

## Consequences

strata publish's rendered HTML now reflects the same literal text Strata's own Markdown renderer already guarantees byte-for-byte in strata export (ADR-0010) -- the two projections no longer disagree on what a record's own content says. Verified directly, post-change: PDR-0004's regex renders with `\d{3}` intact, ADR-0009's `<id>` renders as visible `&lt;id&gt;` text, and no unwanted en-dash appears in the tested cases. The two existing real-Pandoc integration tests (cli/src/render_backend.rs, `renders_shipped_template_with_real_pandoc` and `renders_relationship_link_with_real_pandoc`, both `#[ignore]`-gated on Pandoc being installed) reference PUBLISH_RENDERER_DEFAULT dynamically and were run against the changed constant: both pass unchanged. cargo build, cargo fmt, and cargo clippy (workspace, all targets) are clean. The full workspace test suite has five pre-existing failures in cli/tests/commands/auxiliary.rs (dump/validate/restore stale-projection assertions) confirmed unrelated to this change by reproducing the identical failures against unmodified HEAD before this change was made. A future decision may add a selective Lua-filter rule to restore an intentional `--`-to-en-dash conversion for ordinary prose (not CLI flag text) without reopening this one.

Known blemish, disclosed rather than hidden: this record carries one permanently broken code reference (code-ref-add EDR-0016 implements cli/src/config.rs --symbol PUBLISH_RENDERER_DEFAULT), added before this record author discovered that graphlite does not build a queryable symbol node for top-level const items -- confirmed directly: PUBLISH_RENDERER_DEFAULT is present in rustdoc's own JSON output with visibility crate, so it is not a visibility problem, but graphlite'''s own graph-building step evidently only indexes functions, structs, enums, impls, and modules as resolvable symbols, not plain const declarations. code_reference rows have no identifier of their own and no CLI delete or revise path (EDR-0006 scoped identifier allocation to evidence only), so this reference cannot be removed through the CLI-only mutation boundary (ADR-0002) and direct database editing would violate it. A corrected, working reference (code-ref-add EDR-0016 implements cli/src/config.rs --line-start 11 --line-end 12, no --symbol) has been added alongside it. strata code-ref-check will report exactly one persistent SYMBOL-MISSING finding against this record until graphlite indexes const items as symbols or Strata gains a code-reference removal capability -- neither is proposed by this record. This is real, useful evidence that code-ref-add --symbol is unreliable for referencing plain constants specifically, distinct from functions/types, and is left here rather than concealed.

## Evidence

PDR-0002-EV-001 (the research pass that found and verified all three defects and the fix, against the real installed Pandoc 3.6 binary and Strata's real 49-record corpus). Direct re-verification performed when this EDR was written: `pandoc docs/records/pdr/0004-...md -f markdown-smart-raw_html-raw_tex -t html` preserves `\d{3}`; the same flags against docs/records/adr/0009-...md render `<id>` as `&lt;id&gt;`; the same flags produce no en-dash in the tested ADR-0009 output. cargo build --workspace: clean. cargo test --workspace -- --ignored: both real-Pandoc render_backend tests pass. cargo fmt --all: no additional changes beyond this decision's two-line diff. cargo clippy --workspace --all-targets: no warnings. git stash verification: the five cli/tests/commands/auxiliary.rs failures reproduce identically against unmodified HEAD, confirming they predate and are unrelated to this change.

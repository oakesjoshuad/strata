---
id: "PDR-0002"
title: "Static publication design"
record-type: pdr
status: approved
revision: 3
date: 2026-08-20
slug: static-publication-design
tags: []
relationships:
  produces:
    - "ADR-0017"
    - "ADR-0018"
    - "ADR-0019"
    - "EDR-0011"
  relates-to:
    - "ADR-0001"
    - "ADR-0003"
    - "ADR-0004"
    - "ADR-0006"
    - "ADR-0014"
    - "EDR-0011"
    - "PDR-0004"
  resolves:
    - "RFC-0005"
---

# PDR-0002: Static publication design

## Problem

RFC-0005 decided that Strata should generate a deterministic, navigable static site from
canonical database state, using Pandoc for document rendering and an ordinary static server
for hosting, without making Markdown or HTML canonical. It left the publication command
boundary, manifest, output layout, staging and replacement semantics, backend
configurability, and initial failure policy as open questions. This document settles those
questions for the first `strata publish` implementation.

## Requirements

Carried from RFC-0005's Scope and Proposal: SQLite remains the sole canonical source.
Strata generates deterministic Markdown projections and a publication manifest from that
state. A static-site backend compiles engineering documents and Strata-generated site pages
(indexes, navigation, relationship views, traceability reports) into HTML. The result is a
self-contained static artifact suitable for an ordinary static web server. The first
implementation targets Strata's own records and a local publication directory.

## Constraints

SQLite remains canonical (ADR-0001); Markdown and all backend-derived output remain
generated projections (ADR-0003); Strata owns site information architecture and
graph-derived navigation (ADR-0004). No `dyn` dispatch and no repository-port-style trait
abstraction may be introduced to support multiple site-generator backends (CLAUDE.md rule 2;
ADR-0006) -- there is exactly one real backend today (Pandoc), and a compiled trait
abstraction "for flexibility" with only one implementation is exactly the speculative
abstraction ADR-0006 already rejected for storage. Publication must be deterministic,
locally buildable, and explicit about unresolved references, missing assets, or conflicting
output paths. A failed publication must not leave a partially replaced site. No async,
server-side, or multi-user infrastructure.

## Proposed Design

**Command boundary.** `strata publish` is one command, owned by the CLI composition root
like `export`, `dump`, and `validate`. It runs the full pipeline end to end: validate,
export, build manifest, render, assemble, stage, replace. There is no separate
externally-orchestrated Pandoc step for the first implementation -- Strata already owns
`export`, `dump`, and now (EDR-0011) validate-time staleness detection for both; a
publication command that stopped short of also owning rendering would reintroduce exactly
the multi-command manual chaining EDR-0011 just removed from `validate`, one level up the
stack.

**Pipeline.**

1. Run the equivalent of `strata validate` (`Store::validate` plus EDR-0011's projection
   staleness checks). Any ERROR aborts before anything is written. This is publish's only
   precondition -- it does not re-derive its own separate notion of "safe to publish."
2. Render every record to Markdown in memory, reusing `export.rs`'s existing
   `rendered_records` pipeline rather than re-implementing it.
3. Build a publication manifest (see Data Model) from the record graph: one entry per
   selected record plus entries for Strata-generated site pages (index, per-kind indexes,
   graph/relationship views, traceability report).
4. Generate each site page's Markdown input directly from the manifest and graph queries --
   these are Strata-authored inputs, not backend output, per ADR-0004.
5. Invoke the configured renderer command (see below) once per manifest entry, engineering
   document and site page alike, so every page in the artifact goes through the same
   template and stylesheet.
6. Write all renderer output, plus static assets (CSS, images) Strata ships, into a staging
   directory alongside the configured target (see Staging).
7. Validate the staged output structurally: every manifest path was produced, no two entries
   collided on the same path, no renderer invocation failed.
8. Atomically replace the configured target directory with the staging directory.

**Backend configurability without a trait.** RFC-0005 named Pandoc as the renderer, and the
user has flagged that a future open-source Strata should let other projects swap in a
different static-site generator without touching Strata's Rust code. A compiled backend
trait is exactly the abstraction ADR-0006 already rejected for storage, for the identical
reason: one real implementation today does not justify a dispatch abstraction. The
resolution is to keep the swap point at the process boundary instead of the type system:
`strata publish` invokes a single **externally configured command** per document, resolved
through the same CLI > env > config file > default precedence ADR-0014 already established
(`--renderer-command`, `STRATA_PUBLISH_RENDERER`, `publish_renderer` in
`strata.config.json`, default a bundled Pandoc invocation). The command template receives
the input Markdown path, output HTML path, a metadata file (title, record id, kind, status,
breadcrumb/navigation data drawn from the manifest), and Strata's shipped template/CSS
paths as substituted placeholders, and is run via `std::process::Command`. Strata owns
*what* gets rendered and *where it goes*; the configured command owns turning one Markdown
document plus metadata into one HTML file. Anyone can point this at Pandoc, a wrapped
mdBook invocation, or a project-specific script without a Strata code change or a `Backend`
enum needing a new match arm -- the swap point is configuration, not a compiled abstraction,
so it does not conflict with rule 2 or ADR-0006.

**Deploy target is always a staged artifact, never a live web root.** `strata publish`
resolves a configured target directory (`--publish-target`, `STRATA_PUBLISH_TARGET`,
`publish_target` in the config file, default `.strata/site`, anchored to the repository root
the same way EDR-... configuration defaults already anchor per the "Anchor repository
defaults" work) and writes the complete artifact there. It never writes to a configured
Apache document root or any other live-serving location directly. Installing the artifact
onto a web server -- rsync, a symlink swap, a deploy script -- is explicitly left outside
Strata's domain, consistent with RFC-0005's own non-goals (no hosted deployment, no forge
dependency) and its constraint that hosting stays outside the Strata domain model.

**Staging and atomic replacement.** Publication writes to a sibling directory
(`<target>.staging-<pid>`) next to the configured target, never into the target directly.
Structural validation (step 7 above) runs against the staging directory. Only on success does
publish perform a single `rename` of the staging directory over the target -- an atomic
directory swap on the same filesystem, the same class of guarantee `export`/`dump` do not
currently have but that RFC-0005's "must not leave a partially replaced site" constraint
requires. On any failure, the staging directory is left in place for inspection (not deleted)
and the target directory is untouched.

## Components

`cli::publish` (new module, sibling to `cli::export`/`cli::dump`): owns the pipeline above
and the `Command::Publish` dispatch arm. `cli::manifest` (new module): builds the
publication manifest from `Store` graph queries and `export`'s rendered records; owned
separately from `publish` because the manifest is also the natural place for a later
`strata publish --check` staleness comparison, mirroring EDR-0011's shape for export/dump.
No changes to `store` -- publication reads already-exposed `Store` query surfaces (`export`,
`graph`, `search`/`context`'s underlying queries) and adds no new I/O boundary there, per
rule 5.

## Interfaces

`strata publish [--check] [--renderer-command <CMD>] [--publish-target <PATH>] [--database
<PATH>] [--json]`. `--check` compares the current target's manifest against a freshly
computed one and reports staleness without writing, the same shape `export --check`/`dump
--check` already establish. Renderer and target resolution follow the existing
`ResolvedConfig`/`resolve_path` machinery (ADR-0014, the "anchor repository defaults" work),
extended with two more resolved paths/values rather than a new resolution mechanism.

## Data Model

The manifest is a JSON document written to `<target>/manifest.json`: schema version;
`generated_at`; Strata build identity (the same git-sha-plus-dirty-flag already surfaced by
`strata --version`/`capabilities --json`); source database identity; the renderer command
used; and one entry per published page with `id` (or a synthetic id for site pages), `kind`,
`status`, `revision`, `slug`, `publication_path`, `title`, `tags`, and a content hash of its
rendered Markdown input (reusing the hashing/comparison approach EDR-0011 already
established for staleness detection, so `publish --check` can answer "is the site stale"
the same way `export --check`/`dump --check` already answer that question for their own
projections).

## Failure Modes

Publish aborts before any staging write on: a `strata validate`-equivalent ERROR (including
EDR-0011's projection-staleness checks); a configured renderer command that is missing or
fails validation as a command template. Publish aborts before the final replace (staging
directory left behind, target untouched) on: any renderer invocation exiting non-zero; two
manifest entries resolving to the same output path; a manifest-referenced static asset
missing from disk. No new "required reference" concept is introduced beyond what
`strata validate` already enforces -- publish's precondition is exactly validate's existing
ERROR set plus the path-collision and renderer-failure cases specific to rendering itself.

## Alternatives Considered

A compiled `PublishBackend` trait with a `Pandoc` implementation, so other generators could
be added as Rust implementations later -- rejected. One real backend today does not justify
the abstraction (ADR-0006's own reasoning), and it would reintroduce `dyn`-shaped dispatch
CLAUDE.md rule 2 bans outright; a subprocess boundary gets the same swappability without it.

Keep Pandoc orchestration as an external repository script outside `strata publish` --
rejected per the "Publish scope" decision: it would leave the multi-command manual chaining
EDR-0011 just removed from `validate` fully intact one level up, at the publish boundary.

Write directly to a configured Apache document root -- rejected per explicit direction: the
first publisher always produces a staged artifact; installing it is a separate, unopinionated
step, keeping Strata's domain model host-agnostic as RFC-0005's non-goals already require.

## Evidence

RFC-0005 (the RFC this PDR resolves), specifically its Proposal and Open Questions 1 and 4.
ADR-0001, ADR-0003, ADR-0004 (canonical store, generated projections, site information
architecture ownership). ADR-0006 (no repository port trait without a second real
implementation -- the precedent this PDR extends to backend selection). ADR-0014 (CLI > env
> config file > default precedence, reused for the two new resolved values). EDR-0011 (the
staleness-detection shape this PDR's manifest hashing and `publish --check` reuse).
CLAUDE.md rule 2 and rule 6 (no `dyn` dispatch; no repository port trait without
justification).

## Experiments

None yet. The renderer-command placeholder substitution and staging/atomic-replace mechanics
should be exercised against Strata's own record set as the first real publication target
before any second project adopts this design.

## Risks

A single external renderer invocation per document means publication time scales linearly
with record count and process-spawn overhead; batching multiple documents into one renderer
invocation is a possible later optimization but is not required for Strata's own
current record count. A misconfigured renderer command that exits zero without producing the
expected output file would not be caught by the non-zero-exit check alone; structural
validation (step 7) catches this by checking the output path actually exists, not just the
exit code.

## Open Questions

Publication filters beyond "all records" (RFC-0005 Open Question 2) and public/internal
publication profiles (Open Question 6) remain deferred, per RFC-0005's own scope note that
these may follow once the single-profile pipeline is exercised. The exact manifest JSON
field types and the renderer command's placeholder substitution syntax are implementation
detail left to a follow-on EDR when `strata publish` is actually built, matching how EDR-0001
through EDR-0011 progressively locked down PDR-0001's design during real implementation
rather than upfront.

## Resulting Decisions

ADR: `strata publish` is a single composition-root command owning export, manifest
generation, rendering, and site assembly end to end. ADR: `strata publish` always produces a
staged artifact directory and never writes directly to a live web root or document root. ADR:
the static-site backend is invoked as an externally configured subprocess command, not a
compiled Rust trait or backend enum, keeping generator selection at the configuration
boundary rather than the type system.

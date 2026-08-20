---
id: "ADR-0019"
title: "Static-site backend is an externally configured command, not a compiled trait"
record-type: adr
status: accepted
revision: 3
date: 2026-08-20
slug: static-site-backend-is-an-externally-configured-command-not-a-co
tags: []
relationships:
  relates-to:
    - "ADR-0006"
    - "ADR-0014"
    - "PDR-0002"
---

# ADR-0019: Static-site backend is an externally configured command, not a compiled trait

## Context

RFC-0005 named Pandoc as the renderer for the first static publication implementation.
Strata is meant to be usable by other, eventually open-source, projects, and a Pandoc-only
implementation would force any project wanting a different static-site generator to change
Strata's Rust code. A compiled backend abstraction (a `PublishBackend` trait with a `Pandoc`
implementation, ready for others to add) is the obvious way to get that flexibility, but it
is exactly the shape ADR-0006 already rejected for storage: a trait abstraction over a
domain with exactly one real implementation, justified only by hypothetical future
implementations rather than a second real one that exists today. CLAUDE.md rule 2 also bans
`dyn` dispatch outright, which a multi-backend trait would eventually require to select an
implementation at runtime from configuration.

## Decision

`strata publish` invokes the static-site backend as a single externally configured
**subprocess command**, not a compiled Rust trait or closed backend enum. The command is
resolved through the same CLI > env > config file > default precedence ADR-0014 established
(`--renderer-command`, `STRATA_PUBLISH_RENDERER`, `publish_renderer` in
`strata.config.json`), defaulting to a bundled Pandoc invocation. Strata substitutes input
Markdown path, output HTML path, a metadata file, and its own shipped template/stylesheet
paths into the configured command template and runs it via `std::process::Command` once per
manifest entry. Strata owns what gets rendered and where it goes; the configured command
owns turning one Markdown document plus metadata into one HTML file.

## Considered Options

A compiled `PublishBackend` trait with a `Pandoc` implementation and room for others to add
their own -- rejected. One real backend today does not justify the abstraction; this is the
identical reasoning ADR-0006 already applied to storage, and it would require `dyn`
dispatch or a runtime-selected closed enum to pick an implementation from configuration,
which CLAUDE.md rule 2 prohibits outright.

Hardcode Pandoc as the only supported backend with no configuration point at all --
rejected. This would force any downstream project wanting a different generator to fork and
patch Strata's Rust code, which is a worse flexibility story than a configured command line,
not a better-scoped one.

## Consequences

Swapping the static-site generator -- mdBook, Hugo via a wrapper script, a project-specific
renderer -- requires only a configuration change, never a Strata code change or a new match
arm. Strata's own default ships one Pandoc invocation and template/stylesheet; anyone
depending on different Pandoc flags, a different templating engine, or a non-Pandoc tool
supplies their own command. A misconfigured command that exits zero without producing the
expected file is not caught by exit-code checking alone; PDR-0002's staging validation step
checks that the expected output path actually exists, independent of the command's own exit
status.

## Evidence

RFC-0005, Open Question 1 (implicitly, by naming Pandoc without committing to it as the only
possible backend) and the "keep flexible for open-sourcing" direction given when resolving
that question. ADR-0006 (no repository port trait without a second real implementation),
the direct precedent this decision applies to backend selection instead of storage.
CLAUDE.md rule 2 (no `dyn` dispatch anywhere) and rule 6 (no repository port trait without
justification). ADR-0014, the configuration precedence reused for the new
`publish_renderer` value. PDR-0002 (static publication design), the design record this
decision was extracted from.

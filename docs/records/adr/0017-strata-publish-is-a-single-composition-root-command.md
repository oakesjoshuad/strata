---
id: "ADR-0017"
title: "strata publish is a single composition-root command"
record-type: adr
status: accepted
revision: 3
date: 2026-08-20
slug: strata-publish-is-a-single-composition-root-command
tags: []
relationships:
  relates-to:
    - "EDR-0011"
    - "PDR-0002"
---

# ADR-0017: strata publish is a single composition-root command

## Context

RFC-0005 proposed static-site publication but left open whether `strata publish` should
combine Markdown export, backend invocation, site assembly, and validation into one command,
or whether backend orchestration should remain an external repository script Strata does not
own (RFC-0005 Open Question 1). `strata` already owns `export`, `dump`, and, since EDR-0011,
folds `export`/`dump` staleness detection directly into `validate` specifically to remove the
need to manually chain multiple commands before trusting the result.

## Decision

`strata publish` is one command in the CLI composition root, alongside `export`, `dump`, and
`validate`. It runs the full pipeline end to end -- validate, export, manifest generation,
per-document rendering via the configured backend, site assembly, staging, and atomic
replacement -- as a single deterministic operation. There is no separate externally
orchestrated rendering step for the first implementation.

## Considered Options

Keep backend orchestration as an external repository script that consumes Strata's Markdown
export and a manifest -- rejected. This would leave a publish-time analogue of exactly the
manual multi-command chaining EDR-0011 just removed from `validate` for export/dump
staleness, reintroducing it one level up the stack instead of closing it.

Provide `strata publish` as a thin wrapper that only prints the commands a script should run
-- rejected. This adds an indirection layer with no behavioral benefit over Strata invoking
those steps itself, and it would make "did publication actually succeed" depend on a script
Strata does not control or validate.

## Consequences

`cli::publish` becomes a new module alongside `cli::export`/`cli::dump`, following the same
composition-root shape. `strata publish --check` becomes possible in the same style as
`export --check`/`dump --check`. Any future backend other than the first (Pandoc-invoking)
default is selected through configuration, not by this command's boundary -- see the
companion decision on backend invocation (ADR-relates-to this one) for how that stays
compatible with a single owning command.

## Evidence

RFC-0005, Open Question 1 and its Proposal section. EDR-0011, the precedent this decision
extends from `validate` to `publish` -- collapsing multiple manually-chained commands into
one owned pipeline. ADR-0002 (CLI-only mutation boundary), the existing precedent for the CLI
composition root owning multi-step operations. PDR-0002 (static publication design), the
design record this decision was extracted from.

---
id: "ADR-0003"
title: "Markdown and Pandoc output are generated projections, never canonical"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: markdown-and-pandoc-output-are-generated-projections-never-canon
tags: []
relationships: {}
---

# ADR-0003: Markdown and Pandoc output are generated projections, never canonical

## Context

Conventional ADR tooling (including what `substrate` used before this project) treats
a Markdown file as the record itself: the file on disk is what gets edited, reviewed,
and diffed, and any structure (required sections, a status field) is enforced only by
convention or by a separate linter run after the fact (`substrate`'s
`scripts/check_adrs.sh`, which is not part of its pre-push hook and had already drifted
red unnoticed at least once). ADR-0001 already establishes that Strata's canonical
state lives in SQLite, not in files. This decision addresses a related but separate
question: once SQLite is canonical, what is Markdown output *for*, and can it ever be
edited directly.

## Decision

We will treat Markdown (and any Pandoc-derived output — HTML, PDF, DOCX) as a
deterministic projection of the canonical SQLite record, never as something edited
directly. Rendering the same record twice from identical canonical state must produce
byte-identical Markdown. If committed Markdown in git is checked into a repository
export, CI can verify it matches canonical state (`eng export --check`,
`docs/specification.md` section 28) and fail if it doesn't — but nothing ever treats a
divergence between the file and the database as "the file wins."

## Considered Options

\- **Markdown is canonical**, as in conventional ADR tooling — the file on disk is the
  record, SQLite (if present at all) is a derived index.
\- **Markdown is a generated, read-only projection** of the SQLite record — produced by
  a deterministic renderer, never hand-edited, regenerated whenever the record changes.

## Consequences

\- A record can never be edited by opening its Markdown file in a text editor and
  saving — mutation only happens through ADR-0002's CLI-only boundary, which writes to
  SQLite and then re-renders. This closes off a second, informal write path that would
  otherwise bypass ADR-0002's guarantee entirely.
\- Markdown stays useful for what it's actually good at — human review in a pull
  request, `grep`-ability, working without the CLI installed — without being asked to
  also be the thing that enforces structural correctness. That job belongs to the
  Rust application core (ADR-0002), not to Markdown convention.
\- The renderer itself becomes a piece of code that has to be correct and deterministic
  — a non-deterministic renderer (e.g. one whose output depends on field iteration
  order in a JSON map) would make `eng export --check` unreliable and defeat the point
  of this decision. This is a real implementation constraint, not just a nicety.
\- This decision is explicitly deferred in effect for v0.1 (see PDR-0001's scope cut):
  the deterministic renderer is not part of the first working version. The decision is
  recorded now because it shapes how the SQLite schema and document model are
  designed even before the renderer exists — document content must be structured
  enough to render deterministically later, which constrains EDR-level choices made
  today.

## Evidence

\- `docs/specification.md`, sections 3.1, 18, and 19 (the originating "canonical
  knowledge, projected documents" principle this decision adopts).
\- `feedback_adr_mcp_update_content_gotcha` (session memory, substrate repository): the
  incident that showed what happens when a document can be edited directly with no
  canonical source to fall back to or diff against.

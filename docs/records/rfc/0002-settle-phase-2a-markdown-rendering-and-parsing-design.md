---
id: "RFC-0002"
title: "Settle Phase 2a Markdown rendering and parsing design"
record-type: rfc
status: accepted
revision: 4
date: 2026-08-19
slug: settle-phase-2a-markdown-rendering-and-parsing-design
tags: []
relationships:
  produces:
    - "ADR-0008"
    - "ADR-0009"
    - "ADR-0010"
    - "ADR-0011"
    - "ADR-0012"
    - "EDR-0003"
    - "EDR-0004"
---

# RFC-0002: Settle Phase 2a Markdown rendering and parsing design

## Motivation

Issue #6 identifies the design questions blocking Phase 2a: rendering JSON records to deterministic Pandoc-compatible Markdown, parsing that Markdown back into JSON for future strata new and strata revise input, and generating templates from the same rules. The self-host migration demonstrated that manually constructing multi-paragraph JSON for --document is an avoidable CLI ergonomics gap.

## Problem

If rendering and Markdown input use separate per-kind rules, they will drift: a document rendered by strata render may not parse back to the same fields, and a template may not be accepted by the parser. The specification leaves the document IR as a SHOULD and gives only an illustrative ADR frontmatter example, so implementation would otherwise make several independent judgment calls.

## Scope

Define the design for the bidirectional record-to-Markdown mapping used by strata render, future Markdown input for strata new and strata revise, and strata new --template. Define the document IR, frontmatter contract, filename slug algorithm, per-kind section order and field mapping, export --check comparison, malformed-input errors, and title-renaming path identity.

## Non-Goals

This RFC does not implement rendering, parsing, export, template generation, Markdown input, or title updates. It does not define publication-site assembly, Pandoc output formats beyond the Markdown projection, or an evidence database model beyond the existing structured record document.

## Constraints

The canonical record remains SQLite state; Markdown remains a deterministic projection. Every mutation and future Markdown import must use the validated CLI boundary. RecordKind::required_fields() is authoritative for document fields. The mapping must be deterministic, shared in both directions, and compatible with the four closed record kinds without silently dropping content.

## Proposal

Pursue Phase 2a with a shared Engineering Document IR and one explicit per-kind Markdown contract. Use one common frontmatter shape, a deterministic identifier-prefixed slug, exact ordered section mappings, byte-exact export checking, explicit parse errors, and a creation-time fixed filename slug. Record each independently reversible choice in its own ADR or EDR before implementation issues #7 through #11 begin.

## Alternatives Considered

Continue with direct string templates and parse them with separate ad hoc logic; use one generic frontmatter shape but infer sections from JSON object order; normalize Markdown before export checking; let title changes rename files; or defer the decisions to implementation. These alternatives preserve less of the single-rule guarantee and leave the known drift and stale-file failure modes unresolved.

## Open Questions

The design questions are resolved by the produced records: ADR-0008 chooses the shared IR; ADR-0009 defines frontmatter; EDR-0003 defines filename slugs; EDR-0004 defines bidirectional field mapping; ADR-0010 defines byte-exact --check; ADR-0011 defines malformed-input errors; ADR-0012 fixes the filename slug at creation despite later title changes.

## Outcome

Accepted. Strata should pursue the Phase 2a rendering and parsing design through the seven produced decisions before implementation begins. The RFC is created through the self-hosted CLI as the first record authored natively in Strata rather than imported from pre-existing Markdown.

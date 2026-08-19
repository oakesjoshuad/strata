---
id: "ADR-0008"
title: "Use a shared Engineering Document IR for Markdown rendering and parsing"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: use-a-shared-engineering-document-ir-for-markdown-rendering-and-
tags: []
relationships: {}
---

# ADR-0008: Use a shared Engineering Document IR for Markdown rendering and parsing

## Context

Specification section 18 sketches an Engineering Document IR with Heading, Paragraph, List, CodeBlock, Table, Diagram, RecordReference, and EvidenceReference blocks, but marks an internal Rust IR as a SHOULD. Phase 2a now has two consumers: rendering structured records out to Markdown and parsing Markdown back into structured records for future --file or stdin input. Direct string templating is easy for one direction but gives the parser no stable structure to target.

## Decision

We will use one shared, typed Engineering Document IR for both directions. Rendering maps a validated JSON document to per-kind IR blocks and then to Pandoc-compatible Markdown; parsing maps the Markdown into the same IR and then applies the same per-kind section rules to produce JSON. The IR will include the specification's block families—Heading, Paragraph, List, CodeBlock, Table, Diagram, RecordReference, and EvidenceReference—and will keep domain validation and SQLite behavior outside the presentation layer.

## Considered Options

Direct per-kind string templates with a separate Markdown parser; a Markdown AST with no Strata-owned IR; or one-way rendering now and parser design later. These approaches either make the two directions drift-prone or couple domain behavior directly to a format-specific representation.

## Consequences

The renderer, parser, and template scaffold share one intermediate contract and can be tested for round-trip stability. The IR adds types and a conversion layer before Markdown output, but that cost is justified by two consumers and makes future Pandoc or JSON projections possible without changing domain semantics.

## Evidence

docs/specification.md section 18 requires an engineering document-oriented intermediate representation and names the block families. Sections 19 and 22 require deterministic Pandoc-compatible Markdown while keeping engineering semantics independent of Pandoc. Issue #6 adds Markdown input and templates as consumers of the same mapping.

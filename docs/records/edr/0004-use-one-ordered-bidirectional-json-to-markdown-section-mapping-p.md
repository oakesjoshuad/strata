---
id: "EDR-0004"
title: "Use one ordered bidirectional JSON-to-Markdown section mapping per kind"
record-type: edr
status: accepted
revision: 2
date: 2026-08-19
slug: use-one-ordered-bidirectional-json-to-markdown-section-mapping-p
tags: []
relationships: {}
---

# EDR-0004: Use one ordered bidirectional JSON-to-Markdown section mapping per kind

## Context

RecordKind::required_fields() is the authoritative JSON field list, but the existing Markdown records use human-oriented section names and do not map one-to-one without judgment. Rendering, parsing, and templates must share exact headings and order so a round trip cannot silently drift.

## Decision

We will render a title heading followed by ordered level-two sections, one for every required field, using these exact mappings. RFC: motivation -> Motivation; problem -> Problem; scope -> Scope; non_goals -> Non-Goals; constraints -> Constraints; proposal -> Proposal; alternatives -> Alternatives Considered; questions_for_review -> Open Questions; outcome -> Outcome. PDR: problem -> Problem; requirements -> Requirements; constraints -> Constraints; proposed_design -> Proposed Design; components -> Components; interfaces -> Interfaces; data_model -> Data Model; failure_modes -> Failure Modes; alternatives -> Alternatives Considered; evidence -> Evidence; experiments -> Experiments; risks -> Risks; open_questions -> Open Questions; resulting_decisions -> Resulting Decisions. ADR and EDR: context -> Context; decision -> Decision; alternatives -> Considered Options; consequences -> Consequences; evidence -> Evidence. The parser recognizes only these exact headings at the required level and preserves each section's Markdown block content; the renderer emits them in this order regardless of JSON object order.

## Considered Options

Infer headings from JSON keys; preserve the source Markdown headings as arbitrary aliases; or define separate render and parse mappings. Those alternatives make templates ambiguous or allow the two directions to diverge. A single ordered table per kind gives render, parse, and template generation the same contract.

## Consequences

Every required field has a visible, deterministic place in Markdown, including fields that were absent from some founding hand-written files. Existing prose can be folded into the authoritative field without inventing fields, but future imports must provide every required section. Section content can contain nested Markdown blocks, while section boundaries and headings remain controlled by the mapping.

## Evidence

records/src/kind.rs defines the authoritative required_fields() lists. docs/specification.md sections 18 and 19 require an intermediate document representation and deterministic Markdown. The self-host migration exposed the exact RFC and PDR mapping gaps that this shared table resolves.

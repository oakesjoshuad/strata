---
id: "ADR-0016"
title: "Retain separate ADR and EDR kinds with a positive classification test"
record-type: adr
status: draft
revision: 2
date: 2026-08-19
slug: retain-separate-adr-and-edr-kinds-with-a-positive-classification
tags: []
relationships:
  relates-to:
    - "RFC-0004"
---

# ADR-0016: Retain separate ADR and EDR kinds with a positive classification test

## Context

RFC-0004 leaves one substantive workflow question: whether Strata should retain separate ADR and EDR record kinds, merge them, or broaden ADR while retaining EDR for narrowly implementation-specific choices. The existing model distinguishes architecturally significant choices from implementation-level engineering choices, but it does not provide a positive classification test. ADR-0015 has settled traceability independently; this decision is about retrieval and authoring taxonomy, not lineage requirements.

## Decision

Retain ADR and EDR as separate record kinds. Classify a record as an ADR when its decision changes system architecture, externally observable behavior, a durable boundary between components, a persistence or data-model contract, or a cross-cutting operational constraint. Classify a record as an EDR when its decision is specific to implementing an already-established design within one component, code path, toolchain, or local operational mechanism and does not itself change an architectural boundary. If a decision meets both descriptions, classify it as ADR because the broader constraint is the more durable retrieval anchor. EDR is therefore a positive category for implementation-specific decisions, not a miscellaneous overflow. Each record still documents exactly one decision.

## Considered Options

Merge ADR and EDR into one Decision Record kind -- rejected for now because it would remove a useful retrieval distinction before evidence shows that the distinction causes more classification cost than value.

Broaden ADR to Any Decision Record and retain EDR only for novel implementation detail -- deferred because it changes terminology and the public purpose of ADR without first establishing whether the current architectural-versus-implementation boundary is actually failing in practice.

Keep the current split without a classification test -- rejected because EDR would remain defined negatively as not-ADR, making authoring and retrieval inconsistent.

## Consequences

The four-kind model remains stable, and existing records do not need reclassification solely because this decision is accepted. New records have a positive classification test: architectural scope wins when both scopes apply. Templates and CLI help can continue using ADR and EDR, but their descriptions should be updated to reflect the test. A future review can merge or rename the kinds if actual search and authoring evidence shows the split is not useful. This decision does not add lifecycle gates, traceability requirements, or status behavior.

## Evidence

RFC-0004's reconciled scope question; RFC-0001's four-kind model; docs/specification.md sections 5.3 and 5.4; the current RecordKind purpose strings in records/src/kind.rs; ADR-0015's separate traceability decision; existing accepted ADR and EDR records in the self-hosted database.

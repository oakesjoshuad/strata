---
id: "ADR-0016"
title: "Retain separate ADR and EDR kinds with a positive classification test"
record-type: adr
status: accepted
revision: 5
date: 2026-08-19
slug: retain-separate-adr-and-edr-kinds-with-a-positive-classification
tags: []
relationships:
  relates-to:
    - "RFC-0004"
---

# ADR-0016: Retain separate ADR and EDR kinds with a positive classification test

## Context

RFC-0004 leaves one substantive workflow question: whether Strata should retain separate ADR and EDR record kinds, merge them, or broaden ADR while retaining EDR for narrowly implementation-specific choices. The initial draft test was too broad: externally visible behavior and persistence contracts also appear in implementation-mechanics records. This revision defines the boundary by ownership, architectural scope, canonical-state responsibility, and system-wide policy versus local mechanism. ADR-0015 has settled traceability independently; this decision is about retrieval and authoring taxonomy, not lineage requirements.

## Decision

Retain ADR and EDR as separate record kinds. Classify a record as an ADR when its decision changes ownership between components, an architectural boundary, the canonical source of truth, or a system-wide policy or contract that multiple commands, crates, or components must honor. Classify a record as an EDR when its decision selects an algorithm, format, parameter, or implementation mechanism within an already-established boundary, even if the resulting behavior or artifact is externally visible. If a decision meets both descriptions, classify it as ADR because the broader boundary or policy is the more durable retrieval anchor. EDR is therefore a positive category for implementation mechanics, not a miscellaneous overflow. Each record still documents exactly one decision.

## Considered Options

Merge ADR and EDR into one Decision Record kind -- rejected for now because it would remove a useful retrieval distinction before evidence shows that the distinction causes more classification cost than value.

Broaden ADR to Any Decision Record and retain EDR only for novel implementation detail -- deferred because it changes terminology and the public purpose of ADR without first establishing whether the current architectural-versus-implementation boundary is actually failing in practice.

Keep the current split without a classification test -- rejected because EDR would remain defined negatively as not-ADR, making authoring and retrieval inconsistent.

## Consequences

The four-kind model remains stable, and existing records do not need reclassification solely because this decision is accepted. New records use scope rather than visibility as the primary test: a system-wide boundary or policy is ADR; a local algorithm, format, parameter, or implementation mechanism is EDR. A visible file, CLI result, or persisted value is not automatically architectural. Architectural scope wins when both scopes apply. Templates and CLI help can continue using ADR and EDR, but their descriptions should be updated to reflect this test. A future review can merge or rename the kinds if actual search and authoring evidence shows the split is not useful. This decision does not add lifecycle gates, traceability requirements, or status behavior.

## Evidence

The accepted corpus supports the refined boundary. ADR-0001, ADR-0002, ADR-0003, ADR-0006, ADR-0008, ADR-0009, ADR-0010, ADR-0013, ADR-0014, and ADR-0015 establish canonical ownership, crate or projection boundaries, shared document contracts, or system-wide policy. EDR-0001, EDR-0002, EDR-0003, EDR-0004, EDR-0005, EDR-0006, EDR-0007, and EDR-0008 choose local allocation, search, slug, serialization, validation, lifecycle-label, or configuration mechanics within established boundaries. The specification sections 5.3 and 5.4 and records/src/kind.rs provide the original architectural-versus-implementation distinction. ADR-0015 is a separate traceability decision and does not determine taxonomy.

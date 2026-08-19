---
id: "ADR-0015"
title: "Emit advisory traceability warnings for accepted decisions without qualifying lineage"
record-type: adr
status: accepted
revision: 4
date: 2026-08-19
slug: emit-advisory-traceability-warnings-for-accepted-decisions-witho
tags: []
relationships:
  relates-to:
    - "RFC-0004"
---

# ADR-0015: Emit advisory traceability warnings for accepted decisions without qualifying lineage

## Context

RFC-0004 identified that Strata's relationship graph can express lineage without surfacing when an accepted decision has no visible upstream rationale. EDR-0007 established that Draft is a lifecycle label, not a traceability gate, and RFC-0001 and EDR-0006 establish that standalone and retroactive records are legitimate.

## Decision

Traceability is an advisory validation signal, not a lifecycle gate. When an accepted ADR or EDR has no incoming produces or derived-from relationship from an RFC or PDR, strata validate reports a warning. The warning identifies the decision record and the missing qualifying lineage. A missing edge does not prevent creation, revision, status transition, export, or acceptance. relates-to, constrains, explored-by, and implemented-by do not satisfy this check because they express relevance or implementation relationships rather than decision provenance. Draft and proposed records are not warned by this rule; the check evaluates accepted decisions only, keeping unfinished work and retroactive documentation valid while making accepted decisions' provenance inspectable.

## Considered Options

Require every ADR and EDR to have an RFC/PDR ancestor before acceptance -- rejected because it would block standalone and retroactive decisions and turn the flexible graph into a mandatory pipeline.

Count any incoming relationship as traceability -- rejected because relates-to, constrains, and implementation edges do not establish that an RFC or PDR produced the decision.

Warn on every proposed or draft decision -- rejected because it would create noise during normal authoring and would attach lifecycle semantics to a signal EDR-0007 deliberately kept orthogonal to status.

## Consequences

Accepted ADRs and EDRs without a qualifying produces or derived-from edge become visible during validation without becoming invalid. The warning is deterministic and explainable from the relationship graph. Existing standalone and retroactive records remain valid, but their lack of recorded provenance is no longer silent. The implementation belongs in store validation because it already owns graph and lifecycle checks; the CLI continues to present the resulting validation signal. A later decision may add configurable blocking behavior, but this ADR does not introduce it.

## Evidence

RFC-0004's reconciled open questions and constraints; RFC-0001 and PDR-0001's flexible graph policy; EDR-0007's explicit separation of Draft status from traceability; store/src/validate.rs's existing lifecycle validation and store/src/queries.rs's concrete relationship queries.

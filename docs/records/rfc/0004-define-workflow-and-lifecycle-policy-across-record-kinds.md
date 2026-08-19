---
id: "RFC-0004"
title: "Define workflow and lifecycle policy across record kinds"
record-type: rfc
status: proposed
revision: 2
date: 2026-08-19
slug: define-workflow-and-lifecycle-policy-across-record-kinds
tags: []
relationships:
  relates-to:
    - "PDR-0001"
    - "RFC-0001"
    - "RFC-0003"
---

# RFC-0004: Define workflow and lifecycle policy across record kinds

## Motivation

Issue #14 asks for a workflow and lifecycle-policy review rather than another
implementation decision. Strata has four intentionally different record kinds,
but two questions about their operating policy remain underspecified. RFC and
PDR begin in `draft`, while ADR and EDR begin in `proposed`; and the relationship
graph can express cross-kind lineage without any validation signal when a
decision has no visible upstream rationale. At the same time, Strata's original
ADR/EDR split deserves a second look now that newer MADR evidence points toward
"Any Decision Record" and Oxide provides a real example of a flattened record
kind.

This RFC is a workflow proposal, not a transcription of the current code or of
the two research lineages. NASA/DoD's PDR/CDR gate model is useful evidence for
named entrance and exit criteria, but importing its hard gates into a single-user
software knowledge base would contradict the deliberately flexible graph in
`docs/specification.md:290-321` and the software record practices that permit
retroactive documentation and lightweight decisions.

## Problem

`records/src/kind.rs:55-59` gives RFC and PDR an initial `draft` status but gives
ADR and EDR `proposed`. Their status lists at `records/src/kind.rs:62-84` also
omit `draft`. The specification describes the same asymmetry in sections 5.3
and 5.4 (`docs/specification.md:232-269`) without defining what a not-yet-ready
ADR or EDR should mean operationally.

The graph has the vocabulary to express lineage: the specification's examples
include RFC explored-by PDR and PDR produces ADR/EDR
(`docs/specification.md:290-321`). It does not, however, currently check whether
an approved or accepted decision has such a relationship. This leaves a useful
distinction unobserved: a small standalone decision, a deliberately retroactive
record, and a decision that was expected to result from an RFC or PDR all look
the same. The existing validation precedent is deliberately softer: section 29
shows `WARN PDR-0017 approved but has no resulting decision`
(`docs/specification.md:1013-1040`).

Finally, the current scope boundary is not obviously the best one. ADR means
architecturally significant choice in `records/src/kind.rs:37-43` and
`docs/specification.md:232-249`; EDR means implementation-level engineering
choice in `records/src/kind.rs:42` and `docs/specification.md:251-269`. That
boundary can make readers search a document bible for relevance, while MADR's
v3 change to "Any Decision Record" and Oxide's flattened RFD show credible
alternatives. EDR is also not an established industry term outside Strata, so
its continued existence needs a sharper positive definition than "not ADR."

## Scope

This RFC proposes policy for the meaning and possible use of `draft` across all
four kinds, a checkable but initially advisory form of cross-kind traceability,
and a follow-on re-evaluation of ADR and EDR scope. It covers the interaction
between those policies and retroactive records such as RFC-0001 and EDR-0006.

It also records the boundary with RFC-0003 (issue #13): whether traceability
strictness can later be configured is a consumer question for that configuration
design, not a decision about RFC-0003's format, location, precedence, or schema.

## Non-Goals

This RFC does not change `RecordKind`, status transitions, validation code,
relation vocabulary, search defaults, templates, or generated Markdown. It does
not rename ADR, merge ADR and EDR, or define the final test for assigning a
record to either kind. It does not impose NASA/DoD-style entrance or exit gates
on status transitions.

It does not decide whether Strata should adopt Oxide's Published-versus-Committed
distinction. That is a separate lifecycle question deserving a future RFC if
the distinction matters to Strata's local-first workflow.

## Constraints

The canonical state remains SQLite and Markdown remains its generated projection,
as established by RFC-0001 and PDR-0001. Relationships remain explicit graph
edges rather than prose conventions. The graph must remain flexible: the
specification explicitly says Strata must not require every RFC to have a PDR or
every PDR to produce both ADR and EDR (`docs/specification.md:318-321`).

Validation must distinguish errors from review signals. A missing upstream edge
must not block creation or acceptance merely because a record is being written
retroactively or is intentionally standalone. Any malformed document or invalid
relationship remains an explicit failure under the repository's validation
rules; this RFC proposes only a new warning-shaped policy signal.

Each ADR or EDR will continue to document exactly one decision. Any future
scope change must preserve that rule and must not turn EDR into a miscellaneous
overflow category.

## Proposal

Treat workflow policy as a set of explicit, inspectable conventions rather than
as a mandatory pipeline:

1. Revisit whether ADR and EDR should admit a `draft` state before `proposed`,
   with the operational meaning of that state specified separately. The useful
   question is not merely whether the enum should gain a variant; it is whether
   draft content is discoverable, searchable, and included in lifecycle health
   checks. This policy is independent of the traceability decision.
2. Add a validation signal for decision records whose context suggests an
   upstream RFC/PDR but whose graph has no qualifying lineage edge. The initial
   policy should be a `WARN`, following the precedent in specification section
   29, not a status-transition gate. The check should be explicit about which
   relations qualify; `relates-to` should not silently make the check trivial
   merely because it is easy to add.
3. Keep the graph permissive and retroactive-friendly. An upstream warning is a
   prompt to explain or link the record, not proof that the record is invalid.
   A later policy can distinguish expected lineage from standalone or
   after-the-fact decisions without pretending that chronology is always
   available.
4. Re-examine the ADR/EDR boundary as a separate follow-on decision. The
   recommended direction for that review is to broaden ADR to Any Decision
   Record while retaining EDR for genuinely novel, specific implementation
   details, with a concrete classification test still required. The competing
   option of fully merging ADR and EDR is the central alternative, not a
   straw-man: MADR completed a similar broadening, and Oxide's flattened RFD
   demonstrates a different but credible workflow. Leaving today's split
   unchanged remains a third live option if a test shows that the separation
   materially improves retrieval.

This proposal borrows entrance/exit-criteria thinking as a named warning and
explanation aid, not as NASA's hard gate. It also treats retroactive writing as
normal documentation work: RFC-0001 and EDR-0006 are evidence that a record may
be valuable after the decision. The follow-on design should determine whether
the warning needs an explicit retroactive annotation or whether advisory
validation already covers it.

If ADR expands from Architecture to Any, the eventual decision record must
revisit the ADR purpose string at `records/src/kind.rs:37-43`, the ADR entry in
`docs/specification.md:232-249`, and the `KindArg::Adr` documentation in
`cli/src/args.rs:219`. This RFC flags those consequences; it does not edit them.

## Alternatives Considered

**Import NASA/DoD hard gates.** This would give PDR exit criteria strong meaning
and make downstream decisions auditable, but it is disproportionate to Strata's
single-user software context and conflicts with the specification's explicit
rejection of a rigid pipeline. It is useful source material for warning design,
not a suitable default lifecycle.

**Require every ADR and EDR to have an RFC/PDR ancestor.** This would make the
graph look complete, but it would misclassify standalone decisions and
retroactive records. It would also turn the flexible relationship model in
PDR-0001 into an accidental mandatory workflow. An advisory, qualifying-edge
check preserves the useful prompt without that false certainty.

**Count any `relates-to` edge as traceability.** This minimizes false warnings,
but makes the check nearly vacuous: lateral relevance is not evidence that a
decision was produced by or derived from a proposal. It should be considered as
an explicit alternative during the relation-policy decision, not adopted by
convenience.

**Keep ADR as Architecture and EDR as implementation detail.** This preserves
the current four-kind model and the separation-of-concerns motivation behind
RFC-0001. It remains plausible, but the present wording is not a sufficient
classification test and MADR's completed move away from the architecture label
is evidence against treating the old boundary as self-validating.

**Merge ADR and EDR into Any Decision Record.** MADR's v3 terminology and
Oxide's flattened RFD make this a serious alternative, unlike the evidence
available when RFC-0001 rejected a flattened model. One kind would improve
recall and reduce classification overhead, but could produce the single
document bible this review is trying to avoid and might erase useful
implementation-specific retrieval. The merge-versus-separate choice belongs in
a follow-on ADR/EDR, not in this RFC's outcome.

**Adopt Oxide's Published-versus-Committed lifecycle now.** This could separate
"reviewed decision" from "implemented state," but it introduces a second axis
that is not needed to answer the draft, lineage, or scope questions here. Name
and evaluate it in a future RFC instead of coupling it to this policy pass.

## Open Questions

1. **Draft state:** Should ADR and EDR gain `draft`? What does draft mean
   operationally: excluded from traceability validation, excluded from search
   by default, excluded from lifecycle warnings, or purely advisory metadata?
2. **Qualifying lineage:** Which relations satisfy an upstream RFC/PDR check:
   only `produces` and `derived-from`, or also `explored-by`, `constrains`, or
   `relates-to`? If `relates-to` counts, how is the check prevented from being
   trivially satisfied?
3. **Strictness and RFC-0003:** Should traceability remain a soft warning by
   default with opt-in strict/blocking behavior through RFC-0003's eventual
   mechanism, or is that coupling premature while both RFCs remain unresolved?
   This question does not decide RFC-0003's format, location, precedence, or
   configuration schema.
4. **ADR and EDR scope:** Should ADR become Any Decision Record while EDR stays
   narrowly limited to novel, specific implementation detail; should ADR and
   EDR merge into one kind; or should today's split remain? If separate, what
   concrete classification test proves that a record belongs in EDR rather than
   ADR, rather than merely not fitting an underspecified ADR label?
5. **Retroactive records:** Do RFC-0001 and EDR-0006 require an explicit
   traceability or draft-state carve-out, or does WARN-not-block already cover
   retroactive documentation without a special case?
6. **Published versus committed:** Does Oxide's distinction belong in this
   workflow policy, or should it be a distinct future RFC? The current scope
   recommendation is to defer it.

## Outcome

Proposed. This RFC recommends a soft, explicit traceability signal and a serious
review of draft semantics and ADR/EDR scope, while preserving the flexible graph
and retroactive documentation. It intentionally resolves none of the six
questions above. Follow-on records should settle the status behavior, qualifying
relations, strictness/configuration boundary, ADR/EDR classification or merge,
retroactive treatment, and any Published-versus-Committed lifecycle before
implementation changes are made.

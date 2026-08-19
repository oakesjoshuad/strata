---
id: "RFC-0004"
title: "Define workflow and lifecycle policy across record kinds"
record-type: rfc
status: proposed
revision: 8
date: 2026-08-19
slug: define-workflow-and-lifecycle-policy-across-record-kinds
tags: []
relationships:
  produces:
    - "ADR-0015"
    - "EDR-0007"
  relates-to:
    - "PDR-0001"
    - "RFC-0001"
    - "RFC-0003"
---

# RFC-0004: Define workflow and lifecycle policy across record kinds

## Motivation

Issue #14 asks for a workflow and lifecycle-policy review rather than another implementation decision. Strata has four intentionally different record kinds, and the remaining policy questions concern cross-kind lineage signals, retroactive records, and the ADR/EDR boundary. EDR-0007 has already resolved the draft-state asymmetry for ADR and EDR by adding a consistent Draft phase without imposing traceability gates. RFC-0003, ADR-0014, and EDR-0008 have also settled the configuration boundary that this RFC previously treated as open.

This RFC is a workflow proposal, not a transcription of the current code or of the two research lineages. NASA/DoD's PDR/CDR gate model is useful evidence for named entrance and exit criteria, but importing its hard gates into a single-user software knowledge base would contradict the flexible graph in docs/specification.md:290-321 and the software record practices that permit retroactive documentation and lightweight decisions.

## Problem

The graph has the vocabulary to express lineage: the specification's examples include RFC explored-by PDR and PDR produces ADR/EDR (docs/specification.md:290-321). It does not currently check whether an accepted decision has an appropriate upstream relationship. This leaves a useful distinction unobserved: a standalone decision, a deliberately retroactive record, and a decision expected to result from an RFC or PDR all look the same.

The lifecycle asymmetry is no longer open. EDR-0007 added Draft to ADR and EDR, matching RFC and PDR, while deliberately leaving search, validation, and export status-neutral. RFC-0003's configuration boundary is also settled by ADR-0014 and EDR-0008; any future traceability strictness setting would consume that mechanism rather than redefine it here.

## Scope

This RFC proposes policy for a checkable but initially advisory form of cross-kind traceability, the interaction between that signal and retroactive records, and a follow-on re-evaluation of ADR and EDR scope.

EDR-0007 resolves the draft-status question, and RFC-0003, ADR-0014, and EDR-0008 resolve the configuration boundary. Published-versus-Committed remains outside this RFC and should receive its own proposal if it proves valuable.

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

Treat workflow policy as a set of explicit, inspectable conventions rather than as a mandatory pipeline:

1. Use EDR-0007's Draft status consistently across all four kinds. That decision is complete: Draft is discoverable, searchable, validated, and exported exactly like other statuses, and ADR/EDR must pass through Proposed before acceptance.

2. Use ADR-0015's advisory traceability policy. Accepted ADR/EDR records without incoming produces or derived-from lineage from an RFC/PDR receive a warning; the warning does not block lifecycle changes, and standalone or retroactive records remain valid.

3. Re-examine the ADR/EDR boundary as a separate follow-on decision. The live choices are to retain the split with a concrete classification test, broaden ADR to Any Decision Record while retaining EDR for genuinely novel implementation detail, or merge the kinds.

RFC-0003's configuration design is now accepted, so any future strictness setting would be a consumer of that mechanism rather than an unresolved boundary question in this RFC. Published-versus-Committed remains deferred to a separate RFC if it proves valuable.

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

1. **ADR and EDR scope:** Should ADR become Any Decision Record while EDR stays narrowly limited to novel implementation detail; should ADR and EDR merge; or should today's split remain? If separate, what concrete classification test proves the distinction?

2. **Published versus committed:** Defer this to a separate RFC unless the workflow review finds that the distinction is necessary to explain lifecycle state.

## Outcome

Proposed. EDR-0007 resolves the draft-state question. ADR-0015 resolves the traceability questions: accepted ADR/EDR records without incoming produces or derived-from lineage from an RFC/PDR receive an advisory warning, while standalone and retroactive records remain valid and strictness remains non-blocking by default. RFC-0003, ADR-0014, and EDR-0008 resolve the configuration boundary. The remaining substantive question is the ADR/EDR taxonomy; Published-versus-Committed remains deferred.

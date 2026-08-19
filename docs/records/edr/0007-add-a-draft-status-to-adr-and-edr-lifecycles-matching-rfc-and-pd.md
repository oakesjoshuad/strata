---
id: "EDR-0007"
title: "Add a Draft status to ADR and EDR lifecycles, matching RFC and PDR"
record-type: edr
status: accepted
revision: 2
date: 2026-08-19
slug: add-a-draft-status-to-adr-and-edr-lifecycles-matching-rfc-and-pd
tags: []
relationships:
  relates-to:
    - "EDR-0006"
    - "RFC-0001"
  resolves:
    - "RFC-0004"
---

# EDR-0007: Add a Draft status to ADR and EDR lifecycles, matching RFC and PDR

## Context

RFC-0004 (proposed) opened six workflow-policy questions without resolving them; this EDR resolves the first. `records/src/kind.rs:55-60`'s `initial_status()` gives RFC and PDR an initial `Draft` status but gives ADR and EDR `Proposed`. `records/src/kind.rs:62-85`'s `statuses()` carries the same asymmetry into each kind's full lifecycle list: RFC has `[Draft, Proposed, UnderReview, Accepted, Withdrawn]`, PDR has `[Draft, Review, Approved, Superseded]`, but ADR has only `[Proposed, Accepted, Deprecated, Superseded]` and EDR only `[Proposed, Accepted, Superseded]` -- neither has ever had a `Draft` state. This isn't a code gap diverging from the specification; `docs/specification.md` sections 5.3 and 5.4 never gave ADR or EDR an indicative `draft` stage either, so this EDR extends the original design rather than fixing a divergence from it.

The motivating question, from the project owner directly: is there a status that lets an ADR or EDR exist and be worked on without requiring a link to an associated upstream document first? RFC-0004 already concluded, and this conversation reinforced, that the answer to that question must not be a hard requirement: a mandatory upstream link before a record can exist or advance would directly discourage the exact behavior this project's own history depends on -- RFC-0001 and EDR-0006 both document real decisions after the fact, and RFC-0004's research found explicit, repeated industry guidance that retroactive and standalone decision records are normal practice, not an exception to work around. Requiring a precedent document "would diminish and discourage users from engaging and recording significant historical or impactful decisions" (project owner, this conversation) -- a real risk that a hard gate would trade away for a completeness guarantee the graph does not need.

Given that, `Draft`'s actual job here is narrower than it might first appear: nothing in `store::create` or `store::set_status` (`store/src/mutations.rs`) currently checks the relationship graph at all, so no record of any kind or status has ever needed a linked predecessor to exist. Adding `Draft` to ADR/EDR does not unlock new flexibility that was missing -- it gives ADR and EDR the same "still shaping this, not yet asking for review" phase RFC and PDR already have, for consistency, independent of whatever RFC-0004's still-open traceability question (its Q2/Q5) eventually decides. A `Draft`-status ADR and a `Proposed`-status ADR should be subject to the same future traceability policy, whatever it turns out to be; this EDR does not presuppose an answer to that question.

Checking current behavior directly: `store::search()` (`store/src/queries.rs`) issues no status-filtering WHERE clause at all -- every record is searched regardless of status -- and `store::validate()`'s per-record `validate_document` check (`store/src/validate.rs`) runs unconditionally regardless of status too. RFC and PDR's existing `Draft` status therefore already has zero special behavior anywhere in the current implementation; it is purely a lifecycle label today. That is direct evidence this change is low-risk: adding the same label to ADR and EDR does not require inventing any new filtering or exclusion machinery to stay consistent with existing behavior.

## Decision

We will add `Status::Draft` as the initial status for `RecordKind::Adr` and `RecordKind::Edr` in `records/src/kind.rs`, matching `Rfc`/`Pdr`. `statuses()` gains `Draft` as the first entry for both: ADR becomes `[Draft, Proposed, Accepted, Deprecated, Superseded]`, EDR becomes `[Draft, Proposed, Accepted, Superseded]`. `records/src/validation.rs`'s `validate_transition` gains `(Status::Draft, Status::Proposed)` as a valid transition for both `RecordKind::Adr` (currently lines 112-118) and `RecordKind::Edr` (currently lines 119-122), prepended to their existing transition rules, which are otherwise unchanged -- `Draft` cannot transition directly to `Accepted`, it must pass through `Proposed` first, the same discipline RFC and PDR already enforce for their own `Draft` state.

No new behavior is attached to `Draft` anywhere else in the system. Search, validation, FTS indexing, and Markdown export all continue to treat every status identically, exactly as they do today for RFC and PDR's `Draft` records. This explicitly resolves RFC-0004's Q1 sub-question -- whether draft content is discoverable, searchable, and included in lifecycle health checks -- by choosing consistency with existing behavior over inventing new status-based filtering for only two of the four kinds.

This EDR does not touch RFC-0004's traceability question (Q2, qualifying relations; Q5, retroactive carve-out) at all. Those remain fully open for a separate follow-on ADR, unaffected by whether the record in question happens to be `Draft` or `Proposed` when that policy is eventually applied.

## Considered Options

Give `Draft` exclusionary behavior -- hidden from `search` results by default, skipped by `validate` -- rejected for this change: it would make ADR/EDR's new `Draft` behave differently from RFC/PDR's existing `Draft`, an inconsistency with no current justification, since nothing in this codebase filters by status anywhere today. If default exclusion from search is wanted later, it should be evaluated once, uniformly, across all four kinds' `Draft` status, not introduced piecemeal as a side effect of this EDR.

Add `Draft` to only one of ADR or EDR, not both -- rejected: RFC-0004's own problem statement observes the identical asymmetry affects both kinds equally; nothing distinguishes them for this purpose.

Use a hard traceability requirement instead of, or alongside, a `Draft` status -- for example, requiring a `Draft`-status ADR/EDR to acquire a qualifying upstream link before it can advance to `Proposed` or `Accepted` -- rejected per RFC-0004's own conclusion and the reasoning restated above: this would functionally reintroduce the mandatory-pipeline problem the specification's flexible graph (`docs/specification.md:290-321`) and RFC-0004 both already reject, just relocated to a different transition point. This EDR deliberately keeps `Draft` orthogonal to the traceability question rather than conflating a lifecycle label with a graph precondition.

## Consequences

`records/src/kind.rs` gains two new `Status::Draft` entries in `statuses()` and a two-arm change to `initial_status()`'s match; `records/src/validation.rs` gains two new transition arms. No changes are needed in `store`, `cli`, or the Markdown render/export/parse pipeline -- this is confined entirely to the `records` crate, consistent with rule 5's I/O boundary. Existing accepted ADR/EDR records in the self-hosted database are unaffected; this only changes the status a *newly created* ADR/EDR starts at and which transitions are legal going forward, not any already-recorded state. RFC-0004's Q1 is resolved by this EDR; Q2 through Q6 remain open for separate follow-on records, including the ADR that will need to decide whether a `Draft`-status record should be treated any differently by whatever traceability check that record defines.

## Evidence

`records/src/kind.rs:55-60` (`initial_status`) and `records/src/kind.rs:62-85` (`statuses`): the exact asymmetry this EDR closes. `records/src/validation.rs:112-122`: the `Adr`/`Edr` transition rules this EDR extends. `docs/specification.md` sections 5.3-5.4: confirms ADR/EDR never had an indicative draft stage even in the original design, so this is an extension, not a bugfix. RFC-0004 (`docs/records/rfc/0004-...md`), Q1: the open question this EDR answers. RFC-0001 and EDR-0006: this project's own precedent for retroactive documentation being normal, cited by RFC-0004 and reaffirmed directly in the conversation that produced this EDR as the reason a hard upstream-link requirement was rejected. `store/src/queries.rs`'s `search()` and `store/src/validate.rs`'s `validate()`: checked directly to confirm neither filters by status today, the basis for this EDR's claim that adding `Draft` to ADR/EDR carries no hidden behavioral risk.

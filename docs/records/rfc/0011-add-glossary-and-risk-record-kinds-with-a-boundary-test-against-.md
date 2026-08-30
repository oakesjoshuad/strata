---
id: "RFC-0011"
title: "Add Glossary and Risk record kinds, with a boundary test against kind sprawl"
record-type: rfc
status: draft
revision: 2
date: 2026-08-30
slug: add-glossary-and-risk-record-kinds-with-a-boundary-test-against-
tags: []
relationships:
  produces:
    - "PDR-0004"
  relates-to:
    - "EDR-0006"
    - "RFC-0006"
---

# RFC-0011: Add Glossary and Risk record kinds, with a boundary test against kind sprawl

## Motivation

Strata already tracks RFC/PDR/ADR/EDR for implementation reasoning, and RFC-0006 (proposed) adds Specification, Assessment, and Research for normative requirements, current-state evaluation, and external evidence-gathering. None of these seven kinds cleanly holds two things that recur across a real product build, especially the non-engineering half of architecting a software application: the shared vocabulary that specifications and decisions depend on staying consistent about, and a tracked, living risk that shapes decisions long before it either gets mitigated or materializes into an incident. Both recur often enough, and get referenced from enough otherwise-unrelated records, to justify their own kind rather than living as prose inside an RFC or a paragraph inside an Assessment.

At the same time, the project is explicitly not interested in becoming a general-purpose everything-tracker. Strata should stay opinionated about a small, deliberate set of kinds with a real workflow behind each one, not accrue a kind for every category of document a team might someday want to write down. This RFC states that boundary explicitly, applies it to justify Glossary and Risk, and applies it again to reject five other candidates considered in the same pass -- so the test exists before the next kind is proposed, not only in hindsight the way ADR-0016 had to formalize the ADR/EDR boundary after both already existed.

## Problem

Term drift: multiple specifications, RFCs, and ADRs define or assume a definition for a domain term ("Lead," "lifecycle status," "Owner") independently, and nothing today catches when two records silently disagree about what a term means, or when a term's definition changes and the records that depended on the old meaning go stale without anyone revisiting them. RFC-0006's own Proposal section already had to build a small ad hoc glossary in its "Ubiquitous language" subsection, because there was nowhere else in the current model to put shared vocabulary that many records need to agree on.

Risk has no home with the right lifecycle: RFC-0006's proposed Assessment captures risk as a field inside a dated, largely static evaluation, but a risk is not a snapshot -- it opens, gets monitored, gets mitigated or knowingly accepted, and sometimes materializes, over a timeline that spans many separate evaluations and decisions made along the way. Nothing today tracks one risk as a single, continuous thing across a decision's lifetime, so there is no way to ask "which accepted decisions still have an open risk against them" or "did this decision ship despite a known, unmitigated risk" without re-reading every Assessment's prose by hand.

Both problems are as much product and business reasoning as engineering reasoning: a domain term's meaning and a product risk matter well outside architecture, and the current model has no natural first-class home for either.

## Scope

This RFC defines two additional first-class record kinds, Glossary and Risk -- their purpose question, schema, lifecycle, and relationship to existing and RFC-0006-proposed kinds -- and states an explicit boundary test for admitting any further kind to Strata's closed RecordKind enum, so the project can stay opinionated about a small, deliberate set of kinds rather than drifting toward a general-purpose document store. It covers:

\- Glossary: one canonical term and definition per record, its lifecycle, and how it is referenced from other kinds.
\- Risk: a tracked item with its own open-to-closed lifecycle, its relationship to the decisions and specifications it threatens.
\- A general boundary test for adding a kind, informed by ADR-0016's positive-classification-test precedent, applied here to accept Glossary and Risk and to reject five other candidates considered in the same pass.

The first consumers are the same as RFC-0006's: Strata's own system documentation as the proving ground, then the Substruct CRM's own domain vocabulary and risk register.

## Non-Goals

This RFC does not define the full domain glossary for Substruct or Strata, and does not populate an actual risk register -- both are left to real use once the kinds exist. It does not change or re-litigate RFC-0006's Specification, Assessment, or Research proposal; Glossary and Risk are additive to that model, not a replacement for any part of it. It does not add a Persona/user-journey kind, a standalone Business/Product Decision kind, a Conflict/Reconciliation kind, a Postmortem/Incident kind, or a Vision/Charter kind -- each is considered and rejected in the Alternatives section below against this RFC's own boundary test. It does not make citing a Glossary term or attaching a Risk mandatory on any existing kind; both remain opt-in, attached where they add real traceability value, consistent with the existing graph's tolerance for standalone and retroactive records.

## Constraints

Glossary and Risk must fit the same status-transition, revision, and audit machinery every existing kind already uses -- no bespoke mutation path, matching rule 2's closed, exhaustively-matched RecordKind enum and rule 7's prohibition on silently swallowing a validation failure. A Glossary term's definition change goes through the ordinary revision history, never a silent overwrite, so a record that cited an earlier definition remains explicable after the term is redefined. A Risk's status transitions must be auditable the same way EDR-0015's status-undo audit trail already is for existing kinds, since a risk that was reported closed and is later reopened is exactly the kind of history a reader needs to trust. Both kinds must compose with the existing relationship vocabulary and evidence mechanism rather than duplicating what those already do well; new relations, if needed, extend the vocabulary rather than overload an existing relation's meaning (RFC-0006's own open Question 3 already flags this same concern for Specification's relations). Glossary reuses the existing FTS5 search index rather than introducing a second retrieval mechanism, consistent with PDR-0001's explicit stance that FTS5 is v0.1's actual retrieval plan and not a placeholder to be superseded speculatively. Glossary and Risk both render as one aggregated document per kind rather than one file per record -- an explicit, named deviation from EDR-0003's committed one-file-per-record exported-path convention, not a silent bend of it, and `export --check`'s byte-for-byte guarantee (ADR-0010) must hold against the aggregated file exactly as it does against every other kind's per-record file today.

## Proposal

**The boundary test (apply before adding any kind, including these two)**

A new record kind is justified only when it meets all four of:

1. Distinct lifecycle shape -- its natural status/revision pattern is not just "draft then accepted," and is not already adequately expressed by a field within an existing kind's document or a plain evidence entry.
2. Distinct reuse pattern -- many otherwise-unrelated records need to reference the same instance of it without duplicating its content, and the existing relationship vocabulary plus existing kinds cannot express that reuse cleanly.
3. A positive classification test against every existing kind -- not merely "this doesn't fit anywhere else," the same discipline ADR-0016 required for EDR rather than leaving it as ADR's undefined leftover category.
4. It answers one clear question no existing kind's stated purpose already answers, the same way each existing kind already has one ("Should this problem or proposal be pursued?" for RFC, "What architecturally significant choice was made and why?" for ADR, and so on).

A candidate that fails this test should be represented as a field on an existing kind, a plain evidence entry, or a relationship between existing records -- not a new enum variant. Both kinds below are proposed because they pass all four; five other candidates considered in the same review are rejected below because at least one criterion fails.

**Glossary**

Purpose question: what does this term mean, and where does that meaning apply?

A Glossary record holds one term: its canonical definition, representative examples, explicit non-examples where confusion is likely, an `aliases` list of synonyms and abbreviations the term is also known by, and the scope within which this meaning applies (the whole product, or one bounded area of it). Lifecycle: draft -> accepted -> superseded, so a term can be renamed or redefined over time while the record a decision actually cited when it was written remains legible in history, and the superseding record makes the new meaning discoverable going forward.

Reuse: any record -- an RFC's motivation, a Specification's requirement, an ADR's context -- links to the exact canonical definition it depends on via a new `uses-term` relation, rather than restating or silently assuming a meaning that can drift out from under it.

Search: Glossary is a record like any other, so it is included in the existing FTS5 index (EDR-0002's porter unicode61 tokenizer over title, body, and tags) automatically, with no new search subsystem. The `aliases` field is what does the real work for synonym and abbreviation lookup -- indexing "prospect" and "CRM" as literal alias tokens on the "Lead" and "Customer Relationship Management" entries makes them exact-match findable through the index that already exists, rather than reaching for edit-distance or phonetic fuzzy matching to paper over the same gap. This follows PDR-0001's own precedent directly: vector search and embeddings were deferred as "the actual plan unless a real retrieval failure shows FTS5 is insufficient," not a placeholder pending something fancier, and a bespoke fuzzy-heuristics layer for Glossary lookup specifically would be the same speculative complexity in a new place -- exactly what this RFC's boundary test exists to keep out.

Rendering: canonical storage and rendered projection are separate concerns here (ADR-0003), and they should not use the same granularity. Canonical storage stays one record per term -- that is what lets `uses-term` point at a stable, individually-revisable and individually-supersedable identity, the same reason RFC/PDR/ADR/EDR are each one record. But a glossary term is not a standalone reading unit the way an RFC or ADR is; it is a dictionary entry, and a person or agent looking things up wants one alphabetized document, not a directory of one-paragraph files. `strata export` therefore renders every Glossary-kind record into a single `docs/records/glossary.md`, sorted deterministically (alphabetically by term, record id as tiebreaker), each entry under its own heading with a stable anchor derived the same way EDR-0003 derives a file slug -- so a `uses-term` link can deep-link to `docs/records/glossary.md#lead`, and renaming a term does not move the anchor out from under an existing link, the same discipline ADR-0012 already applies to whole-file slugs, carried down to anchor granularity. `export --check` still byte-compares deterministically; only the cardinality changes, from one file per record to one file assembled from many records, so ADR-0010's guarantee is not weakened. `strata render GLOSSARY-NNNN` is unaffected and still renders just that one entry's fragment, since `render` already operates at record granularity regardless of kind. This is a real, explicit deviation from EDR-0003's committed exported-path convention ("the exported path is `{kind-slug}/{number}-{slug}.md`"), not a silent bend of it, and should get its own EDR once implemented, the same way EDR-0003 and EDR-0004 each separately settled a piece of the rendering story.

Positive test: a Specification states normative behavior a system must exhibit; a Glossary states shared meaning that specifications, decisions, and prose all draw on but is not itself a requirement. An Assessment or a Research record evaluates or investigates something; a Glossary entry does neither, it only defines. Failing to keep this distinction would turn Specification's own vocabulary sections into a second, competing source of term definitions -- exactly the drift this kind exists to prevent.

**Risk**

Purpose question: what could go wrong, how likely and how severe is it, and what is being done about it?

A Risk record holds: the subject and scope at risk, a description of what could go wrong, likelihood and impact, a category (technical, product, security, business, or compliance), the current mitigation and its owner, and status. Lifecycle: `open -> monitoring -> mitigated | accepted -> closed`, with a separate `materialized` terminal state distinct from `closed` so a reader can immediately tell "closed because it was avoided" from "closed because it happened."

Reuse: a decision or specification threatened by a risk links to it via a new `at-risk-from` relation (the risk's own mitigation work links back via `mitigates`); an Assessment can still surface a risk observation in its own prose, but the Risk record is what tracks that one risk continuously across every decision and evaluation that touches it, rather than each dated Assessment re-describing it independently.

Rendering: the same aggregation argument made for Glossary applies here and this RFC now takes the same position for Risk. Canonical storage stays one record per risk, for the same reason as Glossary -- `at-risk-from` and `mitigates` need a stable, individually-revisable identity to point at, and a risk's own status changes over time under its own history. But a risk register is conventionally one artifact: a single table of open items with likelihood, impact, owner, and status, not a folder of one-paragraph files each requiring its own open. `strata export` renders every Risk-kind record into a single `docs/records/risk-register.md`, sorted deterministically (by status, then severity, then id, with an explicit convention for how closed/materialized entries are retained or archived within the same document rather than silently dropped), each entry anchored the same way Glossary's entries are, for the same `at-risk-from`/`mitigates` deep-linking reason. Like Glossary, this is a deviation from EDR-0003's one-file-per-record convention and should get its own EDR alongside Glossary's once implemented, rather than two separate ad hoc exceptions.

Positive test: an Assessment is a dated, largely static evaluation of current state; a Risk is a tracked item with its own ongoing status that many different assessments and decisions reference over its lifetime without duplicating it into each one. A Risk is also not a decision -- accepting a risk is a judgment call an ADR or EDR can make, but the risk itself, and whether it later materializes, is tracked independently of that one decision's own status.

**Considered and rejected in this review, against the boundary test above**

\- Persona / user journey: fails criterion 2 (distinct reuse) more than it seems to -- a persona is best expressed as context inside a Specification's requirements ("an authorized user," tied to a named actor), not as an independently decided, independently reconciled record the way Glossary and Risk are.
\- Standalone Business/Product Decision: fails criterion 3 (positive test) -- a business rule like "the trial period is 14 days" is a Specification requirement (`REQ-BILLING-004`) with an acceptance criterion, not a new kind of decision; ADR/EDR's existing classification test is about architecture versus mechanism, and a business rule is neither.
\- Conflict/Reconciliation record: fails criterion 1 (distinct lifecycle) -- this is already served by the existing `supersedes` relation plus RFC-0010's draft `lint` command, which is specifically aimed at the free-text and status drift a reconciliation record would otherwise exist to catch.
\- Postmortem/Incident: passes the test in principle (distinct lifecycle, real reuse, a real positive test against Assessment) but is premature scope for a greenfield rebuild with no production incidents yet; worth revisiting once Substruct is live rather than speculatively building for it now.
\- Vision/Charter: fails criterion 2 (distinct reuse) -- single-instance per product, no meaningful cross-referencing pattern, and Substruct's own separate Strata database already has its own founding RFC playing this role; a whole enum variant for one document with no distinct revision cadence is a weak trade against every existing match site that would need to handle it.

## Alternatives Considered

Represent Glossary as a structured field inside RFC-0006's proposed Specification kind. Rejected: RFCs and ADRs need to cite the same term too, not only specifications, and once RFC-0006's specification hierarchy has multiple specifications, a specification-scoped glossary field cannot be the one canonical source a term needs.

Represent Risk purely as a field within Assessment. Rejected: this is the status quo RFC-0006 already proposes, and it loses continuity across a risk's lifetime -- each new Assessment would have to re-describe an already-known risk from scratch rather than updating one continuously tracked record, and nothing could cleanly link a specific decision to a specific risk across time.

Do nothing and keep writing terms and risks inline in prose within whichever record happens to be open at the time. This is the actual status quo, and it is the reason RFC-0006's own Proposal section had to build an ad hoc glossary of its own rather than pointing at one.

Add every candidate considered in this review (Persona, Business Decision, Conflict/Reconciliation, Postmortem, Vision/Charter) alongside Glossary and Risk, to be maximally expressive. Rejected as exactly the Swiss-army-knife failure mode this RFC's boundary test exists to prevent -- more kinds than a small team can keep straight is a cost even when each individual kind seems harmless in isolation.

## Open Questions

1. Exact relation names -- `uses-term` and `at-risk-from`/`mitigates` are working names in this RFC; should they fold into RFC-0006's own open relationship-vocabulary question (its Question 3) and be resolved together in one design record, or settled independently here?
2. Should a Glossary term support scoped meanings (a term meaning something narrower within one Specification subtree, per RFC-0006's hierarchy) or must a term have exactly one global meaning within a database?
3. Does Risk need a formal severity/priority scoring convention (e.g. a likelihood x impact matrix) for v1, or is free-text likelihood/impact sufficient until real use shows a need for scoring?
4. Should `strata validate` or the future `lint` command (RFC-0010) check for a Specification requirement or ADR that uses a term with no matching Glossary entry, or a decision whose linked Risk is still open -- and if so, is that an ERROR or an advisory finding, matching the ERROR/INFO split RFC-0010 already proposes?
5. Is this RFC's four-criterion boundary test itself worth its own ADR once Glossary and Risk have been used for real, the way ADR-0016 formalized the ADR/EDR test only after real classification friction showed exactly where the line needed to sit?
6. Should Glossary and Risk land as part of the same design record and migration as RFC-0006's Specification/Assessment/Research, given all five new kinds are proposed close together and a reader will likely want one coordinated picture of the resulting RecordKind enum rather than two separate migrations?
7. Is an `aliases` field on Glossary, indexed through the existing FTS5 table, actually sufficient for real term lookup, or will real use surface a retrieval failure specific enough to justify fuzzy or phonetic matching for this one kind -- the same bar PDR-0001 already set for reconsidering embeddings generally?
8. What exact sort order and anchor-slug algorithm does the aggregated Glossary/risk-register export use, and does it need its own EDR (an EDR-0003-style decision) rather than being folded silently into whichever design record implements this RFC?
9. Should a closed or materialized Risk be archived out of the live risk-register document once resolved, or does it stay listed (perhaps in a separate section) so the document remains a complete history rather than only currently-open items?

## Outcome

Draft. Proposes Glossary and Risk as two new first-class record kinds addressing term-drift and risk-tracking gaps that fall outside RFC-0006's implementation-traceability scope, and states an explicit four-part boundary test for admitting any kind to Strata's RecordKind enum -- applied here to accept Glossary and Risk, and in the same pass to reject Persona, standalone Business/Product Decision, Conflict/Reconciliation, Postmortem, and Vision/Charter as kinds. No kind is added to the compiler-enforced enum by this RFC alone; that follows in a subsequent design record, which Question 6 suggests should likely cover this RFC's two kinds together with RFC-0006's three as one coordinated migration rather than five separate ones. During review, both kinds' proposals were revised to render as one aggregated document per kind (docs/records/glossary.md, docs/records/risk-register.md) rather than one file per record -- canonical storage stays atomic per term/risk for linking, but the generated projection matches how a glossary or a risk register is conventionally read, an explicit deviation from EDR-0003's one-file-per-record convention that should get its own EDR once implemented.

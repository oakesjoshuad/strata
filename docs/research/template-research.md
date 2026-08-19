# Template research: RFC, PDR, ADR, EDR

This file records what real, published precedent exists for each record kind Strata
manages, and states plainly how Strata's own templates (in `docs/records/`) map to or
depart from that precedent. It is reference material, not a canonical record itself —
nothing here is a decision.

## RFC

**Precedent: the Rust RFC template.**
Source: `rust-lang/rfcs`, file `0000-template.md`.
<https://raw.githubusercontent.com/rust-lang/rfcs/master/0000-template.md>

The real template has these sections: Summary, Motivation, Guide-level explanation,
Reference-level explanation, Drawbacks, Rationale and alternatives, Prior art,
Unresolved questions, Future possibilities. It is written for a two-audience split —
"guide-level" explains the feature as if it already shipped, "reference-level" is the
technical detail — because Rust RFCs are read by both users and implementers of the
language. It also carries process metadata (RFC PR number, tracking issue) that only
makes sense inside GitHub's PR-review workflow.

**Precedent: Oxide's RFD (Request for Discussion).**
Source: RFD 1, "Requests for Discussion." <https://rfd.shared.oxide.computer/rfd/0001>
(the content repo itself is private; RFD 1 is published).

Oxide's RFD is deliberately not a decision record. It has six lifecycle states —
`prediscussion`, `ideation`, `discussion`, `published`, `committed`, `abandoned` — and
explicitly welcomes "considerably less than authoritative ideas": incomplete thinking,
unanswered questions, philosophical positions without full specifics. RFD 1 quotes the
original IETF RFC 3 philosophy that contributions should be "timely rather than
polished." Metadata is minimal: authors, state, labels, a link to the discussion PR.

**How Strata's RFC differs from both.**
Strata keeps the Rust RFC's section shape (motivation, scope, alternatives, open
questions) because it maps directly to the spec's stated RFC fields (motivation,
problem, scope, non-goals, constraints, proposal, alternatives, questions for review,
outcome — `docs/specification.md` section 5.1). It does not adopt Rust's two-audience
guide/reference split — that split exists because Rust RFCs serve both end users and
compiler implementers of a shipped language; Strata's RFCs serve one audience (the
person deciding whether to build something), so a single reference-level explanation is
enough. It leans toward Oxide's tone more than Rust's: Strata's RFC lifecycle
(`draft -> proposed -> under-review -> accepted \-> withdrawn`, spec section 5.1) is
closer to Oxide's discussion-first framing than to Rust's PR-and-merge process, since
Strata has no PR-review pipeline of its own. Strata does not adopt Oxide's six-state
lifecycle in full — `prediscussion`/`ideation` add ceremony that a single-person,
single-repo tool does not need; Strata folds both into `draft`.

## PDR

PDR is the least standardized of the four kinds. There is no single dominant
"Preliminary Design Record" template in software the way there is for RFC or ADR.
Two different traditions were checked, and neither maps cleanly:

**Systems-engineering precedent (NASA).**
Source: NASA Systems Engineering Handbook / NPR 7123.1, Preliminary Design Review
criteria. <https://nodis3.gsfc.nasa.gov/displayCA.cfm?Internal_ID=N_PR_7123_0001_&page_name=AppendixG>

NASA's PDR is a hardware/systems-engineering review gate, not a document template —
its purpose is to establish that a design meets requirements with acceptable risk, that
interfaces are identified, and that verification methods are described, before
committing to detailed design. This is a checklist for a review *meeting*, evaluated
against entrance/exit criteria, not a document a single author writes. The literal
checklist (margins, environmental qualification, hardware readiness) does not transfer
to software. What does transfer is the *shape* of the questions it asks: does the
design trace back to stated requirements, what alternatives were traded off, what is
the risk, and what will verification look like before you commit further effort.

**Software precedent (Google design docs).**
Source: "Design Docs at Google," Malte Ubl. <https://www.industrialempathy.com/posts/design-docs-at-google/>

This is the closer analog. Sections: Context and Scope, Goals and Non-Goals, The
Actual Design, Alternatives Considered, Cross-Cutting Concerns. Two details are worth
keeping: the explicit distinction between "non-goals" and "things the design merely
doesn't crash at" (a non-goal is something that could reasonably have been a goal but
was deliberately excluded), and the framing that a design doc's purpose is to surface
design problems while changing the design is still cheap — not to produce a permanent
specification.

**How Strata's PDR differs from both.**
Strata's PDR takes the NASA framing's *questions* (requirements traceability, design
maturity, risk, verification approach) and the Google framing's *sections* (explicit
goals/non-goals split, alternatives considered, cross-cutting concerns), and drops
NASA's review-meeting apparatus entirely — Strata has no review board, so a PDR is a
single authored document, matching Google's model, not a checklist against entrance/
exit criteria. Where the spec's own PDR field list (problem, requirements, constraints,
proposed design, components, interfaces, data model, failure modes, alternatives,
evidence, experiments, risks, open questions, resulting decisions — section 5.2) already
covers what both traditions ask for, so Strata's PDR template follows the spec's field
list directly rather than inventing a third shape.

## ADR

**Precedent: Michael Nygard's original ADR format (2011).**
Source: "Documenting Architecture Decisions," Cognitect blog.
<https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions>

Four sections: Context (the forces at play, written in value-neutral, factual language
— not an argument), Decision (stated in full sentences, active voice: "We will..."),
Status (proposed / accepted / deprecated / superseded, with a reference to whatever
superseded it), Consequences (all of them, not just the favorable ones). Nygard's
stated rationale is directly relevant to why Strata exists at all: without a recorded
decision, new team members either "blindly accept" a choice that may no longer apply or
"blindly change" it without understanding why it was made; large documents don't get
kept up to date and don't get read, so records need to be short and single-purpose.

**Precedent: MADR (Markdown Architectural Decision Records).**
Source: `adr/madr`, current template. <https://raw.githubusercontent.com/adr/madr/main/template/adr-template.md>
and the minimal variant: <https://raw.githubusercontent.com/adr/madr/main/template/adr-template-minimal.md>

MADR extends Nygard's shape with: Context and Problem Statement, Decision Drivers,
Considered Options, Decision Outcome (with a "Confirmation" subsection — how compliance
will be verified, e.g. an automated fitness function or code review), and an optional
per-option Pros and Cons breakdown. MADR's minimal variant strips this down to Context
and Problem Statement, Considered Options, Decision Outcome, Consequences — closer to
Nygard's original four sections than the full template is.

**How Strata's ADR differs from both.**
Strata's ADR uses Nygard's four-section core (Context, Decision, Status, Consequences)
because it is the smallest shape that satisfies the spec's stated ADR fields (context,
decision, alternatives, consequences, evidence — section 5.3) and Strata's own founding
motivation is Nygard's exact argument: undocumented decisions get blindly kept or
blindly broken. It adds MADR's "Considered Options" and evidence citation as required
fields rather than optional ones, because Strata has a first-class `evidence` table
(spec section 8.4) an ADR can reference directly — evidence is not prose, it is a
relationship. Strata does not adopt MADR's "Confirmation" subsection as a separate
field; `eng validate` (the CLI's own validation command, spec section 29) plays that
role structurally instead of being restated as prose in every record.

The one requirement neither Nygard nor MADR enforces, and that Strata treats as
non-negotiable, is **one decision per record**. Nothing in either precedent stops an
author from writing an ADR whose Decision section covers three separable choices —
and the substrate repository's own ADR corpus shows what that costs in practice: a
2026-08-18 audit of its 102 ADRs found 29 that bundle more than one independently
accept/reject/supersede-able decision into a single record (see
`docs/book/src/working/2026-08-18-adr-portability-audit.md` in the substrate repo). The
worst single case bundles nine decisions in one document. Strata's own ADRs (in
`docs/records/adr/`) are written to avoid this deliberately, and `eng validate` should
eventually check for it structurally rather than relying on authors to remember, per
the same non-negotiable this research file exists to support.

## EDR

There is no meaningful external precedent for "Engineering Decision Record" as a named
industry format. It is not a term with a shared canonical shape the way ADR or RFC
are. Some organizations use "EDR" informally for implementation-level or engineering
notebook-style entries, but no published template was found worth citing here, and
inventing a citation would misrepresent what was actually researched.

Strata's EDR template is therefore the ADR shape (Context, Decision, Status,
Consequences) narrowed to implementation-level scope, exactly as the specification
already proposes (section 5.4: algorithms, persistence layouts, serialization,
concurrency, libraries, operational parameters, implementation strategies). The
distinction from an ADR is scope and blast radius, not document structure — an EDR
answers "what implementation-level choice was made," an ADR answers "what
architecturally significant choice was made." Both get the same one-decision-per-record
discipline described above.

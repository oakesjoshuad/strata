---
id: "RFC-0006"
title: "Define hierarchical specifications and traceable implementation composition"
record-type: rfc
status: draft
revision: 2
date: 2026-08-20
slug: define-hierarchical-specifications-and-traceable-implementation-
tags: []
relationships: {}
---

# RFC-0006: Define hierarchical specifications and traceable implementation composition

## Motivation

Strata is being prepared to guide a greenfield implementation of the Substruct
CRM after roughly eight months of iteration, evaluation, and lessons learned.
The existing RFC, PDR, ADR, and EDR records preserve important reasoning, but
they do not yet provide one implementation-facing contract that a frontend or
backend engineer can follow from requirements through code and verification.

The project needs to capture both what the application must do and why the
implementation is shaped the way it is. It also needs to record current
evaluations of the codebase and product: observed strengths, gaps, risks, and
lessons learned. Those assessments are neither decisions nor requirements, but
they are often the evidence that causes a requirement or design to change.

## Problem

The current four record kinds answer different questions but leave an important
composition gap:

\- RFCs propose problems or initiatives.
\- PDRs describe proposed designs.
\- ADRs record architectural choices.
\- EDRs record implementation-level choices.

None is a normative application specification with stable requirement IDs,
acceptance criteria, hierarchical scope, and conformance evidence. Treating a
specification as a concatenation of ADRs and EDRs would be insufficient: a
decision explains a chosen constraint or mechanism, but it does not necessarily
state the complete externally observable behavior that must be implemented.

The lifecycle also currently makes no distinction between an accepted decision
and an implemented decision. An ADR or EDR can be accepted as guidance before
implementation begins. Conversely, a specification can be approved and already
be partially implemented. Treating acceptance and implementation as one status
would make the record history ambiguous and would encourage premature status
changes.

Finally, PDR is not universally understood as the name of a design document;
in other engineering contexts it can mean Preliminary Design Review. Strata's
internal vocabulary should make the distinction between specification and design
explicit without forcing an immediate destructive rename of the existing PDR
corpus.

## Scope

This RFC defines a documentation and traceability model for composing
implementation work from requirements, designs, decisions, code, tests, and
assessments. It covers:

\- a normative Specification artifact with stable requirement identifiers;
\- hierarchical specifications and parent/child decomposition;
\- Design Records as the design-document role currently represented by PDR;
\- Assessments that capture current observations, evidence, risks, and gaps;
\- orthogonal lifecycle and implementation/conformance state;
\- links between specifications, PDRs, ADRs, EDRs, code references, tests, and
  assessments; and
\- generation of implementation-oriented publication packages from that graph.

The first consumer is the Substruct CRM specification. Strata's own broad
specification is the first template and proving ground. The model remains
general enough for other repositories to define their own product or subsystem
specifications.

## Non-Goals

This RFC does not define the complete Substruct product requirements, choose
Substruct architecture, or create its ADR and EDR corpus. It does not make
assessments a substitute for tests, production telemetry, or code review. It
does not require every standalone ADR or EDR in every repository to belong to a
specification. It does not require immediately renaming the serialized PDR kind,
rewriting existing record IDs, or adding a new record kind before the model is
validated in use.

It also does not make generated Markdown the source of truth. Requirements,
decision links, lifecycle state, and verification evidence remain canonical
structured data; Markdown and publication packages remain projections.

## Constraints

The existing RFC/PDR/ADR/EDR corpus and its revision history must remain
readable. Accepted ADRs and EDRs remain historical decision records and are not
silently rewritten to claim implementation completion. The current graph remains
flexible for standalone and retroactive records, while specification-scoped
validation may enforce stronger traceability for a particular implementation
package.

Specification requirements must be precise enough to verify. Where practical,
they should describe observable behavior, interfaces, constraints, quality
attributes, and acceptance criteria rather than prescribe an implementation
detail. Design Records allocate requirements to components and interfaces.
ADRs and EDRs may constrain or refine requirements, but cannot silently replace
the requirement text.

The model must remain compatible with Strata's local-first SQLite architecture,
CLI-only mutation boundary, deterministic projections, closed-kind compiler
checks, and explicit-error policy. It must not require async execution, a server,
a multi-user coordination service, or a speculative repository abstraction.

Markdown syntax and YAML frontmatter parsing must be delegated to a maintained external Rust package with CommonMark/GFM support, an AST or event model, and source-position information. Strata must not silently flatten or discard valid Markdown content while adapting the external representation to its document model.

## Proposal

**Ubiquitous language**

Strata will distinguish these artifact roles:

\- **Assessment:** What do we currently observe about the product, codebase, or
  implementation? An assessment records scope, observations, evidence, risks,
  confidence, recommendations, and date. It is time-bound and revisable.
\- **Specification:** What must the system do, what constraints apply, and how
  will conformance be verified? A specification is normative for its scope.
\- **Design Record:** How should the system be structured to satisfy the
  specification? This is the design-document role currently represented by PDR.
\- **ADR:** Which architectural choice was made, and why?
\- **EDR:** Which implementation-level engineering choice was made, and why?
\- **Implementation evidence:** Which code references, tests, and assessments
  demonstrate that a requirement or decision is being met?

The distinction is semantic, not merely presentational. A specification is not a
verbatim bundle of decisions, and an assessment is not a decision merely because
it recommends work.

**Specifications**

A Specification is a versioned, normative document with stable requirement IDs.
Each requirement may include:

\- a statement of required behavior or quality;
\- rationale and scope;
\- acceptance criteria;
\- dependencies, assumptions, and constraints;
\- priority or release target; and
\- links to supporting decisions and verification evidence.

An example requirement is:

```text
REQ-LEADS-001

The system shall allow an authorized user to create a lead with a name,
source, owner, and lifecycle status.

Acceptance criteria:
\- invalid submissions return field-level errors;
\- unauthorized users cannot create leads;
\- successful creation appears in the lead list; and
\- persistence and retrieval are covered by integration tests.
```

The requirement remains the normative statement. ADRs explain architectural
constraints, EDRs refine implementation mechanisms, and code/tests provide
evidence. The specification may link to all of them without becoming a prose
concatenation of their documents.

**Hierarchy**

Specifications may decompose into child specifications:

```text
SUBSTRUCT-SPEC
├── CRM
│   ├── Leads
│   ├── Contacts
│   └── Opportunities
├── Identity and authorization
├── Notifications
└── Platform and operations
```

A child owns a narrower scope and inherits applicable parent constraints. The
hierarchy must be acyclic. A child must have one parent unless it is a root
specification, and requirement IDs must be unique within the specification tree.
A child may refine a parent requirement but may not silently broaden or contradict
the parent scope. A parent cannot be considered fully implemented while required
child specifications remain incomplete.

The first broad specification should be Strata's own system specification,
decomposed into storage, record semantics, CLI, configuration, rendering,
validation, publication, and agent-integration specifications. The resulting
template should then be used to define the Substruct CRM specification and its
frontend, domain, and platform children.

**Decisions and implementation state**

ADR and EDR lifecycle continues to describe decision authority:

```text
ADR: draft → proposed → accepted → deprecated/superseded
EDR: draft → proposed → accepted → superseded
```

Accepted means that the choice governs future work. It does not mean that the
code already conforms to the choice. A decision may be accepted before work
starts, and its implementation state must remain separately observable through
links and evidence.

Specifications have a separate normative lifecycle:

```text
Specification: draft → review → approved → superseded
```

Approved means that the specification is the current implementation contract.
It does not mean that all requirements are complete. Requirement-level progress
and conformance should be represented independently, for example:

```text
Requirement: planned → in-progress → verified
                         └──────→ blocked/waived
```

An implementation assessment may conclude that a specification is conforming,
partially conforming, or divergent. The system should not add an `implemented`
status to ADRs or EDRs merely to represent this conclusion.

**Design Records and PDR terminology**

PDR remains the serialized kind during the first iteration to avoid a disruptive
rename of existing records. Its human-facing purpose and template should be
described as a Design Record or Design Document: a proposed allocation of
requirements to components, interfaces, data, workflows, and failure behavior.

The implementation RFC that follows this one should decide whether a later
migration to a more explicit serialized kind is worthwhile. A name change must
preserve record identity, relationships, history, generated paths, and external
references if it is ever performed.

**Assessments**

Assessments are current-state evaluations, not decision records. They should be
able to report:

\- the subject and scope evaluated;
\- observations and evidence;
\- risks, gaps, and confidence;
\- recommendations or candidate requirements; and
\- the date, revision, and context of the evaluation.

An assessment may motivate a new RFC, revise a draft specification, or reveal
non-conformance to an approved specification. It must not silently change a
requirement or decision. Whether Assessment becomes a fifth first-class record
kind should be decided after a real Strata and Substruct workflow exercises the
concept; the initial model may represent assessments as a structured artifact
linked to the records they evaluate.

**Traceability**

The intended composition is:

```text
Specification
  ├── contains requirements
  ├── constrained-by ADRs
  ├── refined-by EDRs
  ├── designed-by Design Records
  ├── implemented-by code references
  └── verified-by tests and assessments
```

Specification-scoped validation should be able to find requirements without
supporting design or decisions, accepted decisions not reflected in the current
specification, requirements without implementation evidence, stale references to
superseded decisions, conflicting constraints, and incomplete child specifications.

These checks must be scoped to a specification package. Strata must not impose a
global rule that every standalone ADR or EDR requires an RFC, PDR, or specification
parent; the existing graph intentionally permits standalone and retroactive
records.

**Workflow**

The intended greenfield workflow is:

1. Capture the eight months of Substruct lessons as assessments and RFCs.
2. Establish the broad Strata specification as the reference template.
3. Decompose the Substruct application specification into bounded child specs.
4. Link accepted ADRs as architectural constraints and accepted EDRs as
   implementation refinements.
5. Create Design Records where the specification leaves component allocation or
   interfaces unresolved.
6. Implement requirement by requirement, attaching code and test evidence.
7. Run assessments that report current conformance, gaps, and risks.
8. Publish an implementation package with requirements first and supporting
   decisions, evidence, and assessments expandable beneath them.

**Markdown parsing and frontmatter**

Strata shall use a standards-compliant external Rust Markdown parser for both YAML frontmatter and document bodies rather than extending a hand-rolled Markdown parser. The parser owns frontmatter delimiters, YAML-frontmatter extraction, CommonMark syntax, the selected GFM extensions, source positions, and parser diagnostics. Strata owns the domain schema applied to the parsed frontmatter and record structure: identity, record kind, status, revision, date, slug, tags, relationship names, required fields, and semantic validation.

The first dialect is CommonMark plus the selected GFM features, with YAML frontmatter enabled and unrelated extensions disabled unless a later decision justifies them. A parsed frontmatter node is deserialized into a strict Strata schema with unknown fields rejected, then converted into validated domain values. Top-level level-two headings continue to define the record schema, while deeper headings and other Markdown blocks remain content within their enclosing field and must round-trip without loss.

## Alternatives Considered

Continue using only RFC/PDR/ADR/EDR records and write a hand-maintained
implementation plan. This keeps the current model small but leaves requirements,
acceptance criteria, hierarchy, and conformance outside the canonical system.

Treat PDRs as specifications. This avoids a new artifact, but conflates proposed
design with normative requirements and makes it difficult to describe requirements
that intentionally leave the design open.

Treat ADRs as the specification. This fails because ADRs record choices rather
than complete product behavior, user workflows, acceptance criteria, and quality
requirements.

Treat assessments as status-bearing ADRs. This corrupts the distinction between
what is observed and what has been decided, and makes an accepted historical
decision appear to change whenever the implementation is evaluated.

Add Specification and Assessment kinds immediately and redesign the entire
schema at once. This provides the most explicit model but introduces migration
and lifecycle complexity before the workflow has been exercised. The proposal
therefore makes Specification the target first-class concept while deferring a
first-class Assessment kind until real use justifies it.

Rename PDR immediately to Design Document. This may improve terminology but risks
breaking existing IDs, paths, relationships, history, and external references.
The proposal retains PDR as the serialized kind initially and makes its human
meaning explicit.

## Open Questions

1. Should Specification be a fifth record kind, or should the first implementation
   use a structured specification package composed from existing records?
2. Should Assessment become a first-class record kind, a structured artifact, or
   an external evaluation projection linked to records?
3. What exact relationship vocabulary represents containment, refinement,
   constraint, implementation, and verification without overloading existing
   relation meanings?
4. Should requirement-level progress be stored in canonical SQLite rows, in the
   specification document, or derived from linked implementation evidence?
5. What is the minimum conformance evidence required before a requirement is
   marked verified?
6. Should PDR remain the serialized kind permanently while its human label is
   Design Record, or should Strata provide a versioned migration to a new kind?
7. Which specification-scoped validation checks belong in `strata validate`, and
   which belong in a future specification/conformance command?
8. How should specification packages select and publish child specifications,
   decisions, code references, tests, and assessments together?
9. Which maintained Rust Markdown package best satisfies the required CommonMark/GFM, YAML frontmatter, AST or event, source-position, and serialization needs, and which exact dialect options should Strata enable?

## Outcome

Draft. This RFC proposes that Strata evolve from a decision-record archive into a
traceable implementation-composition system. It separates normative
specifications, explanatory design documents, historical decisions, current-state
assessments, and implementation evidence while preserving the existing ADR and
EDR meaning. The next design record should define the canonical data model,
relationship vocabulary, lifecycle additions, migration strategy, and CLI/query
surfaces. No existing decision is accepted or implemented by this RFC; it defines
the model that will allow those states to be represented accurately.

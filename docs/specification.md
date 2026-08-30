---
date: 2026-08-18
status: Draft Specification
subtitle: Specification for a Rust CLI, SQLite Knowledge Base, LLM Skill
  Interface, and Pandoc Publication Pipeline
title: Engineering Knowledge Record System
---

# 1. Purpose

This specification defines a local-first engineering knowledge system
for managing software-development reasoning and decisions across RFCs,
PDRs, ADRs, and EDRs.

The system replaces a document-first ADR manager with a structured
knowledge base:

-   A Rust CLI binary is the authoritative application interface.
-   SQLite is the canonical persistent store.
-   JSON is used for flexible structured record content and
    machine-oriented CLI interchange.
-   FTS5 provides embedded full-text retrieval.
-   Explicit relational tables represent record relationships,
    revisions, evidence, tags, and code references.
-   LLM skills teach coding agents how to interact with the CLI rather
    than granting agents direct database access.
-   Markdown is a deterministic projection and portability format, not
    the canonical source of truth.
-   Pandoc is the publication backend for HTML, PDF, DOCX, EPUB, and
    other output formats.
-   MCP is an optional future adapter over the same application
    services, not a required architectural component.

The design goal is a durable, inspectable, provider-independent
engineering knowledge system that remains useful to humans, shell
scripts, CI, and LLM coding agents.

# 2. Problem Statement

Conventional ADR tools primarily manage Markdown documents. The desired
workflow is broader:

``` text
Problem / Idea
      |
      v
RFC
Request for Comments
      |
      +---- research / alternatives / experiments
      |
      v
PDR
Preliminary Design Record
      |
      +--------------+
      |              |
      v              v
ADR               EDR
Architecture      Engineering
Decision          Decision
      |              |
      +------+-------+
             |
             v
       Implementation
```

This must not be a rigid pipeline. Records form a graph. A small
implementation decision may require only an EDR. An RFC may lead
directly to an ADR. A PDR may result in multiple ADRs and EDRs.

The system therefore needs to model engineering knowledge rather than
merely generate documents.

# 3. Design Principles

## 3.1 Canonical knowledge, projected documents

SQLite SHALL be the source of truth.

Markdown, HTML, PDF, DOCX, and other publication artifacts SHALL be
projections of canonical state.

``` text
SQLite
  |
  +--> JSON CLI responses
  +--> FTS5 retrieval
  +--> Markdown
  +--> Pandoc publication
  +--> Graph projections
```

## 3.2 Domain behavior belongs in Rust

Record lifecycle rules, identifiers, relationship validation, revision
semantics, and transactional invariants SHALL be enforced by the Rust
application layer.

Neither JSON Schema, XML Schema, Markdown conventions, LLM instructions,
nor Pandoc filters SHALL be relied upon as the authoritative domain
validator.

## 3.3 LLMs express intent; the application enforces truth

LLMs SHALL interact through validated CLI operations.

Agents SHALL NOT receive unrestricted SQLite write access.

``` text
LLM
 |
 | semantic operation
 v
Rust CLI / Application Service
 |
 +-- validate schema
 +-- validate lifecycle
 +-- validate relationships
 +-- allocate identifiers
 +-- create revision
 +-- update FTS
 +-- commit transaction
 |
 v
SQLite
```

## 3.4 One implementation, multiple adapters

The CLI SHALL call the same application services that any future MCP
adapter would call.

MCP SHALL NOT contain independent domain logic.

# 4. Record Model

The canonical abstraction is an Engineering Knowledge Record.

Initial record kinds:

-   RFC - Request for Comments
-   PDR - Preliminary Design Record
-   ADR - Architecture Decision Record
-   EDR - Engineering Decision Record

Conceptually:

``` rust
enum RecordKind {
    Rfc,
    Pdr,
    Adr,
    Edr,
}
```

A record contains at minimum:

``` text
Record
|- RecordId
|- RecordKind
|- title
|- status
|- current structured document
|- revision
|- timestamps
|- tags
|- relationships
|- evidence references
`- code references
```

Record kinds SHOULD share infrastructure while using kind-specific
document schemas and lifecycle policies.

# 5. Record Semantics

## 5.1 RFC

An RFC answers: Should this problem or proposal be pursued?

Typical fields:

-   motivation
-   problem
-   scope
-   non-goals
-   constraints
-   proposal
-   alternatives
-   questions for review
-   outcome

Indicative lifecycle:

``` text
draft -> proposed -> under-review -> accepted
                               \-> withdrawn
```

## 5.2 PDR

A PDR answers: What does the proposed design look like and what evidence
supports it?

Typical fields:

-   problem
-   requirements
-   constraints
-   proposed design
-   components
-   interfaces
-   data model
-   failure modes
-   alternatives
-   evidence
-   experiments
-   risks
-   open questions
-   resulting decisions

Indicative lifecycle:

``` text
draft -> review -> approved -> superseded
```

## 5.3 ADR

An ADR answers: What architecturally significant choice was made and
why?

Typical fields:

-   context
-   decision
-   alternatives
-   consequences
-   evidence

Indicative lifecycle:

``` text
proposed -> accepted -> deprecated -> superseded
```

## 5.4 EDR

An EDR answers: What implementation-level engineering choice was made?

Typical subjects include:

-   algorithms
-   persistence layouts
-   serialization
-   concurrency
-   libraries
-   operational parameters
-   implementation strategies

Indicative lifecycle:

``` text
proposed -> accepted -> superseded
```

# 6. Identifiers

Identifiers SHALL include the record kind and a monotonically allocated
number within that namespace.

Examples:

``` text
RFC-0007
PDR-0012
ADR-0029
EDR-0017
```

The database SHALL enforce uniqueness of `(kind, number)`.

Identifier allocation SHALL occur transactionally in the application
layer.

# 7. Relationships

Relationships SHALL be explicit graph edges rather than conventions
embedded only in prose.

Examples:

``` text
RFC-0007 --explored-by--> PDR-0012
PDR-0012 --produces-----> ADR-0029
PDR-0012 --produces-----> EDR-0017
ADR-0029 --constrains---> EDR-0017
ADR-0044 --supersedes---> ADR-0029
```

Initial relationship vocabulary SHOULD include:

-   relates-to
-   derived-from
-   explored-by
-   produces
-   resolves
-   constrains
-   implements
-   implemented-by
-   supported-by
-   supersedes

Relationship policies MAY restrict valid source and target kinds.

The graph SHALL remain flexible; the system SHALL NOT require every RFC
to have a PDR or every PDR to result in both ADR and EDR records.

# 8. SQLite Persistence

## 8.1 Core record table

A representative schema:

``` sql
CREATE TABLE engineering_record (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL,
    number      INTEGER NOT NULL,
    title       TEXT NOT NULL,
    status      TEXT NOT NULL,
    document    TEXT NOT NULL CHECK(json_valid(document)),
    revision    INTEGER NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE(kind, number)
);
```

The exact physical schema MAY evolve, but commonly queried identity and
lifecycle properties SHOULD remain relational columns.

Type-specific and evolving content MAY reside in the JSON document.

## 8.2 Relationships

``` sql
CREATE TABLE record_relation (
    source_id   TEXT NOT NULL,
    relation    TEXT NOT NULL,
    target_id   TEXT NOT NULL,

    PRIMARY KEY(source_id, relation, target_id),

    FOREIGN KEY(source_id)
        REFERENCES engineering_record(id),

    FOREIGN KEY(target_id)
        REFERENCES engineering_record(id)
);
```

## 8.3 Revisions

Every meaningful record mutation SHALL preserve prior document state.

Representative schema:

``` sql
CREATE TABLE record_revision (
    record_id       TEXT NOT NULL,
    revision        INTEGER NOT NULL,
    document        TEXT NOT NULL CHECK(json_valid(document)),
    changed_at      TEXT NOT NULL,
    changed_by      TEXT,
    change_summary  TEXT,

    PRIMARY KEY(record_id, revision)
);
```

Revision history and decision lifecycle are separate concepts.

## 8.4 Evidence

Evidence SHOULD be first-class.

Representative evidence kinds:

-   benchmark
-   experiment
-   source
-   code-reference
-   issue
-   measurement
-   prototype

Representative schema:

``` sql
CREATE TABLE evidence (
    id          TEXT PRIMARY KEY,
    record_id   TEXT NOT NULL,
    kind        TEXT NOT NULL,
    title       TEXT NOT NULL,
    uri         TEXT,
    content     TEXT,
    metadata    TEXT CHECK(metadata IS NULL OR json_valid(metadata))
);
```

## 8.5 Code references

Engineering decisions SHOULD be traceable to implementation.

Representative schema:

``` sql
CREATE TABLE code_reference (
    record_id   TEXT NOT NULL,
    relation    TEXT NOT NULL,
    path        TEXT NOT NULL,
    symbol      TEXT,
    line_start  INTEGER,
    line_end    INTEGER
);
```

This enables questions such as:

``` text
Which accepted decisions constrain src/storage/writer.rs?
```

# 9. Full-Text Search

FTS5 SHALL provide the initial semantic-retrieval substrate.

Representative index:

``` sql
CREATE VIRTUAL TABLE engineering_record_fts
USING fts5(
    record_id UNINDEXED,
    title,
    body,
    tags,
    tokenize='porter unicode61'
);
```

The searchable body SHALL be deterministically derived from the current
structured record document.

Initial retrieval SHOULD combine:

-   FTS5
-   record kind
-   status
-   tags
-   explicit graph relationships
-   supersession state
-   evidence
-   code references

Vector embeddings SHALL NOT be required initially.

Embeddings MAY be added only if measured retrieval failures justify
their complexity.

# 10. JSON

JSON SHALL serve two distinct purposes.

## 10.1 Flexible canonical record content

A PDR document may resemble:

``` json
{
  "schema": "pdr/v1",
  "problem": "...",
  "requirements": [],
  "constraints": [],
  "design": {
    "summary": "...",
    "components": []
  },
  "alternatives": [],
  "failure_modes": [],
  "risks": [],
  "open_questions": []
}
```

## 10.2 Machine-oriented CLI protocol

All important CLI read and mutation commands SHOULD support structured
JSON output.

Examples:

``` text
eng show ADR-0029 --json
eng search "single writer" --json
eng graph PDR-0012 --json
eng validate --json
eng schema pdr --json
```

JSON is preferred over XML for this interface because it maps naturally
to Rust data structures, SQLite JSON facilities, shell tooling, and
current LLM structured-output ecosystems.

# 11. XML

XML SHALL NOT be the canonical persistence format.

XML Schema is capable of sophisticated document validation, but domain
invariants belong in Rust and the required document structures can be
represented adequately with typed Rust structures and/or JSON Schema.

XML-like markup MAY be useful as an LLM context envelope where strong
record boundaries improve comprehension.

For example:

``` xml
<engineering-context>
  <record id="PDR-0012" kind="pdr" status="approved">
    ...Pandoc Markdown...
  </record>

  <record id="ADR-0029" kind="adr" status="accepted">
    ...Pandoc Markdown...
  </record>
</engineering-context>
```

This use is a presentation protocol, not canonical storage.

# 12. Rust Application Architecture

The implementation SHOULD separate domain/application behavior from
adapters.

``` text
                    Application/Core
                          |
             +------------+-------------+
             |                          |
             v                          v
          CLI Adapter              Future MCP
             |                          |
             +------------+-------------+
                          |
                       SQLite
```

Potential domain interfaces:

``` rust
pub trait EngineeringRepository {
    fn search(&self, query: SearchQuery) -> Result<Vec<SearchHit>>;
    fn get(&self, id: RecordId) -> Result<Record>;
    fn create(&self, command: CreateRecord) -> Result<Record>;
    fn revise(&self, command: ReviseRecord) -> Result<RecordRevision>;
    fn link(&self, command: LinkRecords) -> Result<()>;
}
```

The exact Rust API is non-normative at this stage.

# 13. CLI

The CLI is the primary human, automation, CI, and LLM execution
interface.

Indicative commands:

``` text
eng init

eng new rfc "Canonical persistence model"
eng new pdr "Canonical persistence architecture"
eng new adr "Use SQLite as canonical persistence"
eng new edr "Serialize writes through one writer"

eng show PDR-0012
eng search "single writer"
eng graph PDR-0012
eng history PDR-0012

eng link PDR-0012 produces ADR-0029
eng status PDR-0012 approved

eng validate
eng export
```

The final command grammar SHALL be designed deliberately rather than
copied literally from these examples.

# 14. CLI Self-Description

The binary SHOULD be self-describing so an LLM does not need a
permanently duplicated command specification.

At minimum:

``` text
eng --help
eng <command> --help
eng schema <kind> --json
eng capabilities --json
eng relationships --json
```

`eng capabilities --json` SHOULD describe supported operations and
relevant schemas sufficiently for an agent to adapt to an unfamiliar
binary version.

# 15. LLM Skill

An LLM skill SHALL describe workflow and policy, not reproduce the
entire application implementation.

The skill SHOULD instruct agents to:

1.  Search before creating new records.
2.  Retrieve relevant current and superseded records.
3.  Follow relationships where relevant.
4.  Select the record kind whose semantic purpose fits the work: RFC, PDR, ADR,
    EDR, Specification, Assessment, Research, Glossary, or Risk.
5.  Consult `strata schema` or command help rather than inventing syntax.
6.  Use JSON output for structured machine interaction.
7.  Use validated CLI mutations rather than editing SQLite directly.
8.  Run validation after mutations.
9.  Regenerate or check Markdown projections when required.
10. Record evidence and code references where they materially support a
    decision.

# 16. Context Retrieval

The CLI SHOULD eventually provide an agent-oriented retrieval command.

Example:

``` text
eng context "changing the persistence writer" --json
```

The context operation SHOULD combine relevant:

-   FTS hits
-   graph neighbors
-   supersession chains
-   tags
-   evidence
-   code references

and return a bounded context package suitable for LLM reasoning.

An optional human/LLM-readable representation MAY use an XML envelope
containing Markdown record bodies.

# 17. MCP

MCP is explicitly optional.

An MCP adapter MAY later expose operations such as:

``` text
search_records
get_record
create_record
revise_record
link_records
get_context
validate_records
```

MCP adds:

-   standardized tool discovery
-   typed invocation
-   native integration with MCP-capable hosts
-   structured results and errors
-   optional long-lived process behavior

MCP SHALL NOT add or duplicate:

-   record schemas
-   lifecycle rules
-   identifier allocation
-   revision semantics
-   FTS behavior
-   graph semantics
-   Markdown rendering
-   transaction logic

Those remain application-core responsibilities.

# 18. Document Rendering

The canonical record is not a Markdown document.

A publication layer SHALL transform engineering records into a
document-oriented intermediate representation.

Conceptually:

``` text
Engineering Record
       |
       v
Engineering Document IR
       |
       +--> Pandoc Markdown
       +--> JSON
       `--> future Pandoc JSON AST
```

An internal Rust document IR SHOULD be considered so domain logic is not
coupled directly to Markdown string concatenation.

Potential blocks include:

``` text
Heading
Paragraph
List
CodeBlock
Table
Diagram
RecordReference
EvidenceReference
```

# 19. Pandoc Markdown Projection

The primary portable document projection SHOULD be Pandoc-compatible
Markdown with YAML metadata.

Example:

``` markdown
---
title: "Use SQLite as Canonical Persistence"
identifier: ADR-0029
record-type: adr
status: accepted
revision: 4
date: 2026-08-18
tags:
  - persistence
  - sqlite

relationships:
  derived-from:
    - PDR-0012
  constrains:
    - EDR-0017
---

# Context

...

# Decision

...

# Consequences

...
```

The renderer SHALL be deterministic.

A record rendered twice from identical canonical state SHALL produce
equivalent Markdown.

# 20. Render vs. Publish

The system SHOULD distinguish record rendering from publication
assembly.

`render` projects one record:

``` text
eng render ADR-0029
```

`publish` constructs a document or site from a graph/query:

``` text
eng publish --kind adr
eng publish --root PDR-0012
eng publish --tag persistence
eng publish --status accepted
```

This distinction permits publication structure to be derived from the
knowledge graph instead of from a fixed book hierarchy.

# 21. Publication Manifests

A later version MAY support first-class publication definitions.

Example:

``` json
{
  "id": "PUB-0003",
  "title": "Persistence Architecture",
  "query": {
    "root": "PDR-0012",
    "relations": [
      "derived-from",
      "produces",
      "constrains",
      "supported-by"
    ],
    "depth": 3
  },
  "output": {
    "format": "pandoc-markdown",
    "toc": true
  }
}
```

This allows deterministic design packages, decision compendia, client
deliverables, and other views over the same canonical knowledge graph.

# 22. Pandoc Publication Pipeline

Pandoc SHALL be treated as a document compiler and publication backend,
not as the owner of engineering semantics.

``` text
SQLite knowledge model
        |
        v
      eng
        |
        v
Engineering Document IR
        |
        v
Pandoc Markdown
        |
        v
      Pandoc
   +----+----+-----+
   |    |    |     |
 HTML  PDF DOCX  EPUB
```

Pandoc readers parse source into the Pandoc AST. Writers transform the
AST into target formats.

This permits the engineering system to remain independent of output
format.

# 23. Pandoc Metadata

Generated Markdown SHOULD use YAML metadata for publication-oriented
properties such as:

-   title
-   identifier
-   record type
-   status
-   revision
-   date
-   tags
-   relationships
-   publication title/subtitle

Pandoc templates and filters MAY consume this metadata.

Canonical domain behavior SHALL not depend on Pandoc metadata.

# 24. Pandoc Templates

Pandoc templates SHOULD provide presentation-specific document shells.

Examples:

-   engineering HTML page
-   standalone technical report
-   PDF/LaTeX report
-   client-facing publication

DOCX output MAY use Pandoc reference DOCX files to control Word styles,
page geometry, headers, footers, and related presentation details.

# 25. Pandoc Filters

Pandoc filters MAY implement publication transformations such as:

-   resolving engineering-record references
-   adding backlinks
-   expanding graph views
-   rendering evidence callouts
-   inserting generated diagrams
-   formatting record metadata
-   cross-reference handling

Publication filters SHALL NOT directly own SQLite domain behavior.

The Rust `eng` binary SHOULD resolve knowledge semantics before or while
constructing the publication input.

Lua filters are appropriate for lightweight AST-level publication
behavior.

# 26. Diagrams and Cross-References

The publication system SHOULD support generated diagrams.

Graphviz DOT is a strong candidate for engineering-record relationship
graphs because the CLI can generate it deterministically from relational
graph data.

Pandoc-compatible cross-reference tooling MAY be used for:

-   figures
-   tables
-   sections
-   equations
-   citations

External evidence citations MAY use Pandoc's bibliography/citation
facilities while internal record relationships remain canonical graph
edges.

# 27. Static Documentation Site

Pandoc alone is not a static-site generator and SHALL not be expected to
replace all mdBook behavior.

The Rust application SHOULD own site information architecture and
generate:

``` text
site/
|- index.html
|- records/
|  |- RFC-0001.html
|  |- PDR-0001.html
|  |- ADR-0001.html
|  `- EDR-0001.html
|- topics/
|- decisions/
|- graph/
|- search-index.json
`- assets/
```

The application determines navigation, indexes, graph views, and page
selection.

Pandoc renders documents/pages.

This avoids mdBook's fixed book/chapter hierarchy and allows multiple
graph-derived views:

-   chronology
-   subsystem
-   record type
-   RFC lineage
-   tag
-   status
-   accepted architecture
-   publication package

# 28. Markdown and Git

Markdown SHOULD be committed when human portability, code review,
repository browsing, or operation without the CLI is valuable.

A typical repository MAY contain:

``` text
project/
|- .engineering/
|  `- knowledge.db
|- docs/
|  `- engineering/
|- src/
`- ...
```

CI SHOULD be able to verify generated Markdown:

``` text
eng export --check
```

The command SHOULD fail when committed projections differ from canonical
state.

Git therefore records changes to portable projections while SQLite
maintains structured canonical state and its own revision history.

# 29. Validation and Doctoring

The CLI SHALL provide repository validation.

Indicative output:

``` text
$ eng validate

Engineering record health

OK    ADR-0031 valid
OK    EDR-0008 valid

WARN  PDR-0017 approved but has no resulting decision

ERROR ADR-0034 supersedes ADR-0019 while ADR-0019
      remains marked accepted
```

Validation SHOULD eventually cover:

-   duplicate identifiers
-   malformed documents
-   missing required fields
-   invalid lifecycle transitions
-   broken relationship targets
-   invalid relationship kinds
-   supersession inconsistencies
-   stale projections
-   missing referenced evidence
-   invalid code references where practical

# 30. Security and Trust Boundary

The CLI SHALL validate all mutations.

An LLM SHALL NOT be considered trusted merely because it uses a skill.

Invalid requests such as linking to nonexistent records SHALL fail
without mutating canonical state.

SQLite transactions SHALL preserve consistency across:

-   current record state
-   revision history
-   relationships
-   FTS projection
-   associated metadata

where those items participate in a single logical operation.

# 31. Non-Goals for the Initial Version

The initial system does not require:

-   a server
-   a SaaS service
-   a vector database
-   embeddings
-   MCP
-   a JavaScript runtime
-   direct XML persistence
-   a web application framework
-   multi-user database concurrency beyond what the chosen local
    workflow requires
-   automatic enforcement of source-code architecture rules

These may be revisited based on measured needs.

# 32. Initial Implementation Sequence

A recommended implementation sequence is:

1.  Rust domain and application crates.
2.  SQLite schema and migrations.
3.  JSON document types and kind-specific validation.
4.  Transactional identifier allocation.
5.  Revision history.
6.  Relationship graph.
7.  FTS5 indexing and search.
8.  Core CLI with JSON output.
9.  Self-description commands (`schema`, `capabilities`,
    `relationships`).
10. Deterministic Pandoc Markdown renderer.
11. `validate` / `doctor` functionality.
12. LLM skill.
13. Agent-oriented `context` retrieval.
14. Evidence and code-reference workflows.
15. Pandoc publication orchestration.
16. Static-site generation.
17. Optional MCP adapter only if a concrete integration justifies it.

# 33. Acceptance Criteria for an Initial Useful Release

An initial release is useful when a repository can:

1.  Initialize an embedded knowledge database.
2.  Create RFC, PDR, ADR, and EDR records.
3.  Validate record-kind-specific required content.
4.  Transition records through valid statuses.
5.  Create explicit relationships between records.
6.  Preserve revision history.
7.  Search records through FTS5.
8.  Retrieve a record and its graph context as JSON.
9.  Render every record deterministically as Pandoc-compatible Markdown.
10. Validate database and relationship integrity.
11. Allow an LLM skill to safely search, read, create, revise, and link
    records through the CLI.
12. Produce at least HTML and DOCX/PDF-capable Pandoc input without
    coupling the domain model to those formats.

# 34. Architectural Summary

The resulting system is intentionally small:

``` text
                    Humans / CI / LLM
                           |
                           v
                       Rust CLI
                           |
                           v
                 Domain/Application Core
                           |
                           v
                         SQLite
             +-------------+-------------+
             |             |             |
             v             v             v
           FTS5          JSON        Relationships
             |                           |
             +-------------+-------------+
                           |
                           v
                  Engineering Document IR
                           |
                           v
                    Pandoc Markdown
                           |
                           v
                         Pandoc
                 +---------+---------+
                 |         |         |
                 v         v         v
               HTML       PDF       DOCX
```

The core architectural boundary is:

> Rust and SQLite own engineering knowledge. The CLI exposes validated
> operations. Skills teach LLMs how to use those operations. Markdown is
> a deterministic portable projection. Pandoc owns document compilation
> and publication. MCP remains an optional adapter.

This architecture avoids tying the engineering record system to a
particular LLM vendor, agent protocol, documentation framework, or
output format while retaining an inspectable, self-contained,
local-first source of engineering truth.

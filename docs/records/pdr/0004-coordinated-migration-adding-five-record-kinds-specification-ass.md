---
id: "PDR-0004"
title: "Coordinated migration adding five record kinds: Specification, Assessment, Research, Glossary, Risk"
record-type: pdr
status: draft
revision: 5
date: 2026-08-30
slug: coordinated-migration-adding-five-record-kinds-specification-ass
tags: []
relationships:
  derived-from:
    - "RFC-0006"
    - "RFC-0011"
  relates-to:
    - "RFC-0006"
---

# PDR-0004: Coordinated migration adding five record kinds: Specification, Assessment, Research, Glossary, Risk

## Problem

RFC-0006 and RFC-0011 together propose expanding RecordKind from 4 to 9 variants. Both RFCs stop deliberately short of design: RFC-0006 leaves Questions 3-9 open, RFC-0011 leaves 9 open, and neither states the schema, the lifecycle table, the relation endpoints, the migration surface, or the aggregate-rendering mechanism concretely enough to implement. This document supplies those, verified against the tree at aa7d4f1.

---

This PDR resolves RFC-0006 (proposed) and RFC-0011 (draft) as one coordinated migration, per RFC-0011's own Question 6. It was produced by an Opus planning agent (2026-08-30), briefed with both RFCs' full content and evidence, then independently spot-checked against the live tree before being recorded here: the zero-DDL-migration claim, the mutations.rs line count and check_file_size.sh failure, the RECORD_DIRECTORIES literal, and the store/src/validate.rs relation whitelist were each re-verified directly (grep/wc/script run) and confirmed accurate.

## Requirements

\- R1 All five kinds are creatable, revisable, status-transitionable, linkable, evidence-attachable and exportable through the existing CLI verbs; no new subcommand.
\- R2 strata schema <kind> self-describes every new kind in the existing shape (kind/purpose/schema/required_fields/statuses).
\- R3 export --check remains byte-for-byte (ADR-0010) for both per-record and aggregate projections.
\- R4 strata publish continues to work, including cross-record links into aggregate documents.
\- R5 A Specification requirement is machine-readable and independently checkable by an agent without prose judgment.
\- R6 Anchors and paths are stable under retitle (ADR-0012 / EDR-0003).
\- R7 Every enum-adjacent list either lives behind an exhaustive match or is derived from one; no new hardcoded kind list.
\- R8 No file exceeds 400 lines after the change.

## Constraints

CLAUDE.md rules 1-13 in full. Specifically binding here: no dyn (so aggregate-vs-per-record rendering is an exhaustive match, not a trait); no async; named parameters only; records stays I/O-free; _impl banned; no mod.rs; cargo fmt.

**The Markdown round-trip constrains schema design directly.** `cli/src/parse.rs::blocks_to_value` maps exactly one Markdown block per document field; two or more blocks in a section collapse to `JsonValue::String("")` on the next `revise --file`. A field can therefore hold a paragraph, a list, a fenced code block, a GFM table, or a reference -- never a nested structure. Any schema design here that assumes arbitrary nested JSON (rather than the single-block shapes the parser already round-trips) will silently lose data the first time someone edits it through the normal Markdown workflow rather than `--patch`. This is why Specification's `requirements` and `acceptance_criteria` are proposed as single GFM tables (Proposed Design 5.4) rather than a richer nested structure -- it is the one shape both the parser and the renderer already handle byte-deterministically.

**Superseded in part (2026-08-30): see RFC-0006-EV-004.** The one-block-per-field limit above is a property of today's hand-rolled `cli/src/parse.rs`, not a permanent constraint -- RFC-0006's Question 9 has since been resolved in favor of replacing that parser with `comrak`, which supports arbitrary nested block content (headings, nested lists, tables) per field without the flattening loss described above. This changes the framing of `FieldKind::Table` in Proposed Design 5.4: it is no longer justified by "the one shape the parser already handles" (that reasoning is now obsolete, since a richer nested Prose field is about to become possible for every kind), but the decision itself survives on narrower grounds -- Table remains the right choice for Specification's `requirements`/`acceptance_criteria` specifically because R5 requires them to be machine-checkable by an agent without prose judgment, a guarantee that comes from a declared column schema and validation, not from parser limitations. This PDR's other document fields proposed as `Scalar` (Assessment, Research, Glossary, Risk) should be revisited once the comrak migration lands, since they could reasonably become richer `Prose` fields rather than being artificially kept flat.

## Proposed Design

### 5.1 Kind identity, codes and slugs

| Kind | Code (id prefix) | slug() | CLI word | Export layout |
|---|---|---|---|---|
| Specification | SPEC | specification | specification (alias spec) | per-record -> docs/records/specification/ |
| Assessment | ASMT | assessment | assessment | per-record -> docs/records/assessment/ |
| Research | RSCH | research | research | per-record -> docs/records/research/ |
| Glossary | GLOS | glossary | glossary | aggregate -> docs/records/glossary.md |
| Risk | RISK | risk | risk | aggregate -> docs/records/risk-register.md |

Notes: RecordId::from_str uses split_once('-') so 4-character codes parse fine. code() and slug() are already independent functions, so a 4-char code with a spelled-out slug costs nothing. KindArg is a clap ValueEnum, so the CLI word derives from the variant name; short aliases via #[value(alias = "spec")].

RFC-0011 writes strata render GLOSSARY-NNNN as a working name. Recommend GLOS over GLOSSARY for id length parity, but this is a human decision (see Open Questions) -- ids are permanent and this is the last cheap moment to change it.

docs/research/ (existing, holds template-research.md) does not collide with docs/records/research/.

### 5.2 Schemas

Exact required_fields() tables, matching the existing shape in records/src/kind.rs:93-130.

Specification -- purpose: "What must the system do, what constraints apply, and how is conformance verified?"
(purpose, Scalar), (scope, Scalar), (non_goals, Scalar), (assumptions, Scalar), (constraints, Scalar), (requirements, Table -- new FieldKind), (acceptance_criteria, Table), (verification, Scalar), (open_questions, Scalar)

There is deliberately no parent field: hierarchy is a contains relationship, so the graph is the single source of truth and strata graph SPEC-0002 already shows it.

Assessment -- "What do we currently observe about this codebase, product, or conformance?"
(subject, Scalar -- inward: what was evaluated), (method, Scalar -- commands run / files read, what makes it re-runnable), (observations, Scalar), (gaps, Scalar), (confidence, Scalar), (recommendations, Scalar)

Deliberately no evidence field (unlike ADR/EDR/PDR): an Assessment's evidence belongs in real evidence rows via strata evidence-add, which is exactly the friction RFC-0006-EV-002 recorded. date/revision are record metadata already; duplicating them in the body repeats the mistake CLAUDE.md called out for ## Status.

Research -- "What does external or comparative evidence say about one specific, not-yet-decided question?"
(question, Scalar -- singular by construction, enforces RFC-0006's one-topic rule structurally), (method, Scalar), (findings, Scalar), (comparison, Scalar), (implications, Scalar -- what this means for a decision made elsewhere), (limitations, Scalar)

Sources go in evidence-add <id> source --uri ... (the source evidence kind already exists).

Glossary -- "What does this term mean, and where does that meaning apply?" Title = the term.
(definition, Scalar), (aliases, List -- first production use of FieldKind::List), (scope, Scalar), (examples, Scalar), (non_examples, Scalar)

Risk -- "What could go wrong, how likely and severe is it, and what is being done about it?"
(subject, Scalar), (description, Scalar), (category, Scalar -- validated against {technical, product, security, business, compliance}), (likelihood, Scalar -- free text, see Open Questions), (impact, Scalar -- free text), (mitigation, Scalar), (owner, Scalar)

### 5.3 Lifecycles

Status (records/src/kind.rs:160-175) gains five variants: Open, Monitoring, Mitigated, Materialized, Closed. Specification, Assessment, Research and Glossary need zero new statuses.

statuses() additions:
Specification -> [Draft, Review, Approved, Superseded] (identical to Pdr)
Assessment -> [Draft, Accepted, Superseded]
Research -> [Draft, Accepted, Superseded]
Glossary -> [Draft, Accepted, Superseded]
Risk -> [Open, Monitoring, Mitigated, Accepted, Materialized, Closed]

initial_status(): Draft for the first four; Open for Risk. This is the first kind whose initial status is not Draft; initial_status() is already a per-kind match so it costs one arm.

valid_next_statuses() additions in records/src/validation.rs:
Specification -> (Draft,Review) (Review,Approved) (Approved,Superseded)
Assessment -> (Draft,Accepted) (Accepted,Superseded)
Research -> (Draft,Accepted) (Accepted,Superseded)
Glossary -> (Draft,Accepted) (Accepted,Superseded)
Risk -> (Open,Monitoring) (Open,Mitigated) (Open,Accepted) (Open,Materialized) (Monitoring,Mitigated) (Monitoring,Accepted) (Monitoring,Materialized) (Accepted,Materialized) (Mitigated,Closed) (Accepted,Closed) (Materialized,Closed) (Mitigated,Open) (Closed,Open) -- genuine reopen

Flag: Risk is the first cyclic lifecycle in Strata. Mechanically fine (valid_next_statuses is a flat pair list; status_transition audits every move including undo, per EDR-0015), but it is an architectural first and RFC-0011 explicitly wants reopen history to be trustworthy. Human sign-off item.

### 5.4 FieldKind::Table -- the one new primitive

records::FieldKind gains a third variant Table, accepted by validate_document when the value is a JSON object of exactly {"headers": [String], "rows": [[String]]} -- which is precisely the shape Block::Table already round-trips (render.rs:151-162 -> blocks_to_value L293 -> section_blocks L151-162). cli/src/parse.rs::array_field (L238) generalizes to field_kind(kind, field).

Rationale for adding it rather than making requirements a prose Scalar: the stated goal is a requirement an agent can check itself against without judgment calls. A Scalar is prose. A Table gives strata show SPEC-0001 --json a machine-readable requirement list with zero new SQL, zero new CLI verbs, zero new sub-record entity, and it round-trips byte-deterministically through the existing renderer and parser. It is roughly 20 lines of new code across two files.

Column contracts, validated in a new pure module records/src/validation/specification.rs:
requirements headers == ["id", "requirement", "status"]
acceptance_criteria headers == ["requirement", "criterion"]

plus three pure checks, all inside records (no I/O):
1. every requirements.id is unique within the record and matches ^REQ-[A-Z0-9]+(-[A-Z0-9]+)*-\d{3}$;
2. every requirements.status parses to RequirementStatus -- a sql_enum! enum {Planned, InProgress, Verified, Blocked, Waived}, RFC-0006's own list;
3. every acceptance_criteria.requirement value appears in requirements.id (referential integrity between the two tables).

Known limitation to name in the design record: parse.rs::table_row splits cells on " | " and cells cannot contain newlines. A criterion must therefore be one single-line assertion. That is arguably the right constraint for agent-checkability, but it must be documented rather than discovered.

### 5.5 Relationship vocabulary

records::RELATIONSHIPS goes from 10 to 15. This resolves RFC-0006 Q3 and RFC-0011 Q1 together (they are the same question).

New relations: contains (SPEC -> SPEC, hierarchical decomposition, containment), refines (EDR/ADR -> SPEC, narrows a requirement's mechanism, refinement), uses-term (any -> GLOS, cites a canonical definition), at-risk-from (any -> RISK, this record is threatened), mitigates (any -> RISK, this record's work reduces it).

Deliberately not added:
\- constraint -> constrains already exists (zero uses; this is its first real use).
\- implementation -> implements / implemented-by already exist.
\- verification -> reuse supported-by (zero uses today). RFC-0011's own boundary test applied to relations says don't add verified-by without a positive classification test against supported-by, and none can be produced. Revisit if real use shows ambiguity.
\- No inverse of contains -- strata graph already shows incoming edges (store/src/queries.rs:74, WHERE source_id = :id OR target_id = :id).

Endpoint typing (new, minimal). validate_relationship(source: &RecordId, relation, target: &RecordId) already receives both RecordIds, which carry their kinds, and currently ignores them (records/src/validation.rs:156-166). Add a small table constraining only the five new relations:
contains: source.kind == Specification && target.kind == Specification
refines: target.kind == Specification
uses-term: target.kind == Glossary
at-risk-from: target.kind == Risk
mitigates: target.kind == Risk

The ten existing relations stay unconstrained so nothing in the existing 43-relates-to corpus retroactively breaks. This is a positive, contained addition -- not a general relationship-typing system.

### 5.6 Specification hierarchy -- concrete data model

RFC-0006 leaves this as prose. Concretely:

\- Parent/child is a contains edge, stored in the existing record_relation table. No new table, no new column, no migration.
\- Single parent: an invariant enforced by a new check in store/src/validate.rs (returning an ERROR, not a WARN): SELECT target_id, COUNT(*) c FROM record_relation WHERE relation = 'contains' GROUP BY target_id HAVING c > 1
\- Acyclic: recursive CTE, ERROR: WITH RECURSIVE reach(root, node) AS (SELECT source_id, target_id FROM record_relation WHERE relation = 'contains' UNION SELECT r.root, rr.target_id FROM reach r JOIN record_relation rr ON rr.source_id = r.node AND rr.relation = 'contains') SELECT DISTINCT root FROM reach WHERE root = node ORDER BY root (UNION not UNION ALL -- terminates on a cycle rather than looping.)
\- Requirement-id uniqueness within a tree: not expressible in SQL against JSON tables, so it is Rust in a new store/src/validate/specification.rs: walk each contains forest root, collect requirements.id cells from each member's document, report duplicates as ERROR with both record ids. This is the check that makes REQ-LEADS-001 a genuinely stable, tree-wide identifier.
\- Child may not broaden parent scope: prose discipline, not mechanically checkable. State so explicitly rather than pretending.

### 5.7 Aggregate rendering for Glossary and Risk

Dispatch. A new exhaustive match in records/src/kind.rs: pub enum ExportLayout { PerRecord, Aggregate { file: &'static str } } pub fn export_layout(self) -> ExportLayout. Glossary => Aggregate { file: "glossary.md" }, Risk => Aggregate { file: "risk-register.md" }, everything else PerRecord. Being an exhaustive match, every future kind is forced to choose -- this is the mechanism that turns today's silent RECORD_DIRECTORIES drift into a compile error.

cli/src/export.rs. RECORD_DIRECTORIES is deleted. rendered_records() partitions store.all_records() by export_layout(), renders PerRecord entries as today, and hands each Aggregate group to a new cli/src/render/aggregate.rs. The stale sweep becomes: owned paths = (every PerRecord kind's directory) union (every Aggregate kind's file). If an aggregate kind has zero records, no file is written and an existing one is reported stale/deleted -- the empty-corpus case must be explicit, not accidental.

ADR-0010 is preserved unchanged. differences() (L66-102) compares BTreeMap<PathBuf, String> against on-disk bytes with on_disk == content.as_bytes(). The aggregate file is simply one more entry in that map. Cardinality changes; the comparison does not. No normalization is introduced anywhere.

Anchor-slug algorithm (RFC-0011 Q8, resolved). The anchor is {number:04}-{slug} -- literally the filename stem EDR-0003 already computes, minus .md. So docs/records/glossary.md#0003-lead. Correction (2026-08-30, see PDR-0002-EV-001): this only holds if the renderer actually emits that id. Pandoc's default heading-id scheme (`auto_identifiers`) is title-derived and confirmed unstable -- it drifts both on retitle and when an unrelated heading is inserted earlier in the same document, shifting later headings' positional de-duplication suffixes (e.g. `definition-1` -> `definition-2`). The fix is confirmed and cheap: Pandoc's `header_attributes` extension (on by default) honors an explicit `{#custom-id}` on any heading verbatim, including leading digits. `Block::Heading` (cli/src/document.rs) therefore needs a new optional id field, rendered as a trailing `{#...}` attribute -- `{number:04}-{slug}` on each aggregate entry's own heading, and `{number:04}-{slug}-{field}` on its sub-headings, or those drift positionally by the same mechanism. `--id-prefix` must never be used with this scheme; it prefixes explicit ids too and silently breaks every computed href.
\- Reuses the persisted, immutable slug column (EDR-0003 computes it once at Store::create, ADR-0012 forbids recomputation), so a retitle never moves an anchor -- exactly the property RFC-0011 asked for.
\- Unique by construction via the number prefix, so no collision handling is needed and no new algorithm is invented.
\- Trade-off, named: #lead reads better than #0003-lead. But uses-term is a relationship row (target GLOS-0003), not a hand-typed URL -- the renderer resolves the href -- so determinism beats prettiness.

Sort order (Q8, resolved).
\- Glossary: sort by (slug, number), not by title. slug is already NFKD-normalized, combining-marks-stripped, ASCII-lowercased (store/src/slug.rs), so the ordering is byte-deterministic with zero locale dependence -- a direct application of ADR-0010's no-normalization-surprises stance. Known limitation: titles that slugify to untitled (e.g. CJK terms) cluster; number keeps it deterministic.
\- Risk: sort by (status_rank, number) where status_rank = Open 0, Monitoring 1, Materialized 2, Accepted 3, Mitigated 4, Closed 5 -- attention first. RFC-0011 proposes "status, then severity, then id"; since Q3 resolves to no severity scoring in v1, there is no comparable severity value and the key is dropped. Adding severity later inserts one sort key without changing the document shape.

Document shape: docs/records/glossary.md has aggregate frontmatter, a # Glossary heading, and one ## GLOS-NNNN: Term heading per entry (anchor NNNN-slug) with ### Definition / ### Aliases subsections. docs/records/risk-register.md has a # Risk Register heading, a generated summary table (id/title/status/likelihood/impact/owner), then one ## RISK-NNNN: Title heading per entry with ### Subject / ### Description etc. Entry field headings shift from level 2 to level 3 (entries nest one level deeper) relative to a normal per-record document. strata render GLOS-0003 is unchanged -- it still emits the standard per-record document with # GLOS-0003: Lead and ## Definition, exactly as RFC-0011 says.

The summary table earns its place only in risk-register.md (RFC-0011's own words: "a risk register is conventionally... a single table"). Glossary gets no index table in v1.

Named asymmetry: the aggregate projection is write-only. cli/src/parse.rs cannot read it back, because it expects one record's frontmatter and one # {id}: {title} heading. That is acceptable -- parse is only used for new --file / revise --file on a single record, and CLAUDE.md already forbids hand-editing generated Markdown -- but it must be stated, not discovered.

Q9 resolved: closed/materialized risks stay in the document, sorted last, never archived out. Removing them would (a) make the projection lossy relative to canonical state so export --check stops covering them, (b) break at-risk-from deep links from historical decisions, and (c) contradict the project's whole thesis that history stays legible.

### 5.8 publish -- the one genuinely coupled change

cli/src/manifest.rs::manifest_entry (L102-127) looks up record_path(export_root, record) in the rendered map and hard-errors if absent. Aggregate-kind records have no per-record path, so publish breaks unless changed. Minimum viable fix:

1. ManifestEntry splits publication_path into publication_path: String + anchor: Option<String>. SCHEMA_VERSION 1 -> 2.
2. Aggregate-kind entries get publication_path = "glossary.html", anchor = Some("0003-lead"). site_index (publish.rs:40-44) then yields ../glossary.html#0003-lead, so the Lua xref filter resolves cross-record links into aggregate pages automatically. Correction (2026-08-30, see PDR-0002-EV-001): the earlier claim that cli/assets/site/xref.lua "contains no hardcoded record-kind pattern... needs no change" is wrong -- it was checked but not tested. xref.lua's record-id matcher is hardcoded to exactly three letters (`[A-Z][A-Z][A-Z]%-[0-9][0-9][0-9][0-9]`), confirmed directly, and would silently fail to link any mention of the five new four-letter kind codes (SPEC, ASMT, RSCH, GLOS, RISK). Required fix, verified: widen the pattern to `[A-Z][A-Z][A-Z][A-Z]?%-[0-9][0-9][0-9][0-9]`.
3. content_hash stays per-entry (hash of that entry's own rendered fragment), so changing one term reports one difference rather than N; manifest_differences (L140+) dedupes emitted messages by publication_path.
4. publish::run groups staged files by publication_path so an aggregate document is rendered once, from the aggregate Markdown, not once per record.

This is the largest single piece of the aggregate work and should be its own commit.

## Components

New files:
\- records/src/validation/lifecycle.rs -- transition tables (moved out of validation.rs, which would otherwise exceed 400)
\- records/src/validation/specification.rs -- requirements/acceptance-criteria table contracts, RequirementStatus
\- records/src/kind/schema.rs -- required_fields() tables (moved out of kind.rs)
\- cli/src/render/aggregate.rs -- aggregate document assembly
\- store/src/validate/specification.rs -- hierarchy + tree-wide requirement-id checks

(All per rule 13: module_name.rs + module_name/submodule.rs, no mod.rs.)

Modified -- compiler-enforced:
\- records/src/kind.rs -- 5 sql_enum! variants; ALL: [Self; 9]; code/purpose/slug/initial_status/statuses; new export_layout(); Status +5 variants; FieldKind::Table; both golden tests extended
\- records/src/validation.rs -- valid_next_statuses +5 arms; RELATIONSHIPS: [&str; 15]; Table arm in validate_document; endpoint typing in validate_relationship
\- records/src/lib.rs -- re-exports
\- cli/src/document.rs -- fields() +5 arms (JSON field -> Markdown heading)
\- cli/src/args.rs -- KindArg +5 variants, From +5 arms, and the New doc comment ("Create a new RFC, PDR, ADR, or EDR record") which drives --help

Modified -- silent-drift, must be handled deliberately:
\- cli/src/export.rs -- delete RECORD_DIRECTORIES, derive from RecordKind::ALL + export_layout()
\- store/src/validate.rs -- L37 relation whitelist -> json_each(:relations); L59 lineage kind lists left unchanged, with a comment saying why

Modified -- mechanical:
\- cli/src/manifest.rs, cli/src/publish.rs, cli/src/render.rs (split), cli/src/parse.rs (array_field -> field_kind)
\- store/src/mutations.rs -- split first (section 2.4)
\- CLAUDE.md rule 2 -- currently reads "closed four-variant enum (Rfc, Pdr, Adr, Edr)... If a fifth kind is ever added". Must become nine, and should gain a sentence about the two non-compiler-enforced sites so the rule stops overstating its own guarantee.
\- docs/specification.md sections 4/5 -- already stale (per the documentation audit earlier this session, RFC-0006-EV-002). Update or explicitly mark historical.

Unchanged, verified:
\- store/migrations/* -- no new file
\- cli/src/help.rs HELP_GROUPS -- no new subcommand
\- cli/src/output.rs -- RecordKind::ALL.map(...) (L174) auto-widens [T;4] to [T;9]
\- cli/assets/site/* -- no kind literals
\- store/src/mutations.rs::all_kinds_have_valid_creation_documents -- iterates RecordKind::ALL, so all five kinds get creation coverage for free

## Interfaces

No new subcommands. The full agent workflow falls out of KindArg:

```text
strata new specification "Strata storage specification"
strata new assessment "2026-08-30 documentation audit"
strata new research "Rust Markdown parsers with YAML frontmatter"
strata new glossary "Lead"
strata new risk "Pandoc is an undeclared runtime dependency of publish"
strata schema risk --json
strata template glossary
strata evidence-add RSCH-0001 source "commonmark-rs benchmark" --uri ...
strata revise SPEC-0001 --patch --document '{"requirements": {...}}'
strata link ADR-0021 uses-term GLOS-0003
strata link ADR-0021 at-risk-from RISK-0002
strata status RISK-0002 monitoring
```

`revise --patch` (RFC-0008, already shipped) is what makes mid-session persistence cheap for an agent -- updating one requirement's status is a one-field patch, not a whole-document rewrite. That is the low-friction shape the brief asked for, and it already exists.

## Data Model

No new tables, no new columns, no DDL. Everything lands in:
\- engineering_record.kind -- five new TEXT values
\- engineering_record.status -- five new TEXT values
\- engineering_record.document -- five new JSON schemas
\- record_relation.relation -- five new TEXT values
\- engineering_record_fts.body -- automatically covers aliases (section 2.6)

**Specification hierarchy data model** (detailed in Proposed Design 5.6): parent/child is a `contains` edge in the existing `record_relation` table -- no new table, no new column. Single-parent and acyclic constraints are enforced as new ERROR-level checks in `store/src/validate.rs` (a GROUP BY HAVING count and a recursive CTE respectively). Requirement-id uniqueness within a specification tree is a new pure Rust check in `store/src/validate/specification.rs`, since it is not expressible against JSON table fields in SQL.

## Failure Modes

Two failure modes matter more than the rest because they defeat CLAUDE.md rule 2's stated guarantee that "the compiler must fail to build until every match site is updated": (1) `cli/src/export.rs`'s `RECORD_DIRECTORIES` literal is not updated when a kind is added -- `export` then writes the new kind's files but never sweeps its directory for staleness, so a renamed or deleted record of that kind silently leaves an orphaned file that `export --check` reports clean, weakening ADR-0010's byte-for-byte guarantee without any visible error. (2) `store/src/validate.rs`'s inlined relation whitelist is not updated -- `strata validate` then reports every use of a newly added relation (`uses-term`, `at-risk-from`, etc.) as a false-positive "broken or invalid relationship", and because `strata publish` treats any non-`WARN` validate issue as blocking (cli/src/publish.rs:28-37), this would silently break publish the first time anyone links a record to a Glossary term or a Risk.

Two additional failure modes are less severe but worth naming: an older `strata` binary reading a database that already contains a new-kind record (e.g. `SPEC-0001`) fails loud with `unknown RecordKind: SPEC` in `FromSql` -- correct per rule 7's explicit-error policy, but it also means the committed `docs/db-snapshot.sql` becomes unrestorable by any older build the moment the first new-kind record exists, which is a one-way door worth stating rather than discovering later. And `FieldKind::List` (used by zero kinds today; Glossary's `aliases` would be its first production use) has an untested empty-list path -- `parse_section` returning `Ok(vec![])` for an empty section, converted to `JsonValue::Array(vec![])` -- that `validate_document` already accepts but that has never been exercised by a real kind.

## Alternatives Considered

Deviations and rejected alternatives surfaced during planning, not carried over from either RFC:

\- A `V4__add_kind_check.sql` adding `CHECK(kind IN (...))` constraints was considered and rejected: SQLite cannot `ALTER TABLE ADD CHECK`, so it would require a full table rebuild (against rule 8's spirit that this database is the only copy of the data), and would create a second source of truth for the enum that every future kind would need to migrate. RecordKind's closedness stays enforced entirely in Rust, matching what CLAUDE.md rule 2 already describes.
\- Risk severity/priority scoring (a likelihood x impact matrix) was considered for v1 and deferred (resolves RFC-0011 Q3): free-text likelihood/impact fields are sufficient until real use shows a scoring need, and the risk-register sort order does not depend on it.
\- A `verified-by` relation was considered for Specification's verification links and rejected in favor of reusing the existing, currently-unused `supported-by` relation -- RFC-0011's own relationship boundary test (a positive classification test against every existing relation, not just "doesn't fit anywhere else") could not produce a clean distinction between the two.
\- Making `requirements` a plain prose Scalar field (no new `FieldKind::Table` primitive) was considered as the smaller change. Rejected because it forfeits the stated goal of agent-checkable requirements: a Scalar is prose an agent must interpret; a Table gives `strata show SPEC-0001 --json` a machine-readable requirement list for roughly 20 lines of new code.
\- No new CLI subcommands were considered necessary for any of the five kinds -- the existing `strata new/revise/status/link/evidence-add` verbs plus `KindArg` additions cover the full workflow (R1).

## Evidence

This plan's factual claims about the codebase were independently verified twice: once by the planning agent against the tree at commit aa7d4f1 (using direct reads, grep, and `graphlite`), and again independently after the plan was returned, before being recorded here. Confirmed on the second pass: no `CHECK(kind IN (...))`, `CHECK(status IN (...))`, or `CHECK(relation IN (...))` exists anywhere in store/migrations/*.sql (only `json_valid` and `transition_kind IN ('forward','undo')` checks exist) -- the zero-DDL-migration finding holds. `store/src/mutations.rs` is exactly 456 lines, and `bash scripts/check_file_size.sh` genuinely fails on HEAD right now with that file -- this is a real, pre-existing blocker, not a plan artifact. `cli/src/export.rs:9` genuinely contains `const RECORD_DIRECTORIES: [&str; 4] = ["rfc", "pdr", "adr", "edr"]`, used at lines 82 and 125 for the stale-file sweep only. `store/src/validate.rs` genuinely inlines the ten relation names as a SQL string literal (line ~37) independent of `records::RELATIONSHIPS`.

This PDR also draws on RFC-0006's evidence (EV-001 and EV-002: two substruct research tasks and this session's own documentation audit, the real friction that motivated Specification/Assessment/Research; EV-003: the initial RecordKind blast-radius estimate this plan corrects and narrows) and RFC-0011's evidence (EV-001: the rejected code-snippet-extraction idea, and its citation of EDR-0006's per-record evidence-id precedent, which directly informed this plan's decision not to add any new global identifier scheme).

**Detailed verification findings, by file** (the corrected blast-radius table superseding RFC-0006-EV-003's original 25-file estimate):

| File | Sites | What |
|---|---|---|
| records/src/kind.rs | 7 | `sql_enum!` variant list; `pub const ALL: [Self; 4]` (array length is itself a compile error on change); `code()`; `purpose()`; `slug()`; `initial_status()`; `statuses()`; `required_fields()` |
| records/src/validation.rs | 1 | `valid_next_statuses()` |
| cli/src/document.rs | 1 | `fields()` (EDR-0004 JSON-to-Markdown heading table) |
| cli/src/args.rs | 2 | `enum KindArg`; `impl From<KindArg> for RecordKind` |

Everything else in EV-003's original 25-file list is either a call site that compiles unchanged against a wider enum, or `RecordKind::Adr` used as test fixture data. `cli/src/render.rs` and `cli/src/render_backend.rs` are fully kind-generic (they go through `kind.slug()` / `fields(kind)`, no match of their own); `cli/src/export.rs` is the one genuine problem, covered in Failure Modes.

**explored-by usage, confirmed against the live database (read-only query, no mutation):** relation usage today is `relates-to` 43, `produces` 32, `implements` 4, `resolves` 3, `derived-from` 1. `explored-by`, `constrains`, `implemented-by`, `supported-by`, and `supersedes` all have zero rows. RFC-0006's claim that `explored-by` is present but genuinely unused holds exactly.

**Glossary alias search, confirmed with zero code changes:** `store/src/lib.rs` indexes the entire document JSON as the FTS5 body (`serde_json::to_string(document)`), so a Glossary record's `aliases` array is automatically in the search index the moment the record exists -- RFC-0011's alias-lookup mechanism needs no new code and no schema change.

## Experiments

Verification commands run against the live tree (2026-08-30), independent of any code changes:

\- `grep -n "CHECK" store/migrations/*.sql` -- confirmed only `json_valid(...)` and `transition_kind IN ('forward','undo')` checks exist; no kind/status/relation CHECK constraint.
\- `wc -l store/src/mutations.rs` -- confirmed 456 lines.
\- `bash scripts/check_file_size.sh` -- confirmed it currently reports `ERROR store/src/mutations.rs: 456 lines (limit 400)` on HEAD, independent of this plan.
\- `grep -n "RECORD_DIRECTORIES" cli/src/export.rs` -- confirmed the 4-element literal array and its two use sites (differences(), write_projection()).
\- `sed -n` over `store/src/validate.rs` -- confirmed the inlined ten-relation SQL string literal and the `t.kind IN ('ADR','EDR')` / `s.kind IN ('RFC','PDR')` lineage-check literal.
\- `grep -n "RecordKind::Rfc\|Self::Rfc"` across records/src, cli/src, store/src, kernel/src -- used by the planning agent to build the corrected, narrower blast-radius table in Findings 2.2, superseding RFC-0006-EV-003's original 25-file estimate.

## Risks

\- CLAUDE.md rule 2's "compiler catches everything" promise is partly false today: two string-literal sites (section 2.3). Mitigated by deriving both and adding a test; the rule text itself should be amended so it stops overstating.
\- store/src/validate.rs:37 relation whitelist not updated => publish blocked: publish.rs:28-37 treats non-WARN issues as fatal. Highest-severity single miss in the whole change.
\- Aggregate export breaks the 1:1 file<->record mental model: deliberate and named (RFC-0011); one changed term now reports as one changed file, and git blame on glossary.md mixes terms.
\- publish manifest schema bump: existing .strata/site/manifest.json is gitignored and regenerable, so no data migration; but --check against an old manifest will report everything different once.
\- 400-line ceiling: estimated post-change, kind.rs ~340 without the schema split, validation.rs ~410 over, render.rs 366 -> over. All three splits are planned in Components, not reactive.
\- Status reaches 14 variants shared across 9 kinds: Accepted already means different things for RFC vs ADR; this worsens it slightly. Contained because statuses() and valid_next_statuses() are per-kind.
\- Risk introduces the first cyclic lifecycle (section 5.3). Needs sign-off.
\- Older binary vs newer docs/db-snapshot.sql: loud FromSql failure -- correct per rule 7, but state it.
\- FieldKind::List has zero production users today: Glossary's aliases is the first; needs explicit round-trip tests including the empty-list case (section 2.5).
\- Table cells cannot contain " | " or newlines (section 5.4); document it rather than let an author discover it.

## Open Questions

Resolved by this plan:
\- RFC-0006 Q3 (relation vocabulary): 5 new relations (section 5.5); containment=contains, refinement=refines, constraint=existing constrains, implementation=existing implements/implemented-by, verification=existing supported-by (no verified-by).
\- RFC-0006 Q4 (requirement progress storage): in the specification document, as the status column of the requirements Table field. Not SQLite rows, not derived. Rationale: every other piece of record state already lives in the document and goes through revise + revision history; deriving it would need a per-requirement link target that does not exist and that RFC-0006 does not propose; revise --patch already makes one-field updates cheap.
\- RFC-0006 Q7 (validate vs. lint): validate gets only the structural checks (single parent, acyclic, tree-wide requirement-id uniqueness) as ERRORs. Everything judgment-shaped is advisory and belongs in RFC-0010's lint, not validate.
\- RFC-0011 Q1 (relation names): fold into RFC-0006 Q3 and settle together, as above.
\- RFC-0011 Q2 (scoped glossary meanings): one global meaning per term per database in v1. scope is a descriptive prose field recording where the meaning applies; it is not an enforced resolution mechanism. Subtree-scoped resolution is deferred until RFC-0006's hierarchy has real use.
\- RFC-0011 Q3 (risk severity scoring): no scoring in v1. likelihood and impact are free-text Scalars. Consequence: the register sorts by (status_rank, number), not severity.
\- RFC-0011 Q6 (one migration?): yes, section 9.
\- RFC-0011 Q8 (sort + anchor): anchor = {number:04}-{slug}, reusing the persisted immutable slug (EDR-0003/ADR-0012). Glossary sorts by (slug, number); Risk by (status_rank, number). Section 5.7. And yes, it needs its own EDR (Phase 4).
\- RFC-0011 Q9 (archive closed risks?): retained in the same document, sorted last. Section 5.7.

Deliberately left for the human:
1. Kind codes: SPEC/ASMT/RSCH/GLOS/RISK vs. RFC-0011's GLOSSARY-NNNN. Ids are permanent; this is the last cheap moment.
2. RFC-0006 Q5 -- the minimum conformance evidence before a requirement is verified. Recommendation is no mechanical gate in v1, because workflow.md's "Known gap 2" already concedes that nothing gates any status transition on evidence, and gating requirements while decisions stay ungated would be inconsistent. But the bar itself is a project-owner call.
3. RFC-0006 Q6 -- whether PDR ever gets a real kind-name migration. Nothing in this plan depends on it; deferring costs nothing.
4. RFC-0006 Q8 -- specification package publication. Deliberately untouched; section 5.8 only does the minimum to keep publish working.
5. RFC-0006 Q9 -- which external Rust Markdown/YAML parser. An independent thread that would replace cli/src/parse.rs wholesale. Doing it inside this migration would make an already-broad change unreviewable. It should be its own RFC-0006-derived design record -- and it is the ideal first real Research record once Phase 1 lands.
6. RFC-0011 Q4 -- ERROR vs INFO for term/risk cross-checks. Recommendation is INFO in lint, but lint (RFC-0010) does not exist yet, so this is really a decision about RFC-0010's scope.
7. RFC-0011 Q5 -- whether the four-criterion boundary test becomes its own ADR. Recommend yes, but after Glossary and Risk have real use, matching ADR-0016's own precedent of formalizing only after friction.
8. RFC-0011 Q7 -- whether aliases + FTS5 is sufficient. Only real use can answer; section 2.6 confirms the mechanism works with zero code.
9. Adding FieldKind::Table (section 5.4). This is the one genuinely new primitive in the plan. The fallback -- requirements as prose Scalar -- is smaller but forfeits agent-checkability, which is the stated goal.
10. Assessment/Research/Glossary using accepted as their standing status. It reads slightly oddly for an observation. The alternative costs one new Status variant.
11. Risk's cyclic lifecycle (section 5.3).
12. Whether refines earns its place or refinement folds into constrains. The weakest of the five new relations by RFC-0011's own boundary test.

Not re-litigated: RFC-0011's five rejections (Persona, standalone Business/Product Decision, Conflict/Reconciliation, Postmortem/Incident, Vision/Charter) are treated as settled. No reason found to reopen any of them, and this plan proposes no kind of its own.

Footnote for a possible future RFC: a "what's actionable now" query surface -- synthesizing open Risks, planned/blocked Specification requirements, and draft/proposed RFCs awaiting review into one queryable worklist -- would fit this tool's agent-facing purpose well, and the data model above would support it with no schema change. It is new scope: neither RFC-0006 nor RFC-0011 proposes it, and it is not designed or committed to anywhere in this plan. Noted only so it is not lost.

Critical files for implementation: records/src/kind.rs, records/src/validation.rs, cli/src/export.rs, cli/src/document.rs, store/src/validate.rs

## Resulting Decisions

None yet. This PDR is in draft status, produced for review, and has not been approved. Once approved, expect the review to resolve into: one ADR-level decision covering the RecordKind/relationship-vocabulary/lifecycle expansion itself (following ADR-0016's positive-classification-test precedent for the two new relation-endpoint-typing rules), and separate EDR-level decisions for FieldKind::Table (the specification requirements/acceptance-criteria table contract), the Glossary/Risk aggregate-export deviation from EDR-0003 (one EDR covering both kinds together, per this plan's Phase 4), the Specification hierarchy validation rules (single-parent, acyclic, tree-wide requirement-id uniqueness), and the manifest/publish schema bump enabling anchor-based cross-references into aggregate documents. Phase 0 (splitting store/src/mutations.rs) is independent housekeeping and does not need its own EDR.

---
id: "ADR-0013"
title: "Add a native SQL dump/restore projection as a full-fidelity backup, separate from the Markdown projection"
record-type: adr
status: proposed
revision: 1
date: 2026-08-19
slug: add-a-native-sql-dump-restore-projection-as-a-full-fidelity-back
tags: []
relationships:
  relates-to:
    - "ADR-0001"
    - "ADR-0003"
    - "ADR-0010"
---

# ADR-0013: Add a native SQL dump/restore projection as a full-fidelity backup, separate from the Markdown projection

## Context

ADR-0003 established that Markdown is a generated, read-only projection of canonical SQLite state, useful for human review and repository browsing (spec sections 18-19). It was never designed or claimed to be a full-fidelity backup path, and a concrete test confirms it is not one in practice: recreating ADR-0001 from its own checked-in Markdown file via `strata new adr ... --file docs/records/adr/0001-....md` reproduces the document body correctly but returns `status: proposed, revision: 1` where the real record is `status: accepted, revision: 2`. Tracing why, in cli/src/parse/frontmatter.rs and cli/src/parse.rs: the frontmatter parser fully parses and validates `status`, `revision`, and `relationships`, but document_to_json() (cli/src/parse.rs) only ever reads the body sections into the created document -- the parsed status, revision, and relationships are validated for well-formedness and then discarded, never applied via store::set_status or store::link. There is no bulk import/restore command at all; `new --file` and `revise --file` are the only Markdown-input paths, and neither reconstructs more than one record's current content. Evidence and code references (store/src/evidence.rs, store/src/code_reference.rs) are entirely absent from the Markdown contract (ADR-0008, ADR-0009) and would not round-trip through Markdown even if the above gap were closed.

The `.strata/self-host.db` file itself is gitignored and untracked (commit ef91691), so today canonical state exists in exactly one place with no git-backed recovery path at all if that file is lost or corrupted.

Closing this gap by teaching the Markdown importer to also replay status, revision history, and relationships from frontmatter was considered and rejected as the approach -- see Alternatives.

## Decision

We will add a second, separate git-trackable projection whose sole purpose is full-fidelity backup and restore, generated as plain-text SQL: a native Rust dump (no dependency on the external `sqlite3` binary, keeping the CLI self-contained per spec section 14) that reads `sqlite_master` for schema DDL and each of Strata's own domain tables -- `engineering_record`, `record_relation`, `record_revision`, `evidence`, `code_reference`, in that order to respect foreign-key dependencies -- and emits `INSERT` statements reproducing every row exactly. `engineering_record_fts` is excluded from the dump: its content (title, body, tags) is fully derived from `engineering_record` by store::index_record, so it is rebuilt on restore rather than dumped as redundant data. Refinery's own migration-history table is likewise excluded; a restore target gets its schema and migration history from a normal `Store::open()` migration run, not from replayed rows.

This is a projection with a different purpose than Markdown, not a replacement for it: Markdown remains the human-review artifact (ADR-0003 stands unchanged), while the SQL dump exists specifically so a corrupted or lost `.strata/self-host.db` can be reconstructed byte-for-byte-equivalent in content from git history alone. Like `strata export`, dump generation is an explicit, deliberate step (`strata dump`), not automatic on every mutation, and gets the same CI-verifiable consistency guarantee ADR-0010 gave Markdown: `strata dump --check` fails if the committed file no longer matches canonical state.

## Considered Options

Teach the existing Markdown importer to replay frontmatter status, revision history, and relationships in addition to body content -- rejected as insufficient even if built: evidence and code references have no representation in the Markdown contract at all (ADR-0008, ADR-0009 scoped frontmatter and section mapping to record content and relationships only), so this path could never be a complete backup regardless of how much importer logic were added, only a partial one that silently drops two entire tables. Shell out to the system `sqlite3` binary's `.dump` command -- rejected: adds a dependency on a tool being present on the host outside Strata's own control, which cuts against the CLI being self-contained and self-describing (spec section 14); a native implementation keeps `strata dump`/`strata restore` working anywhere the `strata` binary itself works. Rely on `.strata/self-host.db` staying gitignored with no git-backed recovery path at all, treating out-of-band backup (disk snapshots, litestream, manual copies) as sufficient -- rejected: this project's own founding motivation (RFC-0001) is preventing silent, unrecoverable loss of engineering knowledge; leaving the only copy of canonical state with no git history at all is the same category of risk the Markdown projection was partly meant to mitigate (spec section 28), just left unresolved for the parts Markdown cannot cover. Git LFS to track the binary `.strata/self-host.db` file directly -- rejected: SQLite is a binary format Git cannot meaningfully diff or three-way-merge, so two branches independently mutating canonical state would produce an unresolvable binary conflict rather than a mergeable text conflict; a plain-text SQL dump gets normal `git diff`/merge behavior on the actual row-level changes instead.

## Consequences

A new `strata dump` command performs a full native reimplementation of SQL row serialization (escaping, NULL handling, one `INSERT` per row) for five tables -- this is new surface area analogous in size to the Markdown renderer (ADR-0008), not a small addition, and its exact serialization rules are implementation detail this ADR does not fix (parallel to how ADR-0008 handed detailed section-mapping rules to EDR-0004; a follow-on EDR may be warranted once implementation specifics are settled). `strata restore` becomes the first command that reconstructs a complete database rather than one record, and needs its own transactional-integrity guarantee (ADR-0001's single-writer model, and the rollback guarantee store/src/mutations.rs's revise() already demonstrates, both apply directly). The committed dump file is checked into git alongside `docs/records/`, at a location this ADR does not fix (a follow-on decision or the implementing EDR should place it, e.g. outside `docs/records/` so it is not mistaken for a per-record Markdown file). `strata dump --check` joins `strata export --check` as a second CI-verifiable consistency gate. Markdown's scope and purpose are unchanged by this decision.

## Evidence

cli/src/parse.rs's document_to_json() and cli/src/parse/frontmatter.rs: the traced code path confirming frontmatter status/revision/relationships are validated but discarded, the concrete gap motivating this decision. A reproduction: recreating ADR-0001 from docs/records/adr/0001-sqlite-as-strata-s-canonical-persistent-store.md via `strata new adr ... --file` into a fresh scratch database returned `status: proposed, revision: 1` against the real record's `status: accepted, revision: 2`. ADR-0001, ADR-0003: the prior decisions this one extends (SQLite as canonical store; Markdown as a non-canonical, review-purposed projection) without revising either. ADR-0010: the `export --check` precedent this decision's `dump --check` mirrors. commit ef91691: the decision to gitignore `.strata/self-host.db`, which is what leaves canonical state with no git-backed recovery path today.

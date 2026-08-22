---
id: "EDR-0015"
title: "status undo requires an auditable transition log and immediate latest-revision eligibility"
record-type: edr
status: accepted
revision: 4
date: 2026-08-22
slug: status-undo-requires-an-auditable-transition-log-and-immediate-l
tags: []
relationships:
  implements:
    - "RFC-0009"
---

# EDR-0015: status undo requires an auditable transition log and immediate latest-revision eligibility

## Context

RFC-0009 leaves status undo open because the current record_revision history stores documents and summaries but not status before or after each revision. A predecessor cannot be inferred safely from lifecycle rules: several statuses have multiple possible predecessors, and older status transitions are not represented as structured history. Undo must preserve the complete audit trail and must not silently guess or erase a prior transition.

## Decision

Add a follow-on SQLite migration creating an append-only status_transition table keyed to record_revision by (record_id, revision), with explicit from_status, to_status, and transition_kind values (forward or undo). Every new normal status transition writes a forward row in the same transaction as the engineering_record update and revision row. status --undo is eligible only when the record latest revision is a forward status transition and the current status exactly matches the audited to_status. It returns to the exact audited from_status, writes a normal status revision and an undo row, and reports the undo in history. This is an evidence-authorized reversal of one recorded edge, not a new forward lifecycle transition. Any later revision or an existing undo makes the prior transition ineligible; an undo cannot be undone. Transitions that predate this migration have no audit row and are explicitly not undoable.

## Considered Options

Infer the predecessor from the lifecycle graph; rejected because statuses such as accepted can have multiple predecessors and inference would fabricate history. Add status fields to every revision row and backfill old rows; rejected because historical status values cannot be reconstructed accurately, while a dedicated transition table can preserve a precise boundary. Allow unconditional undo of the most recent transition or toggle repeatedly; rejected because later mutations may make the old transition no longer the operation being corrected and repeated toggling obscures the audit trail. Delete the mistaken transition from history; rejected because history must remain complete and honest.

## Consequences

Undo is deterministic, auditable, transactional, and safe for transitions created after the migration. Existing databases retain all data and migrate without fabricated transition history, but their old status transitions cannot be undone because their predecessors were not persisted. The status command has mutually exclusive transition, list, and undo modes. The implementation adds the V3 migration, dump and restore support for status_transition, store mutations, CLI validation, history summaries, and regression tests.

## Evidence

RFC-0009 and this EDR; store/migrations/V3__add_status_transition_audit.sql; store/src/mutations.rs status_transition writes and undo_status transaction; store/src/dump.rs and store/src/restore.rs audit-table projection support; cli/src/args.rs and cli/src/commands/auxiliary.rs undo mode. cargo test --workspace, cargo clippy --workspace --all-targets -- -D warnings, and cargo fmt pass. An isolated binary smoke test confirmed forward transition, immediate undo to the exact predecessor, explicit history output, repeated-undo rejection, and rejection after a later retitle revision.

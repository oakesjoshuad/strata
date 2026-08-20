---
id: "EDR-0010"
title: "strata link accepts multiple targets in one atomic transaction; multi-source batches deferred"
record-type: edr
status: accepted
revision: 3
date: 2026-08-20
slug: strata-link-accepts-multiple-targets-in-one-atomic-transaction-m
tags: []
relationships:
  relates-to:
    - "ADR-0016"
---

# EDR-0010: strata link accepts multiple targets in one atomic transaction; multi-source batches deferred

## Context

`strata link <source> <relation> <target>` (`cli/src/args.rs`'s `Link` command, `cli/src/main.rs:168`) creates exactly one relationship per invocation. `Store::link` (`store/src/mutations.rs:92-113`) matches that shape: it validates the relation kind and the source/target pair (`validate_relationship`, `records/src/validation.rs:134-144`), confirms both records exist, and inserts one row inside its own transaction.

In practice, linking one record to several others in the same relation is common -- this session alone just linked EDR-0009 to three targets (`relates-to ADR-0016`, `relates-to ADR-0015`, `relates-to ADR-0014`) as three separate CLI invocations. Each call opens and commits its own transaction, so a failure partway through a manual batch (a typo'd id, an invalid relation for the third target) leaves the first N links committed and no record of which ones succeeded -- exactly the kind of partial, silently-diverging state CLAUDE.md rule 7 and this project's existing rollback test (`store/src/mutations.rs`'s `revise_rolls_back_completely_when_a_later_statement_fails`) exist to prevent elsewhere in the codebase. `link` has no equivalent test today because it has never needed one: a single insert is already atomic by virtue of being one statement.

This is a small, implementation-level ergonomics decision within an already-established boundary (one source, one relation, N targets is still "one relationship kind between one record and others," not a new graph concept), so it is EDR-scope under ADR-0016's classification test, not an ADR.

## Decision

Add `Store::link_many(&mut self, source: &RecordId, relation: &str, targets: &[RecordId]) -> Result<Vec<Relationship>, StoreError>`. It opens one transaction, then for each target in order: validates the source/relation/target triple with the same `validate_relationship` call `link` already uses, confirms the target exists, and inserts the row. If any target fails validation, does not exist, or collides with an existing edge (the `PRIMARY KEY(source_id, relation, target_id)` constraint in `store/migrations/V1__initial.sql:18` already rejects duplicates today), the whole transaction is dropped without committing and none of the batch's links are created -- all or nothing, matching `revise`'s existing rollback behavior. On success it commits once and returns every created `Relationship` in input order.

`Store::link` (`store/src/mutations.rs:92`) becomes a thin wrapper: `self.link_many(source, relation, std::slice::from_ref(target)).map(|mut v| v.remove(0))`. This removes the duplicated validate-check-insert body rather than adding a second copy of it, and keeps `link`'s existing signature and every existing caller (`cli/src/main.rs:168`, `store/src/validate.rs:102`, `store/src/dump.rs:158`, `store/src/restore.rs:186-192`, and existing tests) unchanged.

`cli/src/args.rs`'s `Link` command changes its `target: String` field to `targets: Vec<String>` with `#[arg(required = true, num_args = 1..)]`, so `strata link <source> <relation> <target>` continues to work exactly as today (a one-element vector) and `strata link <source> <relation> <target> [<target> ...]` becomes valid. `cli/src/main.rs` parses each target string to a `RecordId`, then calls `store.link_many`. Output: with `--json`, an array of the created relationships (empty-vs-single-vs-many all use the same array shape, no special case for the one-target path); without it, one confirmation line per created link, matching today's single-link output repeated per target.

## Considered Options

Full cross-linking -- multiple sources and multiple targets in one call -- considered and rejected for this EDR, per the discussion that prompted it. Two different semantics are both plausible for "N sources, M targets" and neither is obviously correct without an explicit flag to disambiguate: a zip (pairwise, `sources[i]` links to `targets[i]`, requiring equal lengths) reads naturally for "link each of these new EDRs to its corresponding upstream record," while a cross product (every source to every target) reads naturally for "link this whole batch of EDRs to that whole batch of ADRs." Silently picking one would violate rule 7 the same way an unrelated silent default would; the one case with no ambiguity is a single source broadcast across many targets (or symmetrically, many sources into one target), which is exactly this EDR's scope.

There is also a concrete parser-level obstacle, not just a semantic one: clap's positional-argument model supports one trailing multi-value (`Vec<...>`) positional per command. `strata link <sources>... <relation> <targets>...` cannot be expressed as two adjacent greedy positionals -- clap has no way to know where the source list ends and the relation begins. A real multi-source form would need named, repeatable flags instead of bare positionals (e.g. `strata link --source A --source B <relation> --target C --target D`), which is a bigger CLI surface change than this EDR's ergonomics fix, and changes the existing `Link` command's argument shape rather than extending it. If a concrete need for multi-source batches appears later, that -- flag-based sources and targets, zip when lengths match, hard error rather than an inferred cross product when they don't -- is the shape a follow-on EDR should specify; this EDR intentionally does not build it now, since no real caller needs it yet and speculative flag surface is exactly what CLAUDE.md's general guidance already warns against.

A CLI-only batch (loop calling the existing single-target `store.link` once per target, no new store method) -- rejected: this is the status quo in a thin wrapper, and reproduces the exact partial-failure problem this EDR exists to close, since each `store.link` call commits its own transaction independently.

## Consequences

`store/src/mutations.rs` gains `link_many`; `link` shrinks to a one-line delegation. `cli/src/args.rs`'s `Link.target` becomes `Link.targets: Vec<String>`; `cli/src/main.rs`'s link-dispatch arm parses and calls `link_many`. `cli/src/output.rs` gains a small-array print path alongside the existing single-relationship one. A new test, modeled on `revise_rolls_back_completely_when_a_later_statement_fails`, should seed a batch where a later target is invalid and assert none of the earlier, individually-valid targets were linked. Existing single-target callers and tests are unaffected. `strata capabilities --json`'s description of `link` should note it now accepts multiple targets.

## Evidence

`cli/src/args.rs`'s current `Link` command and `cli/src/main.rs:168`'s dispatch, the single-target shape this EDR extends. `store/src/mutations.rs:92-113` (`Store::link`), the implementation this EDR wraps rather than duplicates. `records/src/validation.rs:134-151` (`validate_relationship`, `validate_relation_kind`), reused unchanged per target. `store/migrations/V1__initial.sql:14-19`, the `record_relation` table's primary key, which already rejects duplicate edges and needs no new constraint. `store/src/mutations.rs`'s `revise_rolls_back_completely_when_a_later_statement_fails`, the direct precedent this EDR's atomicity guarantee follows. ADR-0016, the classification test this EDR is scoped under. This conversation, where the multi-target-only scope (explicitly excluding cross-linking) and the atomic-all-or-nothing requirement were both stated directly by the project owner.

---
id: "EDR-0011"
title: "strata validate folds export and dump staleness checks into one command"
record-type: edr
status: draft
revision: 1
date: 2026-08-20
slug: strata-validate-folds-export-and-dump-staleness-checks-into-one-
tags: []
relationships:
  relates-to:
    - "ADR-0015"
    - "ADR-0016"
---

# EDR-0011: strata validate folds export and dump staleness checks into one command

## Context

Specification section 29 lists "stale projections" and "missing referenced evidence" among the checks `validate` should eventually cover. `store/src/validate.rs`'s `Store::validate` already covers duplicate identifiers, malformed documents, invalid/broken relationship kinds and targets, and supersession inconsistencies (plus ADR-0015's advisory lineage warning) -- but nothing in it, or anywhere else `strata validate` touches, checks whether the committed Markdown under `docs/records/` or the SQL snapshot at `docs/db-snapshot.sql` still match canonical state. "Missing referenced evidence" turns out to already be structurally impossible: `evidence.record_id` and `code_reference.record_id` both carry `REFERENCES engineering_record(id)` (`store/migrations/V1__initial.sql:33,42`), enforced because every connection sets `foreign_keys=ON` per this project's own SQLite connection convention -- an orphaned evidence or code-reference row cannot exist. That leaves stale projections as the one real, actionable gap in section 29's list.

The detection logic already exists, just not inside `validate`. `cli/src/export.rs`'s `differences` (`cli/src/export.rs:55-91`) and `cli/src/dump.rs`'s `difference` (`cli/src/dump.rs:42-50`) already compute exactly this -- each is the function `export --check`/`dump --check` already call to decide pass or fail. Every record created in this project's own history has required manually chaining `export --check` and `dump --check` after `validate` to be sure nothing drifted; CLAUDE.md's own record-keeping discipline says as much explicitly ("always re-export/dump before committing"). `strata validate` reporting clean while a projection is actually stale is a real, currently-live gap, not a hypothetical one.

## Decision

`export.rs` and `dump.rs` each gain one new `pub(crate)` function that returns already-formatted issue strings instead of printing and erroring directly: `export::stale_messages(store: &Store, root: &Path) -> Result<Vec<String>, CliError>` (wrapping the existing `rendered_records` + `differences` + `difference_message` pipeline at `cli/src/export.rs:38-101`) and `dump::stale_messages(store: &Store, path: &Path) -> Result<Vec<String>, CliError>` (wrapping `difference` + `difference_message` at `cli/src/dump.rs:42-58`). Neither `Difference` enum becomes `pub(crate)` or crosses its module boundary -- the new function is the only new surface, keeping each module's internal representation private. `export::run`/`dump::run` are otherwise unchanged; `--check` keeps behaving exactly as it does today, including its own independent messages and exit code, for anyone who wants to check just one projection.

`Command::Validate`'s dispatch (`cli/src/commands/auxiliary.rs:191-211`) calls both new functions using the already-resolved `config.export_target.value` and `config.dump_target.value` (the same `ResolvedConfig` already threaded into this function's signature) and extends `store.validate()`'s issue list with whatever they return, before the existing WARN/ERROR-split logic runs. No new severity concept is introduced: a stale-projection message is not prefixed `WARN `, so it falls through the existing `issue.starts_with("WARN ")` branch (`cli/src/commands/auxiliary.rs:193,200-204`) and is treated as an ERROR -- consistent with `export --check`/`dump --check` already exiting non-zero on any difference today. `strata validate` therefore starts failing (non-zero exit, `"valid": false` in JSON) whenever either projection has drifted, exactly matching what running both `--check` commands separately would already tell you.

`store/src/validate.rs` and `Store::validate`'s signature are untouched -- staleness is inherently filesystem-aware (it reads `docs/records/` and `docs/db-snapshot.sql` off disk), so per rule 5's I/O boundary this stays CLI-composition-root logic layered on top of the store's DB-only checks, the same shape EDR-0009's `context` command already established for composing existing CLI-owned pieces without a new store abstraction.

## Considered Options

Move the staleness comparison into `store` so `Store::validate` covers it directly -- rejected. `store` has no I/O beyond value-object `ToSql`/`FromSql` per rule 5; reading `docs/records/*.md` and `docs/db-snapshot.sql` off disk is squarely CLI-adapter territory, the same boundary that already keeps `export`/`dump`'s file comparison logic in `cli` today.

Make `Difference` and the comparison functions fully `pub(crate)` across `export.rs`/`dump.rs` so `auxiliary.rs` matches on the enum directly and builds its own messages -- rejected. It would duplicate the message-formatting `difference_message` already owns in each module and spread one concept (what "stale" means for this projection) across two places. A single `stale_messages` function per module that returns finished strings keeps the enum and its formatting private to the module that understands it.

Treat stale-projection findings as `WARN` rather than `ERROR` -- rejected. `export --check` and `dump --check` already exit non-zero on any difference; folding the same check into `validate` and downgrading its severity there would make the combined command less strict than running the two standalone commands it replaces, which is the opposite of the consolidation this EDR is for.

A `--skip-projections` flag to opt out of the new checks -- rejected for now. No concrete caller needs partial validation, and a flag whose only purpose is to make `validate` lie about drift by omission cuts against why this EDR exists. If a real need appears later, that's a small follow-on decision, not a reason to hold this one.

## Consequences

`cli/src/export.rs` and `cli/src/dump.rs` each gain one new `pub(crate)` function; both `run` functions are unchanged. `cli/src/commands/auxiliary.rs`'s `Validate` arm grows to call both and merge their output into the existing issues list before the existing WARN/ERROR split. `strata validate` alone is now sufficient to catch drift that previously required also running `export --check` and `dump --check`; those two commands remain available unchanged for anyone who wants to check or fix just one projection. A repository that has never run `export`/`dump` at all (a fresh `strata init` with existing records but no generated Markdown or snapshot yet) will now see `validate` fail with `missing ...` messages for every expected file -- this is correct per this EDR's decision (a missing projection is exactly as stale as a wrong one), but is a visible behavior change worth calling out: `validate` was previously silent about this.

## Evidence

`docs/specification.md:1033-1044` (section 29's validation-coverage list, specifically "stale projections" and "missing referenced evidence"). `store/migrations/V1__initial.sql:33,42`, the foreign-key constraints that make "missing referenced evidence" already structurally impossible. `store/src/validate.rs`, the existing DB-only validation this EDR extends without modifying. `cli/src/export.rs:38-101` (`rendered_records`, `differences`, `difference_message`) and `cli/src/dump.rs:42-58` (`difference`, `difference_message`), the existing staleness-detection logic this EDR reuses rather than reimplements. `cli/src/commands/auxiliary.rs:191-211`, the `Validate` dispatch arm this EDR extends. EDR-0009, the precedent for CLI-composition-root features that reuse existing pieces without a new store abstraction. ADR-0016, the classification test this EDR is scoped under. CLAUDE.md's record-keeping discipline section, which already requires running `export --check`/`dump --check` by hand before every commit -- the manual step this EDR automates into one command.

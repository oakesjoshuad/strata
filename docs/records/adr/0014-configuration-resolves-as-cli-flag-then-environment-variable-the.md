---
id: "ADR-0014"
title: "Configuration resolves as CLI flag, then environment variable, then config file, then default"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: configuration-resolves-as-cli-flag-then-environment-variable-the
tags: []
relationships:
  relates-to:
    - "ADR-0013"
---

# ADR-0014: Configuration resolves as CLI flag, then environment variable, then config file, then default

## Context

RFC-0003 (proposed) scoped a git-backed configuration mechanism but explicitly left precedence unresolved as its Q3: "When a configuration value, an environment variable, and `--database` are all present, which wins? Are environment variables needed at all?" Three hardcoded, non-configurable constants now exist and keep growing: `cli/src/main.rs`'s `DB_PATH` (`.strata/strata.db`), `cli/src/export.rs`'s `Path::new("docs/records")`, and `cli/src/dump.rs`'s `DUMP_PATH` (`docs/db-snapshot.sql`, added after RFC-0003 was written) -- the drift RFC-0003 was written to stop is still accumulating while its open questions remain unanswered.

In discussion, the project owner proposed a CLI-over-environment-over-defaults precedence. Neither a CLI flag nor an environment variable is git-backed on its own, though: RFC-0003's own motivation is that "a team cannot commit an agreed database location... as configuration," and an environment variable no more solves that than the status quo does -- it is host/session-local, not reviewable in a pull request. The discussion surfaced this directly, and the project owner confirmed keeping a git-tracked configuration file in the chain (rather than dropping it for a simpler two-tier CLI/environment-only model) precisely so the mechanism still serves RFC-0003's original problem.

## Decision

Configuration values resolve in this order, highest precedence first:

1. An explicit CLI flag for that invocation (e.g. `--database`).
2. An environment variable specific to that setting, prefixed `STRATA_` followed by the setting name in `SCREAMING_SNAKE_CASE` (e.g. a database-path setting reads `STRATA_DATABASE`). This naming convention is fixed now even though RFC-0003's Q5 (which settings ship in the first wave) is still open -- the pattern applies regardless of which settings eventually populate it.
3. A git-tracked configuration file's value.
4. Strata's hardcoded built-in default (today's `DB_PATH`, `docs/records`, `DUMP_PATH` constants).

This resolves RFC-0003's Q3 in full: environment variables are needed, and the order is CLI, then environment, then file, then default. It also partially informs Q6 (crate ownership): resolution logic that reads a CLI argument, an environment variable, and a config file to produce one effective value is CLI composition-root behavior, following the same pattern `cli/src/main.rs`'s existing `cli.database.unwrap_or_else(|| PathBuf::from(DB_PATH))` already establishes for `--database` alone, not something `records` or `store` should own (rule 5's I/O boundary; ADR-0006's rejection of a repository trait applies equally to a configuration-resolution trait). Whether any part of it belongs in `kernel` (Q7) remains open.

This decision does not fix the configuration file's format, location, or discovery mechanism (RFC-0003 Q1/Q2), the malformed-configuration failure behavior (Q4), or the first-wave setting scope (Q5). Those remain open for a follow-on EDR, which this ADR's four-tier order and environment-variable naming convention are written to accommodate regardless of how they're eventually answered.

## Considered Options

CLI, then environment, then hardcoded default, with no file layer at all -- rejected: this was the alternative directly on the table in discussion, and rejected specifically because it resolves precedence for a mechanism that no longer does what RFC-0003 exists to do. Neither a flag nor an environment variable can be committed and reviewed the way a tracked file can; dropping the file layer trades away the RFC's actual motivation for a simpler design.

File, then environment, then CLI (the git-tracked default wins) -- rejected: a committed default should never override a value the invoking user explicitly typed for that specific run. Every reference CLI tool with a layered configuration model (kubectl, docker, aws-cli, git) puts the explicit flag first for the same reason; reversing it would surprise anyone who already knows those tools, which is exactly the unfamiliar-workflow risk raised earlier in this project's own research into external process conventions.

No environment-variable tier, just CLI then file then default -- rejected: this forecloses a real, distinct use case -- a CI job or a shared shell profile overriding a value for a session or host without either editing a committed file or retyping a flag on every invocation. CLI, file, and environment each answer a different question (this run, the team's agreed default, this machine/session), and collapsing to two tiers loses one of them.

## Consequences

Every future configurable setting -- database path, export target, the dump-snapshot path, and whatever RFC-0003 Q5 eventually adds -- follows the same four-tier resolution instead of each command inventing its own override story, directly addressing the drift already visible across three separately hardcoded constants. The CLI composition root gains a general resolution step ahead of dispatch, generalizing the pattern `--database` already uses today. No code changes yet: `cli/src/main.rs`, `cli/src/export.rs`, and `cli/src/dump.rs` are unmodified by this ADR. Implementation waits on RFC-0003's remaining open questions (format, location, malformed-file behavior, first-wave scope), which a follow-on EDR must settle before this order can actually be wired up.

## Evidence

RFC-0003 (`docs/records/rfc/0003-...md`), specifically Q3 and Q6, the open questions this ADR answers and partially informs. `cli/src/main.rs`'s existing `--database` resolution: the direct precedent for where this logic belongs. `cli/src/export.rs`'s hardcoded `"docs/records"` and `cli/src/dump.rs`'s `DUMP_PATH` constant (`docs/db-snapshot.sql`, added by ADR-0013's implementation after RFC-0003 was written): concrete evidence the hardcoded-constant problem is still growing, now three instances instead of the two RFC-0003 originally cited. This conversation: the project owner proposed CLI-then-environment-then-defaults; discussion surfaced that only a git-tracked file satisfies RFC-0003's original motivating problem, and the owner confirmed keeping that file layer (Option A) rather than dropping it for a simpler model.

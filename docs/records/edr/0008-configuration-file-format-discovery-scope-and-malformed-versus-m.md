---
id: "EDR-0008"
title: "Configuration file format, discovery, scope, and malformed-versus-missing handling"
record-type: edr
status: accepted
revision: 3
date: 2026-08-19
slug: configuration-file-format-discovery-scope-and-malformed-versus-m
tags: []
relationships:
  relates-to:
    - "ADR-0013"
    - "ADR-0014"
    - "EDR-0006"
---

# EDR-0008: Configuration file format, discovery, scope, and malformed-versus-missing handling

## Context

ADR-0014 fixed the precedence order (CLI flag, then environment variable, then config file, then default) for RFC-0003's git-backed configuration mechanism, but deliberately left the file's format, location, discovery, first-wave scope, and malformed-versus-missing behavior open -- RFC-0003's Q1, Q2, Q4, Q5, and Q7. `cli/src/config.rs` implements the mechanism and settles all of them at once, the same way EDR-0006 retroactively documented the evidence-id allocation scheme after `store/src/evidence.rs` shipped it: the concrete mechanics were decided during implementation rather than in a separate design pass first, and this EDR records that decision after the fact rather than before it.

## Decision

The configuration file is JSON (`cli/src/config.rs:44-50`'s `FileConfig`, deserialized with `serde_json`), using the dependency already present in the workspace rather than adding TOML or another format. It is named `strata.config.json` (`cli/src/config.rs:10`) and lives at the repository root, discovered by walking up from the current working directory looking for a `.git` marker (`cli/src/config.rs:158-167`'s `repository_root`) -- the same discovery strategy git, npm, and cargo themselves use, so a user already familiar with those tools does not need to learn a new convention. `FileConfig` is deserialized with `#[serde(deny_unknown_fields)]` (`cli/src/config.rs:45`), so a config key that does not match a known setting is an explicit error, not silently ignored.

First-wave scope is exactly three settings: `database`, `export_target`, and `dump_target` (`cli/src/config.rs:38-42`'s `ResolvedConfig`), matching RFC-0003's original two candidates (database path, export target) plus `dump_target`, the third hardcoded constant that appeared after RFC-0003 was written but before this EDR, closing RFC-0003's Q8 (ADR-0013 coordination) as a side effect: the dump path is not a separately bolted-on special case, it goes through the identical four-tier mechanism as the other two. Default output format and search `--limit` remain out of scope, per RFC-0003's own instruction not to expand scope casually.

A config file that exists but fails to parse, or that contains an unrecognized field, is a hard `CliError` (`cli/src/config.rs:179-182`'s `load_file`, propagating the `serde_json::Error`; the `deny_unknown_fields` case above). A config file that does not exist at all is not an error -- `load_file` returns `Ok(None)` on `ErrorKind::NotFound` (`cli/src/config.rs:174-178`) and resolution falls through cleanly to the next tier. These two cases are kept structurally distinct rather than collapsed into one "config problem" branch, per CLAUDE.md rule 7.

Relative paths behave differently depending on which tier supplied them. A path from the config file is resolved against the repository root (`cli/src/config.rs:130-141`'s `resolve_path`, joining `repository_root` when the file-sourced path `is_relative()`), because a committed file conceptually describes policy for the whole repository regardless of which subdirectory `strata` is invoked from. A path from a CLI flag or an environment variable is left exactly as given, relative to the current working directory if it is relative at all -- preserving `--database`'s pre-existing behavior (the old `cli.database.unwrap_or_else(|| PathBuf::from(DB_PATH))` never anchored to anything but the process's own CWD) rather than silently changing what an existing flag does.

Kernel is not involved (RFC-0003's Q7): all of this is CLI composition-root logic in a new `cli/src/config.rs`, following rule 5's I/O boundary and ADR-0006's rejection of a repository/config abstraction trait, the same reasoning ADR-0014 already applied to crate ownership.

## Considered Options

YAML instead of JSON -- rejected: RFC-0003 asked specifically to compare the two already-present dependencies (`serde_json`, `serde_yaml`) before reaching for a new one; JSON was chosen over YAML for the config file specifically because YAML is already committed to a different, unrelated purpose in this codebase (Markdown frontmatter, ADR-0009) and reusing it here would blur two structurally different documents that happen to share a parser. TOML -- rejected per RFC-0003's own instruction not to add a third format without a concrete reason, and none was found.

Discover the config file only in the current working directory, with no upward search -- rejected: this would silently do nothing for the common case of running `strata` from a subdirectory of a real project, which is exactly the "invocation from a subdirectory" scenario RFC-0003's Q2 named directly.

Anchor CLI- and environment-sourced relative paths to repository root as well, for uniformity with the file tier -- rejected: `--database` already had CWD-relative behavior before this change; changing it as a side effect of adding two new tiers underneath it would be a silent behavior change to existing, working functionality, not something this EDR's scope justifies.

Treat a missing config file the same as a malformed one (both an error) -- rejected: RFC-0003's Q4 explicitly asked for these to be evaluated separately, and collapsing them would make the configuration file mandatory in every repository, contradicting the whole point of a fallback chain that ends in a hardcoded default.

## Consequences

`cli/src/config.rs` is the single place all three first-wave settings resolve through; `cli/src/main.rs`, `cli/src/export.rs`, and `cli/src/dump.rs` no longer contain their own hardcoded path constants. `strata capabilities --json` reports each setting's resolved value and which tier produced it, closing the self-description gap RFC-0003's motivation named. `strata export` and `strata dump` both gained a `--target` flag they did not have before, giving every first-wave setting a complete four-tier story rather than three tiers for two settings and only two tiers (env, default) for the third. Because `deny_unknown_fields` is strict, adding a fourth configurable setting later requires updating `FileConfig` and the setting's schema at the same time as any documentation describing it, rather than letting a stale or misspelled key pass through silently.

This EDR closes RFC-0003's Q1, Q2, Q4, Q5, Q7, and Q8. Combined with ADR-0014's resolution of Q3 and Q6, every open question RFC-0003 raised now has an answer. RFC-0003 itself is not modified by this EDR; its own outcome should be revised separately to point at both resolving records before it is considered for acceptance.

## Evidence

`cli/src/config.rs`, the implementation this EDR documents after the fact, specifically lines 10 (`CONFIG_FILE`), 38-50 (`ResolvedConfig`/`FileConfig`), 58-96 (`resolve`), 98-147 (`resolve_path`), 158-167 (`repository_root`), and 169-182 (`load_file`). RFC-0003 (`docs/records/rfc/0003-...md`), Q1/Q2/Q4/Q5/Q7/Q8, the open questions this EDR answers. ADR-0014, the precedence decision this EDR's mechanics implement. EDR-0006, the direct precedent for documenting an implementation-time mechanics decision retroactively rather than gating it on a design pass first. ADR-0009, the prior decision establishing YAML's role in this codebase (Markdown frontmatter), cited as the reason JSON rather than YAML was chosen for a structurally unrelated document.

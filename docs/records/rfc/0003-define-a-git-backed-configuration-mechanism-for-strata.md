---
id: "RFC-0003"
title: "Define a git-backed configuration mechanism for Strata"
record-type: rfc
status: proposed
revision: 2
date: 2026-08-19
slug: define-a-git-backed-configuration-mechanism-for-strata
tags: []
relationships:
  produces:
    - "ADR-0014"
  relates-to:
    - "ADR-0001"
    - "ADR-0013"
---

# RFC-0003: Define a git-backed configuration mechanism for Strata

## Motivation

Issue #13 proposes a design pass before Strata accumulates more independent path and invocation conventions. The CLI currently defaults its canonical database path through `const DB_PATH: &str = ".strata/strata.db"` at `cli/src/main.rs:25`, selecting that value at `cli/src/main.rs:57` whenever `--database` is absent. Markdown export independently fixes its destination with `Path::new("docs/records")` at `cli/src/export.rs:19`, and there is no flag or environment override for that target. These defaults are reasonable for this repository but are not a shareable workspace policy: a team cannot commit an agreed database location or projection layout as configuration, and every caller or future automation must rediscover the values from source.

ADR-0013 adds a second projection for full-fidelity SQL backup and restore while deliberately leaving the committed dump file location open. If dump/restore chooses its own path independently, Strata will have several project-layout decisions that are related in purpose but unrelated in mechanism. A git-backed configuration RFC can establish the design boundary before that future output path becomes another isolated constant.

## Problem

Strata has canonical state in SQLite and generated projections in the repository, but the locations that connect those two worlds are partly compiled into the CLI and partly fixed by command implementation. The `.strata/self-host.db` database is gitignored by commit ef91691, so placing a future team-shared configuration beside it without examining that ignore boundary would make the phrase git-backed configuration self-contradictory. Conversely, putting configuration in a tracked repository location raises questions about repository-root discovery, nested workspaces, invocation from a subdirectory, and whether the configuration is project policy or user-local state.

The existing command line already supplies one explicit override, `--database`, while export has no corresponding override. A configuration layer would also need to define whether environment variables participate, how explicit command-line values interact with committed values, and whether malformed or undecodable configuration stops the command. CLAUDE.md rule 7 requires decode and validation failures to remain explicit errors; a malformed configuration cannot silently become the current hardcoded default or an empty configuration. The implementation must also respect the crate boundaries: `records` is a value/domain crate with no general I/O, `store` owns SQLite access rather than repository abstractions, and `cli` is the composition root.

## Scope

This RFC scopes one git-backed configuration mechanism for workspace-level Strata settings and the resolution boundary through which commands consume it. The immediate configuration candidates are the default SQLite database path currently named by `cli/src/main.rs:25` and the Markdown export target currently named by `cli/src/export.rs:19`. The RFC also requires the future dump/restore output path from ADR-0013 to be considered as a related consumer, so that its implementing decision can either use this mechanism or explicitly justify why it does not.

The RFC inventories but does not initially require configuration for a default JSON-versus-human-readable output preference or the default search `--limit` currently declared in `cli/src/args.rs`. Those are plausible policy values, but adding them to the first configuration contract could turn a path-resolution question into an unrestricted settings registry. Follow-on work may revisit them after the initial boundary, format, and precedence questions are settled.

## Non-Goals

This RFC does not implement configuration loading, change `--database`, add an export flag, add dump or restore, or alter the `.gitignore` policy. It does not choose a file format, a repository location, an environment-variable naming scheme, a precedence order, or a malformed-file recovery behavior. It does not decide whether output preference or search limit belong in the first version. It does not create an ADR or EDR for any of those choices.

This RFC does not move SQLite connection ownership into `records`, introduce a repository port trait, or make `kernel` a general-purpose application configuration crate. It does not change the canonical-store decision in ADR-0001, the projection boundary in ADR-0003, or the full-fidelity backup purpose established by ADR-0013.

## Constraints

The canonical database remains SQLite state and Markdown remains a generated projection, consistent with ADR-0001 and ADR-0003. A configuration mechanism must work for both the self-hosted workspace and a separately checked-out project that invokes the binary from a meaningful repository context. Its tracked portion must be compatible with Git review and branch changes; it must not quietly live under the gitignored `.strata/` directory merely because the default database does.

Configuration decoding and semantic validation must produce explicit `CliError`-visible failures under CLAUDE.md rule 7. A missing optional configuration file, if the eventual design permits one, is a separate case from a present malformed file and must not be conflated without an explicit decision. Any precedence involving the existing `--database` flag must preserve the ability to override a committed default for one invocation.

The eventual implementation must keep general configuration I/O out of `records`, must not add a speculative storage trait to `store` under ADR-0006, and must leave `Store` responsible for concrete SQLite behavior. `kernel/src/lib.rs:1` currently exposes the shared `build` module and value-level SQL helpers; `kernel/src/build.rs:3-31` derives the build identity by invoking Git during compilation. Whether configuration loading is similarly cross-crate infrastructure or is only CLI composition-root behavior remains an architectural question, not an assumption this RFC settles. The mechanism must remain synchronous and single-process, with no async or thread-safety machinery.

## Proposal

Create a follow-on design sequence for a single workspace configuration surface that can supply, at minimum, the default database path and Markdown export target, and that reserves a principled extension point for ADR-0013's future dump-file path. Begin with a written inventory and precedence matrix, then resolve each independently acceptable choice in its own ADR or EDR before implementation. The resulting command behavior should make its effective configuration explainable to an agent and should preserve explicit command-line control where a command already has it.

The first implementation decision should examine the existing serde_json and serde_yaml dependencies before adding a third configuration format. It should compare a tracked file at repository root with locations adjacent to, but not inside, `.strata/`, including repository-root discovery and nested invocation behavior. It should define the relationship among committed configuration, any environment variables, and `--database`, including explicit errors for malformed data and a testable rule for absent data. It should separately classify database path and export target as first-wave settings, dump-file location as a coordinated future consumer, and output preference/search limit as later candidates unless review finds a compelling reason to include them now.

The implementation owner should be selected after the crate-boundary review. The likely integration point is the CLI composition root because it already selects a database path before constructing `Store` at `cli/src/main.rs:55-75`, but the RFC intentionally leaves open whether reusable parsing/value types belong in `kernel`, whether a CLI-local module is sufficient, and how to avoid giving `records` or `store` responsibilities they do not own. Kernel's existing build identity is evidence to inspect, not evidence that runtime project configuration belongs there.

## Alternatives Considered

Keep both values hardcoded and require each future command to add its own flags. This preserves the current small surface but leaves team policy undiscoverable, duplicates path decisions, and guarantees that dump/restore can drift from export and database conventions.

Use environment variables only. This can support local automation without adding a tracked file, but it cannot express a reviewable workspace policy in Git and makes effective configuration dependent on process state that is not visible in the repository. It also leaves precedence and malformed-value behavior to be designed separately for every command.

Add only more command-line flags, such as an export-root flag and a dump-file flag. This improves one invocation but does not provide a shareable default, and it creates a growing set of command-specific path grammars instead of one configuration boundary.

Put the configuration file under `.strata/` beside the default database. This is operationally convenient but conflicts directly with the stated git-backed requirement because `.strata/` is gitignored by commit ef91691. A file there would be local state unless the ignore policy were changed as a separate, explicit decision.

Choose a new format immediately, such as TOML, because it is commonly used for project configuration. That may be a reasonable outcome, but selecting it before comparing the already-present JSON and YAML dependencies would turn an open design tradeoff into an implementation habit and add a third serialization contract without review.

## Open Questions

The follow-on records should answer these questions explicitly:

1. **File format:** Should configuration use serde_json, serde_yaml, or a newly added format such as TOML? What readability, comments, dependency, schema, and error-reporting tradeoffs matter for a Git-reviewed engineering tool?
2. **File location and discovery:** Should the tracked file live at repository root, in an existing tracked directory, or in another location? How is repository root found when invoked from a subdirectory, and how is that location kept distinct from gitignored `.strata/`?
3. **Precedence:** When a configuration value, an environment variable, and `--database` are all present, which wins? Are environment variables needed at all, and do export and future dump commands get equivalent explicit overrides?
4. **Malformed configuration:** What happens when the file exists but cannot be decoded, contains an unknown field, or contains an invalid path? The answer must be an explicit error rather than a silent hardcoded/default fallback under CLAUDE.md rule 7. What happens when the optional file is absent must be specified separately.
5. **First-wave scope:** Are database path and export target the only values configurable now? Should the future ADR-0013 dump-file path be part of the same first contract or merely reserved for a later consumer? Should default JSON/text output preference and default search limit remain later candidates?
6. **Crate ownership:** Which crate owns configuration value types, file/env decoding, and effective-value resolution? How can the design keep I/O out of `records`, avoid a repository trait in `store`, and leave the CLI as the composition root that constructs `Store`?
7. **Kernel interaction:** Is runtime configuration a cross-crate concern appropriate for `kernel`, analogous to build identity, or is it CLI-only policy? If kernel is involved, what narrowly scoped shared responsibility justifies that dependency rather than a CLI module?
8. **ADR-0013 coordination:** Where should the future SQL dump/restore path be configured, and what guarantees keep dump output, Markdown export, and the canonical database path understandable without conflating their different projection purposes?

## Outcome

Proposed. This RFC asks the project to pursue a git-backed configuration design for the database path and Markdown export target, while coordinating the future ADR-0013 dump-file path and explicitly deferring broader preferences such as default output mode and search limit unless review promotes them into scope. It intentionally resolves none of the format, location, precedence, malformed-file, crate-ownership, or kernel questions above; those are independent design choices for follow-on ADRs or EDRs. Human review should confirm the scope and split before implementation begins.

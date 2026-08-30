# AGENTS.md — Strata

## What this project is

Strata is a local-first Rust CLI backed by a SQLite knowledge base for tracking
engineering reasoning — RFCs, PDRs, ADRs, EDRs, Specifications, Assessments,
Research, Glossary entries, and Risks — as structured, queryable
records instead of hand-maintained Markdown files edited by convention. Markdown
is a generated projection, not the source of truth. See `docs/specification.md`
for the original design specification and `docs/records/` for Strata's own
founding records (start with RFC-0001 and PDR-0001) — those explain *why* each
rule below exists, this file is the condensed, enforceable version.

Strata is infrastructure meant to record the journey of other projects (starting
with the planned `substruct` rebuild) — it is not itself an event-sourced,
multi-bounded-context system, and most of the machinery that shape implies does
not apply here. Read the rules below as what they are: a much smaller, single-
application version of conventions distilled from a larger sibling project
(`substrate`), not a blind copy of it.

## Non-negotiable rules

1. **Errors use `thiserror` only. `anyhow` is banned.** Do not add it to any
   `Cargo.toml` in this workspace, including as a dev-dependency.

2. **No `dyn` dispatch, no `Box<dyn _>`, anywhere.** Storage has exactly one
   implementation (SQLite) and one caller (the CLI) — see ADR-0002 and ADR-0006.
   There is no repository trait to abstract over; `store` exposes concrete
   functions and `cli` calls them directly. `RecordKind` is a closed nine-variant
   enum (`Rfc`, `Pdr`, `Adr`, `Edr`, `Specification`, `Assessment`, `Research`,
   `Glossary`, `Risk`); any per-kind behavior is an exhaustive
   `match`, never a registry or a trait-object dispatch table. If a tenth kind is
   ever added, the compiler must fail to build until every match site is updated.

3. **No async, no tokio, no `Send`/`Sync` bounds.** Strata is a single-threaded,
   single-process CLI: open one connection, run one command, exit. `rusqlite::
   Connection` is deliberately `!Sync` — that already matches this shape exactly,
   so do not add thread-safety machinery nothing in the program needs.

4. **Reader SQL uses named parameters only.** Never interpolate values into a SQL
   string, including `LIMIT`/`OFFSET`. Dynamic `WHERE` clauses go through
   `store`'s `WhereBuilder`, which returns the clause and its named bindings
   together so clause order can never silently desync from binding order. The
   positional `rusqlite::params!` macro is clippy-disallowed for this reason —
   use `named_params!`.

5. **`records` has no I/O beyond value-object `ToSql`/`FromSql`.** It may depend
   on `rusqlite::types` (ADR-0007 — `RecordKind`, `Status`, and `RecordId`
   implement these traits directly, no bridge wrapper) but never on `rusqlite::
   Connection` or any query/transaction API. Only `store` and `cli`'s composition
   root may construct or hold a `Connection` — enforced by `clippy.toml`'s
   `disallowed-types`.

6. **No repository port trait.** `store` exposes a concrete `Store` type with
   inherent methods; `cli` constructs one at startup and calls it directly (ADR-
   0006). Do not introduce a trait abstraction for storage unless a second real
   implementation exists to justify it — not speculatively, not "for testability"
   (a real SQLite `:memory:` connection already gives test isolation for free).

7. **Validation and decode failures are explicit errors, never silently
   swallowed.** A command that turns a bad document into a default or an empty
   result is a bug. This is why `unwrap_or`, `unwrap_or_default`, and
   `unwrap_or_else` are clippy-disallowed — they turn a failure into a default,
   which is exactly what this rule forbids.

8. **Migrations run via `refinery` at startup.** On a schema-history mismatch:
   fail loud and stop. Never delete-and-rebuild the database — this SQLite file
   is the only copy of the data, not a rebuildable cache.

9. **No `_impl` suffix on functions or files.** If you're naming something
   `foo_impl`, the module needs a real responsibility split instead.

10. **No emojis anywhere** — code, comments, commit messages, CLI output,
    generated records.

11. **Run `cargo fmt` before committing.** Default `rustfmt` settings, no custom
   config. Do not hand-format code onto single crammed lines to save space —
   that is a readability regression for every future reader, human or agent.

12. **`kernel` is Strata's internal shared kernel.** It may be imported by this
   workspace's crates, but it is never imported outside this repository and is
   not a promotion candidate for external reuse.

13. **Modules are scoped to one cohesive responsibility, and use the modern
   (Rust 2018+) file layout — no `mod.rs`, ever.** A module lives at
   `module_name.rs`, declared from its parent via `mod module_name;`. If it
   needs its own submodules, they live in a `module_name/` directory next to
   `module_name.rs`, each as its own `submodule.rs` — never
   `module_name/mod.rs`. For example, if `store` needs splitting:

   ```
   store/src/
     lib.rs            # mod mutations; mod queries; mod validate;
     mutations.rs       # create, revise, set_status, link
     queries.rs          # get, history, search, graph
     validate.rs
   ```

   Split by responsibility, not just to relieve a line count — but
   `scripts/check_file_size.sh` (400-line ceiling per file) is the mechanical
   backstop for when that judgment call gets missed. Clippy has no lint that
   can see a file's aggregate size, only a single function's (`too_many_lines`,
   itself off by default) — this script is the actual enforcement here, not a
   decorative extra.

## Workspace layout

```
strata/
  Cargo.toml    # [workspace.dependencies] — single version pin per crate
  kernel/       # internal shared kernel — value-level SQL helpers and build identity
  clippy.toml
  records/      # domain — value types, RecordKind/Status/RecordId, lifecycle rules
  store/        # SQLite adapter — Connection, refinery migrations, WhereBuilder, FTS5
  cli/          # composition root, package name "strata" — produces the strata binary
  docs/         # specification, template research, and Strata's own records
```

No `crates/` wrapper directory — this workspace is small enough that a generic
grouping folder adds nesting without payoff. Crate names are bare (`records`,
`store`, `cli`/`strata`), not prefixed with the product name — this is a private
workspace, not a set of independently published crates, so the name doesn't need
to disambiguate on crates.io.

## Record-keeping discipline

This repository does not use `substrate`'s `adrs` MCP tool or any automated
ADR-management server. Strata's canonical RFC/PDR/ADR/EDR records live in SQLite;
the Markdown files under `docs/records/{rfc,pdr,adr,edr}/` are generated projections
produced by `strata export` and verified by `strata export --check`. They are not
edited directly. The renderer follows the templates researched in
`docs/research/template-research.md`. This is deliberate, not an oversight — the
incident that motivated Strata's own existence (RFC-0001) was exactly this kind
of tool silently corrupting a record on write.

Each ADR or EDR documents **exactly one decision**. If a change bundles two
independently-acceptable choices, write two records, not one record with two
decisions in its `## Decision` section. Generated frontmatter carries the canonical
identity, title, kind, status, revision, date, persisted slug, tags, and relationships.
Status lives in frontmatter only — never as a `## Status` heading in the body, that
field is redundant with the frontmatter and has been stripped everywhere it appeared.

## SQLite connection convention

Every connection (`Store::open` / `Store::open_memory`) sets, before running
migrations: `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, and a
`busy_timeout` (currently 5s). This is what makes two concurrent `strata`
invocations against the same file safe without any Rust-level thread-safety
machinery — the concurrency story lives entirely in SQLite's own locking, not in
the type system.

## Versioning

`Cargo.toml`'s `version` field is bumped only at real milestones — it is not the
source of truth for "what build is this." `cli/build.rs` embeds the current git
short SHA (plus a dirty-tree flag) as SemVer build metadata, surfaced in
`strata --version` and, more importantly, in `strata capabilities --json` — the
self-description surface an LLM agent uses to know exactly which binary build
it's talking to.

## Key reference documents

- `docs/specification.md` — the original system specification (34 sections)
- `docs/research/template-research.md` — external RFC/PDR/ADR/EDR template research
- `docs/records/` — Strata's own founding records; start with RFC-0001 and PDR-0001

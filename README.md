# strata

Local-first Rust CLI backed by a SQLite knowledge base for tracking
engineering reasoning — RFCs, PDRs, ADRs, EDRs, Specifications, Assessments,
Research, Glossary entries, and Risks — as structured, queryable
records instead of hand-maintained Markdown files edited by convention.

## Install

Install the CLI from crates.io; the package is named `strata-cli` and provides
the `strata` executable:

```sh
cargo install strata-cli --locked
```

In GitHub Actions, install the released tool rather than cloning this repository:

```yaml
- run: cargo install strata-cli --version 0.1.0 --locked
- run: strata --version
```

See `CLAUDE.md` for the enforceable rules this codebase follows, and
`docs/specification.md` for the original design specification.

For the RFC -> PDR -> ADR/EDR process itself, including how strata and
graphlite combine to back a proposal with evidence and verified reference
implementations, see `docs/workflow.md`.

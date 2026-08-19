---
id: "ADR-0010"
title: "Make strata export --check a byte-for-byte projection check"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: make-strata-export-check-a-byte-for-byte-projection-check
tags: []
relationships: {}
---

# ADR-0010: Make strata export --check a byte-for-byte projection check

## Context

ADR-0003 makes Markdown a deterministic projection and docs/specification.md section 28 calls for CI verification of committed projections. The check must define whether formatting normalization can hide drift.

## Decision

strata export --check will render the canonical record to the exact expected path and compare the resulting byte sequence directly with the committed file. It will not normalize trailing whitespace, line endings, Unicode normalization, YAML key order, or final-newline presence. A missing file, extra stale file, or any byte difference is a failure.

## Considered Options

Normalize line endings and trailing whitespace before comparison; parse both files into ASTs; or compare only semantic JSON/frontmatter values. Those approaches can accept changes to the human-readable projection and weaken the review and reproducibility guarantee. Byte equality makes deterministic rendering an enforceable contract.

## Consequences

Export output must choose and consistently emit its line endings, whitespace, ordering, and final newline. Developers on platforms with different default line endings cannot rely on local normalization; the generated file must be committed exactly as the renderer emits it.

## Evidence

docs/specification.md section 19 requires deterministic rendering and section 28 says export --check should fail when committed projections differ. ADR-0003 requires byte-identical Markdown for identical canonical state.

---
id: "ADR-0011"
title: "Reject malformed Markdown input with explicit structural errors"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: reject-malformed-markdown-input-with-explicit-structural-errors
tags: []
relationships: {}
---

# ADR-0011: Reject malformed Markdown input with explicit structural errors

## Context

Future strata new and strata revise Markdown input will be a mutation path and therefore part of the CLI trust boundary. A malformed document must not become a default or partially populated JSON document.

## Decision

Markdown input is malformed when the opening and closing YAML frontmatter delimiters are not the first and matching delimiter lines, the YAML cannot be parsed, a required metadata field is missing or has the wrong scalar/sequence/mapping type, the declared id or record-type is invalid, a required mapped section heading is absent, duplicated, at the wrong level, or out of the prescribed order, or section boundaries are ambiguous. The CLI will return a nonzero explicit error naming the file, record kind, and missing or malformed field/section; it will not create or revise a record.

## Considered Options

Accept missing fields as empty strings, infer missing headings from nearby prose, ignore unknown frontmatter, or parse best effort and rely on later validation. Those options violate the explicit-failure rule and can turn a human-readable but structurally invalid document into misleading canonical state.

## Consequences

Templates and hand-authored input must include the complete frontmatter and exact ordered required sections. Errors are actionable at the input boundary, and failed parsing leaves the database unchanged because the CLI validates before mutation.

## Evidence

AGENTS.md rule 7 requires validation and decode failures to remain explicit errors. PDR-0001 requires bad requests to fail without partial mutation, and ADR-0002 makes the CLI the sole validated mutation boundary. Issue #6 specifically requires clear errors for missing sections and broken frontmatter.

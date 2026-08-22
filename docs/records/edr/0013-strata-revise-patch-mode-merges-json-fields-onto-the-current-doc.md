---
id: "EDR-0013"
title: "strata revise patch mode merges JSON fields onto the current document"
record-type: edr
status: accepted
revision: 3
date: 2026-08-22
slug: strata-revise-patch-mode-merges-json-fields-onto-the-current-doc
tags: []
relationships:
  implements:
    - "RFC-0008"
---

# EDR-0013: strata revise patch mode merges JSON fields onto the current document

## Context

RFC-0008 identifies the risk and unnecessary round-tripping cost of wholesale document replacement for small corrections. The existing revise path already validates and records complete documents transactionally, while the Markdown parser requires a complete schema-valid document.

## Decision

Implement --patch for --document only in the first slice. The CLI requires a non-empty JSON object, merges each supplied field onto the stored document, validates the merged result with the existing record-kind rules, records it through the normal revise transaction, and returns the resulting full record. --patch with --file, stdin, no payload, or an empty object is rejected explicitly.

## Considered Options

Keep wholesale replacement only and require callers to reconstruct full documents; rejected because it is the problem RFC-0008 addresses. Implement partial Markdown patching in the same slice; deferred because the current Markdown parser expects complete frontmatter and sections, so partial semantics need a separate parser decision. Add per-field CLI flags; rejected because the surface would grow with every schema field.

## Consequences

JSON callers can make targeted corrections without carrying unrelated fields, while existing wholesale revise behavior remains unchanged. Patch revisions preserve normal revision history, validation, indexing, and rollback behavior. Markdown patching remains a follow-up requiring an explicit design and likely a separate EDR.

## Evidence

RFC-0008; store/src/mutations.rs revise and revise_patch implementation; cli/src/args.rs and cli/src/commands/auxiliary.rs --patch handling; cargo test --workspace passing; isolated binary smoke test confirming merge, preservation, revision history, empty-patch rejection, and file-patch rejection.

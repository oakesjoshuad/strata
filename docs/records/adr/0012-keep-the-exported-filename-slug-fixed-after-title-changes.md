---
id: "ADR-0012"
title: "Keep the exported filename slug fixed after title changes"
record-type: adr
status: accepted
revision: 2
date: 2026-08-19
slug: keep-the-exported-filename-slug-fixed-after-title-changes
tags: []
relationships: {}
---

# ADR-0012: Keep the exported filename slug fixed after title changes

## Context

Issue #11 will allow a record title to change after creation. If the filename is derived from the current title, a title update becomes a file move and can leave a stale old projection or break links and reviews.

## Decision

The filename slug is allocated and persisted at record creation and remains fixed for the record's lifetime, even when title changes. Rendering and export continue to use the stored creation slug; title updates change frontmatter and the document heading but never rename the path. The canonical record must retain this immutable slug alongside its identity so export does not need to infer history from the filesystem.

## Considered Options

Recompute the slug on every title change and delete or rename the stale old file; preserve the old file as an alias; or require manual filename repair. Recomputing makes a metadata edit a path migration and creates stale-file and link-management failure modes, while aliases and manual repair add competing projections.

## Consequences

Paths are stable for cross-references, code review history, and external links. A slug can become semantically stale relative to a renamed title, but that is an intentional stable-identity tradeoff; future tooling may display the current title in metadata without changing the path.

## Evidence

Issue #6 asks for the title-renaming choice in light of issue #11. Issue #11 is concerned with title updates, while ADR-0003 and docs/specification.md section 28 make committed projections and their paths operationally significant. EDR-0003 supplies the deterministic creation-time slug algorithm.

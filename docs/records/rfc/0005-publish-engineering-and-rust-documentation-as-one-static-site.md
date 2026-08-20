---
id: "RFC-0005"
title: "Publish Strata's canonical knowledge as a static site"
record-type: rfc
status: under-review
revision: 7
date: 2026-08-20
slug: publish-engineering-and-rust-documentation-as-one-static-site
tags: []
relationships:
  produces:
    - "PDR-0002"
---

# RFC-0005: Publish Strata's canonical knowledge as a static site

## Motivation

Strata's canonical knowledge is useful only if people and tools can review it without opening the SQLite database. The publication layer should turn the current database state into a deterministic, navigable static site for local review and later distribution. Strata owns the engineering knowledge model, while Pandoc and an ordinary static web server provide portable presentation and hosting.

## Problem

SQLite is the canonical store, but database records are not a convenient publication format. Markdown projections are suitable for review and interchange, while HTML is suitable for browsing. A publication contract is needed to define how Strata derives both from canonical state, how navigation follows the record graph, and how generated output can be safely replaced without introducing a second source of truth. This contract should remain independent of Git hosting providers and should not require a web application or multi-user service.

## Scope

This RFC defines the first static publication contract for Strata:

\- SQLite remains the sole canonical source of record content, identity, status, revisions, and relationships.
\- Strata generates deterministic Markdown projections and publication metadata from that state.
\- Pandoc compiles engineering documents and site pages into static HTML using Strata-owned templates and styles.
\- Strata generates indexes, navigation, relationship views, specification views, and traceability reports from the record graph.
\- The result is a self-contained static artifact suitable for Apache or another ordinary static web server.

The first implementation targets Strata's own records and a local publication directory. Supporting separately configured projects, public/private publication profiles, and additional output formats may follow once this contract is exercised.

## Non-Goals

This RFC does not make Markdown or HTML canonical, replace SQLite, define a Git forge, or require GitHub, GitLab, Forgejo, or Gitea. It does not require a web application, JavaScript runtime, server-side rendering, or a multi-user publication service. It does not make Rustdoc part of the first publication artifact and does not pass completed Rustdoc HTML through Pandoc. Rustdoc assembly and cross-links between Rust API pages and Strata records remain optional follow-on work.

## Constraints

SQLite remains the canonical record store, as established by ADR-0001. Markdown and all Pandoc-derived output remain generated projections, as established by ADR-0003. Strata owns site information architecture and graph-derived navigation, as established by ADR-0004. Persisted record slugs and canonical identifiers are authoritative; publication paths must be derived from them rather than from titles at publish time. Publication must be deterministic, locally buildable, suitable for static hosting, and explicit about unresolved references, missing assets, duplicate identities, or conflicting output paths. A failed publication must not leave a partially replaced site. The design must remain compatible with Strata's single-process local workflow and must not add async, server-side, or multi-user infrastructure.

## Proposal

Build static publication as a deterministic pipeline from canonical state.

First, Strata validates the selected database state and generates Markdown projections plus a publication manifest. The manifest maps each selected record identity to its stable publication path and includes the metadata needed for indexes, navigation, relationship views, specification hierarchy, and traceability reports. Selection may initially default to all records and later support filters such as kind, status, tag, or graph root.

Second, Strata or a publication script passes the generated engineering inputs and explicit site pages to Pandoc. Pandoc renders individual documents using the selected template, metadata, and stylesheet. Strata—not Pandoc—owns page selection, navigation, indexes, and graph-derived views.

Third, publication writes the complete static artifact to a staging directory and replaces the configured output directory only after generation and validation succeed. The artifact contains HTML, CSS, images, and any other required assets and can be served directly by Apache on the local S7 or by another ordinary static server. Hosting is deliberately outside the Strata domain model.

A normal record mutation may be followed by export and publication so the site reflects current database state. The first interface should provide explicit commands with deterministic results; a watch mode or automatic local regeneration can be added later without changing the canonical model. Generated Markdown and HTML may be committed or mirrored for review, but they never override SQLite.

The publication manifest and generated site should identify the database revision, record revisions, Strata build identity, and publication inputs used to produce them. A publication fails on unresolved required references, duplicate identities, output collisions, missing required assets, or invalid record state; it does not silently publish incomplete output.

## Alternatives Considered

Continue publishing only generated Markdown. This preserves portability but does not provide the navigable, reviewable static artifact needed for local use.

Use mdBook or another general static-site generator as the information-architecture owner. This provides an existing renderer and theme, but does not model Strata's record graph or remove the need for Strata-generated navigation and traceability views.

Use a Git forge's Pages feature as the publication system. This couples local review to a hosting provider and makes private publication dependent on provider-specific behavior. Git remains a useful review and distribution surface, but static hosting should work without a forge.

Run all generated output through Pandoc, including Rustdoc HTML. This risks damaging Rustdoc's generator-specific navigation, scripts, CSS, and search behavior. Rustdoc integration may be added later as a separate assembly stage that preserves Rustdoc output intact.

Edit generated Markdown or HTML directly after publication. This creates a second write path and contradicts the canonical SQLite model established by ADR-0001 and ADR-0003.

## Open Questions

1. Which publication filters are required for the first useful site: record kind, status, tag, specification subtree, or graph root? -- still open, deferred by PDR-0002 until the single-profile pipeline is exercised.
2. Which manifest fields are required beyond identity, path, status, and revision? -- PDR-0002's Data Model gives a first answer (schema version, generated_at, Strata build identity, database identity, renderer identity, per-entry content hash); exact JSON field types remain a follow-on EDR once `strata publish` is implemented.
3. Which references are required to resolve before publication, and which may be reported as warnings? -- PDR-0002's Failure Modes answers this at baseline: publish's precondition is exactly `strata validate`'s existing ERROR set (including EDR-0011's projection-staleness checks) plus path-collision and renderer-failure cases specific to rendering.
4. Should public and internal publication profiles be represented in Strata, or remain deployment-level selection of records and fields? -- still open, deferred by PDR-0002 for the same reason as filters.

## Outcome

Draft, revised to define SQLite-to-static-site publication as the core of RFC-0005. Strata owns canonical data, projections, site information architecture, and publication validation; a configured static-site backend renders documents; an ordinary static server hosts the resulting artifact. Git hosting is optional for review, backup, mirroring, and distribution. Rustdoc integration is deferred and must not shape the first implementation boundary.

PDR-0002 (static publication design) now settles the publication command boundary, backend configurability, output target, and staging/replacement semantics: `strata publish` is a single composition-root command (ADR-0017); it always produces a staged artifact directory and never writes to a live document root (ADR-0018); and its static-site backend is invoked as an externally configured subprocess command rather than a compiled Rust trait, so a different generator can be selected without a Strata code change (ADR-0019). This resolves Open Questions 1 and 4. Open Questions 2 and 6 (publication filters, public/internal profiles) remain deferred until the single-profile pipeline is exercised; Open Question 3 (manifest fields) has a first answer in PDR-0002 with exact types left to implementation; Open Question 5 (required references) is answered at baseline by reusing `strata validate`'s existing ERROR set. This RFC remains draft pending implementation and resolution of the remaining open questions.

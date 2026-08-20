---
id: "ADR-0018"
title: "Publication always produces a staged artifact, never a live document root"
record-type: adr
status: accepted
revision: 3
date: 2026-08-20
slug: publication-always-produces-a-staged-artifact-never-a-live-docum
tags: []
relationships:
  relates-to:
    - "ADR-0014"
    - "PDR-0002"
---

# ADR-0018: Publication always produces a staged artifact, never a live document root

## Context

RFC-0005 left open whether the first local publisher should write directly to a configured
Apache document root, or always produce an artifact that a separate deployment step installs
(Open Question 4). RFC-0005's own non-goals already exclude requiring a hosting provider,
forge, or web application, and its constraints require that a failed publication never leave
a partially replaced site.

## Decision

`strata publish` always writes its complete output to a configured **artifact directory**
(default `.strata/site`, resolved through the same CLI > env > config file > default
precedence ADR-0014 established) and never writes directly to a live-serving location such as
an Apache document root. Installing the artifact onto any server -- rsync, a symlink swap, a
deploy script -- is left entirely to the operator and is explicitly outside Strata's domain
model.

## Considered Options

Write directly to a configured Apache document root, staging and replacing that root
in place -- rejected. This couples Strata's publish command to a specific server's
filesystem layout and permissions, contradicts RFC-0005's own non-goal of not requiring a
hosting provider or web application, and complicates the "must not leave a partially
replaced site" guarantee across a location Strata does not fully control (a live web root
may have other files or a running server holding file handles).

Support both modes behind a flag, defaulting to artifact-only -- rejected for the first
implementation. No concrete deployment target has been exercised yet; adding a
direct-write mode before one real deployment workflow justifies it is exactly the kind of
speculative flexibility this project's conventions avoid. A flag can be added later as a
small follow-on decision if a real need appears.

## Consequences

Publication is host-agnostic by construction: the same artifact directory can be installed
under Apache, another static server, or synced to a remote host, with no Strata-side
knowledge of any of them. The staging-and-atomic-replace mechanics (PDR-0002) apply to the
artifact directory itself, not to any live-serving location, which keeps the atomicity
guarantee entirely within Strata's own filesystem control. A user who wants the site served
locally must run a separate, unopinionated install step after `strata publish` -- this is a
deliberate scope boundary, not an oversight.

## Evidence

RFC-0005, Open Question 4, and its Non-Goals section (no hosting provider, no forge, no web
application required). ADR-0014, the configuration precedence this decision reuses for the
new `publish_target` value. PDR-0002 (static publication design), the design record this
decision was extracted from.

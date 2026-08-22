---
id: "RFC-0009"
title: "Make valid status transitions discoverable before they are attempted"
record-type: rfc
status: accepted
revision: 7
date: 2026-08-21
slug: make-valid-status-transitions-discoverable-before-they-are-attem
tags: []
relationships:
  produces:
    - "EDR-0014"
    - "EDR-0015"
---

# RFC-0009: Make valid status transitions discoverable before they are attempted

## Motivation

Each record kind has its own closed lifecycle (`schema <kind>` already reports
the valid status list per kind, e.g. RFC: `draft, proposed, under-review,
accepted, withdrawn`), but nothing reports which transitions *out of* a
record's current status are actually legal. Confirmed directly: `strata
status RFC-0007 accepted` from `draft` fails with `error: validation error:
invalid status transition for RFC: draft -> accepted` -- the error names the
attempted transition but not the set of transitions that would have
succeeded. Discovering the right next status today means either reading the
lifecycle rules in source/records (ADR/PDR-0004, "Define workflow and
lifecycle policy across record kinds") or guessing from the kind's status
list and re-running `status` until one is accepted.

## Problem

A caller (human or agent) who wants to move a record forward has no cheap way
to ask "what am I allowed to transition this record to right now?" before
attempting a transition. Some terminal-adjacent statuses (`withdrawn` for
RFCs, `superseded` for PDR/ADR/EDR) have no path back out per the existing
lifecycle policy; a caller who reaches one by mistake, from a lifecycle they
misjudged, has no supported way to undo it short of direct database
intervention outside the CLI's own validated boundary.

## Scope

Define a way to list a record's currently-valid next statuses, surfaced both
as a distinct query and as part of the existing transition-failure error
message. Define a supported, CLI-level way to undo the most recent status
transition, as a normal validated operation rather than a manual database
edit.

## Non-Goals

This RFC does not change the underlying lifecycle policy itself (which
statuses exist per kind, or which transitions are legal) -- that remains
governed by the existing PDR/ADR-0004 policy. It does not add a general
multi-step transition-history rollback (reverting more than one transition
back); only the single most recent transition is in scope. It does not change
`status`'s existing single-transition argument shape for the common case.

## Constraints

Any listing of valid next statuses must be derived from the same lifecycle
rules `status` already enforces -- it cannot be a second, separately
maintained copy of the transition table that could drift from the real one.
Undo must go through the normal transactional revision path (it is itself a
status transition, recorded as such), not a special bypass, so the record's
`history` remains a complete and honest account of what happened -- silently
erasing the mistaken transition from history would misrepresent what actually
occurred.

## Proposal

Two additions. First, when `status` rejects a transition, include the
record's actual valid next statuses in the error, e.g. `invalid status
transition for RFC: draft -> accepted (valid from draft: proposed,
withdrawn)`, computed from the same lifecycle table the validation check
already consults. Second, add a `--list` mode (e.g. `strata status <ID>
--list`) that reports the current status and valid next statuses without
attempting any transition, for a caller that wants to check before acting.
Third, add `strata status <ID> --undo`, which reverts the record's status to
what it was immediately before its most recent transition, itself performed
and recorded as an ordinary transition (so `history` shows both the original
move and the undo), and which only succeeds for a record whose status has
been transitioned at least once.

## Alternatives Considered

Leave discovery to `schema <kind>`'s existing full status list and require
the caller to already know the lifecycle graph -- the status quo, and what
led directly to an unintended `withdrawn` transition with no supported way
back. Support arbitrary N-step history rollback instead of a single undo --
rejected as unnecessary scope for the problem actually observed (one mistaken
transition), and a bigger surface to get wrong; a single-step undo covers the
concrete failure mode without inventing a general time-travel feature.
Silently allow any status-to-status transition and drop lifecycle validation
entirely -- rejected outright, since enforced lifecycle transitions are the
entire point of PDR/ADR-0004 and Rule 7's stance against swallowing
validation failures.

## Open Questions

Should `--list` be its own subcommand or a flag on `status`, given `status`
otherwise always takes a `<NEW_STATUS>` argument that `--list` has no use
for? Should the undo be time-bounded (only undoable immediately after the
transition, before any other mutation touches the record) or unconditionally
available against the single most recent transition regardless of what else
happened to the record since? Does undo need its own audit marker in
`history` output (so a reader can tell a transition was itself an undo,
rather than an ordinary forward move) or is the existing history log
sufficient as-is?

## Outcome

Accepted and implemented. Strata derives valid next statuses from lifecycle rules, exposes `strata status <ID> --list`, and enriches invalid-transition errors. EDR-0015 design is implemented through the status_transition audit migration and `strata status <ID> --undo`: undo is allowed only for the immediately latest forward status transition, returns to its exact recorded predecessor, records an explicit undo revision, and rejects repeated undo or any later revision. Historical transitions without audit rows remain non-undoable.

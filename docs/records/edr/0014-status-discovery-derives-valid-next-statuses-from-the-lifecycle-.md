---
id: "EDR-0014"
title: "status discovery derives valid next statuses from the lifecycle rules"
record-type: edr
status: accepted
revision: 3
date: 2026-08-22
slug: status-discovery-derives-valid-next-statuses-from-the-lifecycle-
tags: []
relationships:
  implements:
    - "RFC-0009"
---

# EDR-0014: status discovery derives valid next statuses from the lifecycle rules

## Context

RFC-0009 asks callers and agents to discover legal status transitions before attempting them. The existing transition validator was the authoritative lifecycle behavior, but it did not expose its outgoing transitions and invalid-transition errors did not explain the alternatives.

## Decision

Expose a records::valid_next_statuses helper derived from the same exhaustive per-kind transition table used by validation. Include the resulting status names in invalid-transition errors, and add `strata status <ID> --list` as a mutually exclusive discovery mode with human-readable and JSON output. A normal status command still requires a target status. Undo is not implemented in this slice and remains deferred.

## Considered Options

Maintain a second status-transition table for discovery; rejected because it could drift from enforcement. Add a new top-level command instead of `status --list`; rejected because discovery concerns one existing record and belongs beside the transition operation. Implement undo concurrently; deferred because its temporal and audit semantics are a separate design problem.

## Consequences

Agents can query the exact legal next statuses without guessing, and failed transitions explain the available choices. Terminal statuses report no valid next statuses. Lifecycle policy itself is unchanged. Undo remains unavailable through the CLI until a follow-on decision settles its semantics.

## Evidence

RFC-0009; records/src/validation.rs valid_next_statuses and transition error; cli/src/args.rs and cli/src/commands/auxiliary.rs status --list handling; lifecycle tests covering RFC, PDR, ADR, and EDR paths; real binary smoke tests for JSON/human listing and enriched failure output; cargo test --workspace and cargo clippy verification.

---
id: "EDR-0018"
title: "Use graphlite resolve --prefer-file instead of raw symbols search in symbol_locator"
record-type: edr
status: draft
revision: 1
date: 2026-08-30
slug: use-graphlite-resolve-prefer-file-instead-of-raw-symbols-search-
tags: []
relationships:
  derived-from:
    - "EDR-0012"
  relates-to:
    - "EDR-0017"
---

# EDR-0018: Use graphlite resolve --prefer-file instead of raw symbols search in symbol_locator

## Context

EDR-0017's own code references exposed a real integration bug, precisely diagnosed the same day (see EDR-0017's disclosed consequences): `cli/src/symbol_locator.rs::run_symbols` calls `graphlite symbols <query> --file <path>` -- a raw full-text search over symbol signatures, not just names. When a function is called from within another item's body in the same file (e.g. `backfill_code_reference_ids` invoked inside `impl Store`'s `configure` method), the *caller's* signature text also contains the query string as a substring, so `symbols` returns both the real definition and the unrelated caller as separate matches. `locate()`'s ambiguity rule (`[] => Missing, [x] => x, _ => Ambiguous`) then rejects the query outright, even though the correct answer is unambiguous.

Confirmed directly that graphlite already has the right tool for this: `graphlite resolve <query>` uses a ranked, exact-name-preferring strategy (`strategy="name-exact-ranked"` in its own output) and returned exactly one clean candidate for every case `symbols` called ambiguous (`backfill_code_reference_ids`, `Store::update_code_reference`, `Store::remove_code_reference`), with zero flags needed. `resolve` also exposes `--prefer-file <PATTERN>`, built for exactly the file-scoping `symbol_locator.rs` already has on hand (the code reference's own `path` field) but currently applies through the wrong command. This is not a graphlite defect -- graphlite's own `resolve --help` describes it as resolving "an ambiguous symbol query to a deterministic top candidate", which is precisely this problem.

## Decision

Replace `run_symbols`'s query with `graphlite resolve <query> --prefer-file <path>`, parsing the response differently: `resolve`'s XML always places the winning `<symbol .../>` before any `<alternatives>` block, so `tags_with_attribute(&output, "stable_id")` (already used elsewhere in this file, unchanged) applied to `resolve`'s output and taking the *first* matching tag yields the selected candidate, never the alternatives -- no new XML-parsing code needed, just a different top-level command and a `.first()` instead of an exhaustive-match on the full collected list.

Add one verification step `run_symbols` does not have today: compare the winning candidate's own `file` attribute (normalized the same way `path` already is) against the expected `path`. `--prefer-file` is a ranking bias, not a hard filter, so a query with zero real matches in the target file could still return a top candidate from a different file; treat a file mismatch the same as `LocatorError::Missing`, not `Ambiguous` -- a wrong-file match is not a real answer to reject as ambiguous, it is simply not there.

This changes what `LocatorError::Ambiguous` means going forward: since `resolve` deterministically ranks to one winner (or errors when it finds nothing at all), the query-resolution step can no longer produce an ambiguous *result* the way raw `symbols` could. `Ambiguous` remains meaningful only for the second stage of `locate()` (the `graph sym:<stable_id>` lookup's own range-matching logic, `[range] => Ok, [] => Missing, _ => Ambiguous`), which is unrelated to this fix and stays as-is.

## Considered Options

Keep calling `graphlite symbols` but filter the results client-side to exact name/qualified_name matches before applying the ambiguity rule -- rejected: this reimplements `resolve`'s own ranking strategy (`name-exact-ranked`) a second time in Strata, duplicating logic graphlite already maintains and tests, for no benefit over calling the command built for exactly this purpose.

Call both commands -- try `symbols` first, fall back to `resolve --prefer-file` only when `symbols` reports more than one match -- rejected as unnecessary complexity: `resolve` is not a slower or riskier command than `symbols`, there is no cost to calling it directly every time, and a fallback path is one more thing to test and for behavior to silently diverge between.

Leave `symbol_locator.rs` as-is and instead document "avoid --symbol on constants or on functions called from within the same file" as a code-ref-add usage caveat -- rejected: this asks every future author of a code reference to privately know and route around an implementation detail of graphlite's full-text search, rather than fixing the one integration point that actually has the problem.

## Consequences

Once implemented, EDR-0017's three currently-disclosed `SYMBOL-AMBIGUOUS` findings (`backfill_code_reference_ids`, `Store::update_code_reference`, `Store::remove_code_reference`) should resolve cleanly under `strata code-ref-check` without any change to the code references themselves -- they were always correct, only the lookup mechanism was wrong. Those three references should then be re-verified (a plain `code-ref-check` run, no `code-ref-update` needed unless line ranges also drifted) and this record's own evidence should confirm the before/after. `cli/src/args.rs`'s own `Command` reference (used generically, not via `--symbol`) is unaffected by this change and remains whatever it already is.

The risk this doesn't cover: a query that is genuinely ambiguous *within* the correct file (e.g. two distinct items sharing a name in different nested scopes of one file) may still resolve to the wrong one silently, since `resolve`'s ranking picks a single top-scored answer rather than surfacing real in-file ambiguity as an error the way the old `Ambiguous` variant conceptually promised to. This is a real, currently-unquantified trade-off between fewer false-ambiguous rejections (this record's whole goal) and a small chance of one true ambiguity being silently resolved wrong; no evidence of this happening has been found, and it should be watched for rather than solved speculatively here.

## Evidence

EDR-0017's own consequences section (the disclosed blemish that prompted this investigation). Confirmed directly (2026-08-30): `graphlite resolve backfill_code_reference_ids` returns `candidates="1"` cleanly; `graphlite symbols backfill_code_reference_ids --file store/src/connection.rs` (the exact call `run_symbols` makes) returns 2 matches -- the real definition and the enclosing `impl Store` block, whose body happens to call it. `graphlite resolve --help` confirms `--prefer-file <PATTERN>` exists and confirms resolve's own stated purpose ("Resolve an ambiguous symbol query to a deterministic top candidate") is exactly this problem. `cli/src/symbol_locator.rs` (current implementation) and `tags_with_attribute`/`parse_attributes` (the existing, reusable XML scanner) read in full to confirm the fix needs no new parsing code, only a different command and a first-match instead of exhaustive-match.

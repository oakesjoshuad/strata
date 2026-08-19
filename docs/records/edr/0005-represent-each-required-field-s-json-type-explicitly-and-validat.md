---
id: "EDR-0005"
title: "Represent each required field's JSON type explicitly and validate it"
record-type: edr
status: accepted
revision: 2
date: 2026-08-19
slug: represent-each-required-field-s-json-type-explicitly-and-validat
tags: []
relationships:
  relates-to:
    - "PDR-0001"
---

# EDR-0005: Represent each required field's JSON type explicitly and validate it

## Context

records/src/kind.rs's default_document() decides whether a required field is scalar (JsonValue::String) or list-shaped (JsonValue::Array) using a hardcoded if/else chain over field-name string literals. That chain is the only place this type information exists as a declaration; everywhere else that needs it either duplicates the list or infers it indirectly. cli/src/parse.rs's array_field() is one such indirect consumer: to determine whether a Markdown section should parse as a list, it calls kind.default_document("") and inspects the JSON type of the resulting placeholder value for that field, rather than consulting a real declaration. Issue #10's review found the concrete cost of this: nothing declares that, say, 'motivation' must be a string and 'alternatives' must be an array, so records/src/validation.rs's validate_document() had no way to check it, and a record whose 'motivation' field held Array([]) (the wrong type entirely, not just an empty value) passed validation as though it were valid content.

## Decision

We will add an explicit, authoritative field-type declaration to RecordKind: a FieldKind enum (Scalar, List) and a function pairing each of a kind's required_fields() with its FieldKind, replacing the implicit knowledge currently duplicated across default_document()'s if/else chain and array_field()'s document-construction inference. default_document() and array_field() are both rewritten to read from this one table instead of their own separate logic. validate_document() gains a new check: for every required field, the document's JSON value must match its declared FieldKind -- a Scalar field must decode to a non-empty JsonValue::String, a List field must decode to a JsonValue::Array (empty is a legitimate value for a list field, not an error). This subsumes and replaces the existing ad hoc string-emptiness check, which only ever covered the Scalar case by accident.

## Considered Options

Leave the type list implicit and give validate_document its own separate copy of the field-name list -- rejected, this adds a third duplicated copy of the same knowledge instead of fixing the duplication that caused the bug in the first place. Have validate_document call default_document() and compare JSON types the way array_field() already does -- rejected, this still treats a placeholder-construction function as the source of truth by side effect rather than by declaration, and constructing a full throwaway document on every validation call just to inspect one field's type is indirect and wasteful compared to a direct table lookup. Do nothing beyond what #10 already fixed at the template layer -- rejected, #10's fix closed one specific path to this bug (blank template sections); it did not close the general case, and nothing stops a hand-typed --document JSON argument or a future import path from producing the same wrong-typed result today.

## Consequences

records/src/kind.rs gains a small FieldKind enum and one authoritative per-kind field-type table. default_document(), array_field() (cli/src/parse.rs), and validate_document() (records/src/validation.rs) all read from that one table; a future required field only needs its type declared once for all three call sites to behave correctly, closing off the exact drift that produced this bug. Existing real records are unaffected: the self-hosted database already passed a manual type check as of #10's fix (19/19 records, every scalar field a real non-empty string, every list field a real array), so this check should pass cleanly against current data with no retroactive breakage, and will only reject genuinely wrong-typed input going forward. A new ValidationError variant is needed to distinguish 'wrong type' from the existing EmptyField/MissingField cases, since a wrong-typed field is a different failure than an empty one.

## Evidence

Issue #10's review (oakesjoshuad/strata#10): found Array([]) passing validate_document for a field that should have been a non-empty String, traced to default_document() producing empty JSON values for blank template sections and validate_document() never checking field type at all. Issue #12 (oakesjoshuad/strata#12): filed to track this gap after #10's narrow template-layer fix. records/src/kind.rs: default_document()'s hardcoded field-name if/else chain, the sole current declaration of field type. cli/src/parse.rs: array_field()'s indirect type inference via a throwaway default_document("") call. AGENTS.md / CLAUDE.md rule 7: 'A command that turns a bad document into a default or an empty result is a bug' -- the rule this gap violates and this decision closes.

# ADR-0004 — Ratified Phase 1 Syntax Contract

- **Status:** Accepted — approved by the project owner via Engineering
  Directive 006 (2026-10-12). Binding until superseded.
- **Date:** 2026-10-12

## Context

Phase 1 required precise lexical and syntactic contracts before any
frontend code could be written. Directive 004 produced the proposal set
under `docs/proposals/phase-1/`; Directive 005 corrected the inconsistencies
found in an independent review; Directive 006 ratified the result and
authorized the `xl-lexer` implementation stage.

This ADR records the ratification itself. It deliberately does not
duplicate the grammar: the binding content is the archived original
specification plus the accepted proposal documents it refines.

## Decision

The project owner approved, via Engineering Directive 006:

- The grammar and token inventory (`grammar-rfc.md`), including the
  corrections from Directive 005 (tail-expression decision order,
  parenthesized assignment targets, function-type EBNF, numeric literal
  representation, float overflow policy).
- The lexer contract (`lexer-contract.md`), including the structured
  interpolation design and the span invariant clarification recorded in
  §5.5.
- The parser/AST contract (`parser-ast-contract.md`) as the target for the
  future parser (not implemented by this directive).
- The conformance plan (`conformance-plan.md`) and the decision packet
  (`decision-packet.md`), whose Category A rules and Category B amendments
  (B-1…B-4, with the B-4 resource-safety clarifications) are accepted.
- Category C items remain deferred; G-5 (struct-literal construction)
  remains unresolved.

PD-002 through PD-006 are closed by `docs/pending-decisions.md`; PD-009
(public API compatibility) and PD-010 (diagnostic format details) remain
open.

## Alternatives

- **Drafting a new combined specification document.** Rejected: it would
  duplicate the archived original and the proposals, creating three
  competing texts.
- **Treating CI success or implementation as approval.** Rejected: the
  governance model requires explicit owner approval; this ADR records that
  approval, not an inference.

## Consequences

- The proposals are binding contracts for Phase 1 implementation; changes
  go through the specification change control and a new owner decision.
- The `xl-lexer` implementation stage is authorized (Directive 006); the
  parser, AST crate, and CLI remain unauthorized.
- Any deviation discovered during implementation must be resolved against
  the contracts, not silently.

## Specification impact

Implements the owner-approved refinement of the original specification's
syntax for Phase 1. The archived original (`docs/spec-v0.1-original.txt`)
retains priority for any conflict; the accepted packet's rules fill the
areas the original leaves open.

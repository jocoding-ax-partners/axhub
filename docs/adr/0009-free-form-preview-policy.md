# ADR 0009 — Free-form preview policy

## Status

Accepted for v0.2.0. Amended 2026-06-08 (#180): the Rust consent/preauth HMAC gate was removed; destructive commands are now gated solely by the SKILL preview plus explicit human approval. The references below are updated to that model (the gate had already been unwired from `hooks.json` in #170).

## Context

The plugin has two safety layers:

1. SKILL-level preview copy that explains the human-facing action.
2. An explicit human approval step — an `AskUserQuestion` preview with a non-interactive abort default — before the single destructive command runs. (The earlier Rust consent parsing plus PreToolUse HMAC-token verification was removed in #180.)

Structured `AskUserQuestion` registry coverage is useful for questions with stable options, but some preview cards need rich free-form identity fields, command details, or risk notes that do not fit a small option schema.

## Decision

Free-form preview cards are allowed when a SKILL owns the wording and the destructive command is still gated by an explicit preview plus human approval step.

The registry baseline remains responsible for per-question safe defaults. A SKILL that adds a structured `AskUserQuestion` JSON block must register `safe_default` and `rationale` in `tests/fixtures/ask-defaults/registry.json`.

Free-form preview cards must include enough stable fields for human verification. For deploy-like actions this means target, environment/profile, branch or source, expected effect, and a safe cancel/dry-run path when available.

## Consequences

- The final enforcement point is the explicit human approval step the SKILL renders, not the prose preview alone and not an automated token gate.
- Free-form preview text can evolve without registry churn, as long as the preview keeps the stable human-verification fields above and the registry `safe_default` stays tested.
- Tests must lock, for every new destructive action family, the SKILL's preview wording plus its non-interactive abort default (registry `safe_default` + the SKILL's headless guard).

## Non-goals

- This ADR does not add helper bootstrap, dependency install, or remote template fetch behavior.
- This ADR does not change plugin manifest schema.

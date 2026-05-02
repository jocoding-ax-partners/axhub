# ADR 0009 — Free-form preview policy

## Status

Accepted for v0.2.0.

## Context

The plugin has two safety layers:

1. SKILL-level preview copy that explains the human-facing action.
2. Rust helper consent parsing plus PreToolUse verification that blocks destructive Bash commands unless the HMAC-bound token matches the action and context.

Structured `AskUserQuestion` registry coverage is useful for questions with stable options, but some preview cards need rich free-form identity fields, command details, or risk notes that do not fit a small option schema.

## Decision

Free-form preview cards are allowed when a SKILL owns the wording and the destructive command is still protected by the Rust consent gate.

The registry baseline remains responsible for per-question safe defaults. A SKILL that adds a structured `AskUserQuestion` JSON block must register `safe_default` and `rationale` in `tests/fixtures/ask-defaults/registry.json`.

Free-form preview cards must include enough stable fields for human verification. For deploy-like actions this means target, environment/profile, branch or source, expected effect, and a safe cancel/dry-run path when available.

## Consequences

- The final enforcement point is `axhub-helpers preauth-check`, not the prose preview.
- Free-form preview text can evolve without registry churn, as long as the consent action/context stays tested.
- Tests must lock destructive actions in `consent/parser.rs` and at least one preauth deny/allow cycle for every new action family.

## Non-goals

- This ADR does not add helper bootstrap, dependency install, or remote template fetch behavior.
- This ADR does not change plugin manifest schema.

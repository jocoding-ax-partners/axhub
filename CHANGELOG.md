# Changelog

All notable changes to the axhub Claude Code plugin will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows [Semantic Versioning](https://semver.org/).

## [Unreleased] — M0 scaffold

### Added
- Plugin manifest (`.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json`).
- Directory scaffold: `commands/`, `skills/`, `hooks/`, `bin/`, `docs/`, `tests/`.
- README skeleton with Korean-first vibe coder onboarding.
- LICENSE (MIT), .gitignore, CHANGELOG.

### Pending (next: M0 continuation → M0.5 baseline → M1.5 GO/KILL gate)
- `src/axhub-helpers/index.ts` — TypeScript on Bun runtime, single multi-cmd binary built via `bun build --compile`. Subcommands: session-start, preauth-check, consent-mint, consent-verify, resolve, preflight, classify-exit, redact. (PLAN audit row 66 — language reversed from Go to TS for Claude Code ecosystem consistency, vibe coder readability, Bun cold start sufficient for 50ms hook gate.)
- `hooks/hooks.json` with `{"hooks": {...}}` wrapper, command-based HMAC PreToolUse consent.
- `docs/{vibe-coder-quickstart, troubleshooting, org-admin-rollout}.ko.md`.
- `tests/corpus.jsonl` (n≥200, risk-stratified, ≥40 adversarial, frozen model + temp=0).
- `tests/hook-fixtures/v0/*.json` (pinned hook payload schemas).
- `.goreleaser.yaml` for multi-arch cosign-signed binary releases.
- Plugin compatibility matrix: ax-hub-cli `>=0.1.0,<0.2.0`.

## Plan
See `PLAN.md` for the full design history (6 phases of review, 65 audit-tracked decisions).

## Plugin ↔ ax-hub-cli compatibility

| Plugin | ax-hub-cli min | ax-hub-cli max |
|---|---|---|
| 0.1.x | 0.1.0 | < 0.2.0 |

# Changelog

All notable changes to the axhub Claude Code plugin will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

Nothing yet.

## [0.1.5] — 2026-04-24

Phase 9 — Windows keychain bridge ship. macOS + Linux + helper binary unchanged.

### Added

- `src/axhub-helpers/keychain-windows.ts` — Windows Credential Manager bridge via PowerShell + `Add-Type` PInvoke against `advapi32!CredReadW`. ASCII sentinel scheme (`AXHUB_OK:<base64>` / `ERR:NOT_FOUND` / `ERR:LOAD_FAIL`) for locale-independent classification. Stock Win10/11 — no `Install-Module` required.
- 5 4-part Korean error messages for Windows scenarios (감정 / 원인 / 해결 / 다음액션): ExecutionPolicy block, NOT_FOUND, PInvoke load failure, EDR/AMSI quarantine (signal-kill or exit ∈ {-1, 0xC0000409}), spawnSync throws.
- `tests/keychain.test.ts` — extracted `parseKeyringValue` decoder tests (8 cases) from `tests/token-init.test.ts`.
- `tests/keychain-windows.test.ts` — 6 mocked-runner cases covering all pre-mortem scenarios.

### Changed

- `src/axhub-helpers/keychain.ts` — Windows branch (previously deferred error message) now delegates to `readWindowsKeychain()`. Linux + macOS branches unchanged.
- Skills + docs updated additively: `skills/auth/SKILL.md`, `skills/deploy/references/headless-flow.md`, `skills/deploy/references/recovery-flows.md`, `docs/pilot/admin-rollout.ko.md`, `src/axhub-helpers/list-deployments.ts`, `bin/README.md` — Windows mentions added alongside existing macOS/Linux content.

### Honest tradeoff (EDR)

v0.1.5 Windows binary is **not Authenticode-signed** (deferred to v0.1.6). EDR / AMSI / corporate AV will likely classify the inline PInvoke against `advapi32!CredReadW` as a Mimikatz-pattern threat and block the call. The Korean EDR error message (`keychain-windows.ts:ERR_EDR`) explicitly owns this — recommends `AXHUB_TOKEN` env var as the legitimate workaround until v0.1.6 code-signing makes EDR allowlist requests viable.

### Deferred to v0.1.6

- Format-parity for existing macOS + Linux Korean errors (one-line → 4-part empathy template). Tracked in https://github.com/jocoding-ax-partners/axhub/issues/1.
- Authenticode code-signing for `windows-amd64.exe` → EDR allowlist legitimization.

### Test baseline

- `bun test` → 349 pass / 5 skip / 0 fail / 2256 expect() / 354 tests across 16 files.
- `bunx tsc --noEmit` → clean.
- `bash tests/docs-link-audit.sh` → `Broken: 0`.

## [0.1.0] — 2026-04-24

First public release. Korean-first natural-language deploy/manage for vibe coders, wrapping ax-hub-cli (`>=0.1.0,<0.2.0`).

### Added

#### Core helper binary (TypeScript on Bun)

- `src/axhub-helpers/index.ts` — single multi-cmd binary built via `bun build --compile`. Subcommands: `session-start`, `preauth-check`, `consent-mint`, `consent-verify`, `resolve`, `preflight`, `classify-exit`, `redact`.
- `src/axhub-helpers/consent.ts` — HMAC consent token mint/verify (jose JWT HS256). Bound to `{tool_call_id, action, app_id, profile, branch, commit_sha}`. PreToolUse deterministic deny-gate.
- `src/axhub-helpers/preflight.ts` — CLI version range gate (semver) + auth status preflight. Exit code precedence 64 > 65 > 0.
- `src/axhub-helpers/catalog.ts` — 4-part Korean error empathy templates per axhub exit code (감정 + 원인 + 해결 + 버튼).
- `src/axhub-helpers/redact.ts` — NFKC normalize + secret/cross-team URL redaction filter.
- `src/axhub-helpers/resolve.ts` — live profile/app/branch/commit resolution (no cached app_id for mutations).
- `src/axhub-helpers/telemetry.ts` — opt-in observability envelope (default OFF, gated by `AXHUB_TELEMETRY=1`).

#### Plugin surface

- 11 skills under `skills/`: apis, apps, auth, clarify, deploy, doctor, logs, recover, status, update, upgrade. Each with Korean trigger lexicon + workflow.
- 9 slash commands under `commands/`: apis, apps, deploy, doctor, help, login, logs, status, update.
- `hooks/hooks.json` — `{"hooks": {...}}` wrapper with SessionStart + PreToolUse + PostToolUse hook chain.
- `.claude-plugin/{plugin,marketplace}.json` — plugin manifest with `repository` as string (Phase 6 incident #1 fix), all required keys per Claude Code loader.

#### Cross-arch distribution

- `bun run build:all` — 5 cross-arch helper binaries: darwin-arm64 (58.3M), darwin-amd64 (63.0M), linux-amd64 (99.2M), linux-arm64 (96.8M), windows-amd64.exe (109.6M).
- `bin/install.sh` — POSIX shell auto-selector with OS+arch detection (`AXHUB_OS`/`AXHUB_ARCH` env overrides for testing). Symlinks (Unix) or copies (Windows).
- `tests/install.test.sh` — 5 positive + 3 negative arch detection cases.

#### Release pipeline

- `.github/workflows/release.yml` — tag-triggered (`v*.*.*`) cross-arch build + cosign keyless signing (sigstore OIDC, no long-lived keys) + manifest.json + checksums.txt + GitHub Release upload.
- `scripts/release/manifest.ts` — JSON manifest generator (sha256 + arch + size_bytes per binary, plus plugin/helper version).
- `scripts/release/verify-release.sh` — user-side verification script: manifest signature → per-binary signature → sha256 cross-check.
- `docs/RELEASE.md` — maintainer + user verification guide. Documents `AXHUB_REQUIRE_COSIGN=1` advisory + `AXHUB_ALLOW_UNSIGNED` warning (IT-only escape hatch).
- Cosign sidecar advisory in session-start: warns when `AXHUB_REQUIRE_COSIGN=1` and `.sig` missing (advisory only, doesn't block).

#### Test suite

- 295 passing unit/integration tests across 11 files / 2136 expect() assertions / typecheck clean.
- `tests/consent.test.ts` — 56 tests covering parser bypass hardening (T-ADV-PARSE-1..8) + 3 closed gotcha classes (trailing-delimiter, nested-shell, quoted-subcommand) + dead-path `deploy_logs_kill` v0.2 reservation test (17 corpus assertions across full v0.1.0 CLI surface).
- `tests/manifest.test.ts` — 86 tests / 358 expect() validating plugin.json, marketplace.json, hooks.json structure (`hookEventName` presence required — Phase 6 incident #2 fix), commands frontmatter, skills frontmatter (Phase 6 Q1 — `allowed-tools` removed).
- `tests/fuzz-parser.ts` — deterministic mulberry32 PRNG, 1100 randomized variants (1000 standard + 100 gotcha-class). 1100/1100 caught with default seed `0xc67434fd`. Reproducible across runs.
- `tests/fixtures/` — 38 hand-curated frozen contract files (10 destructive, 8 read-only, 8 adversarial, 4 unicode, 4 profile/headless, 4 negative). `_curated.ts` source-of-truth generator + drift detection.
- `tests/corpus.100.jsonl` + `tests/corpus-schema.test.ts` — 100-row stratified scoring corpus, all rows with `expected_cmd_pattern`. Schema invariants validated.
- `tests/telemetry.test.ts` — 16 tests for opt-in envelope shape, file mode 0600, opt-out default.
- `tests/codegen.test.ts` — catalog ↔ markdown drift detection (8 tests).
- `tests/release-config.test.ts` — 18 shape assertions on `.github/workflows/release.yml` + `manifest.ts` + `verify-release.sh`.
- `tests/e2e/staging.test.ts` — gated real-CLI integration (skipped when `AXHUB_E2E_STAGING_TOKEN` unset).
- `tests/docs-link-audit.sh` — every `references/X.md` mention in `SKILL.md` files resolves on disk (Broken: 0).

#### Korean documentation

- 11 SKILL.md files with Korean trigger lexicon (informal/honorific/demo variants).
- `skills/deploy/references/error-empathy-catalog.md` — 13 exit-code entries with 4-part Korean templates + interpolation placeholders.
- `skills/deploy/references/error-empathy-catalog.generated.md` — auto-generated runtime snapshot (regen via `bun run codegen:catalog`).
- `skills/deploy/references/{nl-lexicon, recovery-flows, headless-flow, telemetry}.md` — Korean reference docs.
- `skills/apis/references/privacy-filter.md` — cross-team scope isolation rules per Phase 6 §16.17 / row 46.
- `docs/pilot/` — first-customer pilot prep kit (5 docs, 476 lines): README, onboarding-checklist, feedback-template, admin-rollout.ko, exit-criteria.

#### Scoring infrastructure

- `tests/score.ts` — 4-metric scoring (trusted-completion, unsafe-trigger-precision, recovery-rate, baseline-delta). M1.5 GO/KILL gate logic.
- `tests/baseline-results.docs-only.{20,100}.json` — docs-only Claude predictions (M0.5 + M2.5 scopes).
- `tests/plugin-arm-results.{20,100}.json` — plugin-arm predictions (M0.5 + M2.5 scopes).
- M1.5 v2 verdict: trusted 91% / unsafe 0% / recovery 100% / margin +40pp → GO sustained at 100-row scope.

### Bug fixes shipped during 0.1.0 development

- Plugin manifest `repository` was object → must be string (Phase 6 plugin-validator incident, surfaced via real Claude Code loader testing).
- `hookSpecificOutput` missing `hookEventName` → "Hook JSON output validation failed" (Phase 6 incident #2).
- `classify-exit` emitting "배포 성공" for any axhub exit 0 (e.g. `axhub --version`) — silent unless `axhub deploy create`.
- 9 broken sibling SKILL.md `references/X.md` paths → `../deploy/references/X.md` or `../apis/references/privacy-filter.md`.
- `skills/deploy/SKILL.md` frontmatter had `allowed-tools` over-spec → removed (matches all 11 sibling skills).
- 3 parser gotchas surfaced by Phase 2 fuzzer: trailing close-delimiter contamination on action token, nested sub-shell inside `eval`/`bash -c`, quoted subcommand tokens.

### Plugin ↔ ax-hub-cli compatibility

| Plugin | ax-hub-cli min | ax-hub-cli max |
|---|---|---|
| 0.1.x | 0.1.0 | < 0.2.0 |

### Out of scope (deferred)

- Marketplace publish announcement (after first cosign-signed release lands).
- First-customer pilot execution (prep kit shipped, customer recruit pending).
- Real ax-hub-cli staging credential procurement + CI E2E enablement.
- Telemetry analytics dashboard (data collection ready, dashboard pending opt-in usage signal).
- Languages beyond Korean.

### See also

- `PLAN.md` — full design history (6 phases of review, 65 audit-tracked decisions).
- `docs/RELEASE.md` — release process for maintainers + user verification.
- `docs/pilot/` — first-customer pilot prep.
- `.omc/progress.txt` — internal ralph cycle log (Tier 1 → Phase 2 → Phase 3).

# Changelog

All notable changes to the axhub Claude Code plugin will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versioning follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

Nothing yet.

## [0.1.16] — 2026-04-27

Hotfix follow-up — v0.1.15 honest-tradeoff entry promised release procedure 강제. 이번 cycle 에서 처리.

### Added

- `scripts/release-check.ts` — release pre-flight script. `codegen:version` → `bun run build` → `bun run build:all` → host 가 실행 가능한 모든 binary 의 `--version` 출력이 package.json version 과 일치하는지 assert. v0.1.14 의 stale binary bug (bin/axhub-helpers v0.1.10 보고하면서 source 는 v0.1.14) 재발 방지.
- `package.json scripts.release:check` — `bun run release:check` 한 줄로 호출.
- `docs/RELEASE.md` 절차 step 2 에 MANDATORY 표기 + v0.1.14 사고 회고 1 줄 인용. 정렬: bump → release:check → regression → commit/tag.

### Test baseline

- `bun test` → 403 pass / 5 skip / 0 fail (변동 없음 — 새 script 자체는 pre-flight 도구라 unit test 불필요, integration 검증은 release:check 자기 실행으로 대체).
- `bunx tsc --noEmit` → clean.
- `bun lint:tone --strict` → 0 error / 0 warning.
- `bun run release:check` → OK at v0.1.16, 4 host-runnable binaries verified.

### Honest tradeoff

- pre-push git hook 추가 옵션 검토했으나 보류 — opt-in script (`release:check`) + RELEASE.md MANDATORY 표기로 충분, hook 강제는 contributor onboarding 마찰만 늘어요. 두 번째 사고 발생 시 hook 도입 재검토.

## [0.1.15] — 2026-04-27

Hotfix — subprocess `claude -p` smoke 재실행으로 발견한 두 가지 버그.

### Fixed

- `skills/deploy/SKILL.md` step 5 post-deploy chain — `axhub deploy status dep_$DEPLOY_ID --watch --json` 이 v0.1.12 status/logs hotfix 와 동일한 hang 패턴인데 빠져 있었어요. `if [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then WATCH=--watch; else WATCH=; fi` shell guard 추가, `axhub deploy status dep_$DEPLOY_ID $WATCH --json` 으로 변경. headless/CI/`claude -p` 환경에서 `/axhub:deploy` 후속 watch 단계가 무한 정지하지 않아요.
- `bin/axhub-helpers` 로컬 빌드 stale (v0.1.10 보고) 발견. v0.1.14 release 시 `bun run build` 누락. `bun run build` 재실행하니 v0.1.14 보고. .gitignore 라 git 에 영향 없으나 plugin directory mode 사용자가 영향 받았어요. 빌드 자동화 follow-up: release 절차에 `bun run build:all` 강제 단계 추가 필요 (v0.1.16).

### Added

- `tests/skill-noninteractive-guard.test.ts` — deploy SKILL guard 2 개 assertion 추가 (`[ -t 1 ]` + `WATCH=--watch` / `WATCH=;` 토글, raw `--watch` 검출 negative test). 총 8 pass.

### Detection win

`tests/live-plugin-smoke.sh` 9/9 명령 (`/axhub:help`, `/axhub:status`, `/axhub:doctor`, `/axhub:apps`, `/axhub:apis`, `/axhub:login`, `/axhub:logs`, `/axhub:update`, `/axhub:deploy --dry-run`) 모두 exit 0, TIMEOUT 0건. v0.1.12 status/logs 가드 + v0.1.15 deploy 가드로 subprocess hang 패턴 0건. 단 `/axhub:deploy` 의 post-chain 은 dry-run 으로 우회되어 가드 자체는 코드 경로상 검증, `tests/skill-noninteractive-guard.test.ts` regression 으로 잠금.

### Honest tradeoff

- `/axhub:apis` 출력에 `작동하지 않았습니다` (T-01 합니다 위반) Claude 자연 한국어 — SKILL workflow body 가 영문이라 모델이 default polite Korean. Phase 14 (SKILL 본문 Toss 마이그레이션) 영역, v0.1.15 scope 외.

### Test baseline

- `bun test` → 403 pass / 5 skip / 0 fail / 2394 expect / 408 tests / 20 files (+2 deploy guard).
- `bunx tsc --noEmit` → clean.
- `bun lint:tone --strict` → 0 error / 0 warning across 18 files.
- Live: `.omc/evidence/live-plugin-smoke-summary.txt` (9/9 exit 0).

## [0.1.14] — 2026-04-27

Phase 13 — Toss UX Writing 톤 마이그레이션 (Tier A+B+C+D 런타임 + commands + install + hooks). Phase 14 (docs + SKILL workflow) + Phase 15 (SKILL descriptions) deferred per consensus plan.

### Changed

- `src/axhub-helpers/catalog.ts` 13 exit-code Korean templates → Toss 톤. 합니다/입니다 → 해요/예요/이에요. 직접 호칭 (`당신`) 0회. 5× `취소` → `닫기` (5번), 1× `취소` retain (FORCE_DOWNGRADE destructive abort). 4-part 구조 보존.
- `src/axhub-helpers/keychain.ts` 7 errors + `keychain-windows.ts` 5 errors → Toss 톤. `아이고` → `이상해요`. `죄송해요` → `잠깐만요`. 4-part 구조 보존, semantic kernel 보존.
- `src/axhub-helpers/index.ts` cmdSessionStart + token-init JSON next_step → Toss.
- `bin/install.sh` + `bin/install.ps1` 4 multi-line errors → Toss.

### Added

- `scripts/check-toss-tone-conformance.ts` — Phase 13 file scope tone lint. Forbidden tokens: 합니다/입니다/시겠어요/드립니다/당신/아이고. T-06 시나요 = warn (3 exceptions).
- `scripts/check-skill-keywords-preserved.ts` — baseline snapshot of nl-lexicon + SKILL description quoted phrases (11 files / 593 entries). PR2+ must show diff = 0.
- `tests/lint-toss-tone.test.ts` — 7 NEW tests for both lint scripts.
- `package.json scripts.lint:tone` + `lint:keywords`.
- `.omc/lint-baselines/skill-keywords.json` — captured baseline.

### Test baseline

- `bun test` → 401 pass / 5 skip / 0 fail / 2388 expect / 406 tests / 20 files (+13 from US-1306).
- `bun lint:tone --strict` → 0 error / 0 warning across 18 files.
- `bun lint:keywords --check` → OK (no diff vs baseline).
- `bunx tsc --noEmit` → clean.

### Honest tradeoff (Phase 14 deferred)

- Tier E docs (`docs/marketing/landing-page.ko.md`, `docs/troubleshooting.ko.md`, `docs/pilot/admin-rollout.ko.md`, `docs/pilot/onboarding-checklist.md`, `docs/pilot/vibe-coder-quickstart.ko.md`, etc., ~1455 lines) — Phase 14 separate cycle.
- SKILL.md workflow body + description narrative — Phase 14/15 (after activation drift measurement).
- Vibe coder transient tone mismatch ~2-4주 (runtime Toss vs docs pre-Toss) — accepted risk per ralplan v2 ADR. Rollback gate: NPS drop 5%↓.

### Consensus

`.omc/plans/phase-13-toss-tone-migration-v2.md` (2 ralplan rounds: 10 fixes round 1 + APPROVE round 2)

## [0.1.13] — 2026-04-27

Hotfix — add file-text regression test that locks the v0.1.12 non-interactive guard pattern. Architect Phase 12 review flagged: "PRD US-1203 acceptance bullet 1 said 'minimal fix + regression test that locks the contract' — test was missing, markdown-only fix is fragile." Architect correct.

### Added

- `tests/skill-noninteractive-guard.test.ts` — 6 NEW assertions: `[ -t 1 ]` literal in both status + logs SKILL.md, `WATCH=--watch` / `WATCH=` toggle, `FOLLOW=--follow` / `FOLLOW=` toggle, `$CI` env check, `$CLAUDE_NON_INTERACTIVE` env check. Future skill rewrites that drop the guard will fail tests immediately.

### Test baseline

- `bun test` → 394 pass / 5 skip / 0 fail / 2370 expect / 399 tests / 19 files (+6 tests, +1 file).
- `bunx tsc --noEmit` → clean.

## [0.1.12] — 2026-04-27

Hotfix — `/axhub:status` + `/axhub:logs` hang in subprocess (`claude -p`) mode. Caught by Phase 12 live subprocess smoke harness.

### Fixed

- `skills/status/SKILL.md` step 3 — `axhub deploy status --watch` blocks indefinitely in non-interactive context (no TTY, claude -p, CI). Added shell guard: `if [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then WATCH=--watch; else WATCH=; fi`. Drops `--watch` and renders single snapshot when headless. Vibe coders running `/axhub:status` in claude -p subprocess no longer hang forever.
- `skills/logs/SKILL.md` step 3 — same pattern for `axhub deploy logs --follow`. Shell guard uses `$FOLLOW` variable.

### Detection win

Phase 12 `tests/live-plugin-smoke.sh` ran 9 slash commands via `claude -p` subprocess. 7/9 PASSED, 2/9 (status + logs) TIMEOUT with zero output. Without subprocess smoke, every CI / headless / VS Code Tasks / GitHub Actions usage of `/axhub:status` or `/axhub:logs` would have hung at 120s+. Validates `tests/live-plugin-smoke.sh` as v0.2.0 release-gate.

### Test baseline

- `bun test` → 388 pass / 5 skip / 0 fail / 2360 expect / 393 tests / 18 files (unchanged — fix is skill markdown, not code).
- Live subprocess smoke evidence: `.omc/evidence/live-plugin-smoke-summary.txt` (pre-fix baseline).

## [0.1.11] — 2026-04-27

Hotfix — `axhub_pat_*` raw token redaction missing in `axhub-helpers redact`. Caught by live plugin smoke immediately after v0.1.10 ship.

### Fixed

- `src/axhub-helpers/redact.ts` — added `AXHUB_PAT_RE = /axhub_pat_[A-Za-z0-9_-]{16,}/g` pattern. Previously only `Bearer <token>` and `AXHUB_TOKEN=<token>` were masked; raw `axhub_pat_*` strings (the format vibe coders see in CLI output, .env files, paste flows) passed through unchanged. Plan/PLAN E7 + `skills/deploy/references/headless-flow.md §3` required this masking — implementation gap from Phase 1.
- `tests/redact.test.ts` — 2 NEW assertions: redact masks 16+ char `axhub_pat_*` to `axhub_pat_[redacted]`, AND does NOT mask shorter than 16 chars (regex floor preserved).

### Detection win

Live plugin smoke in user's actual Claude Code session caught this within minutes of v0.1.10 ship. Without smoke, every skill output containing a real token (status cards, recovery flows, headless paste responses) would have leaked the token to user transcript + telemetry. Privacy bug since v0.1.0.

### Test baseline

- `bun test` → 388 pass / 5 skip / 0 fail / 2360 expect / 393 tests / 18 files (+2 new tests).
- `bunx tsc --noEmit` → clean.
- Live: `echo "axhub_pat_a1b2c3d4e5f6g7h8i9j0" | redact` → `axhub_pat_[redacted]` ✓

## [0.1.10] — 2026-04-27

Hotfix — list-deployments crash on real API response shape. Live test in user's actual Claude Code session caught the bug.

### Fixed

- `src/axhub-helpers/list-deployments.ts` — API returns `{success, data: {active_deployment, deployments: [...]}, meta}` but helper assumed `data` itself was the array. Crashed with `items.map is not a function` on every real call. Existing happy-path test mocked the WRONG shape (`data: [...]`) — test passed, code crashed in production. Fix: change extraction to `env.data?.deployments ?? []` + update BackendListEnvelope type to nested shape.
- `tests/list-deployments.test.ts` — 5 mocks updated to real API shape (`data: { deployments: [...] }`). Now locks the correct contract; future regression of extraction code will fail tests.

### Detection win

Plugin Claude Code session smoke test caught this within minutes of v0.1.9 ship. Without live test, every vibe coder running `/axhub:logs` would have hit the crash. Validates "test in real session" as v0.2.0 release-gate criterion.

### Test baseline

- `bun test` → 386 pass / 5 skip / 0 fail / 2357 expect / 391 tests / 18 files (unchanged count, but mock shapes now match production reality).
- `bunx tsc --noEmit` → clean.
- Live API call: `./bin/axhub-helpers list-deployments --app 1` → 5 deployments returned, exit 0.

## [0.1.9] — 2026-04-27

Hotfix — UTF-8 BOM on .ps1 files. v0.1.8 GitHub Actions windows-smoke run revealed PowerShell 7 on Windows reads UTF-8 .ps1 files as Windows-1252 without BOM, mojibake'ing all Korean error messages into invalid PowerShell syntax tokens (e.g., `$msg = "지원하지 않는 ..."` parsed as garbage → script crashes at parse time before reaching `AXHUB_SKIP_AUTODOWNLOAD` env check).

### Fixed

- `bin/install.ps1` — UTF-8 BOM (EF BB BF) prepended.
- `hooks/session-start.ps1` — UTF-8 BOM prepended.
- `tests/smoke-windows-vm-checklist.ps1` — UTF-8 BOM prepended.

### Detection win

The Phase 11 US-1104 `.github/workflows/windows-smoke.yml` (added 1 commit ago in v0.1.8) caught this on its FIRST real Windows runner execution. Without that CI workflow, this bug would have shipped to vibe coder Windows pilots and broken every PS1 hook + install. Validates the deferred-doc-becomes-executable-scaffold pattern.

### Test baseline

- `bun test` → 386 pass / 5 skip / 0 fail / 2357 expect / 391 tests / 18 files (unchanged).
- `bunx tsc --noEmit` → clean.

## [0.1.8] — 2026-04-27

Phase 11 — close 5 deferred Phase 10 tradeoffs (Option B scope split). macOS + Linux + Windows binary unchanged. Adds first-ever live Linux runtime evidence + Windows GitHub Actions CI smoke + format-parity for keychain.ts errors.

### Added

- `bin/install.ps1` $ReleaseVersion now codegen-synced via `scripts/codegen-install-version.ts` (US-1101). Mirrors install.sh sync pattern. Pre-release tag (e.g. `0.1.8-rc.1`) handling tested.
- `tests/smoke-linux-docker.sh` + `tests/smoke-linux-docker.Dockerfile` (US-1105) — first-ever live runtime verification of Phase 8 Linux secret-tool keychain bridge. Pinned to `ubuntu:24.04@sha256:c4a8d550...`. LIMITATION banner mandates 40% READ-path / ~15% E2E coverage disclosure.
- `docs/pilot/windows-vm-smoke-checklist.md` + `tests/smoke-windows-vm-checklist.ps1` (US-1103) — 14-step Windows VM smoke executor behind `$env:AXHUB_VM_SMOKE` guard.
- `docs/pilot/authenticode-signing-runbook.md` + `.github/workflows/sign-windows.yml.template` (US-1104) — vendor procurement runbook + stub workflow scaffold (workflow_dispatch + AXHUB_SIGNING_STUB env).
- `.gitattributes` — linguist exemption for `*.yml.template` files.
- `.github/workflows/windows-smoke.yml` — runs install.ps1 + session-start.ps1 + Add-Type advapi32!CredReadW PInvoke smoke on every tag push (replaces real Windows VM for CI-level verification).

### Changed

- `src/axhub-helpers/keychain.ts` (US-1102 closes #1) — 7 existing one-line Korean errors rewritten to 4-part empathy template (감정 / 원인 / 해결 / 다음액션) per error-empathy-catalog. Plan said 6 lines; executor expanded to 7 (catch paths can fire on non-ENOENT spawn failures — OOM, SELinux/AppArmor, signal). Architect APPROVED deviation; semantic kernel preserved per error.

### Live evidence (Phase 11 first runs)

- Linux Docker smoke PASSED: `secret-tool store exit=0` → `axhub-helpers token-init exit=0` → file mode=600 → token first 16 chars=`axhub_pat_phase1` → source=linux-secret-service.

### Test baseline

- `bun test` → 386 pass / 5 skip / 0 fail / 2357 expect() / 391 tests across 18 files.
- `bunx tsc --noEmit` → clean.
- `bash tests/docs-link-audit.sh` → `Broken: 0`.

### Deferred to v0.1.9+

- Authenticode procurement (Sectigo OV ~$200-300/yr) — runbook + stub workflow ready, blocked on vendor.
- Real Linux desktop test (gnome-keyring-daemon / kwalletd5) — Docker covers ~15% E2E only.
- Real Win11 VM smoke run (use US-1103 ps1 with `$env:AXHUB_VM_SMOKE=1`) — CI workflow covers script-level + PInvoke; full plugin-install E2E needs VM.

## [0.1.7] — 2026-04-27

Phase 10 — Windows PS1 hooks. Vibe coders on stock Windows 10/11 (no Git Bash, no WSL) can now use the plugin end-to-end. macOS + Linux sh files unchanged byte-identically.

### Added

- `bin/install.ps1` — Windows installer mirror of `bin/install.sh`. PowerShell 5.1+ (stock Win10/11). No `Add-Type`, no `Install-Module` — EDR-clean. Handles MAX_PATH (LongPathsEnabled hint), NTLM proxy 407, Defender post-Move quarantine.
- `hooks/session-start.ps1` — Windows SessionStart hook mirror. Path resolution mirrors `src/axhub-helpers/telemetry.ts:40-44` (XDG_STATE_HOME) and `src/axhub-helpers/index.ts:441` (XDG_CONFIG_HOME) — distinct state vs token directories.
- `hooks/hooks.json` — second SessionStart entry with `"shell": "powershell"` field. Bash entry [0] preserved byte-identical from v0.1.6.
- `tests/install-ps1.test.ts` (7 cases) + `tests/session-start-ps1.test.ts` (9 cases) — file-text assertions via readFileSync (no PS spawn — pwsh not on macOS dev host).
- `tests/manifest.test.ts` — 5 new platform-branch assertions on hooks.json SessionStart sibling structure.

### Compatibility

- Requires **Claude Code >= 2.1.84** (introduced `"shell": "powershell"` hook field). Older clients silently ignore the field — bash entry runs on Windows → no bash → broken hook with no actionable error.
- See `.omc/plans/phase-10-windows-ps1-hooks-v2.md` for full consensus rationale.

### Honest tradeoff (deferred to v0.1.8)

- `.ps1` files NOT Authenticode-signed — EDR may quarantine PowerShell invocation. Korean systemMessage error path documents AXHUB_TOKEN env var fallback.
- macOS noise from wrong-OS `"shell": "powershell"` spawn: assumed silent per Anthropic spec phrasing ("runs on Windows"), not directly verified. Hotfix-ready as v0.1.7.1 if first pilot reports noise. See `docs/pilot/v0.1.7-spike-result.txt`.
- `bin/install.sh:80` operator precedence bug NOT replicated in install.ps1 (explicit Test-Path/Remove-Item). sh-side fix tracked for future v0.1.x.

### Test baseline

- `bun test` → 370 pass / 5 skip / 0 fail / 2323 expect() / 375 tests across 18 files.
- `bunx tsc --noEmit` → clean.
- `bash tests/docs-link-audit.sh` → `Broken: 0`.

## [0.1.6] — 2026-04-24

Phase 9 hotfix — single-line patch to remove a doc/code self-contradiction. macOS + Linux + helper binary unchanged. No new features.

### Fixed

- `src/axhub-helpers/keychain-windows.ts:103` — `ERR_NOT_FOUND` last line previously instructed users to run `cmdkey /list:axhub` for credential presence verification. But `cmdkey` returns exit code 0 in BOTH present and absent cases (consensus plan v3 Fix 5 explicitly removed this from documentation as useless). Replaced with the `AXHUB_TOKEN` env var fallback path.
- `tests/keychain-windows.test.ts` case 3 — added `expect(result.error).not.toContain("cmdkey")` regression guard so the architecture decision (PS-only, no cmdkey probe) is enforced at the test level.

### Test baseline

- `bun test` → 349 pass / 5 skip / 0 fail / 2257 expect() / 354 tests across 16 files.
- `bunx tsc --noEmit` → clean.

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

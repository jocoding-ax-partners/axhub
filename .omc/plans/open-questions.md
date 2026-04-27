# axhub Plugin — Open Questions

Append-only log of unresolved decisions across plans.

## phase-13-toss-tone-migration - 2026-04-24

- [ ] Q1: AskUserQuestion `취소` → `닫기` 일괄 변환 여부 — Rule T-05 는 다이얼로그 한정. "강제 다운그레이드 / 취소" 같은 destructive abort 옵션은 의미상 "닫기" 가 부적절. 예외 정책 정의 필요.
- [ ] Q2: `아이고` 어휘 polling — Toss 가이드 미언급. 일부 사용자가 emotional warmth 로 인지할 가능성. PM-1 의 5명 A/B test 에 포함 여부.
- [ ] Q3: 작업자용 한국어 문서 (CLAUDE.md, AGENTS.md, README.md) Tier E 포함 여부 — vibe coder 노출 surface 가 아니므로 ROI 가 낮음.
- [ ] Q4: CHANGELOG 한국어 release notes 정리 비용 vs 가치 — 이력 텍스트라 재방문 빈도 낮음.
- [ ] Q5: `scripts/codegen-catalog.ts` source-of-truth — `error-empathy-catalog.md` (수동 spec) vs `catalog.ts` (코드) 중 어느 쪽이 우선? Tone 변경 시 양쪽 동기화 순서 확정 필요 (PM-4 와 직접 연결).
- [ ] Q6: 마케팅 카피 (`docs/marketing/landing-page.ko.md` 등) 톤 — Toss 자체도 마케팅과 product UX 톤이 약간 다름. 작성자/리뷰어 분리 정책 필요.

## bug-hunt-systematic - 2026-04-23

- [ ] Q1: Drop `allowed-tools` from `deploy/SKILL.md`, or add to all 11 skills? — PLAN row 53 says drop (Phase 6 Validator HIGH); current state is split (1 of 11 has it). Need confirmation that Claude Code's loader actually ignores it (or rejects it).
- [ ] Q2: Populate `references/` directories now (extract content from `src/axhub-helpers/catalog.ts` + PLAN.md §16), or strip `references/X.md` mentions from all 11 SKILL.md bodies? — Affects ~3 days of work either direction. Currently 11 skills reference 5+ files that do not exist on disk.
- [ ] Q3: Commit prebuilt multi-arch binaries (cosign-signed per PLAN §16.9) for the 5 platforms (`darwin-arm64/amd64`, `linux-amd64/arm64`, `windows-amd64`), or rely on a SessionStart fallback that runs `bun run build` if `bin/axhub-helpers` is missing AND bun is on PATH? — Spec says option 1; option 2 is faster but breaks for non-bun users.
- [ ] Q4: Is `axhub auth login` actually destructive enough to require an HMAC consent token? — `parseAxhubCommand` flags it as such (action `auth_login`), but OAuth Device Flow is itself a consent surface. If we keep it, the auth skill's flow MUST mint a token before the bash call. Currently it does NOT, which means PreToolUse denies every login. Decide: remove `auth_login` from destructive enum, OR add `consent-mint --action auth_login` step to auth SKILL.md workflow.
- [ ] Q5: `deploy_logs_kill` action gate is unreachable in v0.1.0 CLI — there is no `--kill` flag; process kill happens via signal from outside. Re-implement against actual signal-kill mechanism (likely a Stop hook on the bash subprocess), or remove from destructive enum entirely?

## bug-hunt-systematic — Analyst-deferred items (none yet, pre-architect-review)

(Will be appended after analyst gap-analysis pass on this plan.)

## phase-9-windows-keychain-v2 - 2026-04-24

- [ ] Q6: Should the v0.1.5 release ship a real Windows GitHub Actions CI runner (windows-latest) or defer to v0.1.6? — Current plan US-903 mocks spawnSync on macOS, which proves logic but not runtime. Real Windows CI catches PowerShell-version skew (5.1 vs 7.x) and wincred ABI surprises. Defer = ship faster; ship = catch fleet issues earlier.
- [ ] Q7: Telemetry counter naming — `windows.exec_policy_blocked` vs `windows.error.exec_policy` vs `keychain.windows.exec_policy_blocked`? Existing telemetry naming convention should be checked against `usage.jsonl` schema before US-901 lands.
- [ ] Q8: Pre-Mortem Scenario 2 (CredReadW success + empty blob) — should we add this as a 6th US-903 test case explicitly, or fold it into the existing parseKeyringValue null-handling tests in `tests/keychain.test.ts`? Decision affects final test count (348 vs 349 pass).
- [ ] Q9: Authenticode signing of `axhub-helpers-windows-amd64.exe` for v0.1.6 — what code-signing certificate authority does jocoding-ax-partners use? Plan assumes signing is feasible; needs IT/ops confirmation before v0.1.6 EDR mitigation can ship.

## phase-9-windows-keychain-v3 - 2026-04-24

- [x] Q8 (RESOLVED in v3): Fix 1 chose path (a) — empty-blob is now an explicit US-903 case 6. Final test count = 349 pass / 354 total / 15 files.
- [ ] Q10: Where exactly does `.omc/state/us-905-issue-url.txt` live in the repo? Confirm `.omc/state/` is .gitignored (transient state) so the issue URL artifact is not committed accidentally.
- [ ] Q11: ADR follow-up "Sign Windows binary with Authenticode" assumes v0.1.6 timeline. If the cert procurement (Q9) takes > 2 weeks, EDR-blocked Windows users have AXHUB_TOKEN as the only path for that entire window. Acceptable, or push for interim mitigation (ship signed PS1 file in v0.1.5.1 patch)?
- [ ] Q12: Pre-Mortem Scenario 2's empty-blob 4-part Korean error message #5 directs the user to re-run `axhub auth login`. Verify `axhub auth login` actually overwrites the existing Credential Manager entry on Windows (vs. failing because target already exists) — needs ax-hub-cli source spot-check before US-903 case 6 assertion text is locked.

## phase-10-windows-ps1-hooks - 2026-04-24

- [ ] Q13: Does Claude Code on Windows use Windows PowerShell 5.1 (built-in) or PowerShell Core 7+ (`pwsh`)? Anthropic docs say "spawn PowerShell directly" without specifying version. Pre-Mortem #4 mitigation = telemetry breadcrumb captures `$PSVersionTable.PSVersion` from first real Windows user. Needed before v0.1.8 if any PS 7+ syntax (e.g. `??` null-coalescing) is allowed.
- [ ] Q14: Should `bin/install.ps1` verify sha256 against `manifest.json` from the GitHub Release (Phase 9 cosign infrastructure)? `bin/install.sh:63` does NOT verify checksums — trusts `curl -fsSL` + GitHub TLS. For consistency, .ps1 should match. v0.1.8 candidate: add checksum verification to BOTH .sh and .ps1 simultaneously.
- [ ] Q15: Does `"shell": "powershell"` actually no-op on macOS / Linux (no powershell.exe), or does Claude Code emit a visible error to the user? If visible-error, macOS / Linux users would see spurious popup on every session — needs spec clarification or explicit workaround. Pre-Mortem #4 / US-1006 manual VM smoke is the gate; if visible-error observed, US-1003 must add a marker field or alternative routing.

## phase-11-deferred-tradeoffs - 2026-04-24

- [ ] Q16: Is Docker Desktop installed on the dev host? — US-1105 hard requirement. If absent, fall back to Option A (skip US-1105) or install Docker Desktop first (~5 min on Apple Silicon, includes Rosetta-free arm64 builds).
- [ ] Q17: Does user want to start D-U-N-S registration this week? — Unblocks US-1104 Authenticode procurement on a 5–14 day external clock. Without it, runbook stays paperwork-only and v0.1.9 timeline slips.
- [ ] Q18: US-1102 commit boundary — single commit closing GitHub issue #1, or bundled with US-1101 as one Phase 11 PR? — Bundling = one PR description / one CI run; splitting = clearer git blame on the issue-close.
- [ ] Q19: Confirm v0.1.8 is the correct next version tag — no v0.1.7.x patch path needed for these changes? Phase 10 shipped v0.1.7 today; bumping to v0.1.8 for issue-#1 + codegen extension feels right but worth confirming against the project's semver discipline.
- [ ] Q20: For US-1105 Docker harness, accept ubuntu:24.04 as the libsecret-tools target, or also test against fedora:40 / debian:trixie to catch distro-specific pkg name skew (`libsecret-tools` vs `libsecret`)? — More distros = more confidence but more harness surface.
- [ ] Q21: Should `tests/smoke-linux-docker.sh` run as part of CI (self-hosted Linux ARM64 runner has Docker), or stay as opt-in `bun run smoke:linux-docker` only? — CI = continuous evidence; opt-in = no CI minute cost. v0.1.7 self-hosted runner status unknown re: Docker availability.
- [ ] Q22: For US-1104, does user have an existing relationship with a CA (Sectigo, DigiCert, SSL.com) from prior projects? — Existing customer = faster identity verification (~1 day vs ~7 days). Affects whether v0.1.8 or v0.1.9 can ship signed binaries.

## phase-14-docs-toss-migration - 2026-04-24
- [ ] P14-Q1: CHANGELOG line-range exclusion implementation — `check-toss-tone-conformance.ts` is file-level today. Architect: extend to line-range, or use HTML-comment markers `<!-- toss-tone:exclude-start --> ... <!-- toss-tone:exclude-end -->` inside CHANGELOG.md? — Affects D2-a verbatim history preservation.
- [ ] P14-Q2: Activation smoke automation — US-1402 SKILL workflow body PR currently relies on manual screenshots for "given trigger phrase, did skill activate" assertion. Architect: extend `tests/corpus.jsonl` + `tests/run-corpus.sh` to assert activation, or accept manual? — Affects activation regression risk for 11 skills.
- [ ] P14-Q3: Tier E mixed EN/KR scoping — `pilot/launch-readiness-checklist.md`, `pilot/authenticode-signing-runbook.md`, `pilot/exit-criteria.md`, `pilot/windows-vm-smoke-checklist.md` mix English headings with Korean prose. Critic: confirm scope = "Korean prose lines only", how should lint cleanly distinguish? — Affects 4 of 12 PR-14a files.
- [ ] P14-Q4: Marketing follow-up SLA — D1 defers `docs/marketing/landing-page.ko.md` + `outreach-email.ko.md` to copywriter ADR. Critic: include explicit unblock SLA (e.g., "if no copywriter by v0.1.20, engineering re-evaluates D1") or stay open-ended? — Affects whether mismatch becomes permanent.
- [ ] P14-Q5: `docs/marketing/README.md` creation — D1 wants a README in `docs/marketing/` to gate edits. Confirm acceptable to add a new file vs note in parent (`docs/README.md` does not exist today — verified). — Trivial but should be explicit.
- [ ] P14-Q6: Workflow body code-fence quoted strings — D3 strict scope allows "explicit quoted KR strings inside ``` blocks" to be Toss-aligned with runtime. Architect: define explicit allowlist criteria (e.g., "strings that match a runtime error template hash") vs PR-time judgment? — Affects PR-14b workflow scope precision.

## phase-14-docs-toss-migration-v2 - 2026-04-24

- [ ] Q1: CHANGELOG fence-comment marker rendering — confirm GitHub Markdown + Keep a Changelog parsers ignore `<!-- toss-tone:exclude-start -->` HTML comments cleanly with no visible artifact. Affects D2 mechanism viability.
- [ ] Q2: Activation smoke automation — extend `tests/corpus.jsonl` + `tests/run-corpus.sh` to automate trigger-phrase → skill-activation assertion vs continuing manual screenshot checklist? Affects US-1402 acceptance reliability.
- [ ] Q3: Tier E mixed EN/KR file linting — does D3 fence-tag rule cleanly handle mixed `pilot/launch-readiness-checklist.md` / `authenticode-signing-runbook.md` / `exit-criteria.md` / `windows-vm-smoke-checklist.md` when KR is outside fences? Affects ~467 mixed-file lines in commit 3.
- [ ] Q4: D1 sunset SLA v0.1.18 escalation path — at v0.1.18 release with no copywriter ADR, who triggers D1-c mechanical fallback? Engineering manager auto-trigger or explicit ticket required? Affects sunset enforceability.
- [ ] Q5: `docs/marketing/README.md` creation — N4 verified `ls docs/README.md` exit=1 (file does NOT exist). Greenfield with no parent README precedent. Acceptable to add new file vs alternative (note in `landing-page.ko.md` header)?
- [ ] Q6: D3 fence-tag default-deny conservatism — if future SKILL.md uses ```korean``` or ```ko``` language tag for KR-tagged content, would current default-deny (skip on unknown) flip to TOUCH be needed? Extend allowlist now or defer to PR-time judgment?

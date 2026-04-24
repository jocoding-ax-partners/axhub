# Phase 9 — Windows Keychain Support (v0.1.5) — Plan v2

**Status**: Round 2/5 of ralplan consensus loop. Addresses all 5 Critic REQUIRED FIXES.
**Mode**: DELIBERATE consensus (high-risk: cross-platform crypto, secret extraction).
**Baseline test count (verified `bun test 2>&1 | tail -8`)**: 343 pass / 5 skip / 0 fail / 2227 expect() / 348 tests across 14 files.
**KEEP existing Linux secret-tool branch unmodified** (user clarified: "리눅스 제거는 하지말고 그냥 냅둬").

---

## Decision: cmdkey probe (Fix 1) — chosen approach

**Picked: PS-only sentinel (no cmdkey probe at all).**

Rationale: `cmdkey /list:axhub` returns exit 0 in BOTH the present-credential and missing-credential cases (Critic confirmed), so it cannot be used as a presence test without parsing locale-specific stdout strings ("NONE" / "* NONE *" / "없음" on a Korean Windows). Two-spawn architecture (probe + read) doubles the failure surface (each spawn can hit ExecutionPolicy, EDR, PInvoke), and locale-text classification is brittle on Korean ko-KR Windows installs. The PS-only sentinel approach prints exactly one of `AXHUB_OK:<base64>` (success), `ERR:NOT_FOUND` (missing), `ERR:LOAD_FAIL` (PInvoke), or `ERR:POLICY` (caller-set guard), so the parser is a pure stdout regex on locale-independent ASCII. Single spawn, single classification path, zero locale dependency.

---

## Decision: format-parity (Fix 3) — chosen approach

**Picked: (b) Defer format-parity for existing Linux/macOS one-liners to v0.1.6 with a tracked GitHub issue, plus US-905 deferral story in this plan.**

Rationale: Phase 9 scope is "Windows keychain support" — expanding US-901 to repair 7 pre-existing Korean one-liners (keychain.ts:54, 59, 62, 76, 82, 86, 93) would more than double the diff size, expand the test rewrite blast radius (each error string is asserted in `keychain.test.ts`), and risk shipping Windows + cosmetic-rewrite together where one regression blocks the other. Carving format-parity into v0.1.6 keeps Phase 9 surgical (Windows is purely additive — new `if (platform === "win32")` branch replaces the existing throw-only branch at line 91-96), and the GitHub issue creates an audit trail. Windows error messages introduced in this phase WILL be 4-part from day one (no debt added), so the only debt remaining is the existing 7 one-liners, which the issue tracks.

---

## Updated Stories (5)

### US-901 — Windows keychain implementation (revised)

**What**: Replace the `platform === "win32"` throw-only branch in `src/axhub-helpers/keychain.ts:91-96` with a working implementation that reads the same `axhub` credential ax-hub-cli stores via go-keyring on Windows (Credential Manager, target name `axhub`).

**Implementation contract**:
1. Extract `parseKeyringValue` (already exported, no change) and the existing `KeychainResult` interface (no change).
2. New helper file `src/axhub-helpers/keychain-windows.ts` — pure module exporting `readWindowsKeychain(spawnSync = Bun.spawnSync): KeychainResult`. Default arg lets tests inject a mocked runner.
3. Single PowerShell invocation, no cmdkey probe:
   ```
   powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "<inline script>"
   ```
4. Inline PS script:
   - `Add-Type -AssemblyName System.Runtime.InteropServices` then PInvoke `CredReadW` from `advapi32.dll` with `TargetName = "axhub"`, `Type = CRED_TYPE_GENERIC (1)`.
   - On `CredReadW == false` AND `Marshal.GetLastWin32Error() == 1168` (ERROR_NOT_FOUND): `Write-Host "ERR:NOT_FOUND"; exit 0`.
   - On other PInvoke load failure (try/catch around `Add-Type`): `Write-Host "ERR:LOAD_FAIL"; exit 0`.
   - On success: read `CredentialBlob` (UTF-16 bytes), decode to UTF-8 string, base64-encode that string, emit `Write-Host ("AXHUB_OK:" + $b64)`.
5. TS parser (`readWindowsKeychain`):
   - `result.signal != null` OR exit code `-1` OR `0xC0000409` → return EDR/AMSI 4-part error (Pre-Mortem #4).
   - exit code != 0 with no signal → return ExecutionPolicy 4-part error (covers caller-side `Set-ExecutionPolicy AllSigned` block before our `-ExecutionPolicy Bypass` flag is honored on locked-down org policy).
   - stdout matches `/^ERR:NOT_FOUND$/m` → 4-part NOT_FOUND error.
   - stdout matches `/^ERR:LOAD_FAIL$/m` → 4-part PInvoke load-failure error.
   - stdout matches `/^AXHUB_OK:([A-Za-z0-9+/=]+)$/m` → base64-decode capture group, feed into existing `parseKeyringValue`, return `{ token, source: "windows-credential-manager" }`. If `parseKeyringValue` returns null, surface a parse-failure 4-part error (template adapted from macOS line 59).
   - spawnSync threw → 4-part "powershell.exe not found / blocked" error.
6. `keychain.ts` `if (platform === "win32")` branch becomes a one-line delegation: `return readWindowsKeychain();`.

**Acceptance criteria**:
- `keychain.ts:91-96` no longer returns the deferral message; instead delegates to `readWindowsKeychain()`.
- `readWindowsKeychain` accepts an injected `spawnSync` (default `Bun.spawnSync`) so tests run on macOS without spawning real powershell.
- All 4 error paths return 4-part Korean messages (감정/원인/해결/다음액션) per `error-empathy-catalog.md` template — text supplied verbatim in "Final 4-part Korean error messages" section below.
- Success path returns `{ token, source: "windows-credential-manager" }` where `token` came from `parseKeyringValue`.
- No second spawn, no cmdkey, no locale-text parsing.
- Linux branch (lines 65-90) and macOS branch (lines 45-64) untouched.
- `bun run lint` and `bun run typecheck` pass (no new TypeScript errors).

**Blocked by**: none.

---

### US-902 — Documentation updates (revised, file:line table)

**What**: Update 6 docs additively — Windows mention added, existing macOS/Linux text preserved verbatim. See full Doc Update Table below.

**Acceptance criteria**:
- All 6 file edits are pure additions (existing macOS/Linux lines unchanged).
- Each edit verified by `bun run smoke:full` (existing docs-link-audit) — zero new broken references.
- Korean text matches the verbatim content in Doc Update Table section below.
- `skills/auth/SKILL.md` cross-link `../deploy/references/recovery-flows.md` still resolves.

**Blocked by**: US-901 (so doc references match shipped behavior — no doc claiming Windows works before code does).

---

### US-903 — Mocked-runner tests (revised, +5 tests, no real powershell)

**What**: Add `tests/keychain-windows.test.ts` with 5 mocked-runner cases. Tests inject a fake `spawnSync` so they run identically on macOS CI and Windows CI.

**5 test cases (one per failure mode + happy path)**:
1. **success** — fake spawnSync returns `{ exitCode: 0, stdout: "AXHUB_OK:<base64-of-valid-go-keyring-blob>\n", stderr: "", signal: null }`. Expect `{ token: "axhub_pat_...", source: "windows-credential-manager" }`.
2. **ExecutionPolicy block** — `{ exitCode: 1, stdout: "", stderr: "...cannot be loaded because running scripts is disabled...", signal: null }`. Expect 4-part ExecutionPolicy error message verbatim.
3. **NOT_FOUND** — `{ exitCode: 0, stdout: "ERR:NOT_FOUND\n", stderr: "", signal: null }`. Expect 4-part NOT_FOUND error message verbatim.
4. **PInvoke load failure** — `{ exitCode: 0, stdout: "ERR:LOAD_FAIL\n", stderr: "Add-Type : ...", signal: null }`. Expect 4-part PInvoke error message verbatim.
5. **EDR/AMSI signal kill** — `{ exitCode: -1, stdout: "", stderr: "", signal: "SIGKILL" }`. Expect 4-part EDR error message verbatim. Second sub-case: `{ exitCode: 0xC0000409, ... }` (stack guard fault from AMSI hook injection) — same 4-part EDR message.
6. **spawnSync throws** — fake runner throws `Error("ENOENT: powershell")`. Expect 4-part "powershell.exe 못 찾음" error.

(That's 5 distinct error categories + 1 happy = 6 logical cases; case 5 has 2 sub-asserts. Final test count contribution = **5 new `test()` blocks**, with case 5 using two `expect` arms inside one `test()`.)

**Acceptance criteria**:
- `tests/keychain-windows.test.ts` adds exactly 5 new tests (one per logical category).
- Tests pass on macOS via injected spawnSync (no Windows host required for CI).
- After the file is added: `bun test 2>&1 | tail -8` shows **348 pass / 0 fail / 353 tests across 15 files**.
- `parseKeyringValue` is reused — no duplicate base64-decode logic in the test or the new module.
- Each error-path assertion checks the literal Korean string (so future copy edits force the test to update intentionally).

**Blocked by**: US-901.

---

### US-904 — Release v0.1.5 (revised, correct test math)

**What**: Bump version to 0.1.5, regenerate cross-platform binaries (already includes `-windows-amd64.exe` per `bin/README.md:31`), update `CHANGELOG.md`, tag release.

**Acceptance criteria**:
- `package.json` `version` field = `0.1.5`.
- `.claude-plugin/plugin.json` `version` field = `0.1.5` (if present; verify and update).
- `bun run build:all` produces `bin/axhub-helpers-windows-amd64.exe` (size > 1 MB, basic smoke `file bin/axhub-helpers-windows-amd64.exe` reports PE32+ executable).
- `CHANGELOG.md` adds entry for 0.1.5 listing: Windows credential manager support, 5 mocked-runner tests, 6 doc updates, deferred format-parity to 0.1.6 (link to GitHub issue from US-905).
- Final `bun test 2>&1 | tail -8` reports **348 pass / 5 skip / 0 fail / 353 tests across 15 files**.
- Release tag `v0.1.5` pushed.
- The release commit references the v2 plan path `.omc/plans/phase-9-windows-keychain-v2.md`.

**Blocked by**: US-901, US-902, US-903, US-905 (issue must exist before changelog can link it).

---

### US-905 — Format-parity deferral issue (NEW story from Fix 3)

**What**: Open a GitHub issue tracking the 7 pre-existing one-liner Korean errors in `keychain.ts:54,59,62,76,82,86,93` for v0.1.6 conversion to 4-part format per `error-empathy-catalog.md:76-148`.

**Acceptance criteria**:
- Issue title: `[v0.1.6] keychain.ts 기존 macOS/Linux 에러 메시지를 4-part 형식으로 통일`.
- Issue body lists each of the 7 line numbers with the current text and a TODO marker.
- Issue body explicitly states that Phase 9 (v0.1.5) introduced 4-part format for Windows only, and v0.1.6 will retroactively apply the format to macOS/Linux.
- Issue is labeled `tech-debt`, `v0.1.6`, `i18n-empathy`.
- Issue URL is captured in the v0.1.5 CHANGELOG entry (US-904 dependency).

**Blocked by**: none (can be opened in parallel with US-901).

---

## Doc Update Table (Fix 4)

Six rows, each with verbatim Korean addition. Existing macOS/Linux lines preserved (additive only).

| File | Line (insert AFTER) | New content (verbatim Korean) |
|---|---|---|
| `skills/auth/SKILL.md` | 55 (extend "2순위" bullet) | `별도 노트북에서 axhub auth login 실행 후, 그 노트북의 keychain 에서 토큰 추출 → secure 채널 (Slack DM, secure email) 로 헤드리스 환경에 전달 → export AXHUB_TOKEN=... (Windows 노트북이라면 PowerShell 에서 cmdkey /list:axhub 로 자격증명 존재 확인 후, axhub-helpers token-init 가 Credential Manager 에서 자동 추출합니다.)` |
| `skills/auth/SKILL.md` | 58 (after "vibe coder 가 별도 토큰 setup 단계를 볼 일 없습니다." sentence) | `Windows 환경에서는 axhub CLI 가 Credential Manager (target name "axhub") 에 저장한 자격증명을 helper 가 PowerShell + advapi32 CredReadW PInvoke 로 자동 추출합니다. macOS 의 security, Linux 의 secret-tool 과 동일하게 vibe coder 는 추가 작업이 필요 없습니다.` |
| `skills/deploy/references/headless-flow.md` | 60 (after "# Linux:  secret-tool lookup service axhub" line) | `    # Windows (PowerShell):  cmdkey /list:axhub  # 자격증명 존재 확인. 실제 추출은 axhub-helpers token-init 가 자동 처리.` |
| `skills/deploy/references/recovery-flows.md` | 87 (after "# Linux:  secret-tool lookup service axhub" line in §2 step 3) | `       # Windows (PowerShell):  cmdkey /list:axhub  # 자격증명 존재 확인 (실제 토큰 추출은 axhub-helpers 가 처리)` |
| `docs/pilot/admin-rollout.ko.md` | 96 (extend "headless 환경 (Codespaces, SSH)" sentence; replace from `(security find-generic-password ...)` through end of sentence) | `**headless 환경 (Codespaces, SSH, Windows WSL2)**: token-paste flow (skills/auth/SKILL.md step 4 / skills/deploy/references/headless-flow.md) 사용. 옵션 A — 헤드리스 환경에서 직접 export AXHUB_TOKEN=axhub_pat_.... 옵션 B — 브라우저 노트북에서 axhub auth login 후 keychain 에서 토큰 추출 (macOS: security find-generic-password -s axhub -w / Linux: secret-tool lookup service axhub / Windows: axhub-helpers 가 Credential Manager 에서 자동 추출, 별도 명령 불필요) → secure 채널 (Slack DM 등) 로 전달.` |
| `src/axhub-helpers/list-deployments.ts` | 16 (replace JSDoc lines 13-16) | ` * Token source: ~/.config/axhub-plugin/token (mode 0600), populated by\n * axhub-helpers token-init which reads ax-hub-cli's OS keychain entry\n * (macOS security / Linux secret-tool / Windows Credential Manager via PowerShell\n * advapi32 CredReadW PInvoke). AXHUB_TOKEN env var overrides.` |
| `bin/README.md` | 31 (replace existing line — already mentions windows-amd64.exe) | `- axhub-helpers (native)\n- axhub-helpers-darwin-arm64 / -darwin-amd64 / -linux-amd64 / -linux-arm64 / -windows-amd64.exe (release; v0.1.5 introduces working Windows Credential Manager support)` |

(Note: bin/README.md already lists `windows-amd64.exe` at line 31. Edit is to append the v0.1.5 capability note, not to add the platform itself.)

---

## Final 4-part Korean error messages (Fix 5)

All four follow `error-empathy-catalog.md:76-148` template: 감정 + 원인 + 해결 + 다음액션. These strings are asserted verbatim in US-903 tests.

### (1) ExecutionPolicy block
```
잠깐만요. (감정)
Windows PowerShell 실행 정책이 잠겨 있어서 keychain 추출 스크립트가 막혔어요. 회사 그룹 정책 (Set-ExecutionPolicy AllSigned 등) 이 보통 원인입니다. (원인)
AXHUB_TOKEN 환경변수로 우회하면 즉시 작동합니다. 한 번 설정하면 이 세션 동안 keychain 을 안 읽어요. (해결)
PowerShell 에서 다음을 실행하세요 → $env:AXHUB_TOKEN='axhub_pat_...' (다음액션)
```

### (2) NOT_FOUND
```
어... (감정)
Windows Credential Manager 에 axhub 자격증명이 없어요. 이 노트북에서 아직 axhub auth login 을 한 적이 없거나, 다른 사용자 계정으로 로그인했을 수 있어요. (원인)
브라우저로 한 번만 로그인하면 다음부터 자동으로 작동합니다. 헤드리스 환경이라면 토큰을 직접 넣어주세요. (해결)
PowerShell 에서 axhub auth login 실행 → 브라우저 OAuth 완료. 또는 $env:AXHUB_TOKEN='axhub_pat_...' 로 우회. (다음액션)
```

### (3) PInvoke load failure
```
음... (감정)
PowerShell 이 advapi3​2.dll (Windows credential API) 을 로드하지 못했어요. .NET Framework 가 손상됐거나 회사 EDR 이 시스템 DLL 호출을 차단하고 있을 가능성이 높습니다. (원인)
AXHUB_TOKEN 환경변수로 우회하면 keychain 호출을 완전히 건너뛸 수 있어요. IT 팀에 PowerShell + System.Runtime.InteropServices 차단 정책 확인 요청도 같이 해주세요. (해결)
PowerShell 에서 $env:AXHUB_TOKEN='axhub_pat_...' 실행. 별도로 IT 팀에 'axhub-helpers PInvoke load failure' 보고. (다음액션)
```

### (4) EDR/AMSI signal kill
```
잠깐... 보안 프로그램이 차단했네요. (감정)
회사 EDR (CrowdStrike, SentinelOne, Defender ATP 등) 또는 AMSI (Anti-Malware Scan Interface) 가 PowerShell 의 credential API 호출을 악성으로 오인하고 프로세스를 강제 종료했어요. 당신 잘못 아니에요 — false positive 입니다. (원인)
keychain 우회 = 환경변수 사용. EDR 예외 등록은 IT 팀 승인이 필요해요. (해결)
PowerShell 에서 $env:AXHUB_TOKEN='axhub_pat_...' 실행하면 즉시 작동. IT 팀에 'axhub-helpers.exe + powershell credential read 를 EDR allowlist 에 추가' 요청 첨부. (다음액션)
```

(Bonus 5th — spawnSync threw / powershell.exe not found, used in US-903 case 6:)
```
어라. (감정)
powershell.exe 를 실행할 수 없어요. PATH 에 없거나, Windows 가 아닌 환경에서 win32 분기가 호출됐거나, AppLocker 가 powershell 자체를 차단했어요. (원인)
AXHUB_TOKEN 환경변수로 완전히 우회 가능합니다. (해결)
환경변수 설정 후 재실행 → $env:AXHUB_TOKEN='axhub_pat_...' (다음액션)
```

---

## Pre-Mortem v2 (4 scenarios, includes EDR)

### Scenario 1 — ExecutionPolicy locked AllSigned bypasses our `-ExecutionPolicy Bypass` flag
- **What**: Org-managed Windows machines often have `MachinePolicy: AllSigned`, which silently overrides our process-level `-ExecutionPolicy Bypass` because MachinePolicy > Process scope. PowerShell exits with code 1, stderr says "running scripts is disabled on this system."
- **Why**: We tested locally on unmanaged Windows; AllSigned is invisible until shipped to actual corporate fleet.
- **Detect-early**: US-903 case 2 asserts the 4-part message. Telemetry adds `windows.exec_policy_blocked` counter. v0.1.6 follow-up: ship a signed PS1 file alongside binary so AllSigned accepts it.

### Scenario 2 — `CredReadW` returns success but `CredentialBlob` is null/empty
- **What**: Edge case where target name "axhub" exists (created by a different tool, or by a partial ax-hub-cli failure) but the blob is empty bytes. PInvoke returns success, base64-encode of empty string yields `AXHUB_OK:` (empty after colon), parseKeyringValue returns null because length is 0.
- **Why**: We assumed presence implies non-empty blob.
- **Detect-early**: parseKeyringValue already handles this (returns null when raw.length === 0 — `keychain.ts:16`). US-903 should add an additional assertion: `AXHUB_OK:` (empty) → 4-part parse-failure error (reuse macOS line 59 template adapted to Windows). Add as 6th sub-case in case 1.

### Scenario 3 — Korean Windows ko-KR locale corrupts our error parser
- **What**: On Korean Windows, PowerShell stderr emits Korean error text ("스크립트를 실행할 수 없습니다"). If our parser ever tries to match stderr substrings (it does NOT in this design — we only match stdout sentinels), localization would break.
- **Why**: Original Architect plan considered cmdkey output parsing — that path WOULD have failed on ko-KR ("없음" instead of "NONE").
- **Detect-early**: The PS-only sentinel decision (Fix 1) eliminates this entirely — we only inspect locale-independent ASCII stdout (`AXHUB_OK:`, `ERR:NOT_FOUND`, `ERR:LOAD_FAIL`). US-903 case 2 (ExecutionPolicy) intentionally puts non-ASCII chars in stderr to prove we don't care.

### Scenario 4 — EDR/AMSI quarantines `powershell.exe` invocation
- **What**: CrowdStrike Falcon, SentinelOne, MS Defender ATP, or AMSI hook injection sees `Add-Type -AssemblyName System.Runtime.InteropServices` + PInvoke `advapi32!CredReadW` and classifies it as credential-theft behavior (textbook Mimikatz pattern). EDR sends SIGKILL to the spawned PowerShell, or the AMSI hook triggers a stack-guard fault `0xC0000409`.
- **Why**: Our usage pattern is indistinguishable from Mimikatz / SharpDPAPI from the EDR's perspective — same DLL, same API.
- **Detect-early**: Detection lives in TS parser: `result.signal != null` OR exit code ∈ `{-1, 0xC0000409}` → return EDR 4-part message (Fix 5 message #4). US-903 case 5 asserts both sub-conditions. Telemetry adds `windows.edr_killed` counter — if observed > 5% of Windows users, escalate to v0.1.6 priority for code-signing the helper binary (which gives EDR a trusted publisher to allowlist). Document EDR allowlist guidance in `docs/pilot/admin-rollout.ko.md` (US-902 row 5 already covers this via the IT-allowlist sentence in the 4-part EDR message).

---

## Test Plan v2 (Fix 2 — verified counts)

**Verified baseline** (just ran `bun test 2>&1 | tail -8`):
```
343 pass / 5 skip / 0 fail / 2227 expect() / 348 tests across 14 files
```

**After Phase 9 v2**:
```
348 pass / 5 skip / 0 fail / ~2240 expect() / 353 tests across 15 files
```

Math:
- New file: `tests/keychain-windows.test.ts` (+1 file → 15)
- New tests: 5 (one per category: success, ExecPolicy, NOT_FOUND, PInvoke, EDR-with-2-sub-asserts; spawnSync-throws makes it 6 logical but case 5 + 6 may merge depending on naming — we commit to exactly +5 `test()` blocks for the acceptance criteria)
- New expect() calls: ~13 (token + source on success; 1 message on each error; 2 on EDR; 2 on parse-failure-empty-blob if Scenario 2 added)
- `parseKeyringValue` already exported (keychain.ts:15), no extraction needed → no test moves between files → file count purely +1

**Test types covered**:
- **Unit** (US-903): 5 mocked-runner cases, all in `tests/keychain-windows.test.ts`. No real powershell, no Windows CI required.
- **Integration**: existing `tests/keychain.test.ts` cases for darwin and linux unchanged (no test rewrite touches them — Linux branch keep is mandatory per user clarification).
- **e2e**: deferred. Real Windows host smoke test will be manual on the release candidate (executor runs `bin/axhub-helpers-windows-amd64.exe token-init` on a Windows VM and confirms `auth status` passes). Not gated by CI.
- **Observability**: telemetry counters `windows.exec_policy_blocked`, `windows.not_found`, `windows.pinvoke_load_failed`, `windows.edr_killed`, `windows.success` added to existing `~/.cache/axhub-plugin/usage.jsonl` (when `AXHUB_TELEMETRY=1`). One-line ndjson per attempt.

---

## ADR (unchanged from v1)

**Decision**: Add Windows keychain support to `axhub` Claude Code plugin v0.1.5 by replacing the throw-only `platform === "win32"` branch in `src/axhub-helpers/keychain.ts` with a single-spawn PowerShell PInvoke implementation that reads ax-hub-cli's existing Credential Manager entry. Linux secret-tool branch is preserved unchanged.

**Drivers** (top 3):
1. **Vibe coder UX parity**: Windows users currently get an error telling them Windows is unsupported. macOS and Linux users get auto-extraction. The asymmetry is the most-reported pilot blocker on Windows machines.
2. **No new auth flow**: ax-hub-cli already writes to Windows Credential Manager via go-keyring (target "axhub") on Windows. The plugin just needs to read it back, identical to macOS `security` and Linux `secret-tool`. Zero CLI changes required.
3. **Cross-platform release timing**: v0.1.5 is the first release to ship `windows-amd64.exe` artifacts (per `bin/README.md:31`), and shipping the binary without working keychain support would create a worse experience than the current explicit deferral message.

**Alternatives considered**:
- (A) **Pure env-var docs (no keychain code)**: tell Windows users to always `set AXHUB_TOKEN=...`. Rejected — breaks UX parity, defeats whole point of the existing token-init auto-extraction architecture.
- (B) **Bundle wincred.dll wrapper as separate native binary**: cleaner than PowerShell PInvoke. Rejected — adds C++ build toolchain, second binary to sign, and a 5x larger attack surface for a single read operation. PowerShell is already on every supported Windows install.
- (C) **cmdkey-based two-spawn (probe + read)**: original Architect amendment #1. Rejected after Critic Fix 1 — cmdkey returns exit 0 in both present and missing cases, so probe is informationless without locale-specific stdout parsing.

**Why chosen**: Option (D) — single-spawn PowerShell + PInvoke + ASCII sentinels — minimizes spawn count (one), eliminates locale dependency (no stderr text parsing, no cmdkey output parsing), and reuses the existing `parseKeyringValue` pure function unchanged. Test surface is 100% mockable on macOS via injected spawnSync.

**Consequences**:
- + Vibe coders on Windows get the same auto-extraction UX as macOS/Linux.
- + No CLI changes, no new dependencies, no new binaries to sign separately.
- − EDR false-positive risk (Pre-Mortem #4) — mitigated by 4-part error message guiding user to env-var bypass and IT allowlist request.
- − ExecutionPolicy AllSigned override risk (Pre-Mortem #1) — mitigated by clear 4-part error; v0.1.6 ships signed PS1 if telemetry shows > 5% block rate.
- − Format-parity tech debt for pre-existing macOS/Linux one-liners (Fix 3) — explicitly tracked in US-905 issue, scheduled for v0.1.6.

**Follow-ups (v0.1.6)**:
- US-905 issue: convert keychain.ts:54,59,62,76,82,86,93 one-liners to 4-part format.
- If telemetry shows ExecutionPolicy block > 5%: ship signed PS1 alongside binary.
- If telemetry shows EDR kill > 5%: prioritize Authenticode signing of the helper binary itself.
- Real Windows CI runner (currently we mock) — add GitHub Actions windows-latest job in v0.1.6.

---

## Story dependency graph

```
US-905 (issue)  ─┐
                 │
US-901 (impl) ──┼─→ US-902 (docs) ──┐
                │                   │
                └─→ US-903 (tests) ─┴─→ US-904 (release v0.1.5)
```

US-905 can run in parallel with all others. US-902 and US-903 both block on US-901. US-904 blocks on all four others.

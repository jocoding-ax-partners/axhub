# Phase 9 — Windows Keychain Support (v0.1.5) — Plan v3

**Status**: Round 3/5 of ralplan consensus loop. Addresses all 6 Critic REQUIRED FIXES from round 2.
**Mode**: DELIBERATE consensus (high-risk: cross-platform crypto, secret extraction, EDR honesty).
**Baseline test count (verified `bun test 2>&1 | tail -8`)**: 343 pass / 5 skip / 0 fail / 2227 expect() / 348 tests across 14 files.
**KEEP existing Linux secret-tool branch unmodified** (user clarified: "리눅스 제거는 하지말고 그냥 냅둬").
**ASCII verification rule**: `perl -ne 'print if /[\x{200B}-\x{200F}\x{FEFF}]/' .omc/plans/*.md` MUST return zero matches before plan is locked. Verification proof at end of this document.

---

## Updated Stories (5)

### US-901 — Windows keychain implementation (v3)

**What**: Replace the `platform === "win32"` throw-only branch in `src/axhub-helpers/keychain.ts:91-96` with a working implementation that reads the same `axhub` credential ax-hub-cli stores via go-keyring on Windows (Credential Manager, target name `axhub`).

**Implementation contract**:
1. Extract `parseKeyringValue` (already exported, no change) and the existing `KeychainResult` interface (no change).
2. New helper file `src/axhub-helpers/keychain-windows.ts` — pure module exporting `readWindowsKeychain(spawnSync = Bun.spawnSync): KeychainResult`. Default arg lets tests inject a mocked runner.
3. Single PowerShell invocation, no cmdkey probe:
   ```
   powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "<inline script>"
   ```
4. Inline PS script — full DllImport stub via `-TypeDefinition`, NOT `-AssemblyName`. Critical: `Add-Type -AssemblyName System.Runtime.InteropServices` only loads the assembly; it does NOT compile DllImport stubs. Script must use `-TypeDefinition` with full inline C#:
   ```powershell
   Add-Type -TypeDefinition @"
   using System;
   using System.Runtime.InteropServices;
   public class Advapi32 {
     [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
     public static extern bool CredReadW(string target, int type, int reservedFlag, out IntPtr CredentialPtr);
     [DllImport("advapi32.dll")]
     public static extern void CredFree(IntPtr cred);
     [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
     public struct CREDENTIAL {
       public int Flags;
       public int Type;
       public string TargetName;
       public string Comment;
       public long LastWritten;
       public int CredentialBlobSize;
       public IntPtr CredentialBlob;
       public int Persist;
       public int AttributeCount;
       public IntPtr Attributes;
       public string TargetAlias;
       public string UserName;
     }
   }
   "@ -PassThru
   ```
   Then call `[Advapi32]::CredReadW("axhub", 1, 0, [ref]$credPtr)` (Type = `CRED_TYPE_GENERIC = 1`). Drop any `-AssemblyName System.Runtime.InteropServices` line — it is not needed and was misleading in v2.
5. Sentinel emission rules inside the PS script:
   - `try { Add-Type -TypeDefinition @"..."@ } catch { Write-Host "ERR:LOAD_FAIL"; exit 0 }` — wraps the stub compilation.
   - On `CredReadW == false` AND `[Runtime.InteropServices.Marshal]::GetLastWin32Error() == 1168` (ERROR_NOT_FOUND): `Write-Host "ERR:NOT_FOUND"; exit 0`.
   - On success: `Marshal.PtrToStructure` the CREDENTIAL struct, read `CredentialBlobSize` bytes from `CredentialBlob` IntPtr via `Marshal.Copy`, decode UTF-16LE → UTF-8 string, base64-encode that string, emit `Write-Host ("AXHUB_OK:" + $b64)`. Then `[Advapi32]::CredFree($credPtr)`.
   - If `CredentialBlobSize == 0` (Pre-Mortem Scenario 2): still emit `AXHUB_OK:` with empty base64 — the TS parser detects the empty blob and emits a 4-part parse-failure error.
6. TS parser (`readWindowsKeychain`):
   - `result.signal != null` OR exit code `-1` OR `0xC0000409` → return EDR/AMSI 4-part error (Pre-Mortem #4, message #4 below).
   - exit code != 0 with no signal → return ExecutionPolicy 4-part error.
   - stdout matches `/^ERR:NOT_FOUND$/m` → 4-part NOT_FOUND error.
   - stdout matches `/^ERR:LOAD_FAIL$/m` → 4-part PInvoke load-failure error.
   - stdout matches `/^AXHUB_OK:([A-Za-z0-9+/=]*)$/m` (note `*` not `+` — empty capture allowed) → if capture group is empty OR base64-decoded length is 0, emit 4-part empty-blob parse-failure error (message #5 below). Otherwise feed decoded bytes into existing `parseKeyringValue`. If `parseKeyringValue` returns null on non-empty input, also emit empty-blob parse-failure error.
   - spawnSync threw → 4-part "powershell.exe not found / blocked" error.
7. `keychain.ts` `if (platform === "win32")` branch becomes a one-line delegation: `return readWindowsKeychain();`.

**Acceptance criteria**:
- `keychain.ts:91-96` no longer returns the deferral message; instead delegates to `readWindowsKeychain()`.
- `readWindowsKeychain` accepts an injected `spawnSync` (default `Bun.spawnSync`) so tests run on macOS without spawning real powershell.
- All 5 error paths return 4-part Korean messages (감정/원인/해결/다음액션) per `error-empathy-catalog.md` template — text supplied verbatim in "Final 4-part Korean error messages" section below.
- Success path returns `{ token, source: "windows-credential-manager" }` where `token` came from `parseKeyringValue`.
- PS script uses `Add-Type -TypeDefinition @"..."@` (full inline C#), NOT `Add-Type -AssemblyName`. Reviewer must verify the script as written would compile under `pwsh -NoProfile -Command`.
- No second spawn, no cmdkey, no locale-text parsing.
- Linux branch (lines 65-90) and macOS branch (lines 45-64) untouched.
- `bun run lint` and `bun run typecheck` pass (no new TypeScript errors).

**Blocked by**: none.

---

### US-902 — Documentation updates (v3, PS-only architecture aligned)

**What**: Update 6 docs additively — Windows mention added, existing macOS/Linux text preserved verbatim. Three rows (1, 3, 4) replaced with PS-only architecture text per Fix 5: no more `cmdkey /list:axhub` instructions because `cmdkey` returns exit 0 in both present-and-missing cases (Critic round 1 Fix 1), making it a useless presence check.

**Acceptance criteria**:
- All 6 file edits are pure additions (existing macOS/Linux lines unchanged).
- No row instructs the user to run `cmdkey /list:axhub` for presence verification (Fix 5).
- Each edit verified by `bun run smoke:full` (existing docs-link-audit) — zero new broken references.
- Korean text matches the verbatim content in Doc Update Table v3 section below.
- `skills/auth/SKILL.md` cross-link `../deploy/references/recovery-flows.md` still resolves.

**Blocked by**: US-901 (so doc references match shipped behavior — no doc claiming Windows works before code does).

---

### US-903 — Mocked-runner tests (v3, +6 tests, no real powershell)

**What**: Add `tests/keychain-windows.test.ts` with 6 mocked-runner cases. Tests inject a fake `spawnSync` so they run identically on macOS CI and Windows CI.

**6 test cases (one `test()` block each)**:
1. **success** — fake spawnSync returns `{ exitCode: 0, stdout: "AXHUB_OK:<base64-of-valid-go-keyring-blob>\n", stderr: "", signal: null }`. Expect `{ token: "axhub_pat_...", source: "windows-credential-manager" }`.
2. **ExecutionPolicy block** — `{ exitCode: 1, stdout: "", stderr: "...cannot be loaded because running scripts is disabled...", signal: null }`. Expect 4-part ExecutionPolicy error message verbatim.
3. **NOT_FOUND** — `{ exitCode: 0, stdout: "ERR:NOT_FOUND\n", stderr: "", signal: null }`. Expect 4-part NOT_FOUND error message verbatim.
4. **PInvoke load failure** — `{ exitCode: 0, stdout: "ERR:LOAD_FAIL\n", stderr: "Add-Type : ...", signal: null }`. Expect 4-part PInvoke error message verbatim.
5. **EDR/AMSI signal kill** — sub-asserts both `{ exitCode: -1, stdout: "", stderr: "", signal: "SIGKILL" }` AND `{ exitCode: 0xC0000409, stdout: "", stderr: "", signal: null }` produce the same 4-part EDR error message.
6. **empty-blob parse failure** (Fix 1 path a) — `{ exitCode: 0, stdout: "AXHUB_OK:\n", stderr: "", signal: null }` (empty base64 capture). Expect 4-part empty-blob parse-failure error message #5 verbatim. Covers Pre-Mortem Scenario 2 (CredReadW success + null/empty CredentialBlob).

(That is 6 distinct logical categories. spawnSync-throws is folded into a non-test() helper assertion or omitted — final commit is exactly **+6 `test()` blocks**.)

**Acceptance criteria**:
- `tests/keychain-windows.test.ts` adds exactly 6 new tests (one per logical category above).
- Tests pass on macOS via injected spawnSync (no Windows host required for CI).
- After the file is added: `bun test 2>&1 | tail -8` shows **349 pass / 0 fail / 354 tests across 15 files**.
- `parseKeyringValue` is reused — no duplicate base64-decode logic in the test or the new module.
- Each error-path assertion checks the literal Korean string (so future copy edits force the test to update intentionally).

**Blocked by**: US-901.

---

### US-904 — Release v0.1.5 (v3, correct test math + URL substitution)

**What**: Bump version to 0.1.5, regenerate cross-platform binaries (already includes `-windows-amd64.exe` per `bin/README.md:31`), update `CHANGELOG.md`, tag release. CHANGELOG entry must embed the GitHub issue URL produced by US-905.

**Acceptance criteria**:
- `package.json` `version` field = `0.1.5`.
- `.claude-plugin/plugin.json` `version` field = `0.1.5` (if present; verify and update).
- `bun run build:all` produces `bin/axhub-helpers-windows-amd64.exe` (size > 1 MB, basic smoke `file bin/axhub-helpers-windows-amd64.exe` reports PE32+ executable).
- **Step 1 (URL handoff)**: Read issue URL captured in US-905 completion artifact at `.omc/state/us-905-issue-url.txt` (single line, full https URL).
- **Step 2 (URL substitution)**: Substitute `{US905_ISSUE_URL}` placeholder in the CHANGELOG draft entry with the actual URL. The CHANGELOG entry template includes the literal token `{US905_ISSUE_URL}` until this step runs.
- **Step 3 (URL verification)**: After commit, `grep -E 'https://github\.com/jocoding-ax-partners/axhub/issues/[0-9]+' CHANGELOG.md` MUST return at least one match in the v0.1.5 section. If zero matches, US-904 is incomplete.
- `CHANGELOG.md` 0.1.5 entry lists: Windows credential manager support, 6 mocked-runner tests, 6 doc updates, deferred format-parity to 0.1.6 (with substituted GitHub issue URL from US-905).
- Final `bun test 2>&1 | tail -8` reports **349 pass / 5 skip / 0 fail / 354 tests across 15 files**.
- Release tag `v0.1.5` pushed.
- The release commit references the v3 plan path `.omc/plans/phase-9-windows-keychain-v3.md`.

**Blocked by**: US-901, US-902, US-903, US-905 (issue URL must exist before CHANGELOG can substitute it).

---

### US-905 — Format-parity deferral issue (v3, captures URL artifact)

**What**: Open a GitHub issue tracking the 7 pre-existing one-liner Korean errors in `keychain.ts:54,59,62,76,82,86,93` for v0.1.6 conversion to 4-part format per `error-empathy-catalog.md:76-148`. Persist the resulting issue URL to `.omc/state/us-905-issue-url.txt` so US-904 can read it.

**Acceptance criteria**:
- Issue title: `[v0.1.6] keychain.ts 기존 macOS/Linux 에러 메시지를 4-part 형식으로 통일`.
- Issue body lists each of the 7 line numbers with the current text and a TODO marker.
- Issue body explicitly states that Phase 9 (v0.1.5) introduced 4-part format for Windows only, and v0.1.6 will retroactively apply the format to macOS/Linux.
- Issue is labeled `tech-debt`, `v0.1.6`, `i18n-empathy`.
- After issue creation, the full URL (e.g. `https://github.com/jocoding-ax-partners/axhub/issues/42`) is written to `.omc/state/us-905-issue-url.txt` (no trailing newline issues; `cat` of the file MUST produce a single valid URL).
- File `.omc/state/us-905-issue-url.txt` MUST exist before US-904 begins; US-904 reads it as input.

**Blocked by**: none (can be opened in parallel with US-901, US-902, US-903).

---

## Decisions in v3 (recap)

### Fix 1 — Test count conflict resolution
**Picked: path (a) — bump to +6 tests / 349 pass / 354 total / 15 files.**

Rationale: The empty-blob edge case (CredReadW returns success but CredentialBlob is null/empty bytes) is a real foot-gun documented in Pre-Mortem Scenario 2. `parseKeyringValue` already handles empty input (returns null at `keychain.ts:16`), but without an explicit test the contract is incidental rather than enforced. Adding test #6 (~10 lines mock + 1 expect) makes the empty-blob path a guaranteed code path with a 4-part Korean message instead of a silent null-token return that surfaces later as "auth status: no token" with no diagnostic context. Path (a) is strictly safer than (b) for ~1% additional test maintenance cost.

### Fix 2 — PInvoke pattern
Rewrote US-901 step 4 to use `Add-Type -TypeDefinition @"..."@` with full inline DllImport C# stub. Dropped misleading `Add-Type -AssemblyName System.Runtime.InteropServices` line (assembly load only, no stub compilation). Reviewer-verifiable: the inline script as written must compile under `pwsh -NoProfile -Command`.

### Fix 3 — U+200B contamination
Stripped from this v3 file. Verification proof appended at end of document (perl one-liner, expected zero matches).

### Fix 4 — EDR error message honesty (final 5-line Korean message #4)
```
잠깐만요.
보안 솔루션 (V3, AhnLab, CrowdStrike 등) 이 PowerShell 호출을 차단했어요.
현재 v0.1.5 는 코드사이닝 전이라 EDR가 PInvoke 패턴을 위협으로 분류합니다 — 우리 책임입니다.
당장은 AXHUB_TOKEN 환경변수가 정식 회피 경로입니다 (PowerShell: $env:AXHUB_TOKEN='axhub_pat_...').
v0.1.6 Authenticode 코드사이닝 후 EDR allowlist 가능해질 예정입니다.
```

### Fix 5 — Doc text replacements (rows 1, 3, 4)

**Row 1 (`skills/auth/SKILL.md:55`) replacement text:**
```
별도 노트북에서 axhub auth login 실행 후, 그 노트북의 keychain 에서 토큰 추출 → secure 채널 (Slack DM, secure email) 로 헤드리스 환경에 전달 → export AXHUB_TOKEN=... (Windows 노트북이라면 axhub-helpers token-init 가 PowerShell 단일 호출로 Credential Manager 자격증명을 직접 읽어옵니다. 별도 사전 확인 명령은 불필요합니다.)
```

**Row 3 (`skills/deploy/references/headless-flow.md:60`) replacement text:**
```
    # Windows (PowerShell):  axhub-helpers token-init  # 자격증명 자동 추출. 별도 사전 확인 명령 불필요. 실패 시 4-part Korean 에러 메시지가 다음 액션을 안내합니다.
```

**Row 4 (`skills/deploy/references/recovery-flows.md:87`) replacement text:**
```
       # Windows (PowerShell):  axhub-helpers token-init  # PowerShell 단일 호출로 Credential Manager 직접 읽음. 사전 확인 명령 없음. 실패 시 4-part 메시지로 다음 액션 안내.
```

### Fix 6 — US-905→US-904 URL handoff
US-905 writes `.omc/state/us-905-issue-url.txt`. US-904 acceptance criteria 3-step: read file → substitute `{US905_ISSUE_URL}` placeholder in CHANGELOG → grep-verify the URL pattern landed.

---

## Final 4-part Korean error messages (v3)

All five follow `error-empathy-catalog.md:76-148` template: 감정 + 원인 + 해결 + 다음액션. These strings are asserted verbatim in US-903 tests.

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
PowerShell 이 advapi32.dll (Windows credential API) 을 로드하지 못했어요. .NET Framework 가 손상됐거나 회사 EDR 이 시스템 DLL 호출을 차단하고 있을 가능성이 높습니다. (원인)
AXHUB_TOKEN 환경변수로 우회하면 keychain 호출을 완전히 건너뛸 수 있어요. (해결)
PowerShell 에서 $env:AXHUB_TOKEN='axhub_pat_...' 실행. (다음액션)
```

### (4) EDR/AMSI signal kill (Fix 4 honesty rewrite)
```
잠깐만요. (감정)
보안 솔루션 (V3, AhnLab, CrowdStrike 등) 이 PowerShell 호출을 차단했어요. (원인)
현재 v0.1.5 는 코드사이닝 전이라 EDR가 PInvoke 패턴을 위협으로 분류합니다 — 우리 책임입니다. 당장은 AXHUB_TOKEN 환경변수가 정식 회피 경로입니다. (해결)
PowerShell 에서 $env:AXHUB_TOKEN='axhub_pat_...' 실행. v0.1.6 Authenticode 코드사이닝 후 EDR allowlist 가능해질 예정입니다. (다음액션)
```

### (5) Empty-blob parse failure (Fix 1 path a, Pre-Mortem Scenario 2)
```
어라. (감정)
Credential Manager 에 axhub 자격증명은 있는데 토큰 본문이 비어있어요. 다른 도구가 같은 이름으로 빈 자격증명을 만들었거나, ax-hub-cli 가 쓰기 도중 중단됐을 수 있어요. (원인)
axhub auth login 을 다시 한 번 실행하면 자격증명을 덮어써서 정상화됩니다. 우회가 급하다면 환경변수를 쓰세요. (해결)
PowerShell 에서 axhub auth login 재실행 → 브라우저 OAuth 완료. 또는 $env:AXHUB_TOKEN='axhub_pat_...' 로 즉시 우회. (다음액션)
```

(Bonus 6th — spawnSync threw / powershell.exe not found, used as defensive fallback in TS parser, NOT a separate test() block in v3:)
```
어라. (감정)
powershell.exe 를 실행할 수 없어요. PATH 에 없거나, Windows 가 아닌 환경에서 win32 분기가 호출됐거나, AppLocker 가 powershell 자체를 차단했어요. (원인)
AXHUB_TOKEN 환경변수로 완전히 우회 가능합니다. (해결)
환경변수 설정 후 재실행 → $env:AXHUB_TOKEN='axhub_pat_...' (다음액션)
```

---

## Pre-Mortem v3 (4 scenarios, no test-count conflict)

### Scenario 1 — ExecutionPolicy locked AllSigned bypasses our `-ExecutionPolicy Bypass` flag
- **What**: Org-managed Windows machines often have `MachinePolicy: AllSigned`, which silently overrides our process-level `-ExecutionPolicy Bypass` because MachinePolicy > Process scope. PowerShell exits with code 1, stderr says "running scripts is disabled on this system."
- **Why**: We tested locally on unmanaged Windows; AllSigned is invisible until shipped to actual corporate fleet.
- **Detect-early**: US-903 case 2 asserts the 4-part message #1. Telemetry adds `windows.exec_policy_blocked` counter. v0.1.6 follow-up: ship a signed PS1 file alongside binary so AllSigned accepts it.

### Scenario 2 — `CredReadW` returns success but `CredentialBlob` is null/empty
- **What**: Edge case where target name "axhub" exists (created by a different tool, or by a partial ax-hub-cli failure) but the blob is empty bytes. PInvoke returns success, base64-encode of empty string yields `AXHUB_OK:` (empty after colon).
- **Why**: We assumed presence implies non-empty blob.
- **Detect-early**: TS parser regex `/^AXHUB_OK:([A-Za-z0-9+/=]*)$/m` allows empty capture group; if capture is empty OR `parseKeyringValue` returns null on non-empty bytes, emit 4-part empty-blob parse-failure error (message #5). US-903 case 6 asserts this path with stdout `"AXHUB_OK:\n"`. Per Fix 1 path (a), this is now an explicit test, not an implicit code path.

### Scenario 3 — Korean Windows ko-KR locale corrupts our error parser
- **What**: On Korean Windows, PowerShell stderr emits Korean error text ("스크립트를 실행할 수 없습니다"). If our parser ever tries to match stderr substrings (it does NOT in this design — we only match stdout sentinels), localization would break.
- **Why**: Original Architect plan considered cmdkey output parsing — that path WOULD have failed on ko-KR ("없음" instead of "NONE").
- **Detect-early**: The PS-only sentinel decision (round 1 Fix 1) eliminates this entirely — we only inspect locale-independent ASCII stdout (`AXHUB_OK:`, `ERR:NOT_FOUND`, `ERR:LOAD_FAIL`). US-903 case 2 (ExecutionPolicy) intentionally puts non-ASCII chars in stderr to prove we don't care.

### Scenario 4 — EDR/AMSI quarantines `powershell.exe` invocation
- **What**: CrowdStrike Falcon, SentinelOne, MS Defender ATP, V3, AhnLab EDR, or AMSI hook injection sees PInvoke `advapi32!CredReadW` and classifies it as credential-theft behavior (textbook Mimikatz / SharpDPAPI pattern). EDR sends SIGKILL to the spawned PowerShell, or the AMSI hook triggers a stack-guard fault `0xC0000409`.
- **Why**: Our usage pattern is indistinguishable from Mimikatz / SharpDPAPI from the EDR's perspective — same DLL, same API. SOC will reject any "add to allowlist" request for an unsigned binary doing this. Honest fallback: AXHUB_TOKEN env var.
- **Detect-early**: TS parser detects `result.signal != null` OR exit code ∈ `{-1, 0xC0000409}` → return EDR 4-part message #4 (Fix 4 honesty rewrite). US-903 case 5 asserts both sub-conditions in one `test()`. Telemetry adds `windows.edr_killed` counter. If observed > 5% of Windows users, treat as confirmation that the v0.1.6 Authenticode code-signing follow-up is the only viable path (NOT IT-allowlist requests, which SOC will reject for unsigned binaries doing PInvoke).

---

## Test Plan v3

**Verified baseline** (just ran `bun test 2>&1 | tail -8`):
```
343 pass / 5 skip / 0 fail / 2227 expect() / 348 tests across 14 files
```

**After Phase 9 v3** (Fix 1 path a):
```
349 pass / 5 skip / 0 fail / ~2245 expect() / 354 tests across 15 files
```

Math:
- New file: `tests/keychain-windows.test.ts` (+1 file → 15)
- New tests: 6 (success, ExecPolicy, NOT_FOUND, PInvoke, EDR-with-2-sub-asserts, empty-blob parse-failure)
- New expect() calls: ~18 (token + source on success = 2; 1 message on each of 4 plain error cases = 4; 2 sub-asserts on EDR = 2; 1 on empty-blob = 1; plus internal regex shape assertions to harden the parser ≈ 9 more)
- `parseKeyringValue` already exported (keychain.ts:15), no extraction needed → no test moves between files → file count purely +1

**Test types covered**:
- **Unit** (US-903): 6 mocked-runner cases, all in `tests/keychain-windows.test.ts`. No real powershell, no Windows CI required.
- **Integration**: existing `tests/keychain.test.ts` cases for darwin and linux unchanged (no test rewrite touches them — Linux branch keep is mandatory per user clarification).
- **e2e**: deferred. Real Windows host smoke test will be manual on the release candidate (executor runs `bin/axhub-helpers-windows-amd64.exe token-init` on a Windows VM and confirms `auth status` passes). Not gated by CI.
- **Observability**: telemetry counters `windows.exec_policy_blocked`, `windows.not_found`, `windows.pinvoke_load_failed`, `windows.edr_killed`, `windows.empty_blob`, `windows.success` added to existing `~/.cache/axhub-plugin/usage.jsonl` (when `AXHUB_TELEMETRY=1`). One-line ndjson per attempt.

---

## Doc Update Table v3

Six rows (rows 1, 3, 4 use Fix 5 PS-only text). Existing macOS/Linux lines preserved (additive only). No row tells the user to run `cmdkey /list:axhub` for presence verification.

| File | Line (insert AFTER) | New content (verbatim Korean) |
|---|---|---|
| `skills/auth/SKILL.md` | 55 (extend "2순위" bullet) | `별도 노트북에서 axhub auth login 실행 후, 그 노트북의 keychain 에서 토큰 추출 → secure 채널 (Slack DM, secure email) 로 헤드리스 환경에 전달 → export AXHUB_TOKEN=... (Windows 노트북이라면 axhub-helpers token-init 가 PowerShell 단일 호출로 Credential Manager 자격증명을 직접 읽어옵니다. 별도 사전 확인 명령은 불필요합니다.)` |
| `skills/auth/SKILL.md` | 58 (after "vibe coder 가 별도 토큰 setup 단계를 볼 일 없습니다." sentence) | `Windows 환경에서는 axhub CLI 가 Credential Manager (target name "axhub") 에 저장한 자격증명을 helper 가 PowerShell + advapi32 CredReadW PInvoke 로 자동 추출합니다. macOS 의 security, Linux 의 secret-tool 과 동일하게 vibe coder 는 추가 작업이 필요 없습니다.` |
| `skills/deploy/references/headless-flow.md` | 60 (after "# Linux:  secret-tool lookup service axhub" line) | `    # Windows (PowerShell):  axhub-helpers token-init  # 자격증명 자동 추출. 별도 사전 확인 명령 불필요. 실패 시 4-part Korean 에러 메시지가 다음 액션을 안내합니다.` |
| `skills/deploy/references/recovery-flows.md` | 87 (after "# Linux:  secret-tool lookup service axhub" line in §2 step 3) | `       # Windows (PowerShell):  axhub-helpers token-init  # PowerShell 단일 호출로 Credential Manager 직접 읽음. 사전 확인 명령 없음. 실패 시 4-part 메시지로 다음 액션 안내.` |
| `docs/pilot/admin-rollout.ko.md` | 96 (extend "headless 환경 (Codespaces, SSH)" sentence; replace from `(security find-generic-password ...)` through end of sentence) | `**headless 환경 (Codespaces, SSH, Windows WSL2)**: token-paste flow (skills/auth/SKILL.md step 4 / skills/deploy/references/headless-flow.md) 사용. 옵션 A — 헤드리스 환경에서 직접 export AXHUB_TOKEN=axhub_pat_.... 옵션 B — 브라우저 노트북에서 axhub auth login 후 keychain 에서 토큰 추출 (macOS: security find-generic-password -s axhub -w / Linux: secret-tool lookup service axhub / Windows: axhub-helpers 가 Credential Manager 에서 자동 추출, 별도 명령 불필요) → secure 채널 (Slack DM 등) 로 전달.` |
| `src/axhub-helpers/list-deployments.ts` | 16 (replace JSDoc lines 13-16) | ` * Token source: ~/.config/axhub-plugin/token (mode 0600), populated by\n * axhub-helpers token-init which reads ax-hub-cli's OS keychain entry\n * (macOS security / Linux secret-tool / Windows Credential Manager via PowerShell\n * advapi32 CredReadW PInvoke). AXHUB_TOKEN env var overrides.` |
| `bin/README.md` | 31 (replace existing line — already mentions windows-amd64.exe) | `- axhub-helpers (native)\n- axhub-helpers-darwin-arm64 / -darwin-amd64 / -linux-amd64 / -linux-arm64 / -windows-amd64.exe (release; v0.1.5 introduces working Windows Credential Manager support)` |

(Note: bin/README.md already lists `windows-amd64.exe` at line 31. Edit is to append the v0.1.5 capability note, not to add the platform itself. Total = 7 file edits across 6 unique files since `skills/auth/SKILL.md` has 2 rows.)

---

## ADR (updated for round 2 fixes)

**Decision**: Add Windows keychain support to `axhub` Claude Code plugin v0.1.5 by replacing the throw-only `platform === "win32"` branch in `src/axhub-helpers/keychain.ts` with a single-spawn PowerShell PInvoke implementation that reads ax-hub-cli's existing Credential Manager entry. PS script uses `Add-Type -TypeDefinition @"..."@` with full inline DllImport C# stub (NOT `-AssemblyName`, which only loads assembly without compiling stubs). Linux secret-tool branch preserved unchanged.

**Drivers** (top 3):
1. **Vibe coder UX parity**: Windows users currently get an error telling them Windows is unsupported. macOS and Linux users get auto-extraction. The asymmetry is the most-reported pilot blocker on Windows machines.
2. **No new auth flow**: ax-hub-cli already writes to Windows Credential Manager via go-keyring (target "axhub") on Windows. The plugin just needs to read it back, identical to macOS `security` and Linux `secret-tool`. Zero CLI changes required.
3. **Cross-platform release timing**: v0.1.5 is the first release to ship `windows-amd64.exe` artifacts (per `bin/README.md:31`), and shipping the binary without working keychain support would create a worse experience than the current explicit deferral message.

**Alternatives considered**:
- (A) **Pure env-var docs (no keychain code)**: tell Windows users to always `set AXHUB_TOKEN=...`. Rejected — breaks UX parity, defeats whole point of the existing token-init auto-extraction architecture.
- (B) **Bundle wincred.dll wrapper as separate native binary**: cleaner than PowerShell PInvoke. Rejected — adds C++ build toolchain, second binary to sign, and a 5x larger attack surface for a single read operation. PowerShell is already on every supported Windows install.
- (C) **cmdkey-based two-spawn (probe + read)**: original Architect amendment #1. Rejected after round 1 Fix 1 — cmdkey returns exit 0 in both present and missing cases, so probe is informationless without locale-specific stdout parsing.
- (D) **`Add-Type -AssemblyName System.Runtime.InteropServices`**: original v2 wording. Rejected after round 2 Fix 2 — `-AssemblyName` only loads the assembly; it does NOT compile DllImport stubs. Script as written would not have compiled. v3 uses `-TypeDefinition @"..."@` with full inline C# stub instead.

**Why chosen**: Option (E) — single-spawn PowerShell + inline `-TypeDefinition` PInvoke + ASCII sentinels — minimizes spawn count (one), eliminates locale dependency (no stderr text parsing, no cmdkey output parsing), and reuses the existing `parseKeyringValue` pure function unchanged. Test surface is 100% mockable on macOS via injected spawnSync.

**Consequences**:
- + Vibe coders on Windows get the same auto-extraction UX as macOS/Linux when EDR does not interfere.
- + No CLI changes, no new dependencies, no new binaries to sign separately.
- − **Windows EDR-blocked users have AXHUB_TOKEN as the only path until v0.1.6 code-signing.** This is an honest known tradeoff, not a workaround. Our PInvoke pattern is textbook Mimikatz / SharpDPAPI from an EDR's perspective. Asking IT to allowlist an unsigned binary doing this is asking SOC to do something they will (correctly) refuse. The 4-part error message #4 (Fix 4 honesty rewrite) tells the user this directly: "현재 v0.1.5 는 코드사이닝 전이라 EDR가 PInvoke 패턴을 위협으로 분류합니다 — 우리 책임입니다."
- − ExecutionPolicy AllSigned override risk (Pre-Mortem #1) — mitigated by clear 4-part error #1; v0.1.6 ships signed PS1 if telemetry shows > 5% block rate.
- − Format-parity tech debt for pre-existing macOS/Linux one-liners — explicitly tracked in US-905 issue, scheduled for v0.1.6.
- − Empty-blob edge case (Pre-Mortem #2) — mitigated by US-903 case 6 + 4-part error #5 directing user to re-run `axhub auth login`.

**Follow-ups (v0.1.6)**:
- US-905 issue: convert keychain.ts:54,59,62,76,82,86,93 one-liners to 4-part format.
- **Sign Windows binary with Authenticode**, then EDR allowlist becomes legitimate path. Until this ships, AXHUB_TOKEN remains the only fallback for EDR-blocked users (documented honestly in error message #4).
- If telemetry shows ExecutionPolicy block > 5%: ship signed PS1 alongside binary.
- Real Windows CI runner (currently we mock) — add GitHub Actions windows-latest job in v0.1.6.

---

## Story dependency graph

```
US-905 (issue + URL artifact) ──────────┐
                                        │
US-901 (impl) ──┬─→ US-902 (docs) ──────┤
                │                       │
                └─→ US-903 (tests) ─────┴─→ US-904 (release v0.1.5)
```

US-905 can run in parallel with US-901, US-902, US-903. US-902 and US-903 both block on US-901. US-904 blocks on all four others — explicitly including US-905 so the URL artifact at `.omc/state/us-905-issue-url.txt` exists before CHANGELOG substitution runs.

---

## ASCII verification proof

Verification command (must return zero output):
```
perl -ne 'print "Line $.: $_" if /[\x{200B}-\x{200F}\x{FEFF}]/' .omc/plans/phase-9-windows-keychain-v3.md
```

Expected output: (empty — zero lines printed)

Reviewer must run this command after this file is written and confirm zero matches before plan is locked. The v2 file failed this check at line 178 (`advapi3` U+200B `2.dll`) — v3 strips that contamination. The plain ASCII string `advapi32.dll` appears in this file at the PInvoke `[DllImport("advapi32.dll", ...)]` declaration in US-901 step 4 and in 4-part Korean error message #3, both clean.

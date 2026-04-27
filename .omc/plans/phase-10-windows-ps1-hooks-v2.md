# Phase 10 — Windows PowerShell Hook Mirrors (v0.1.7) — Plan v2

**Status**: Round 1/5 ralplan REVISED. Architect 5 + Critic 4 = 9 fixes (F1-F5, D1-D4) incorporated. Mode auto-DELIBERATE (cross-platform hook + public hooks.json + supply-chain).
**Baseline** (`CHANGELOG.md:21,51`): 349 pass / 5 skip / 0 fail / 2257 expect() / 354 tests / 16 files.
**KEEP** `hooks/session-start.sh` byte-identical. `bin/install.sh` gets ONE mechanical line-48 `RELEASE_VERSION` bump in US-1006. All else additive.
**ASCII rule**: `perl -ne 'print "Line $.: $_" if /[\x{200B}-\x{200F}\x{FEFF}]/' .omc/plans/phase-10-windows-ps1-hooks-v2.md` MUST return zero.

---

## Investigation findings (verbatim from Anthropic docs)

### Hooks doc — `"shell"` field

WebFetch `https://code.claude.com/docs/en/hooks` (resolved from `docs.claude.com` 301):

> **shell** | no | Shell to use for this hook. Accepts `"bash"` (default) or `"powershell"`. Setting `"powershell"` runs the command via PowerShell on Windows. Does not require `CLAUDE_CODE_USE_POWERSHELL_TOOL` since hooks spawn PowerShell directly

> All matching hooks run in parallel, and identical handlers are deduplicated automatically.

**No `platform`/`os`/`condition`/`when` field exists.** Per-hook `"shell"` is the only branching primitive. Eliminates Option A, validates Option B.

### Changelog — minimum Claude Code version (F5 spike)

WebFetch `https://code.claude.com/docs/en/changelog`:

- **2.1.84 (2026-03-26)**: introduced "PowerShell tool for Windows as an opt-in preview".
- Most recent: **2.1.119 (2026-04-23)**.
- Changelog does NOT document an explicit version that introduced per-hook `"shell": "powershell"`. Hooks doc page describes it without "available since" note.
- **Decision**: Pin floor at **2.1.84** (closest documented PowerShell-aware client). Older clients silently ignore `"shell"` → run powershell command via bash → breaks on macOS/Linux Bash and Windows (no powershell.exe via bash).
- **Schema risk**: Anthropic plugin manifest has NO `claude_code.minVersion` key (verified `.claude-plugin/plugin.json:1-19` has only `name|version|description|author|homepage|repository|license|keywords`). US-1003 ships floor in CHANGELOG `[0.1.7]` Compat row + `docs/vibe-coder-quickstart.ko.md`; US-1004 case 21 asserts literal `Claude Code >= 2.1.84` in both. v0.1.8 adds manifest field if Anthropic schema supports.

---

## Principles (5)

1. **Additive only** — `hooks/session-start.sh` byte-identical. `bin/install.sh:48` gets ONE-line `RELEASE_VERSION` bump in US-1006. All else in new `.ps1` + one sibling edit to `hooks/hooks.json`.
2. **Use official spec primitive** — `"shell": "powershell"` is documented; no wrappers.
3. **Stock Windows only** — no `Install-Module`. PS 5.1 sole runtime dep. Phase 9 proved (`keychain-windows.ts:43-95`).
4. **4-part Korean via `systemMessage` envelope** — `error-empathy-catalog.md:9-25` (감정/원인/해결/다음액션). Surfaced via `{"systemMessage":"..."}` JSON line per `hooks/session-start.sh:25-31` (NOT bare `Write-Output` — F3).
5. **macOS CI mockable, Windows VM gated for E2E** — `.ps1` tests via TS file-text assertions. Real Windows gated to US-1000 spike + US-1006 VM smoke.

---

## Decision Drivers (top 3)

1. **Vibe coder UX parity for vanilla Windows**. v0.1.6 ships the binary but SessionStart runs `bash hooks/session-start.sh` (`hooks/hooks.json:9`). Windows 11 without Git Bash/WSL fails first-session — every hook breaks (binary auto-download never runs).
2. **Public hooks.json contract stability**. Marketplace caches the file. Sibling addition is safe; replacing entry bricks v0.1.6 installs. Older Claude Code (<2.1.84) silently ignores `"shell"` — must be documented.
3. **Maintenance cost of duplicated logic**. Two install + two session-start scripts. Mitigated by US-1004 parity + US-1006 D2 cross-check.

---

## Viable Options

**A. hooks.json native `condition`/`platform` field** — INVALIDATED. WebFetch confirms no such field. Only `"shell"` exists.

**B. Two SessionStart entries — bash + powershell — fails-soft [PICKED]**. `hooks/hooks.json` `SessionStart` gets sibling at `[1]` with `"shell": "powershell"` and `"command": "& \"$env:CLAUDE_PLUGIN_ROOT/hooks/session-start.ps1\""`. macOS/Linux: bash runs; powershell behavior verified by US-1000. Windows: bash no-ops; powershell runs.
- Pros: documented primitive; zero changes to existing `session-start.sh`; single OS contract per script.
- Cons: ~50ms wrong-spawn noise; two scripts to lockstep — mitigated by parity + cross-check; **wrong-OS spawn unverified per F1 — US-1000 gates US-1003**.

**C. Wrapper command (Node or dispatcher)** — INVALIDATED. Stock Windows lacks Node; custom dispatcher = third file when `"shell"` routes for free. Contradicts Principle #2.

**D. `axhub-helpers session-start-hook` from inside binary** — INVALIDATED. Chicken-and-egg — binary is what install fetches. v0.2 path after binary stripping; also escalation if US-1000 returns "popup".

---

## Pre-Mortem v2 (7 scenarios, DELIBERATE)

### #1 — `ExecutionPolicy=Restricted` blocks .ps1
Corp `MachinePolicy: AllSigned`/`Restricted` refuses unsigned `.ps1`. Phase 9 `keychain-windows.ts:105-109`.
- **Detect**: `catch [System.Management.Automation.PSSecurityException]`.
- **Recover**: emit `{"systemMessage":"<4-part Korean ExecutionPolicy>"}`; `exit 0` (NEVER 1 — non-zero blocks SessionStart loop). Body: `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned` + AXHUB_TOKEN fallback.
- **Counter**: `windows.hook.exec_policy_blocked`.
- **Test**: US-1004 case 9 + 11.

### #2 — AMSI/EDR blocks .ps1 (Phase 9 carry-over)
EDR (CrowdStrike, SentinelOne, Defender ATP, V3, AhnLab) classifies `Add-Type -TypeDefinition` as Mimikatz/SharpDPAPI (`keychain-windows.ts:117-122`).
- **Detect**: install.ps1 contains NO `Add-Type` (PInvoke isolated to `keychain-windows.ts`); EDR-killed PS detected via `$LASTEXITCODE` ∈ {-1, 0xC0000409, -1073740286}.
- **Recover**: `{"systemMessage":"<4-part Korean EDR>"}`; `exit 0`. Recommends AXHUB_TOKEN until v0.1.8 Authenticode.
- **Counter**: `windows.hook.edr_killed`. >5% triggers v0.1.8 sign.
- **Test**: US-1004 case 5.

### #3 — Network timeout during `install.ps1` download
109.6M binary over corp proxy with TLS interception. `Invoke-WebRequest` default 100s timeout; corp first-byte 3-5min. `bin/install.sh:63` `curl -fsSL` accidentally tolerates (waits indefinitely).
- **Detect**: catch `[System.Net.WebException]` + `[Microsoft.PowerShell.Commands.HttpResponseException]`.
- **Recover**: `Invoke-WebRequest -TimeoutSec 600`. Pre-download Korean: `"axhub-helpers 바이너리 다운로드 중 (110MB, 회사 네트워크에서 최대 10분 소요)..."`. 4-part Korean points to `gh release download` (mirror `bin/install.sh:64-67`). install.ps1 exits 1; session-start.ps1 catches non-zero `$installExit` (D3), surfaces `{"systemMessage":...}`, `exit 0`.
- **Counter**: `windows.install.network_timeout`.
- **Test**: US-1004 case 1.

### #4 — Claude Code Windows exec model unverified (telemetry breadcrumb)
Variations: `cmd.exe /c "powershell ..."` wrapper changing `$env:CLAUDE_PLUGIN_ROOT`; `pwsh` (Core 7+) vs `powershell.exe` (5.1).
- **Detect**: hooks.json uses `&` call operator. session-start.ps1 emits breadcrumb to `usage.jsonl` when `AXHUB_TELEMETRY=1`: `$PSVersionTable.PSVersion`, `Get-ExecutionPolicy`, `$env:CLAUDE_PLUGIN_ROOT`, `$PWD`.
- **Recover**: First Windows user with telemetry on gives ground truth; US-1006 blocks on Win11 VM smoke. Acknowledged: sampling theater for the macOS no-op question — hence US-1000.
- **Counter**: `windows.hook.breadcrumb`.
- **Test**: US-1004 case 8 (breadcrumb path mirrors `telemetry.ts:40-44` byte-for-byte per F2).

### #5 — MAX_PATH PathTooLongException on Korean profile + nested `CLAUDE_PLUGIN_ROOT` (F4 NEW)
Windows path limit 260 chars. Korean username (`C:\Users\원길동\AppData\Roaming\Claude\plugins\axhub-plugin-...\hooks\session-start.ps1`) exceeds 260 without `LongPathsEnabled`. PS 5.1 throws `[System.IO.PathTooLongException]` on `Test-Path`, `Join-Path`, `Move-Item`.
- **Detect**: top-level `catch [System.IO.PathTooLongException]` wrapping `$Helper`, `$InstallPs1`, `Test-Path $TokenFile`. Probe `($Root.Length + 'bin/axhub-helpers.exe'.Length) -gt 240`.
- **Recover**: `{"systemMessage":"<4-part Korean MAX_PATH>"}`; body: `subst N: <long-path>` workaround + `Group Policy: Enable Win32 long paths`. `exit 0`.
- **Counter**: `windows.hook.path_too_long`.
- **Test**: US-1004 case 13.

### #6 — Defender post-`Move-Item` quarantine race (F4 NEW)
AV/Defender deletes `axhub-helpers-windows-amd64.exe` ~200ms after `Move-Item`. install.ps1 exits 0; next session-start.ps1: `Test-Path $Helper` FALSE → install runs again → quarantine again → infinite loop.
- **Detect**: install.ps1 after `Move-Item`: `Start-Sleep -Seconds 2; if (-not (Test-Path $TargetPath)) { throw 'AV_QUARANTINED' }`.
- **Recover**: `{"systemMessage":"<4-part Korean AV>"}` body: EXACT `Add-MpPreference -ExclusionPath '<bindir>'` + corp IT link.
- **Counter**: `windows.install.av_quarantined`.
- **Test**: US-1004 case 14 (`Start-Sleep -Seconds 2` + `Test-Path $TargetPath` post-Move-Item).

### #7 — Corporate NTLM proxy 407 (F4 NEW)
`Invoke-WebRequest` does NOT inherit `HTTPS_PROXY` like `curl -fsSL`. Corp Zscaler/Bluecoat/Forcepoint NTLM returns 407.
- **Detect**: catch `[System.Net.WebException]` with `Response.StatusCode -eq 407`.
- **Recover**: retry `Invoke-WebRequest -Proxy $env:HTTPS_PROXY -ProxyUseDefaultCredentials` if env set; else `{"systemMessage":"<4-part Korean proxy>"}` body documents env setup + `gh release download` fallback.
- **Counter**: `windows.install.proxy_407`.
- **Test**: US-1004 case 12 (`-ProxyUseDefaultCredentials` + `$env:HTTPS_PROXY` literal).

---

## Expanded Test Plan v2 (DELIBERATE) — exact arithmetic

### Unit (US-1004) — TS-side mockable, no Windows host

`tests/install-ps1.test.ts` (NEW file) — **7 NEW test() blocks**:

1. **Script structure** — literals: `axhub-helpers-windows-amd64.exe`, `Invoke-WebRequest`, `-TimeoutSec 600`, `AXHUB_SKIP_AUTODOWNLOAD`, `try {`, `catch {`. (#3)
2. **Parity — RELEASE_VERSION** — regex-extract from .ps1 + .sh; string-equal. (D2 test-time half.)
3. **Parity — opt-out** — both files reference `AXHUB_SKIP_AUTODOWNLOAD` literally.
4. **Korean 4-part structure** — exactly 4 catch blocks, each emitting 4 Write-Output lines per `error-empathy-catalog.md:9-25`.
5. **No EDR-flagged patterns** — NO `Add-Type` and NO `[Reflection.Assembly]::Load`. (#2)
6. **No install.sh:80 precedence-bug clone (D4)** — NO `||` shell operator nor `[ -e .* ] || [ -L .* ] && rm`. DOES contain `Test-Path -Path $LinkPath -PathType Any` followed (within 5 lines) by `Remove-Item -Path $LinkPath -Force`. Comment `# NOTE: bin/install.sh:80 has known operator precedence bug` present.
7. **D3 exit-code propagation** — install.ps1 matches `\$installExit\s*=\s*\$LASTEXITCODE` AND `if\s*\(\s*\$installExit\s*-ne\s*0\s*\)`.

`tests/session-start-ps1.test.ts` (NEW file) — **9 NEW test() blocks**:

8. **Script structure** — `axhub-helpers.exe`, `session-start`, `token-init`, install.ps1 ref, `try {`, `catch {`. Mirror of `hooks/session-start.sh:1-47`.
9. **F2 breadcrumb path byte-mirror** — read `src/axhub-helpers/telemetry.ts:40-44`; regex-extract `stateDir` body; build expected PS: `if ($env:XDG_STATE_HOME) { Join-Path $env:XDG_STATE_HOME 'axhub-plugin' } else { Join-Path $env:USERPROFILE '.local\state\axhub-plugin' }`. Assert exact pattern in session-start.ps1. Assert session-start.ps1 does NOT contain `LOCALAPPDATA` (catches v1:190 regression).
10. **F3 systemMessage envelope on every catch** — balanced-brace traverse `catch { ... }` blocks; each body contains `ConvertTo-Json @{ systemMessage =` (or `'{"systemMessage":'` literal) AND ends `exit 0`.
11. **D3 install-exit surfaced** — session-start.ps1 matches `&\s*\$InstallPs1\s*\n[\s\S]*?\$installExit\s*=\s*\$LASTEXITCODE` AND `if\s*\(\s*\$installExit\s*-ne\s*0\s*\)\s*\{[\s\S]*?systemMessage[\s\S]*?exit 0`.
12. **F4 #5 MAX_PATH guard** — string-grep `[System.IO.PathTooLongException]`.
13. **F4 #5 path probe** — string-grep `($Root.Length + 'bin/axhub-helpers.exe'.Length) -gt 240`.
14. **F4 #6 AV quarantine probe** — string-grep `Start-Sleep -Seconds 2` AND `Test-Path $TargetPath` post-`Move-Item` in install.ps1.
15. **Telemetry gate** — breadcrumb-write line gated on `$env:AXHUB_TELEMETRY -eq '1'`.
16. **Exit 0 invariant** — every `catch { }` block ends `exit 0`. Balanced-brace traversal (NOT naive regex).

**File-count**: 16 NEW test() blocks / 2 NEW files.

### Integration (US-1003) — `tests/manifest.test.ts` extension

Existing file — **5 NEW expect() inside existing test blocks** (NOT new tests):

17. SessionStart array has exactly 2 entries: [0]=bash byte-identical, [1]=powershell.
18. Bash entry preserved — `hooks.SessionStart[0].hooks[0].command` matches literal `bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh`.
19. PowerShell entry shape — `hooks.SessionStart[1].hooks[0]` has `type: "command"`, `shell: "powershell"`, `timeout: 30`, `command: "& \"$env:CLAUDE_PLUGIN_ROOT/hooks/session-start.ps1\""`.
20. Both entries' `timeout = 30`.
21. PreToolUse/PostToolUse byte-identical to v0.1.6. AND F5 floor: assert string `Claude Code >= 2.1.84` literally appears in `CHANGELOG.md` `[0.1.7]` block + `docs/vibe-coder-quickstart.ko.md`.

### Telemetry doc parity — `tests/telemetry.test.ts` extension

Existing file — **1 NEW expect() inside existing test block**:

22. Read `skills/deploy/references/telemetry.md`; assert all 7 Phase 10 events appear verbatim: `windows.hook.exec_policy_blocked`, `windows.hook.edr_killed`, `windows.install.network_timeout`, `windows.hook.breadcrumb`, `windows.hook.path_too_long`, `windows.install.av_quarantined`, `windows.install.proxy_407`.

### E2E — manual Windows VM smoke (gated to release)

`docs/RELEASE.md` `## Phase 10 — Windows VM smoke` (US-1005):

23. Maintainer spins up Win11 Hyper-V/VirtualBox/AWS Workspaces.
24. Install Claude Code (≥ 2.1.84 per F5) + axhub plugin via marketplace.
25. Open session. Confirm SessionStart fires, no error popup, `axhub-helpers.exe` exists at `bin/` after first session.
26. With `AXHUB_TELEMETRY=1`: confirm breadcrumb written to `${env:USERPROFILE}\.local\state\axhub-plugin\usage.jsonl` (per F2 — NOT `%LOCALAPPDATA%`) with `$PSVersionTable` data.
27. Smoke breadcrumb JSON pasted into v0.1.7 GitHub Release notes. **No tag without this evidence.**

### Observability counters (7)

| # | Event | Pre-Mortem |
|---|---|---|
| 28 | `windows.hook.exec_policy_blocked` | #1 |
| 29 | `windows.hook.edr_killed` | #2 |
| 30 | `windows.install.network_timeout` | #3 |
| 31 | `windows.hook.breadcrumb` | #4 |
| 32 | `windows.hook.path_too_long` | #5 NEW |
| 33 | `windows.install.av_quarantined` | #6 NEW |
| 34 | `windows.install.proxy_407` | #7 NEW |

### Test math (D1 reconciled)

Baseline: **349 pass / 5 skip / 0 fail / 2257 expect() / 354 tests / 16 files**.

Deltas:
- install-ps1.test.ts (cases 1-7): +7 NEW test(), +1 file.
- session-start-ps1.test.ts (cases 8-16): +9 NEW test(), +1 file.
- manifest.test.ts (cases 17-21): +5 NEW expect() in existing tests, 0 tests.
- telemetry.test.ts (case 22): +1 NEW expect() in existing test, 0 tests.

**Final**: 349+16=**365 pass** / 5 skip / 0 fail / 354+16=**370 tests** / 16+2=**18 files** / ~2257+16+5+1=**~2279 expect()**.

US-1006 release gate (D1): `bun test 2>&1 | tail -5` MUST report **365 pass / 5 skip / 0 fail / 370 tests across 18 files**.

D1 note: v1's 364/363 numbers were wrong (claimed 9 NEW test() but listed 14 numbered cases across 2 NEW files). v2 counts each NEW `test()` (16) vs each NEW `expect()` in existing tests (6).

---

## PRD Stories v2 (US-1000 .. US-1006 — 7 stories, US-1000 NEW spike)

### US-1000 — Wrong-OS spawn behavior spike (F1 NEW, BLOCKING)

**Goal**: Answer Open Question #3 ("does powershell entry on macOS show error popup?") deterministically before US-1003. Anthropic docs SILENT; #4 telemetry is sampling theater for the macOS question.

**Steps**:
1. Set up minimal stub in fresh `~/.claude/plugins/`:
   - `stub-plugin/.claude-plugin/plugin.json` — `name=stub`, `version=0.0.1`.
   - `stub-plugin/hooks/hooks.json`:
     ```json
     {"hooks":{"SessionStart":[{"hooks":[{"type":"command","shell":"powershell","command":"powershell -Command 'exit 0'"}]}]}}
     ```
2. On macOS: launch `claude` in fresh terminal/temp dir; capture stderr + session pane for 30s.
3. Document one of:
   - **silent**: no user-visible message → US-1003 ships as-designed.
   - **visible-error**: error in pane → US-1003 must add at top of every `.ps1`: `if ($PSVersionTable.Platform -ne 'Win32NT' -and $PSVersionTable.PSEdition -ne 'Desktop') { exit 0 }` (PS 5.1 lacks `$IsWindows`).
   - **popup**: blocking dialog → US-1003 BLOCKED. Escalate to Anthropic; explore Option D for v0.2.
4. Repeat probe on Linux (Docker `ubuntu:24.04` no powershell.exe).
5. Write `docs/dev/ps-wrong-os-spawn-2026-04-24.md` with macOS + Linux observations + chosen US-1003 contract.

**Acceptance**: spike doc exists; US-1003 contract step 3 updated; Open question #3 closed in `.omc/plans/open-questions.md` with cite.

**Blocked by**: none. **Estimate**: 1 day.

---

### US-1001 — `bin/install.ps1`

Pure PS mirror of `bin/install.sh:1-91`. Same OS/arch detection, `AXHUB_PLUGIN_RELEASE` override, `AXHUB_SKIP_AUTODOWNLOAD` opt-out, target `axhub-helpers-windows-amd64.exe`, release URL pattern.

**Contract**:
1. `#Requires -Version 5.1`.
2. `$RELEASE_VERSION = 'v0.1.7'`. `$env:AXHUB_PLUGIN_RELEASE` overrides (mirror `bin/install.sh:48`).
3. Assert `[System.Environment]::Is64BitOperatingSystem` (4-part Korean Win32 unsupported per `bin/install.sh:35-38`).
4. `$BinDir = Split-Path -Parent $PSCommandPath`. `$TargetName = "axhub-helpers-windows-amd64.exe"`. `$TargetPath = Join-Path $BinDir $TargetName`.
5. **MAX_PATH proactive probe (#5/F4)**: if `($BinDir.Length + $TargetName.Length) -gt 240` → 4-part Korean MAX_PATH warning + counter `windows.hook.path_too_long`.
6. If NOT `Test-Path $TargetPath` AND `$env:AXHUB_SKIP_AUTODOWNLOAD -ne '1'`:
   - `$Url = "https://github.com/jocoding-ax-partners/axhub/releases/download/$RELEASE_VERSION/$TargetName"`.
   - Korean progress message (110MB / 회사 네트워크 / 최대 10분).
   - **Proxy-aware (#7/F4)**:
     ```powershell
     $iwrArgs = @{ Uri = $Url; OutFile = "$TargetPath.tmp"; UseBasicParsing = $true; TimeoutSec = 600 }
     if ($env:HTTPS_PROXY) { $iwrArgs.Proxy = $env:HTTPS_PROXY; $iwrArgs.ProxyUseDefaultCredentials = $true }
     Invoke-WebRequest @iwrArgs
     ```
   - Catch `[System.Net.WebException]`: `Response.StatusCode -eq 407` → counter `windows.install.proxy_407` + 4-part Korean proxy; else generic timeout → counter `windows.install.network_timeout` + 4-part Korean network. Both exit 1.
   - On success: `Move-Item "$TargetPath.tmp" $TargetPath -Force`.
   - **AV quarantine probe (#6/F4)**: `Start-Sleep -Seconds 2`; `if (-not (Test-Path $TargetPath)) { throw 'AV_QUARANTINED' }`. Catch sentinel → counter `windows.install.av_quarantined` + 4-part Korean AV exclusion message.
7. **Symlink/copy (D4 — explicit, NOT install.sh:80 buggy precedence)**:
   ```powershell
   $LinkPath = Join-Path $BinDir 'axhub-helpers.exe'
   # NOTE: bin/install.sh:80 has known operator precedence bug; PS mirror uses
   # explicit Test-Path/Remove-Item to avoid replicating it. Follow-up: fix
   # install.sh:80 in v0.1.8 or later.
   if (Test-Path -Path $LinkPath -PathType Any) {
     Remove-Item -Path $LinkPath -Force
   }
   Copy-Item $TargetPath $LinkPath -Force
   ```
8. `Write-Host "axhub-helpers -> $TargetName (OS=windows, arch=amd64)"`.
9. Top-level `try {<body>} catch [System.IO.PathTooLongException] {...} catch {...}` each emitting 4-part Korean and `exit 1` (install script may exit non-zero — SessionStart wrapper translates per D3).

**6 user-visible Korean error paths** (4-part each): Win32 unsupported / network timeout / proxy 407 / AV quarantine / MAX_PATH / EDR-AMSI / top-level catch-all.

**Acceptance**: file at `bin/install.ps1`; pure ASCII outside Korean blocks; no `Add-Type` / `[Reflection.Assembly]::Load` / inline C#; D4 explicit pattern + comment present; #5, #6, #7 detection asserted by US-1004 cases 1, 6, 12, 14; `bin/install.sh` byte-identical to v0.1.6 (RELEASE_VERSION line 48 bump deferred to US-1006).

**Blocked by**: US-1000 (informs guard); implementation can begin in parallel.

---

### US-1002 — `hooks/session-start.ps1`

Pure PS mirror of `hooks/session-start.sh:1-47`. Check `bin/axhub-helpers.exe`, run `install.ps1` if missing, run `axhub-helpers.exe token-init` if helper token file missing AND `axhub auth status` shows authenticated, exec `axhub-helpers.exe session-start` with stdin pass-through.

**Contract**:
1. `#Requires -Version 5.1`.
2. **Optional wrong-OS guard (gated on US-1000 outcome)**: if visible-error/popup, prepend `if ($PSVersionTable.Platform -ne 'Win32NT' -and $PSVersionTable.PSEdition -ne 'Desktop') { exit 0 }` as FIRST line. If silent, omit.
3. `$Root = $env:CLAUDE_PLUGIN_ROOT`. Validate non-empty → emit `Write-Output (ConvertTo-Json @{ systemMessage = '[axhub] CLAUDE_PLUGIN_ROOT 환경변수가 비어있어요. <4-part Korean>' } -Compress)`, `exit 0`.
4. **MAX_PATH proactive guard (#5)**: `try {...}` wraps `[System.IO.PathTooLongException]` catch.
5. `$Helper = Join-Path $Root 'bin/axhub-helpers.exe'`. `$InstallPs1 = Join-Path $Root 'bin/install.ps1'`.
6. **Install + propagation (D3 fix)**: if NOT `Test-Path $Helper`:
   - if NOT `Test-Path $InstallPs1`: emit `{"systemMessage":"[axhub] install.ps1 없음 — 플러그인 install 손상. <4-part Korean>"}`, `exit 0`.
   - else:
     ```powershell
     & $InstallPs1
     $installExit = $LASTEXITCODE
     if ($installExit -ne 0) {
       Write-Output (ConvertTo-Json @{ systemMessage = '[axhub] helper 바이너리 설치 실패 (exit ' + $installExit + '). <4-part Korean>' } -Compress)
       exit 0  # Don't block session, but surface failure
     }
     ```
7. **F2 — XDG state dir parity** (NOT `%LOCALAPPDATA%`):
   ```powershell
   $StateDir = if ($env:XDG_STATE_HOME) {
     Join-Path $env:XDG_STATE_HOME 'axhub-plugin'
   } else {
     Join-Path $env:USERPROFILE '.local\state\axhub-plugin'
   }
   ```
8. Token-init guard (mirror `session-start.sh:37-45`): if `$env:AXHUB_SKIP_AUTODOWNLOAD -ne '1'`:
   - `$TokenDir = $StateDir` — but `session-start.sh:38` uses `$XDG_CONFIG_HOME:-$HOME/.config`. Open question #4 (must verify which one helper expects via `gitnexus_context({name: "tokenInit"})` BEFORE implementation).
   - `$TokenFile = Join-Path $TokenDir 'token'`.
   - If NOT `Test-Path $TokenFile` AND `Get-Command axhub.exe -ErrorAction SilentlyContinue`:
     - `$AuthStatus = & axhub.exe auth status --json 2>$null`.
     - If `$AuthStatus -match '"user_email"'`: `& $Helper token-init *>&1 | Out-String | Write-Error`. Swallow all errors.
9. **Telemetry breadcrumb (#4)**: if `$env:AXHUB_TELEMETRY -eq '1'`: `try {...} catch { }` writing JSON line to `Join-Path $StateDir 'usage.jsonl'` containing `event: 'windows.hook.breadcrumb'`, `ps_version: $PSVersionTable.PSVersion.ToString()`, `exec_policy: (Get-ExecutionPolicy)`, `claude_plugin_root: $env:CLAUDE_PLUGIN_ROOT`. Telemetry NEVER blocks hot path per Phase 2 US-105.
10. Final: `& $Helper session-start` (PS forwards stdin to child by default).
11. **F3 top-level catch envelope**:
    ```powershell
    try {
      <body>
    } catch [System.IO.PathTooLongException] {
      Write-Output (ConvertTo-Json @{ systemMessage = '[axhub] 경로 길이 한계 초과 — <4-part Korean MAX_PATH>' } -Compress)
      exit 0
    } catch [System.Management.Automation.PSSecurityException] {
      Write-Output (ConvertTo-Json @{ systemMessage = '[axhub] PowerShell ExecutionPolicy 차단 — <4-part Korean>' } -Compress)
      exit 0
    } catch {
      Write-Output (ConvertTo-Json @{ systemMessage = '[axhub] 알 수 없는 오류 — <4-part Korean catch-all>' } -Compress)
      exit 0
    }
    ```

**6 user-visible Korean error paths**: CLAUDE_PLUGIN_ROOT empty / install.ps1 missing / install non-zero exit / MAX_PATH / ExecutionPolicy / catch-all.

**Acceptance**: file at `hooks/session-start.ps1`; every catch emits `{"systemMessage":...}` AND ends `exit 0` (US-1004 cases 10 + 16); `hooks/session-start.sh` byte-identical to v0.1.6; breadcrumb writes to F2-correct path (case 9); D3 `$installExit` + non-zero check present (case 11); ASCII clean.

**Blocked by**: US-1001 (filename + contract); informed by US-1000.

---

### US-1003 — `hooks/hooks.json` platform-conditional SessionStart

Sibling SessionStart entry alongside existing bash entry. Existing preserved byte-identical.

**Final shape** (subject to US-1000 outcome):

```json
{
  "description": "axhub plugin hooks: SessionStart diagnostics + PreToolUse HMAC consent gate (Go binary, deterministic) + PostToolUse exit-code Korean classifier",
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh",
            "timeout": 30
          }
        ]
      },
      {
        "hooks": [
          {
            "type": "command",
            "shell": "powershell",
            "command": "& \"$env:CLAUDE_PLUGIN_ROOT/hooks/session-start.ps1\"",
            "timeout": 30
          }
        ]
      }
    ],
    "PreToolUse": [...unchanged...],
    "PostToolUse": [...unchanged...]
  }
}
```

Rules: bash entry at [0] byte-identical to v0.1.6 `hooks/hooks.json:5-13`. PowerShell entry at [1]; `&` call operator required for PS to exec quoted path with env var expansion. PreToolUse / PostToolUse arrays untouched.

**F5 floor surfacing** (manifest schema lacks `claude_code` block):
- `CHANGELOG.md [0.1.7]` Compat row: `**Compat**: Requires Claude Code >= 2.1.84 (PowerShell tool support). Older clients silently ignore the "shell" field — see docs/vibe-coder-quickstart.ko.md.`
- `docs/vibe-coder-quickstart.ko.md` adds: `필수: Claude Code 2.1.84 이상 (PowerShell 지원). 2.1.84 미만에서는 SessionStart hook 이 작동하지 않습니다.`

**Acceptance**: `hooks/hooks.json` validates (existing `tests/manifest.test.ts` machinery); US-1004 cases 17-21 pass; `bun run smoke:full` passes; v0.1.6 macOS/Linux on Claude Code ≥ 2.1.84 still work; F5 floor strings present (case 21).

**Blocked by**: US-1000, US-1001, US-1002.

---

### US-1004 — Tests for the .ps1 layer

Add `tests/install-ps1.test.ts` (7 NEW test()) + `tests/session-start-ps1.test.ts` (9 NEW test()). Extend `tests/manifest.test.ts` (+5 expect()). Extend `tests/telemetry.test.ts` (+1 expect()).

**Contract**: pure file-text reads — no PowerShell runtime needed (verified `pwsh`/`powershell` not on macOS dev host). Each test ~5 lines. Cases 10 + 16 use balanced-brace traversal.

**Acceptance**: 2 new files, 16 NEW test() blocks; 2 extended (+5 manifest expect, +1 telemetry expect); final `bun test 2>&1 | tail -5` reports **365 pass / 5 skip / 0 fail / 370 tests across 18 files / ~2279 expect()**; `bunx tsc --noEmit` clean; all assertions literal-string or regex against .ps1 text (works on macOS CI).

**Blocked by**: US-1001, US-1002, US-1003.

---

### US-1005 — Documentation updates

5 additive doc edits:

1. `docs/RELEASE.md` — append `## Phase 10 — Windows VM smoke` per E2E rows 23-27.
2. `docs/troubleshooting.ko.md` — 4 NEW sections: "Windows ExecutionPolicy 막혔을 때" (AXHUB_TOKEN fallback); "MAX_PATH (260자) 초과 — Korean profile 사용자" (Group Policy long-paths); "Defender 가 axhub-helpers.exe 격리할 때" (Add-MpPreference exclusion); "회사 NTLM proxy 407 — Invoke-WebRequest 인증" (HTTPS_PROXY env var).
3. `docs/vibe-coder-quickstart.ko.md` — "Windows 노트북 사용자" subsection: `v0.1.7부터 Windows 11 (PowerShell 5.1+, Claude Code 2.1.84 이상) 자동 지원. Git Bash / WSL 불필요. 단 회사 EDR 차단 시 AXHUB_TOKEN 환경변수 우회 — troubleshooting.ko.md 참조.` (F5 floor verbatim — US-1004 case 21.)
4. `skills/deploy/references/telemetry.md` — add 7 new event rows. US-1004 case 22 asserts strings.
5. `docs/dev/ps-wrong-os-spawn-2026-04-24.md` — created in US-1000; referenced from US-1003 contract step 3.

**Acceptance**: all 5 pure additions; `bash tests/docs-link-audit.sh` `Broken: 0`; `bun run smoke:full` passes.

**Blocked by**: US-1000, US-1001, US-1002, US-1003, US-1004.

---

### US-1006 — v0.1.7 release ship

**Contract**:
1. **Version bumps to `0.1.7`** at 7 sync points (D2 — install.ps1 is #7): `package.json`; `.claude-plugin/plugin.json`; `.claude-plugin/marketplace.json`; `bin/install.sh:48` (mechanical bump — additive-only exception); `bin/install.ps1` (`$RELEASE_VERSION = 'v0.1.7'`); `src/axhub-helpers/telemetry.ts:17-18` (PLUGIN_VERSION + HELPER_VERSION); `src/axhub-helpers/index.ts:104` (PLUGIN_VERSION).
2. **D2 mandatory pre-tag cross-check** (race-protection between US-1004 test-time parity and US-1006 bump):
   ```bash
   # Step 1: parity test ran on this exact commit
   bun test 2>&1 | grep -E "RELEASE_VERSION parity"   # MUST show v0.1.7=v0.1.7
   # Step 2: both files literally contain v0.1.7
   grep -E "RELEASE_VERSION='?v0\.1\.7'?" bin/install.sh
   grep -E "\\\$RELEASE_VERSION\s*=\s*'v0\.1\.7'" bin/install.ps1
   # Step 3: filesystem-level parity (D2 race fix)
   diff <(grep -oE "v[0-9]+\.[0-9]+\.[0-9]+" bin/install.sh | head -1) \
        <(grep -oE "v[0-9]+\.[0-9]+\.[0-9]+" bin/install.ps1 | head -1)
   # MUST exit 0
   ```
3. **Build**: `bun run build:all` produces all 5 binaries; `file bin/axhub-helpers-windows-amd64.exe` reports PE32+.
4. **Test gate** (D1 exact pin): `bun test 2>&1 | tail -5` reports **365 pass / 5 skip / 0 fail / 370 tests across 18 files**; `bunx tsc --noEmit` clean; `bun run smoke:full` clean.
5. **CHANGELOG `[0.1.7]`**: Added (bin/install.ps1, hooks/session-start.ps1, hooks.json sibling, 7 telemetry counters, VM smoke section, 4 troubleshooting sections); Changed (SessionStart array now 2 entries; macOS/Linux unchanged); **Compat**: `Requires Claude Code >= 2.1.84 (PowerShell tool support)` — F5 verbatim; **Honest tradeoff**: .ps1 not Authenticode-signed v0.1.7 (deferred v0.1.8); test baseline as step 4.
6. **Manual Windows VM smoke** (US-1005 rows 23-27): breadcrumb JSON pasted into GitHub Release notes. **No tag without this.**
7. **Tag + push**: `git tag v0.1.7 && git push origin main --tags`. Release workflow auto-fires per `docs/RELEASE.md:52`; cosign keyless signs all binaries (Phase 9 pipeline).

**Acceptance**: 7 sync points = `0.1.7`; D2 cross-check exits 0; D1 test pin matches; CHANGELOG references this v2 plan path; Win VM breadcrumb in Release notes; `git diff --stat v0.1.6 HEAD -- bin/install.sh hooks/session-start.sh` shows ONLY line-48 bump on install.sh, ZERO on session-start.sh; `git diff --stat v0.1.6 HEAD -- hooks/hooks.json` shows ONLY the 1-block addition.

**Blocked by**: US-1000, US-1001, US-1002, US-1003, US-1004, US-1005.

---

## Story dependency graph v2

```
US-1000 (spike, 1d) -+
US-1001 (install.ps1) -+
US-1002 (session-start.ps1) -+-> US-1003 (hooks.json) -> US-1004 (tests) -> US-1005 (docs) -> US-1006 (release v0.1.7)
                                  ^ US-1000 informs                                         ^ D2 cross-check gates tag
```

US-1001 + US-1002 can run in parallel. US-1000 gates only US-1003 contract (whether to add wrong-OS guard).

---

## ADR v2 — incorporates 9 round-1 fixes

**Decision**: Add Windows PowerShell mirrors (`bin/install.ps1`, `hooks/session-start.ps1`) and register sibling SessionStart entry in `hooks/hooks.json` using `"shell": "powershell"`, so vibe coders on vanilla Windows 10/11 (no Git Bash, no WSL) can use the plugin end-to-end. `bin/install.sh` + `hooks/session-start.sh` byte-identical (install.sh:48 gets one mechanical bump in US-1006). Compiled `axhub-helpers-windows-amd64.exe` unchanged from Phase 9. Pin Claude Code floor at 2.1.84 via CHANGELOG + quickstart strings (manifest schema lacks `claude_code` block per F5). Gate US-1003 contract on US-1000 spike for wrong-OS spawn.

**Drivers**: see Decision Drivers section.

**Alternatives considered**: A (no `condition`/`platform` field per WebFetch), C (Node dispatcher — Windows lacks Node), D (binary self-install — chicken-and-egg; v0.2 escalation path if US-1000 returns "popup").

**Why chosen**: Option B uses documented `"shell"` field, zero changes to `session-start.sh`, isolates Windows failure modes (ExecutionPolicy, EDR, MAX_PATH, AV, NTLM proxy) into powershell entry. Test surface 100% mockable on macOS CI. US-1000 removes the only deterministic-blocker.

**Consequences**:
- + Vanilla Windows 10/11 vibe coders get auto-bootstrap parity, no Git Bash/WSL prerequisite.
- + Zero edits to compiled `axhub-helpers-windows-amd64.exe`. Phase 9 keychain bridge integrates as-is.
- + macOS/Linux on Claude Code ≥ 2.1.84 see zero change (pending US-1000) — bash[0] byte-identical, powershell[1] no-ops on Unix.
- + 7 telemetry counters + 7 pre-mortems (4 v1 + 3 v2 NEW: MAX_PATH, AV quarantine, NTLM proxy).
- + D2 race-protection: parity asserted at test-time (US-1004 case 2) AND release-time (US-1006 step 2 cross-check).
- − v0.1.7 .ps1 NOT Authenticode-signed. EDR may quarantine (#2). AXHUB_TOKEN fallback documented.
- − Two scripts to lockstep — mitigated by parity + cross-check.
- − hooks.json schema dependency on `"shell": "powershell"`. Spec change forces test failure (US-1004 case 19).
- − No real Windows CI; manual VM smoke only. v0.1.8: GitHub Actions windows-latest runner.
- − F5 floor (Claude Code ≥ 2.1.84) lives in CHANGELOG + quickstart strings, NOT manifest field — older clients silently ignore `"shell"`. Risk: vibe coder on 2.1.83 hits broken hook with no actionable error. Mitigation: README + quickstart prominently document; v0.1.8 adds manifest field if/when Anthropic schema supports.
- − install.sh:80 retains operator-precedence bug (D4 — explicit non-replication in PS, v0.1.8 follow-up).

**Follow-ups (v0.1.8)**:
- Authenticode sign .ps1; legitimize EDR allowlist requests.
- GitHub Actions windows-latest runner — automate VM smoke.
- File issue: fix `bin/install.sh:80` operator precedence bug.
- Telemetry-driven: `exec_policy_blocked` >5% → ship signed .ps1; `edr_killed` >5% → prioritize Authenticode; `av_quarantined` >5% → virustotal submission; `path_too_long` >5% → move install location out of `%APPDATA%\Claude\plugins\`.
- Manifest: add `claude_code.minVersion` if/when Anthropic schema supports.
- **Open for Critic**: `cmd.exe` shim as third-fallback for PS GPO-disabled users? NOT in scope — Anthropic spec documents only `bash` and `powershell`.

---

## Open questions (also persisted to `.omc/plans/open-questions.md`)

1. PS 5.1 vs `pwsh` 7+ on Windows? Docs say "spawn PowerShell directly" but don't specify version. #4 telemetry captures `$PSVersionTable.PSVersion`. Needed before v0.1.8 if PS 7+ syntax allowed.
2. Should `bin/install.ps1` verify sha256 against `manifest.json` (Phase 9 cosign)? `bin/install.sh:63` does NOT — trusts `curl -fsSL` + GitHub TLS. v0.1.8: add to BOTH simultaneously.
3. (CLOSED via US-1000 — answer pending.) `"shell": "powershell"` no-op on macOS/Linux, or visible error? US-1000 answers; reopens only if "popup".
4. (NEW) `session-start.sh:38` uses `$XDG_CONFIG_HOME` for token file; `telemetry.ts:40` uses `$XDG_STATE_HOME` for telemetry. Which one does `axhub-helpers token-init` actually write to? US-1002 step 8 must match helper; verify via `gitnexus_context({name: "tokenInit"})` BEFORE US-1002 implementation.
5. (NEW) Does Anthropic plugin manifest schema have a `claude_code` block? F5 says NO. Re-check before v0.1.8 — if added, US-1003 v3 ships the field.

---

## ASCII verification proof

```
perl -ne 'print "Line $.: $_" if /[\x{200B}-\x{200F}\x{FEFF}]/' /Users/wongil/Desktop/work/jocoding/axhub/.omc/plans/phase-10-windows-ps1-hooks-v2.md
```

Expected: empty. Zero matches confirms ASCII-clean (no U+200B per Phase 9 lesson).

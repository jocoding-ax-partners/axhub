# Phase 10 — Windows PowerShell Hook Mirrors (v0.1.7) — Plan v1

**Status**: Round 1/N ralplan consensus. Auto-deliberate (cross-platform hook + public hooks.json contract).
**Mode**: DELIBERATE — pre-mortem (4) + expanded test plan (unit/integration/e2e/observability).
**Baseline** (`CHANGELOG.md:21,51`): 349 pass / 5 skip / 0 fail / 2257 expect() / 354 tests / 16 files.
**KEEP** `bin/install.sh` and `hooks/session-start.sh` byte-identical. Additive only. Compiled `axhub-helpers` subcommands unchanged.
**ASCII rule**: `perl -ne 'print "Line $.: $_" if /[\x{200B}-\x{200F}\x{FEFF}]/' .omc/plans/phase-10-windows-ps1-hooks.md` MUST return zero matches before lock.

---

## Investigation findings (verbatim from Anthropic docs)

WebFetch of `https://code.claude.com/docs/en/hooks` (resolved from docs.claude.com 301):

> **shell** | no | Shell to use for this hook. Accepts `"bash"` (default) or `"powershell"`. Setting `"powershell"` runs the command via PowerShell on Windows. Does not require `CLAUDE_CODE_USE_POWERSHELL_TOOL` since hooks spawn PowerShell directly

> All matching hooks run in parallel, and identical handlers are deduplicated automatically.

**No `platform` / `os` / `condition` / `when` field exists.** The supported branching primitive is the per-hook `"shell"` field. Multiple entries under the same event/matcher all execute in parallel; PowerShell is spawned directly (not via cmd.exe). This eliminates Option A and validates Option B.

---

## Principles (5)

1. **Additive only** — `bin/install.sh` and `hooks/session-start.sh` stay byte-identical. New work in new `.ps1` files plus one sibling-entry edit to `hooks/hooks.json`.
2. **Use the official spec primitive** — Anthropic's `"shell": "powershell"` is the documented per-hook gate. Don't invent platform-detection wrappers when the spec already provides one.
3. **Stock Windows only** — no `Install-Module`, no third-party PS modules. PS 5.1 (Win10/11 default) is the sole runtime dep. Phase 9 proved this works (`keychain-windows.ts:43-95`).
4. **4-part Korean errors** — every user-visible failure follows `error-empathy-catalog.md:9-25` template (감정 / 원인 / 해결 / 다음액션).
5. **Mockable on macOS CI** — `.ps1` tests run on existing macOS runners via TS file-text assertions. No Windows host required for unit tests. Real Windows execution is gated to manual VM smoke (Pre-Mortem #4).

---

## Decision Drivers (top 3)

1. **Vibe coder UX parity for vanilla Windows**. v0.1.6 ships `axhub-helpers-windows-amd64.exe` + Credential Manager bridge but `SessionStart` still bootstraps via `bash hooks/session-start.sh` (`hooks/hooks.json:9`). On Windows 11 without Git Bash / WSL this fails immediately — every other hook is broken because the binary auto-download never runs.
2. **Public hooks.json contract stability**. Marketplace consumers cache the file. Adding a sibling entry under the existing `SessionStart` array is safe; replacing it would brick installed v0.1.6 plugins.
3. **Maintenance cost of duplicated logic**. Two install scripts and two session-start scripts means future changes (e.g., bumping `RELEASE_VERSION` `bin/install.sh:48`) need to land in both. Mitigation: `tests/install-ps1.test.ts` asserts the `.ps1` and `.sh` files agree on `RELEASE_VERSION`, target binary names, and the `AXHUB_SKIP_AUTODOWNLOAD` opt-out (US-1004 cases 2-3, 7).

---

## Viable Options

### A. hooks.json native `condition` / `platform` field
**INVALIDATED.** WebFetch confirms no such field exists. Only `"shell"` is supported.

### B. Two SessionStart entries — bash + powershell — fails-soft on wrong platform [PICKED]

**How**: `hooks/hooks.json` `SessionStart` array gets a second sibling entry at `[1]` with `"shell": "powershell"` and `"command": "& \"$env:CLAUDE_PLUGIN_ROOT/hooks/session-start.ps1\""`. Both always exist. On macOS / Linux the bash entry runs and powershell entry no-ops (no powershell.exe on PATH → spawn fails silently per "all matching hooks run in parallel" semantics). On Windows the bash entry no-ops (no Git Bash) and powershell entry runs.

**Pros**: uses documented `"shell"` field as designed; zero changes to existing `session-start.sh`; each script has a single OS contract; fails-soft is real per Anthropic spec.

**Cons**: spawns powershell.exe on Unix and bash on Windows in failed-spawn paths (~50ms noise, below human perception); two scripts to keep in lockstep on `RELEASE_VERSION` / target names — mitigated by US-1004 parity assertions.

### C. Wrapper command that detects platform internally (Node or dispatcher)
**INVALIDATED.** Stock Windows lacks Node — adds a runtime dep we don't currently require. A custom .sh / .ps1 / .cmd dispatcher is a third file when `"shell"` already does the routing for free. Contradicts Principle #2.

### D. `axhub-helpers session-start-hook` runs install logic from inside the binary
**INVALIDATED.** Chicken-and-egg — the binary is what install.sh / install.ps1 downloads. If binary missing, calling it to install itself fails. Possible v0.2 path after build-time stripping reduces 109.6M binary, but not v0.1.7 scope.

---

## Pre-Mortem (4 scenarios, DELIBERATE)

### Scenario 1 — `ExecutionPolicy=Restricted` blocks .ps1 on org-managed Windows
Corporate `MachinePolicy: AllSigned` / `Restricted` refuses unsigned `.ps1`. PS exits 1, `session-start.ps1` never runs. Phase 9 hit this in `keychain-windows.ts:105-109`.
**Mitigation**: US-1002 wraps body in `try { ... } catch { Write-Output '<4-part Korean>'; exit 0 }`. Exit 0 (NOT 1) so Claude Code doesn't retry-loop. Counter `windows.hook.exec_policy_blocked`. `docs/troubleshooting.ko.md` documents AXHUB_TOKEN fallback.

### Scenario 2 — AMSI / EDR blocks .ps1 (Phase 9 carry-over)
EDR (CrowdStrike, SentinelOne, Defender ATP, V3, AhnLab) classifies `Add-Type -TypeDefinition` as Mimikatz / SharpDPAPI (`keychain-windows.ts:117-122`). Download-then-execute is also a red-flag pattern.
**Mitigation**: `install.ps1` contains NO `Add-Type` (pure `Invoke-WebRequest` + `Move-Item`). PInvoke stays isolated to `keychain-windows.ts`. `session-start.ps1` only spawns `axhub-helpers.exe`. EDR-killed PS detected via exit code ∈ {-1, 0xC0000409} or signal-kill → 4-part Korean message #2. Counter `windows.hook.edr_killed`. If > 5%, GO signal for v0.1.8 Authenticode signing.

### Scenario 3 — Network timeout during `install.ps1` download (slow corp network)
109.6M binary over corp proxy with TLS interception. `Invoke-WebRequest` default timeout 100s; corp first-byte often 3-5min. macOS `bin/install.sh:63` accidentally tolerates this (`curl -fsSL` waits indefinitely).
**Mitigation**: `Invoke-WebRequest -TimeoutSec 600` (10min). Pre-download Korean progress message: `"axhub-helpers 바이너리 다운로드 중 (110MB, 회사 네트워크에서 최대 10분 소요)..."`. On `WebException`: 4-part Korean error pointing to `gh release download` manual fallback (mirror `bin/install.sh:64-67`). Counter `windows.install.network_timeout`.

### Scenario 4 — Claude Code Windows exec model unverified
Zero Windows execution in this plan. Possible variations: `cmd.exe /c "powershell ..."` wrapper changing `$env:CLAUDE_PLUGIN_ROOT` expansion, or `pwsh` (Core 7+) vs `powershell.exe` (5.1) changing PInvoke compatibility. 100% dev on macOS.
**Mitigation**: hooks.json uses `&` call operator with quoted path (resilient to PS version arg-quoting). `session-start.ps1` emits breadcrumb to `usage.jsonl` (when `AXHUB_TELEMETRY=1`) capturing `$PSVersionTable.PSVersion`, `Get-ExecutionPolicy`, `$env:CLAUDE_PLUGIN_ROOT`, `$PWD` — first Windows user with telemetry on gives ground truth. US-1006 blocks on manual Win11 VM smoke. Both .ps1 use PS 5.1 LCD syntax (no `??`, no `-Parallel`).

---

## Expanded Test Plan (DELIBERATE)

### Unit (US-1004) — TS-side mockable, no Windows host

`tests/install-ps1.test.ts` (new, +1 file → 17) — 5 cases:

1. **Script structure** — read `bin/install.ps1` as text; assert literals: `axhub-helpers-windows-amd64.exe`, `Invoke-WebRequest`, `-TimeoutSec 600`, `AXHUB_SKIP_AUTODOWNLOAD`, `try {`, `catch {`.
2. **Parity — RELEASE_VERSION** — regex-extract `$RELEASE_VERSION = '...'` from .ps1 and `RELEASE_VERSION="${AXHUB_PLUGIN_RELEASE:-...}"` from .sh; assert equal. Catches version-bump drift.
3. **Parity — opt-out env var** — both files reference `AXHUB_SKIP_AUTODOWNLOAD` literally.
4. **Korean 4-part error structure** — `install.ps1` contains exactly 4 catch blocks each emitting 4 separate `Write-Output` lines (감정/원인/해결/다음액션 per `error-empathy-catalog.md:9-25`).
5. **No EDR-flagged patterns** — assert `install.ps1` does NOT contain `Add-Type` or `[Reflection.Assembly]::Load`. Defends Pre-Mortem #2 at code level.

`tests/session-start-ps1.test.ts` (new, +1 file → 18) — 4 cases:

6. **Script structure** — assert: `axhub-helpers.exe`, `session-start`, `token-init`, install.ps1 reference, `try {`, `catch {`. Mirror of `hooks/session-start.sh:1-47` semantics.
7. **Parity — token-init guard** — both files contain `AXHUB_SKIP_AUTODOWNLOAD` and the conditional skipping `token-init` when set to `1`.
8. **Telemetry breadcrumb emit** — contains the breadcrumb-write line gated on `$env:AXHUB_TELEMETRY -eq '1'`. Catches accidental removal of the only Windows-side ground-truth signal.
9. **Exit 0 on every catch path** — every `catch { }` block ends with `exit 0` (NOT `exit 1` or `throw`). Per Anthropic spec, non-zero exit blocks SessionStart, bricking every Windows session on any hook bug.

### Integration (US-1003) — hooks.json platform branching

`tests/manifest.test.ts` extension (existing file, +5 assertions):

10. SessionStart array has 2 entries: [0] = bash (preserved byte-identical), [1] = powershell (new).
11. Bash entry preserved — `hooks.SessionStart[0].hooks[0].command` matches literal `bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh`.
12. PowerShell entry shape — `hooks.SessionStart[1].hooks[0]` has `type: "command"`, `shell: "powershell"`, `timeout: 30`, `command: "& \"$env:CLAUDE_PLUGIN_ROOT/hooks/session-start.ps1\""`.
13. Both entries' timeout = 30 (fail-fast on skew).
14. PreToolUse / PostToolUse byte-identical to v0.1.6.

### E2E — manual Windows VM smoke (gated to release)

`docs/RELEASE.md` new section `## Phase 10 — Windows VM smoke` (US-1005):

15. Maintainer spins up Win11 Hyper-V (or VirtualBox / AWS Workspaces — anything with PowerShell 5.1).
16. Install Claude Code + axhub plugin via marketplace.
17. Open Claude Code session. Confirm SessionStart fires, no error popup, `axhub-helpers.exe` exists at `bin/` after first session.
18. With `AXHUB_TELEMETRY=1`: confirm `%LOCALAPPDATA%\axhub-plugin\usage.jsonl` contains the breadcrumb event with `$PSVersionTable` data.
19. Smoke breadcrumb JSON pasted into v0.1.7 GitHub Release notes as evidence. **No tag without this evidence.**

### Observability (US-1004 case 5)

Add 4 telemetry counters via `event` field (no schema change — `event` is free-form per `telemetry.ts:46-54`):

20. `windows.hook.exec_policy_blocked` (Pre-Mortem #1)
21. `windows.hook.edr_killed` (Pre-Mortem #2)
22. `windows.install.network_timeout` (Pre-Mortem #3)
23. `windows.hook.breadcrumb` (Pre-Mortem #4 ground truth)

`tests/telemetry.test.ts` extension (existing, +1 assertion): assert all 4 names appear verbatim in `skills/deploy/references/telemetry.md`.

### Test math

Baseline 349 / 354 / 16 files → Phase 10 final **364 pass / 5 skip / 0 fail / ~2275 expect() / 363 tests / 18 files** (= 349 + 9 install-ps1 + 5 manifest + 1 telemetry; +2 files).

---

## PRD Stories (US-1001 .. US-1006)

### US-1001 — `bin/install.ps1`

Pure PowerShell mirror of `bin/install.sh:1-91`. Same OS / arch detection, same `AXHUB_PLUGIN_RELEASE` override, same `AXHUB_SKIP_AUTODOWNLOAD` opt-out, same target name (`axhub-helpers-windows-amd64.exe`), same release URL pattern.

**Contract**:
1. `#Requires -Version 5.1`.
2. Hard-code `$RELEASE_VERSION = 'v0.1.7'`. `$env:AXHUB_PLUGIN_RELEASE` overrides (mirror `bin/install.sh:48`).
3. Assert `[System.Environment]::Is64BitOperatingSystem` (4-part Korean error otherwise — Win32 unsupported per `bin/install.sh:35-38`).
4. `$BinDir = Split-Path -Parent $PSCommandPath`. `$TargetName = "axhub-helpers-windows-amd64.exe"`. `$TargetPath = Join-Path $BinDir $TargetName`.
5. If NOT `Test-Path $TargetPath` AND `$env:AXHUB_SKIP_AUTODOWNLOAD -ne '1'`:
   - `$Url = "https://github.com/jocoding-ax-partners/axhub/releases/download/$RELEASE_VERSION/$TargetName"`.
   - `Write-Host "axhub-helpers 바이너리 다운로드 중 (110MB, 회사 네트워크에서 최대 10분 소요)..."`.
   - `Invoke-WebRequest -Uri $Url -OutFile "$TargetPath.tmp" -UseBasicParsing -TimeoutSec 600`.
   - On `WebException`: 4-part Korean network-timeout error, exit 1.
   - On success: `Move-Item "$TargetPath.tmp" $TargetPath -Force`.
6. `Copy-Item $TargetPath (Join-Path $BinDir 'axhub-helpers.exe') -Force` (mirror `bin/install.sh:82-86` — Windows uses copy because symlinks need admin).
7. `Write-Host "axhub-helpers -> $TargetName (OS=windows, arch=amd64)"`.
8. Top-level `try { <body> } catch { Write-Output '<4-part Korean catch-all>'; exit 1 }`.

**4 user-visible Korean error paths** (each 4-part per template): Win32 unsupported / network timeout / ExecutionPolicy block (propagated from session-start.ps1 catch) / EDR-AMSI quarantine.

**Acceptance**:
- File exists at `bin/install.ps1`.
- Pure ASCII outside Korean string blocks (`perl -ne 'print if /[\x{200B}-\x{200F}\x{FEFF}]/' bin/install.ps1` returns zero).
- No `Add-Type`, no `[Reflection.Assembly]::Load`, no inline C#.
- Exit 0 on success, 1 on user-actionable error.
- `bin/install.sh` byte-identical to v0.1.6 (additive only — exception: line 48 `RELEASE_VERSION` bump in US-1006).

**Blocked by**: none.

---

### US-1002 — `hooks/session-start.ps1`

Pure PowerShell mirror of `hooks/session-start.sh:1-47`. Check `bin/axhub-helpers.exe`, run `install.ps1` if missing, run `axhub-helpers.exe token-init` if helper token file missing AND `axhub auth status` shows authenticated, exec `axhub-helpers.exe session-start` with stdin pass-through.

**Contract**:
1. `#Requires -Version 5.1`.
2. `$Root = $env:CLAUDE_PLUGIN_ROOT`. Validate non-empty → emit JSON `{"systemMessage":"[axhub] CLAUDE_PLUGIN_ROOT 환경변수가 비어있어요."}`, exit 0 otherwise.
3. `$Helper = Join-Path $Root 'bin/axhub-helpers.exe'`. `$InstallPs1 = Join-Path $Root 'bin/install.ps1'`.
4. If NOT `Test-Path $Helper`: invoke `& $InstallPs1 *>&1 | Out-String | Write-Error`. On install failure emit JSON `{"systemMessage":"[axhub] helper 바이너리 설치 실패. 진단: /axhub:doctor"}`, exit 0. If install.ps1 missing emit JSON `{"systemMessage":"[axhub] install.ps1 없음 — 플러그인 install 손상. 재설치: /plugin install axhub@axhub"}`, exit 0.
5. Token-init guard (mirror `session-start.sh:37-45`):
   - If `$env:AXHUB_SKIP_AUTODOWNLOAD -ne '1'`:
     - `$LocalAppData = if ($env:LOCALAPPDATA) { $env:LOCALAPPDATA } else { Join-Path $env:USERPROFILE 'AppData\Local' }` — PS 5.1 has no `??` operator (Pre-Mortem #4).
     - `$TokenDir = Join-Path $LocalAppData 'axhub-plugin'`. `$TokenFile = Join-Path $TokenDir 'token'`.
     - If NOT `Test-Path $TokenFile` AND `Get-Command axhub.exe -ErrorAction SilentlyContinue`:
       - `$AuthStatus = & axhub.exe auth status --json 2>$null`.
       - If `$AuthStatus -match '"user_email"'`: invoke `& $Helper token-init *>&1 | Out-String | Write-Error`.
     - Swallow all errors (mirror `session-start.sh:42` `|| true`).
6. Telemetry breadcrumb (Pre-Mortem #4):
   - If `$env:AXHUB_TELEMETRY -eq '1'`: write JSON line to `Join-Path $LocalAppData 'axhub-plugin\usage.jsonl'` containing `event: 'windows.hook.breadcrumb'`, `ps_version: $PSVersionTable.PSVersion.ToString()`, `exec_policy: (Get-ExecutionPolicy)`, `claude_plugin_root: $env:CLAUDE_PLUGIN_ROOT`. Wrap in `try { } catch { }` (telemetry never blocks hot path per Phase 2 US-105).
7. Final: `& $Helper session-start` (PS forwards stdin to child by default).
8. Top-level `try { <body> } catch { Write-Output '<4-part Korean catch-all JSON>'; exit 0 }`. **Critical**: every catch path ends `exit 0` (US-1004 case 9 invariant).

**4 user-visible Korean error paths** (Pre-Mortem #1, #2, spawn-of-child fail, top-level catch-all): variants of `keychain-windows.ts:105-128` adapted to hook context, all referencing AXHUB_TOKEN env var fallback.

**Acceptance**:
- File at `hooks/session-start.ps1`.
- All catch blocks end `exit 0`.
- `hooks/session-start.sh` byte-identical to v0.1.6.
- Telemetry breadcrumb only writes when `AXHUB_TELEMETRY=1` (privacy default-OFF preserved per `telemetry.ts:38`).
- ASCII verification clean.
- Breadcrumb event name = `windows.hook.breadcrumb` (matches US-1004 case 5 / `telemetry.md`).

**Blocked by**: US-1001.

---

### US-1003 — `hooks/hooks.json` platform-conditional SessionStart

Add second SessionStart entry alongside existing bash entry. Existing entry preserved byte-identical.

**Final shape**:

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

Rules: bash entry at [0] byte-identical to v0.1.6 `hooks/hooks.json:5-13`. PowerShell entry at [1]; the `&` call operator is required for PS to exec a quoted path with env var expansion. PreToolUse / PostToolUse arrays untouched.

**Acceptance**:
- `hooks/hooks.json` validates against Anthropic hook schema (existing `tests/manifest.test.ts` machinery).
- US-1004 cases 10-14 pass.
- `bun run smoke:full` passes.
- v0.1.6 macOS / Linux installations still work (hook order preserved, bash entry at [0] unchanged).

**Blocked by**: US-1001, US-1002.

---

### US-1004 — Tests for the .ps1 layer

Add `tests/install-ps1.test.ts` (5 cases) + `tests/session-start-ps1.test.ts` (4 cases) per Expanded Test Plan. Extend `tests/manifest.test.ts` (+5 hooks.json branching assertions). Extend `tests/telemetry.test.ts` (+1 doc-parity assertion).

**Contract**: pure file-text reads — no PowerShell runtime needed (verified `pwsh` and `powershell` not on macOS dev host). Each test ~5 lines: read file → regex.test → expect. Case 9 (every catch ends `exit 0`) parsed via balanced-brace traversal, not naive regex.

**Acceptance**:
- 2 new test files, 9 new tests; 2 extended files (+5 manifest, +1 telemetry).
- Final `bun test 2>&1 | tail -8` reports **364 pass / 5 skip / 0 fail / ~2275 expect() / 363 tests / 18 files**.
- `bunx tsc --noEmit` clean.
- All assertions are literal-string or regex against .ps1 file text (no PS spawn — works on macOS CI).

**Blocked by**: US-1001, US-1002, US-1003.

---

### US-1005 — Documentation updates

4 additive doc edits:

1. `docs/RELEASE.md` — append `## Phase 10 — Windows VM smoke` section per E2E rows 15-19.
2. `docs/troubleshooting.ko.md` — add "Windows ExecutionPolicy 막혔을 때" with US-1002 4-part Korean message + "AXHUB_TOKEN 환경변수 + bun run build:all 수동 호출이 정식 경로" fallback.
3. `docs/vibe-coder-quickstart.ko.md` — add "Windows 노트북 사용자" subsection: "v0.1.7부터 Windows 11 (PowerShell 5.1+) 자동 지원. Git Bash / WSL 불필요. 단 회사 EDR 차단 시 AXHUB_TOKEN 환경변수 우회 — troubleshooting.ko.md 참조."
4. `skills/deploy/references/telemetry.md` — add 4 new event rows (`windows.hook.exec_policy_blocked`, `windows.hook.edr_killed`, `windows.install.network_timeout`, `windows.hook.breadcrumb`). US-1004 case 5/extension asserts these strings present.

**Acceptance**:
- All 4 are pure additions (verify `git diff --stat` shows insertions only).
- `bash tests/docs-link-audit.sh` reports `Broken: 0`.
- `bun run smoke:full` passes.

**Blocked by**: US-1001, US-1002, US-1003, US-1004.

---

### US-1006 — v0.1.7 release ship

Bump version across 6 sync points, regenerate binaries (Windows already ships per Phase 9), update CHANGELOG, run manual Windows VM smoke, tag + push.

**Contract**:
1. **Version bumps to `0.1.7`** at 6 sync points: `package.json`, `.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json` (per `tests/manifest.test.ts` cross-consistency), `bin/install.ps1` (`$RELEASE_VERSION`), `bin/install.sh:48` (mechanical bump — additive-only exception), `src/axhub-helpers/telemetry.ts:17-18` (PLUGIN_VERSION + HELPER_VERSION), `src/axhub-helpers/index.ts:104` (PLUGIN_VERSION).
2. **Build**: `bun run build:all` produces all 5 binaries; `file bin/axhub-helpers-windows-amd64.exe` reports PE32+.
3. **Test gate**: 364 pass / 5 skip / 0 fail / ~2275 expect() / 363 tests / 18 files; `bunx tsc --noEmit` clean; `bun run smoke:full` clean.
4. **CHANGELOG `[0.1.7]`**: Added (.ps1 files + hooks.json entry + 4 telemetry counters + Windows VM smoke section); Changed (SessionStart array now 2 entries, macOS / Linux unchanged); Honest tradeoff (.ps1 not Authenticode-signed v0.1.7, deferred v0.1.8, AXHUB_TOKEN fallback documented); Test baseline as in step 3.
5. **Manual Windows VM smoke** (US-1005 rows 15-19): breadcrumb JSON pasted into GitHub Release notes. **No tag without this.**
6. **Tag + push**: `git tag v0.1.7 && git push origin main --tags`. Release workflow auto-fires per `docs/RELEASE.md:52`; cosign keyless signs all binaries (Phase 9 pipeline reused).

**Acceptance**: all 6 version sync points = `0.1.7`; test baseline matches; CHANGELOG references this plan path; Win VM breadcrumb appended to Release notes; `git diff --stat v0.1.6 HEAD -- bin/install.sh hooks/session-start.sh` shows ONLY the line-48 bump on `install.sh`, ZERO edits on `session-start.sh`; `git diff --stat v0.1.6 HEAD -- hooks/hooks.json` shows ONLY the 1-block addition.

**Blocked by**: US-1001, US-1002, US-1003, US-1004, US-1005.

---

## Story dependency graph

```
US-1001 (install.ps1) ----+
                          |
US-1002 (session-start.ps1) -+--> US-1003 (hooks.json) --> US-1004 (tests) --> US-1005 (docs) --> US-1006 (release v0.1.7)
```

US-1001 and US-1002 can run in parallel (US-1002 only needs US-1001's filename).

---

## ADR

**Decision**: Add Windows PowerShell mirrors of axhub plugin's automation scripts (`bin/install.ps1`, `hooks/session-start.ps1`) and register a sibling SessionStart hook entry in `hooks/hooks.json` using Anthropic's documented `"shell": "powershell"` field, so vibe coders on vanilla Windows 10 / 11 (no Git Bash, no WSL) can use the plugin end-to-end. Existing `bin/install.sh` and `hooks/session-start.sh` byte-identical (v0.1.6 baseline). Compiled `axhub-helpers-windows-amd64.exe` subcommands unchanged from Phase 9.

**Drivers** (top 3):
1. **Windows vibe coder UX parity**: v0.1.6 ships the Windows binary + Credential Manager bridge but SessionStart still runs `bash hooks/session-start.sh`, which fails on vanilla Windows. Without this fix, every Windows user hits broken-plugin first-session.
2. **Public hooks.json contract stability**: marketplace surface. Sibling-entry addition (additive only) is the only safe path; replacing the existing entry would brick installed v0.1.6 plugins.
3. **Maintenance cost vs duplication**: two install scripts and two session-start scripts. Mitigated by US-1004 cases 2 / 3 / 7 — automated parity assertions on RELEASE_VERSION, AXHUB_SKIP_AUTODOWNLOAD literal, and structural mirroring.

**Alternatives considered**:
- (A) hooks.json native `condition` / `platform` field: invalidated by direct WebFetch — no such field. Only `"shell"` exists.
- (C) Wrapper command (Node or dispatcher): rejected. Stock Windows lacks Node. Custom dispatcher = third file when `"shell"` does the routing for free.
- (D) `axhub-helpers session-start-hook` runs install logic: chicken-and-egg. Binary itself is what install fetches. Possible v0.2 path after build-time stripping, not v0.1.7 scope.

**Why chosen**: Option B maps to Anthropic's documented `"shell"` field, requires zero changes to existing `session-start.sh` (preserving macOS / Linux contract byte-identically), isolates Windows failure modes (ExecutionPolicy, EDR) into the powershell entry. Test surface 100% mockable on macOS CI.

**Consequences**:
- + Vanilla Windows 10 / 11 vibe coders get auto-bootstrap parity, no Git Bash / WSL prerequisite.
- + Zero edits to compiled `axhub-helpers-windows-amd64.exe`. Phase 9 keychain bridge integrates as-is.
- + macOS / Linux users see zero behavior change — bash entry at [0] byte-identical, new powershell entry at [1] no-ops on Unix.
- − Windows v0.1.7 .ps1 NOT Authenticode-signed. EDR may quarantine (Pre-Mortem #2). 4-part Korean error #2 owns this; AXHUB_TOKEN fallback documented.
- − Two scripts to maintain in lockstep — mitigated by US-1004 parity assertions.
- − Hooks.json schema dependency on `"shell": "powershell"`. If Anthropic changes semantics, both .ps1 re-route. US-1004 case 12 asserts the literal string — spec change forces test failure.
- − No real Windows CI; manual VM smoke only. v0.1.8: GitHub Actions windows-latest runner.

**Follow-ups (v0.1.8)**:
- Authenticode sign .ps1 (PS honors Authenticode on .ps1 same as binaries). Once signed, EDR allowlist becomes legitimate.
- GitHub Actions windows-latest runner — automate the manual VM smoke.
- Telemetry: if `windows.hook.exec_policy_blocked` > 5%, ship signed .ps1.
- Telemetry: if `windows.hook.edr_killed` > 5%, prioritize Authenticode signing of .ps1 + .exe.
- **Open for Critic**: add `cmd.exe` shim as third-fallback for users with PowerShell GPO-disabled? NOT in scope — Anthropic spec documents only `bash` and `powershell`.

---

## Open questions (also persisted to `.omc/plans/open-questions.md`)

1. Does Claude Code on Windows use Windows PowerShell 5.1 (built-in) or PowerShell Core 7+ (`pwsh`)? Docs say "spawn PowerShell directly" but don't specify version. Pre-Mortem #4 mitigation = telemetry breadcrumb captures `$PSVersionTable.PSVersion`. Needed before v0.1.8 if any PS 7+ syntax is allowed.
2. Should `bin/install.ps1` verify sha256 against `manifest.json` from GitHub Release (Phase 9 cosign infrastructure)? `bin/install.sh:63` does NOT verify checksums — trusts `curl -fsSL` + GitHub TLS. For consistency, .ps1 should match. v0.1.8 candidate: add checksum verification to BOTH simultaneously.
3. Does `"shell": "powershell"` actually no-op on macOS / Linux (no powershell.exe), or does Claude Code emit a visible error? If the latter, macOS / Linux users see spurious error popup every session — needs spec clarification. Pre-Mortem #4 / US-1006 manual VM smoke is the gate; if visible error observed, US-1003 must add a marker field or workaround.

---

## ASCII verification proof

```
perl -ne 'print "Line $.: $_" if /[\x{200B}-\x{200F}\x{FEFF}]/' .omc/plans/phase-10-windows-ps1-hooks.md
```

Expected output: empty. Reviewer runs this after the file is written; zero matches confirms ASCII-clean (no U+200B per Phase 9 lesson).

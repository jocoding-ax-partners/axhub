# Windows VM smoke checklist (Phase 11 US-1103)

Manual 14-step checklist for next pilot session. Companion executor at
`tests/smoke-windows-vm-checklist.ps1` codifies steps as PowerShell behind
`$env:AXHUB_VM_SMOKE` guard.

## Prerequisites

- Real Windows 11 VM (Parallels / UTM / Azure free tier / GitHub Actions windows-latest runner)
- Claude Code 2.1.84+ installed inside VM
- ax-hub-cli available on PATH (latest release)
- PowerShell 5.1+ (default on Win10/11)

## 14 manual steps

1. **Provision Win11 VM**
   - Capture: hypervisor name + VM specs (CPU/RAM/disk)
   - Apple Silicon host: Win11 ARM via Parallels recommended (UTM has known sound/clipboard quirks)
   - Cloud alt: Azure B2s free tier or GitHub Actions windows-latest

2. **Install Claude Code 2.1.84+**
   - `winget install Anthropic.ClaudeCode` (or download .msi from claude.ai/download)
   - Verify: `claude --version` shows 2.1.84 or higher

3. **Install plugin**
   - In a new claude session: `/plugin install axhub@axhub`
   - Verify: `~/.claude/plugins/marketplaces/axhub-marketplace/axhub/` exists

4. **Confirm current universal hook config does not auto-fire session-start.ps1**
   - Start fresh claude session
   - Verify: no `shell:powershell` SessionStart hook is registered in `hooks/hooks.json`
   - Reason: the universal PowerShell sibling caused visible startup errors on non-Windows hosts. Stock Windows auto-SessionStart requires future platform-specific hook packaging.

5. **Confirm install.ps1 auto-downloads windows-amd64.exe**
   - First session should trigger install.ps1 if `bin\axhub-helpers.exe` missing
   - Verify file appears at `~/.claude/plugins/marketplaces/axhub-marketplace/axhub/bin/axhub-helpers.exe`
   - Capture: file size (should be ~109.6M PE32+ x86-64)

6. **Verify cmdkey shows credential post-axhub-auth-login**
   - Run `axhub auth login` (browser OAuth flow)
   - After successful login: `cmdkey /list:axhub` should show TargetName=axhub credential
   - NOTE: cmdkey only shows existence — DOES NOT print the credential value

7. **Run axhub-helpers token-init**
   - Manually: `& "~/.claude/plugins/marketplaces/axhub-marketplace/axhub/bin/axhub-helpers.exe" token-init`
   - Should run inline PowerShell + Add-Type advapi32!CredReadW + parse keyring envelope
   - Verify: stdout JSON shape `{stored_at, source, redacted_token, next_step}`
   - Capture: `source` should be "windows-credential-manager"

8. **Verify token file written**
   - Path: `$env:USERPROFILE\.config\axhub-plugin\token`
   - Verify: file exists, ~60 bytes (axhub_pat_ + 32 hex chars)
   - NTFS permissions: file should be readable only by current user (verify via `Get-Acl`)
   - Capture: first 16 chars (must be `axhub_pat_<8 hex>`)

9. **PreToolUse + PostToolUse hooks fire on bash commands**
   - In claude session: ask "/axhub:status"
   - Verify: PreToolUse fires (consent gate, may prompt approval)
   - Verify: PostToolUse fires on bash exit (Korean systemMessage if non-zero)
   - Capture: transcript snippet showing both hooks

10. **ExecutionPolicy Restricted fallback test**
    - In admin PowerShell: `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy Restricted`
    - Restart claude session
    - Verify: install.ps1 + session-start.ps1 fail gracefully with 4-part Korean systemMessage pointing to AXHUB_TOKEN env var
    - **Restore:** `Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned`

11. **AMSI/EDR detection test (skip if no AV available)**
    - If V3, AhnLab, CrowdStrike, or other EDR installed:
      - Force AMSI inspection on PowerShell process
      - Verify: 4-part Korean systemMessage references "보안 솔루션" + AXHUB_TOKEN fallback + Authenticode roadmap
    - If no AV: skip, document as "deferred to enterprise pilot"

12. **MAX_PATH test (Korean profile + deep root)**
    - If user profile name is Korean (e.g., `홍길동`) AND CLAUDE_PLUGIN_ROOT > 200 chars:
      - Verify: PathTooLongException catch fires with 4-part Korean systemMessage referencing LongPathsEnabled
    - If not applicable: simulate by `subst Z: <very-long-path>` then run from Z:

13. **NTLM proxy test (skip if no corp proxy)**
    - If corp proxy with NTLM auth:
      - Set `$env:HTTPS_PROXY="http://proxy.corp:8080"`
      - Run install.ps1
      - Verify: HTTP 407 catch fires with 4-part Korean systemMessage referencing -ProxyUseDefaultCredentials
    - If no proxy: skip

14. **Capture full transcript**
    - Save claude session transcript to `docs/pilot/v0.1.x-windows-vm-result.txt`
    - Include: VM specs, Claude Code version, plugin version, all 14 step outcomes
    - Pass/fail summary table at top

## Acceptance for next pilot session

- Steps 1-9: MUST PASS (core happy path)
- Steps 10, 12, 14: MUST RUN (capture evidence even if can't fully exercise)
- Steps 11, 13: SKIP if env unavailable (document as deferred)

## Companion executor

`tests/smoke-windows-vm-checklist.ps1` runs the same 14 steps as PowerShell
functions behind `if ($env:AXHUB_VM_SMOKE -eq '1')` guard. Use during real
VM session for reproducibility.

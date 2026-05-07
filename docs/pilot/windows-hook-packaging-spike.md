# Windows hook packaging spike — 2026-05-07

## Decision

Stock Windows automatic SessionStart stays **deferred**. Keep the universal plugin hook config on the existing POSIX-safe path:

```json
"command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh"
```

Do not add a universal `shell: "powershell"` SessionStart sibling. Native Windows support remains Tier 2 unless a Windows-specific hook package, host-gated hook config, or wrapper path is proven in a real Windows Claude Code smoke.

## Evidence checked

Official Claude Code docs checked on 2026-05-07:

- Hooks reference: <https://code.claude.com/docs/en/hooks>
- Plugins reference: <https://code.claude.com/docs/en/plugins-reference>
- Tools reference: <https://code.claude.com/docs/en/tools-reference>
- Status line reference: <https://code.claude.com/docs/en/statusline>

Findings:

1. Plugin hooks are bundled from `hooks/hooks.json` and merge when the plugin is enabled.
2. Command hooks require a `command` field.
3. Windows command hooks can use `shell: "powershell"`; Claude Code auto-detects `pwsh.exe` and falls back to `powershell.exe`.
4. The checked plugin docs describe `hooks` as a path/string/array/object component, but do not document a per-OS hook gate for plugin hook entries.
5. The checked plugin docs describe plugin `bin/` executables as added to the **Bash tool** PATH. That does not prove native Windows hook or statusLine resolution for extensionless `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers`.
6. Status line docs say Windows status line commands run through Git Bash when installed, or PowerShell when Git Bash is absent. They recommend invoking PowerShell explicitly for a PowerShell status line command.

## Current repo constraints

- `hooks/hooks.json` currently has one universal SessionStart hook: `bash ${CLAUDE_PLUGIN_ROOT}/hooks/session-start.sh`.
- `tests/manifest.test.ts` intentionally rejects universal PowerShell SessionStart siblings because non-Windows hosts without PowerShell previously surfaced startup errors.
- `hooks/session-start.ps1` and `bin/install.ps1` exist, but they are explicit/manual or future Windows-specific hook-package paths today.
- `UserPromptSubmit`, `PreToolUse`, and `PostToolUse` currently call `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers ...`; native Windows `.exe` resolution for those hook contexts is unproven by official docs.

## Tier decision

| Tier | Status after spike |
|---|---|
| Tier 1 macOS/Linux automatic | Supported. Keep bash SessionStart and POSIX statusline/install paths green. |
| Tier 2 Windows native manual/headless/explicit PowerShell | Supported by docs and scripts: `bin/install.ps1`, `hooks/session-start.ps1` manual smoke, `AXHUB_TOKEN`, token-import, and `.exe` helper paths. |
| Tier 3 Windows automatic SessionStart | Blocked. Need platform-gated hook packaging or a Windows wrapper plus real Windows smoke evidence. |
| Tier 4 Git Bash/WSL fallback | Supported as POSIX-compatible lanes with shell boundary documented. |

## Exit criteria for Tier 3 promotion

Before claiming stock Windows automatic SessionStart support, all of these must be true:

1. Official docs or a real Claude Code smoke proves that a Windows hook config can be installed without evaluating PowerShell hooks on macOS/Linux.
2. Native Windows hook commands resolve the helper path safely, either by using `axhub-helpers.exe` explicitly or through a wrapper tested in PowerShell/cmd contexts.
3. `UserPromptSubmit`, `PreToolUse`, and `PostToolUse` hook paths are verified on native Windows, not only SessionStart.
4. statusLine guidance is verified for Windows PowerShell and Git Bash cases.
5. `tests/manifest.test.ts`, macOS/Linux checks, and Windows smoke all pass without startup-noise regressions.

## Implementation consequence

Proceed with Option A from `.omx/plans/windows-cross-platform-compatibility.md`:

- Keep universal `hooks/hooks.json` bash-only for SessionStart.
- Tighten README, `bin/README.md`, and Windows smoke checklist wording so they do not imply automatic native Windows SessionStart today.
- Document native Windows as manual/headless/explicit PowerShell until the Tier 3 gates above are satisfied.

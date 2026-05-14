# bin/

This directory holds the compiled `axhub-helpers` binary (single multi-command Rust executable). Primary source lives in `../crates/axhub-helpers`.

Claude Code plugin docs state that plugin `bin/` executables are added to the **Bash tool** `PATH` while the plugin is enabled, so POSIX/Bash-path skills and hooks may invoke the helper as `axhub-helpers <subcommand>`. Windows native `.ps1` mirrors ship for install, session-start, and statusline (v0.5.12+). Use the explicit `powershell.exe -NoProfile -ExecutionPolicy Bypass -File` invocation form — as done in `.github/workflows/windows-smoke.yml` — since bare `.ps1` paths are blocked under stock Win10/11 `ExecutionPolicy=Restricted` and are absent from `cmd` `PATHEXT`.

## Build

Native (current arch):

```bash
bun install
bun run build  # wraps cargo build --release -p axhub-helpers
```

All platforms (release):

The authoritative release build runs in `.github/workflows/release.yml` across the Rust target matrix. `bun run build:all` is available for maintainers with all Rust targets/linkers installed.

Smoke test:

```bash
bun run smoke
```

## Build outputs (gitignored)

- `axhub-helpers` (native Rust helper)
- `axhub-helpers-darwin-arm64` / `-darwin-amd64` / `-linux-amd64` / `-linux-arm64` / `-windows-amd64.exe` (release Rust helpers)
  - `windows-amd64.exe`: PowerShell + Add-Type PInvoke against `advapi32!CredReadW` for keychain. No `Install-Module` required, stock Win10/11.

## Windows installer (Phase 10 v0.1.7+)

- `bin/install.ps1` — PowerShell 5.1+ mirror of `install.sh`. It is the native Windows installer for explicit/manual smoke and for a future Windows-specific hook package.
- `hooks/session-start.ps1` can call `install.ps1`, but it is **not** registered in the universal `hooks/hooks.json` today. Universal PowerShell SessionStart stays disabled until platform-specific hook packaging or a Windows wrapper is proven safe.
- Claude Code supports `"shell": "powershell"` for command hooks on Windows, but the checked plugin docs do not document a per-OS plugin hook gate.
- No `Install-Module`, no `Add-Type` in the installer (EDR-clean — different from helper token extraction, which uses inline PInvoke).

## Contract

All subcommands accept JSON on stdin and emit JSON on stdout. stderr carries diagnostics. Exit codes mirror ax-hub-cli (0/1/64/65/66/67/68). Hook-facing subcommands return `{"hookSpecificOutput": {...}, "systemMessage": "..."}` per Claude Code hook spec.

# bin/

This directory holds the compiled `axhub-helpers` binary (single multi-command Bun-compiled executable). Source lives in `../src/axhub-helpers/index.ts`.

Claude Code adds this `bin/` to `PATH` while the plugin is enabled, so skills/commands/hooks invoke the helper as `axhub-helpers <subcommand>` (no path needed).

## Build

Native (current arch):

```bash
bun install
bun run build
```

All platforms (release):

```bash
bun run build:all
```

Smoke test:

```bash
bun run smoke
```

## Build outputs (gitignored)

- `axhub-helpers` (native)
- `axhub-helpers-darwin-arm64` / `-darwin-amd64` / `-linux-amd64` / `-linux-arm64` / `-windows-amd64.exe` (release)

## Contract

All subcommands accept JSON on stdin and emit JSON on stdout. stderr carries diagnostics. Exit codes mirror ax-hub-cli (0/1/64/65/66/67/68). Hook-facing subcommands return `{"hookSpecificOutput": {...}, "systemMessage": "..."}` per Claude Code hook spec.

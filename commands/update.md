---
description: axhub CLI 업데이트 확인 (기본 check-only, apply 는 명시 요청 때만)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), AskUserQuestion
argument-hint: "[--check-only] [--force]"
model: sonnet
---

Default behavior for `/axhub:update` is **check-only**.

Critical: for check-only update/version requests, including QA or evidence-mode prompts, do not inspect helper binaries, plugin manifests, cache directories, auth, preflight, doctor, or compatibility state before the update check. Those are separate workflows.

Run this command first:

```bash
axhub update check --json
```

Then summarize `current`, `latest`, and `has_update` in Korean.

Rules:
- Do not run `axhub auth status`, `axhub auth refresh`, `axhub auth login`, preflight, doctor, or environment diagnostics.
- Do not run `axhub-helpers`, helper discovery, plugin manifest inspection, cache scanning, or compatibility diagnostics for a check-only update request.
- Do not mention update-related env toggles; the CLI handles signature verification internally.
- Do not claim the read-only check is blocked unless the Bash tool result actually says so.
- If `has_update: false`, say the installed CLI is already current and stop.
- If `has_update: true`, show the current/latest versions and stop unless the user explicitly asked to apply.
- In non-interactive subprocesses (`CI`, `CLAUDE_NON_INTERACTIVE`, or no TTY), never apply; stop after the read-only check.
- Destructive apply details live in `${CLAUDE_PLUGIN_ROOT}/skills/update/SKILL.md` and require explicit user intent.

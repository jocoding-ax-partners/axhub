---
description: deploy 스킬로 현재 앱을 axhub 에 배포 (슬래시 진입로 — 자연어 흐름과 동일한 안전 가드 적용)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(git:*), AskUserQuestion
argument-hint: "[app-slug] [--commit <sha>] [--dry-run]"
model: sonnet
---

Trigger the axhub `deploy` skill with arguments: $ARGUMENTS.

IMPORTANT: In `claude -p`, CI, `$CLAUDE_NON_INTERACTIVE`, or any AskUserQuestion denial/unavailable context, never ask a text fallback question. A final response containing `진행할까요?` or a numbered approval menu is a bug. Force `DEPLOY_DECISION=dry_run`, do not mint consent, and keep executing the dry-run command path so CLI/auth errors are still surfaced.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/deploy/SKILL.md`. Interactive slash invocation is explicit consent for invoking the skill, but it does NOT bypass the AskUserQuestion preview card or the HMAC consent token requirement — the PreToolUse hook will still verify consent before any destructive bash call. In `claude -p`, CI, `$CLAUDE_NON_INTERACTIVE`, or any headless/denied AskUserQuestion context, follow the deploy SKILL headless rule: do not call AskUserQuestion, force the registry safe default (`--dry-run`), and continue through the dry-run command path. If the user passes `--dry-run`, propagate it to `axhub deploy create`; if the user passes `--commit`, bind that SHA as the deploy commit. Branch is resolved from git context and is not forwarded to `axhub deploy create`.

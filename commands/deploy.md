---
description: deploy 스킬로 현재 앱을 axhub 에 배포 (슬래시 진입로 — 자연어 흐름과 동일한 안전 가드 적용)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(git:*), AskUserQuestion
argument-hint: "[app-slug] [--branch <name>] [--dry-run]"
model: sonnet
---

Trigger the axhub `deploy` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/deploy/SKILL.md`. Slash invocation is explicit consent for invoking the skill, but it does NOT bypass the AskUserQuestion preview card or the HMAC consent token requirement — the PreToolUse hook will still verify consent before any destructive bash call. If the user passes `--dry-run`, propagate it to `axhub deploy create`.

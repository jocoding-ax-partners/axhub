---
description: 현재 프로필의 axhub 앱 목록/관리 (삭제 등 파괴적 작업은 승인 필요)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(jq:*)
argument-hint: "[--all] [--slug-prefix <name>]"
model: haiku
---

Trigger the axhub `apps` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/apps/SKILL.md`. Slash invocation starts the apps skill, but destructive app operations still require the AskUserQuestion preview card and explicit execute decision before any mutation command.

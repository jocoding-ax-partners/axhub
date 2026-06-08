---
description: axhub 배포 로그 보기 (빌드 로그 또는 런타임 pod 로그, --follow 스트림 가능)
allowed-tools: Bash(axhub-helpers:*), Bash(axhub:*), Bash(jq:*)
argument-hint: "[deployment-id] [--source build|pod]"
model: haiku
---

Trigger the axhub `logs` skill with arguments: $ARGUMENTS.

Apply the workflow defined in `${CLAUDE_PLUGIN_ROOT}/skills/logs/SKILL.md`. Logs are read-only; slash invocation does not bypass preview/confirmation rules for any follow-up destructive workflow.

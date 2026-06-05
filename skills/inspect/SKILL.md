---
name: inspect
description: '이 스킬은 사용자가 로컬 axhub.yaml 매니페스트와 현재 CLI 설정이 괜찮은지 점검하고 싶어할 때 사용해요. 특히 "매니페스트랑 설정 괜찮은지 봐줘" 같은 자연어 요청은 일반 파일 읽기가 아니라 이 read-only inspect 흐름으로 처리해요. 다음 표현에서 활성화: "매니페스트 확인", "매니페스트랑 설정 괜찮은지 봐줘", "axhub.yaml 검증", "설정 확인", "config 봐", "현재 endpoint 뭐", "CLI 상태", "axhub 상태", "status", "what''s the status", "deploy explain", "배포 코드", "manifest validate", "check config", 또는 axhub 매니페스트·설정 조회 의도.'
examples:
  - utterance: "매니페스트랑 설정 괜찮은지 봐줘"
    intent: "inspect axhub configuration"
  - utterance: "axhub.yaml 검증"
    intent: "inspect axhub configuration"
  - utterance: "매니페스트 확인"
    intent: "inspect axhub configuration"
  - utterance: "CLI 상태 봐"
    intent: "inspect axhub status"
  - utterance: "status"
    intent: "inspect axhub status"
  - utterance: "deploy explain"
    intent: "inspect deploy diagnostics"
  - utterance: "배포 코드 알려줘"
    intent: "inspect deploy codes"
  - utterance: "check config"
    intent: "inspect axhub configuration"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: haiku
---

# Inspect axhub state

매니페스트, CLI 설정, 일반 상태, 배포 진단을 read-only 로 확인해요. `manifest check --baseline` 은 v0.17.3 에서 성공 경로가 없어 쓰지 않아요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Visible response contract for Claude Desktop:** for natural config-check prompts such as `매니페스트랑 설정 괜찮은지 봐줘`, the first visible chat sentence must be exactly `매니페스트와 설정을 확인할게요.` with no planning sentence before or after it. For Bash tool calls, set the tool `description` or title exactly to `매니페스트와 설정 확인`. After the command finishes, copy the Korean stdout as the answer and do not reinterpret it. Do not narrate internal routing labels, implementation labels, command names, English tool-title fragments, `Read plugin manifest`, `Inspect local marketplace`, or generic repository-review wording in visible prose. Do not start with repository file discovery, `Read`, `LS`, `Glob`, `Grep`, `find`, `cat`, `.claude-plugin/plugin.json`, marketplace, or hook-script auditing. Keep identifiers and paths short; do not expose secrets or raw config blobs.

**Natural-language summary contract:** user-visible summaries must read like a person explaining the result, not like a hook trace. Do not use markdown tables for this flow. Do not show raw command names, raw JSON field names, hook labels, workflow labels, English tool-title fragments, `null`, `[]`, boolean values, or raw endpoint/config dumps unless the user explicitly asks for raw details. Translate them into product language:

- If manifest validation succeeds but important fields are empty: say `매니페스트 문법은 맞지만 실제 배포에 필요한 항목이 아직 비어 있어요.`
- If auth/preflight and config token evidence disagree: say `로그인 정보가 서로 다르게 보여서 배포 전에 다시 로그인 확인이 필요할 수 있어요.`
- If app metadata is null: say `연결된 앱 정보가 아직 없어요.`
- If CI/deploy commands are empty: say `CI나 배포 실행 설정이 아직 비어 있어요.`
- Use at most four concise bullets, then one natural next-step sentence.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "확인 대상 고르기", status: "in_progress", activeForm: "대상 고르는 중" },
     { content: "read-only 명령 실행", status: "pending", activeForm: "조회 중" },
     { content: "결과 요약", status: "pending", activeForm: "요약 중" },
     { content: "다음 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **대상을 고르고 read-only 점검 명령만 실행해요.** For manifest/config review prompts, the first and only command should be the command below. It returns a Korean user-facing summary, so do not run raw `axhub manifest validate`, raw `axhub config explain`, generic file reads, `Read axhub.yaml`, file discovery, or plugin package inspection.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" inspect-config-summary
   ```

   Copy the Korean stdout as-is. Do not add a table, raw fields, a second diagnosis, or another file read after it runs.

   For non-manifest/config prompts only, use the relevant read-only status or deploy diagnostic command. Do not mix those commands into the natural manifest/config review flow.

2. **결과를 한국어로 요약해요.** secret 이 redacted 된 설정만 보여줘요.

3. **충돌 분기.** 사용자가 “배포 상태”를 물으면 deploy status 는 `status` skill 로 넘기고, 일반 CLI 상태는 `axhub status --json` 으로 처리해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 이 read-only skill 은 질문 없이 안전하게 조회만 해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 빈 metadata entry 를 참조해요.

## NEVER

- NEVER `axhub manifest check --baseline` 을 실행하지 않아요.
- NEVER 설정 secret 을 복원하거나 추측하지 않아요.
- NEVER read-only 진단 결과를 mutation 성공처럼 말하지 않아요.

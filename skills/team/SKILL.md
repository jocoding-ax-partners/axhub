---
name: team
description: '이 스킬은 사용자가 axhub 워크스페이스나 앱에 팀원을 초대하거나, 초대 목록을 보거나, 앱 접근 권한을 주고받고 싶어할 때 사용해요. 다음 표현에서 활성화: "팀원 초대", "초대해", "멤버 초대", "사람 추가", "협업자 추가", "초대 목록", "초대 취소", "접근 권한 줘", "앱 공유", "공유해", "invite", "add member", "team invite", "share app", "grant access", 또는 axhub 팀·접근 관리 의도. 멤버 role 변경·비활성화는 admin 영역이라 다루지 않아요.'
examples:
  - utterance: "팀원 초대해"
    intent: "manage team invitations"
  - utterance: "초대 목록 봐"
    intent: "manage team invitations"
  - utterance: "이 앱 공유해"
    intent: "manage app access"
  - utterance: "invite teammate"
    intent: "manage team invitations"
  - utterance: "grant access"
    intent: "manage app access"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Team invitations and access

워크스페이스 초대와 앱 접근 권한을 CLI 경계 안에서 처리해요. 읽기는 바로 조회하고, 초대/취소/접근 변경은 preview 와 consent 뒤에만 실행해요.

## Claude Desktop natural-language path

For pure user phrases like `팀원 초대해`, `초대 목록 봐`, or `이 앱 공유해`, do not ask whether the user means a Claude/OMC multi-agent team. In an AXHub project, these phrases mean AXHub workspace invitation or app access management.

First visible chat sentence must be exactly `팀 작업을 확인할게요.`

Use one Bash tool first:

```bash
axhub-helpers team-summary --user-utterance "<latest user sentence>"
```

The Bash tool title/description must be exactly `팀 작업 확인`.

Copy the Korean stdout as the answer. If an invitation target is missing, ask naturally for the person's email and role. If a workspace or app is missing, ask for that target naturally. Do not send invitations, cancel invitations, resend invitations, or change app access until the user has provided the target, seen a Korean preview, and explicitly approved.

Do not mention or display route labels, slash commands, skill names, `preflight`, `tenant`, `current_team_id`, raw tenant IDs, raw user IDs, raw JSON fields, command names, raw command lines, ToolSearch, Claude/OMC multi-agent team comparisons, hidden routing values, internal workflow labels, English tool-title fragments, or raw emails that the user did not type.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s
' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d	%s
",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

**Tenant grounding.** 팀 초대/멤버 조회는 tenant-scoped 예요. 사용자가 tenant 를 명시하지 않았으면 preflight 의 active team 만 사용하고, active team 이 없으면 실행을 멈춰요. `tenants[]` 첫 항목을 추측해서 쓰지 않아요.

```bash
TENANT="${AXHUB_TENANT:-$(printf '%s
' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')}"
if [ -z "$TENANT" ]; then
  echo "현재 workspace 를 특정할 수 없어요. workspace skill 로 tenant 를 확인하거나 AXHUB_TENANT 를 명시해요." >&2
  exit 64
fi
```

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "작업 확인", status: "in_progress", activeForm: "작업 고르는 중" },
     { content: "대상 resolve", status: "pending", activeForm: "대상 확인 중" },
     { content: "preview", status: "pending", activeForm: "preview 준비 중" },
     { content: "동의 받고 실행", status: "pending", activeForm: "실행 중" },
     { content: "결과 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **작업을 분기해요.** 목록 조회는 read-only, 초대 발송·취소·재발송·접근 변경은 mutation 이에요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

2. **작업 선택 질문을 써요.** 비대화형 기본은 `list`예요.

   ```json
   {"questions":[{"question":"팀·접근 관련 어떤 작업을 할까요?","header":"팀 작업","options":[{"label":"목록 보기","description":"read-only 로 확인해요"},{"label":"초대","description":"메일 초대를 보낼 준비를 해요"},{"label":"접근 변경","description":"앱 접근을 바꿀 준비를 해요"}]}]}
   ```

3. **읽기 명령을 실행해요.**

   ```bash
   axhub invitations list --status pending --expires-within 168h --tenant "$TENANT" --json
   axhub members list --tenant "$TENANT" --json
   axhub members me --tenant "$TENANT" --json
   axhub access check --app "$APP_ID" --json
   ```


   Mutation preview 에서는 아래 질문 중 해당 작업을 써요.

   ```json
   {"questions":[{"question":"이 초대를 보낼까요?","header":"초대","options":[{"label":"중단","description":"초대를 보내지 않아요"},{"label":"보내기","description":"초대 메일을 보내요"}]}]}
   ```

   ```json
   {"questions":[{"question":"이 앱 접근을 변경할까요?","header":"앱 접근","options":[{"label":"중단","description":"권한을 바꾸지 않아요"},{"label":"변경","description":"앱 접근을 변경해요"}]}]}
   ```

4. **mutation 은 preview 후 실행해요.**

   Consent binding 은 helper parser 와 같은 action/context 로 맞춰요: `invitation_send={email,tenant,role}`, `invitation_bulk={source,tenant,role}`, `invitation_cancel={invitation_id,tenant}`, `invitation_resend={invitation_id,tenant,role}`, `access_grant|access_revoke={app_id}`, `access_invite|access_uninvite={app_id,user}` 예요.

   ```bash
   axhub invitations send "$EMAIL" --role member --tenant "$TENANT" --json
   axhub invitations bulk --from-file users.csv --role member --strict --execute --tenant "$TENANT" --json
   axhub invitations cancel "$INVITE_ID" --execute --tenant "$TENANT" --json
   axhub invitations resend "$INVITE_ID" --role member --execute --tenant "$TENANT" --json
   axhub access grant --app "$APP_ID" --json
   axhub access revoke --app "$APP_ID" --execute --json
   axhub access invite --app "$APP_ID" --user "$USER_ID" --execute --json
   axhub access uninvite --app "$APP_ID" --user "$USER_ID" --execute --json
   ```

## NEVER

- NEVER `members set-role`, `members deactivate`, `members reactivate` 를 vibe skill 로 실행하지 않아요.
- NEVER 비대화형에서 초대 메일이나 접근 변경을 자동 실행하지 않아요.
- NEVER access grant 로 다른 사용자를 추가할 수 있다고 말하지 않아요. self-grant 또는 access invite 를 구분해요.

---
name: publish
description: '이 스킬은 사용자가 만든 axhub 앱을 마켓플레이스에 공개 심사로 제출하고 싶어할 때 사용해요. 다음 표현에서 활성화: "이 앱 공개 심사 넣고 싶어", "앱 공개", "게시", "게시해", "마켓에 올려", "퍼블리시", "심사 제출", "심사 올려", "스토어에 올려", "submit for review", "make public", 또는 axhub 앱 공개 심사 제출 의도.'
examples:
  - utterance: "이 앱 공개 심사 넣고 싶어"
    intent: "check app review submission readiness"
  - utterance: "앱 공개 심사 올려"
    intent: "submit app for marketplace review"
  - utterance: "paydrop 퍼블리시"
    intent: "submit app for marketplace review"
  - utterance: "마켓에 올려"
    intent: "submit app for marketplace review"
  - utterance: "publish this app"
    intent: "submit app for marketplace review"
  - utterance: "submit for review"
    intent: "submit app for marketplace review"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Publish app review request

axhub 앱을 마켓플레이스 공개 심사로 제출해요. 현재 CLI v0.17.3 은 제출 생성까지만 구현되어 있고 `--watch` 는 backend 실행에서 읽지 않으니 승인/반려 polling 을 약속하지 않아요.

## Claude Desktop natural-language path

Use this path first for normal human prompts such as `이 앱 공개 심사 넣고 싶어`, `앱 공개 심사 올려줘`, `마켓에 올려`, or `스토어에 올려줘`.

1. First visible sentence, exactly: `공개 심사 준비를 확인할게요.`
2. Use exactly one Bash tool call. Bash description/title, exactly: `공개 심사 준비 확인`
3. Bash command:

   ```bash
   axhub-helpers publish-summary --user-utterance "<latest user sentence>"
   ```

4. Copy the Korean stdout as the answer and stop unless the user has already provided both a review note and explicit approval.
5. If a review note is missing, ask for the note naturally. Do not submit, confirm approval, or call `axhub publish` yet.
6. Actual submission requires a Korean preview of the target app and note, then explicit approval. Submission is an external marketplace mutation.

Do not read `quality.json`, local state files, QA result files, plugin source, repo files, or package files before the preparation summary. Do not write `publish 스킬`, `publish 흐름`, `/axhub:publish`, `Preflight`, `auth ok`, `current_app`, `review=...`, command names, raw JSON fields, raw review status fields, file contents, ToolSearch narration, or English tool-title fragments in visible text.

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

**Tenant 선택 (axhub-tenant-picker:L1).** 모든 fence 에서 `.axhub/state/tenant.json` 을 다시 읽어요 (cross-block source of truth). 명시 override → 캐시 re-read → tenants list → preflight fallback 순으로 tenant 를 결정해요.

```bash
# axhub-tenant-picker:L1 — canonical tenant resolver (매 fence .axhub/state/tenant.json re-read)
TENANT_CACHE=".axhub/state/tenant.json"
TENANT_CACHE_TTL="${AXHUB_TENANT_CACHE_TTL_SECS:-28800}"
AXHUB_TENANT="${AXHUB_TENANT:-}"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"

# Precedence 1: 명시 AXHUB_TENANT env/flag override → 즉시 사용, picker skip
if [ -z "$AXHUB_TENANT" ]; then
  # Precedence 2: .axhub/state/tenant.json re-read — cross-block source of truth
  if [ -f "$TENANT_CACHE" ]; then
    _T=$(jq -r '.tenant // empty' "$TENANT_CACHE" 2>/dev/null || true)
    _TS=$(jq -r '.ts // 0' "$TENANT_CACHE" 2>/dev/null || echo '0')
    _NOW=$(date +%s 2>/dev/null || echo '0')
    _AGE=$(( _NOW - _TS ))
    if [ -n "$_T" ] && [ "$_AGE" -ge 0 ] && [ "$_AGE" -lt "$TENANT_CACHE_TTL" ]; then
      AXHUB_TENANT="$_T"
    else
      rm -f "$TENANT_CACHE"
    fi
  fi

  if [ -z "$AXHUB_TENANT" ]; then
    # Precedence 3: axhub tenants list → needs_pick(≥2) / auto(1) / fallback(0·fail)
    _TENANTS_JSON=$(axhub tenants list --json 2>/dev/null || echo '[]')
    _COUNT=$(printf '%s' "$_TENANTS_JSON" | jq 'if type=="array" then length else 0 end' 2>/dev/null || echo '0')
    if [ "$_COUNT" -eq 1 ]; then
      AXHUB_TENANT=$(printf '%s' "$_TENANTS_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
      mkdir -p "$(dirname "$TENANT_CACHE")"
      _TS_NOW=$(date +%s 2>/dev/null || echo '0')
      printf '{"tenant":"%s","source":"auto","ts":%s}\n' "$AXHUB_TENANT" "$_TS_NOW" > "$TENANT_CACHE"
    elif [ "$_COUNT" -ge 2 ]; then
      CANDIDATES_JSON="$_TENANTS_JSON"
      if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
        # non-TTY: active fallback + 경고 (R4 fail-wrong guard — L1 bash 위치 필수)
        AXHUB_TENANT=$(printf '%s' "$_TENANTS_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
        echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant(\`$AXHUB_TENANT\`)로 진행해요"
      else
        NEEDS_PICK="true"
      fi
    else
      # Precedence 4: preflight current_team_id fallback
      AXHUB_TENANT=$(printf '%s' "${PREFLIGHT_JSON:-{}}" | jq -r '.current_team_id // empty' 2>/dev/null || true)
    fi
  fi
fi
export AXHUB_TENANT
export NEEDS_PICK
export CANDIDATES_JSON
```

`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요.

**Tenant picker (axhub-tenant-picker:L2).** `NEEDS_PICK=true` 이고 대화형 TTY 일 때만 실행해요. `CANDIDATES_JSON` 에서 후보 목록을 읽어 AskUserQuestion 으로 사용자에게 선택을 요청해요. 선택 결과를 `.axhub/state/tenant.json` 에 `{tenant, source:"picker", ts}` 형태로 기록해요 (이후 fence 가 re-read 해서 상속).

```typescript
if (NEEDS_PICK === "true") {
  const candidates = JSON.parse(CANDIDATES_JSON);
  AskUserQuestion({
    questions: [{
      question: "어떤 tenant 로 진행할까요?",
      header: "Tenant",
      multiSelect: false,
      options: candidates.map((t: { id?: string; slug?: string; name?: string }) => ({
        label: t.name ?? t.slug ?? t.id ?? "unknown",
        description: `ID: ${t.id ?? t.slug}`,
      })),
    }],
  });
  // 선택된 tenant ID 를 .axhub/state/tenant.json 에 write-back
  // mkdir -p .axhub/state && echo '{"tenant":"<선택값>","source":"picker","ts":<epoch>}' > .axhub/state/tenant.json
}
```

AskUserQuestion 답변을 받은 뒤 선택된 tenant ID 를 `AXHUB_TENANT` 로 확정하고 `.axhub/state/tenant.json` 에 `{"tenant": "<id>", "source": "picker", "ts": <epoch>}` 를 기록해요. 이후 fence 가 이 파일을 re-read 해서 같은 tenant 를 재사용해요.

**Non-interactive AskUserQuestion guard (D1):** `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 환경에서는 L2 AskUserQuestion 을 건너뛰어요 — L1 블록이 이미 active fallback + 경고를 처리했어요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 `picker` 채널 참조.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "앱 확인", status: "in_progress", activeForm: "앱 확인 중" },
     { content: "제출 사유 확인", status: "pending", activeForm: "사유 확인 중" },
     { content: "제출 preview", status: "pending", activeForm: "preview 준비 중" },
     { content: "동의 받고 제출", status: "pending", activeForm: "제출 중" },
     { content: "결과 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **앱과 note 를 확인해요.** app id/slug 는 preflight, manifest, 또는 `axhub apps mine --json` 후보에서 좁혀요. note 는 1000자를 넘기면 줄이거나 다시 받아요.

2. **Preview card 를 보여줘요.** 앱, note 길이, 공개 심사 제출의 외부 노출 효과를 한국어로 요약해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

3. **AskUserQuestion 으로 제출을 확인해요.**

   ```json
   {"questions":[{"question":"이 앱을 마켓플레이스 심사에 제출할까요?","header":"심사 제출","options":[{"label":"중단","description":"제출하지 않아요"},{"label":"제출","description":"심사 요청을 생성해요"}]}]}
   ```

4. **동의 후 제출해요.** note 원문은 저장하지 않고 sha256 digest 만 계산해 로그/미리보기에는 digest 만 써요.

   ```bash
   NOTE_LENGTH=$(printf '%s' "$NOTE" | python3 -c 'import sys; print(len(sys.stdin.read()))')
   NOTE_DIGEST=$(printf '%s' "$NOTE" | shasum -a 256 | awk '{print "sha256:"$1}')
   APPROVAL_CONTEXT_JSON=$(jq -nc \
     --arg app "$APP" \
     --arg note_length "$NOTE_LENGTH" \
     --arg note_digest "$NOTE_DIGEST" \
     '{tool_call_id:"pending",action:"publish_submit",app_id:$app,profile:"",branch:"",commit_sha:"",context:{note_length:$note_length,note_digest:$note_digest}}')
   ```

   다음 Bash 호출에서만 destructive publish 를 실행해요. preview 확인과 실제 `axhub publish` 를 한 Bash block 에 섞지 않아요.

   ```bash
   axhub publish --app "$APP" --note "$NOTE" --json
   ```

5. **결과를 안내해요.** review/request id 가 있으면 보여주고, `--watch` 는 현재 CLI 미구현이라 심사 상태는 나중에 다시 확인하라고 안내해요.

## NEVER

- NEVER `--watch` 가 승인/반려까지 polling 한다고 말하지 않아요.
- NEVER 비대화형에서 자동 제출하지 않아요.
- NEVER note 1000자 초과를 그대로 보내지 않아요.
- NEVER preview 확인과 `axhub publish` 를 같은 Bash block 에 넣지 않아요.

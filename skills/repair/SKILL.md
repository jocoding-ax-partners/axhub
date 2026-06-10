---
name: repair
description: '이 스킬은 사용자가 axhub CLI 는 설치됐지만 PATH 에 없어서 수리하고 싶어할 때 사용해요. 다음 표현에서 활성화: "PATH 고쳐줘", "PATH 수리", "PATH 등록", "axhub PATH 고쳐", "axhub path repair", "repair path", 또는 doctor 가 on_disk_not_on_path 를 감지한 뒤 PATH 수리를 제안한 상황.'
examples:
  - utterance: "PATH 고쳐줘"
    intent: "repair axhub CLI PATH"
  - utterance: "axhub PATH 등록해줘"
    intent: "repair axhub CLI PATH"
  - utterance: "axhub 경로 고쳐줘"
    intent: "repair axhub CLI PATH"
  - utterance: "repair path"
    intent: "repair axhub CLI PATH"
  - utterance: "fix axhub path"
    intent: "repair axhub CLI PATH"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Repair

axhub CLI 가 이미 디스크에는 있지만 새 터미널의 PATH 에 없을 때, shell rc 또는 Windows User PATH 를 안전하게 고쳐요. 이 스킬은 CLI 설치나 로그인은 하지 않고 PATH 수리만 맡아요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `진단해줘`, `다시 로그인해줘`, or `처음부터 안내해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Preflight (설치 위치 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 `cli_state`, `cli_on_path`, `cli_resolved_path` 를 확인해요. `on_disk_not_on_path` 면 PATH 수리 대상이에요. `cli_present:false` 면 PATH 수리보다 CLI 설치가 먼저예요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`cli_present:false` 이면 수리를 실행하지 말고 `axhub 설치해줘`라고 말하면 된다고 안내해요. 단, `cli_state` 가 `axhub_bin_invalid` 면 설치 안내 대신 — `AXHUB_BIN` 환경변수가 잘못된 경로 (`cli_resolved_path` 값) 를 가리키는 상태라 재설치로 해결 안 되고, `unset AXHUB_BIN` (셸 프로필 export 제거) 또는 올바른 경로로 수정 후 새 세션에서 다시 시도하라고 안내하고 멈춰요. `cli_present:true` 이고 `cli_on_path:true` 면 이미 PATH 에 있어요. `cli_present:true`, `cli_on_path:false`, `cli_state:on_disk_not_on_path` 면 수리를 진행할 수 있어요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "설치 위치 확인",      status: "in_progress", activeForm: "위치 보는 중" },
     { content: "PATH 수리 동의 확인", status: "pending",     activeForm: "동의 묻는 중" },
     { content: "PATH 수리 실행",      status: "pending",     activeForm: "PATH 고치는 중" },
     { content: "수리 후 재진단",      status: "pending",     activeForm: "다시 보는 중" },
     { content: "결과 안내",           status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **Classify current state.** Use the preflight JSON only for routing.

   - `cli_state:axhub_bin_invalid` → stop with: `AXHUB_BIN 환경변수가 잘못된 경로 (cli_resolved_path 값) 를 가리키고 있어요. 재설치로는 해결 안 돼요. unset AXHUB_BIN (셸 프로필 export 제거) 또는 올바른 경로로 수정 후 새 세션에서 다시 시도해 주세요.`
   - `cli_present:false` (그 외) → stop with: `axhub CLI 가 아직 없어요. 먼저 "axhub 설치해줘"라고 말해 주세요.`
   - `cli_on_path:true` → stop with: `PATH 는 이미 괜찮아요. "진단해줘"로 전체 상태를 다시 볼 수 있어요.`
   - `on_disk_not_on_path` → continue.
   - `AXHUB_DISABLE_PATH_REPAIR=1` → stop with: `PATH 자동 수리가 꺼져 있어요. 직접 고치거나 값을 비운 뒤 다시 "PATH 고쳐줘"라고 말해 주세요.`

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — PATH 수리 질문은 `나중에`예요.

2. **Ask before mutating PATH.** PATH 수리는 shell rc 파일 또는 Windows User PATH 를 변경하므로 반드시 동의를 받아요.

   ```json
   {
     "questions": [{
       "question": "PATH 를 고칠까요?",
       "header": "PATH 수리",
       "multiSelect": false,
       "options": [
         {"label": "고치기", "description": "설치된 axhub 경로를 shell rc 또는 Windows User PATH 에 추가해요"},
         {"label": "나중에", "description": "지금은 안내만 보고 파일이나 PATH 를 바꾸지 않아요"}
       ]
     }]
   }
   ```

   `나중에`를 고르면 수리 명령을 실행하지 말고, `PATH 고쳐줘`라고 다시 말하면 이어갈 수 있다고 안내해요.

3. **Run PATH repair.** 동의가 `고치기`일 때만 helper를 실행해요. preflight 의 `cli_resolved_path` 가 있으면 그 parent directory 를 `--dir` 로 넘겨요. 없으면 helper가 installer 후보를 다시 찾아요.

   ```bash
   # dir 를 알 때
   "$HELPER" repair-path --json --dir "<parent-of-cli_resolved_path>"

   # dir 를 모를 때
   "$HELPER" repair-path --json
   ```

   JSON 의 `repaired`, `already_present`, `disabled`, `install_dir`, `shell_rc`, `backup_path`, `error` 를 읽어 한국어로 요약해요. raw JSON, 파일 전체 내용, 환경변수 전체 PATH 값은 사용자에게 노출하지 않아요.

4. **Self-heal check.** 수리 명령이 끝나면 같은 helper로 재진단해요.

   ```bash
   "$HELPER" preflight --json
   ```

   새 터미널을 열기 전에는 현재 프로세스 PATH 가 그대로일 수 있어요. 그래서 `repair-path` 가 성공했고 `preflight` 가 여전히 `cli_on_path:false` 여도 실패로 단정하지 않아요. `shell_rc` 또는 Windows User PATH 에 반영됐고 새 터미널에서 적용된다고 안내해요.

5. **Render final card.**

   성공 예시:

   ```text
   PATH 수리 완료:
     ✓ axhub 설치 경로를 PATH 설정에 추가했어요.
     ✓ 기존 설정 파일은 백업했어요.

   다음 단계: 새 터미널을 열고 "진단해줘"라고 말해 주세요.
   ```

   이미 적용된 상태:

   ```text
   PATH 는 이미 괜찮아요.
   다음 단계: "진단해줘"로 전체 상태를 다시 볼 수 있어요.
   ```

   실패 예시:

   ```text
   PATH 수리를 끝내지 못했어요.
   원인: <짧은 이유>
   다음 단계: 새 터미널을 열어도 안 되면 "진단해줘"라고 말해 주세요.
   ```

## NEVER

- NEVER CLI 설치, 업데이트, 로그인, 앱 생성, 배포를 이 스킬에서 직접 실행하지 않아요.
- NEVER `AXHUB_DISABLE_PATH_REPAIR=1` 을 무시하지 않아요.
- NEVER 비대화형 subprocess 또는 CI 에서 PATH 를 자동 변경하지 않아요.
- NEVER raw PATH 전체값, token, user email, shell rc 파일 전체 내용을 사용자에게 보여주지 않아요.
- NEVER doctor 스킬을 mutation surface 로 바꾸지 않아요. doctor 는 진단, repair 는 PATH 수리만 맡아요.

## Additional Resources

- `../doctor/SKILL.md` — `on_disk_not_on_path` 진단 후 자연어 handoff.
- `../onboarding/SKILL.md` — first-run 전체 준비 흐름.

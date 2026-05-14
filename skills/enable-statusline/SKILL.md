---
name: enable-statusline
description: '이 스킬은 사용자가 axhub statusline 을 활성화하고 싶을 때 사용해요. 다음 표현에서 활성화: "statusline 켜줘", "status line 활성화", "statusline 활성화", "statusline 보여줘", "enable statusline", "activate statusline", "상태표시줄 켜줘", "상태줄 활성화". `~/.claude/settings.json` 에 wiring snippet 을 paste 하도록 클립보드 복사 + step-by-step 가이드를 제공해요.'
examples:
  - utterance: "statusline 켜줘"
    intent: "enable statusline in ~/.claude/settings.json"
  - utterance: "statusline 활성화"
    intent: "enable statusline in ~/.claude/settings.json"
  - utterance: "enable statusline"
    intent: "enable statusline in ~/.claude/settings.json"
  - utterance: "상태표시줄 켜줘"
    intent: "enable statusline in ~/.claude/settings.json"
  - utterance: "activate statusline"
    intent: "enable statusline in ~/.claude/settings.json"
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: haiku
---

# Enable Statusline (선택 활성화)

Claude Code plugin manifest 가 statusLine 필드를 직접 지원하지 않아서 statusline 은 opt-in 방식이에요. `~/.claude/settings.json` 에 wiring snippet 을 직접 paste 하면 활성화돼요.

**플랫폼 지원 범위:** macOS / Linux / Windows + Git Bash / Windows + WSL 만 지원해요. Windows native (PowerShell-only, Git Bash · WSL 둘 다 없음) 는 미지원이에요 — wiring snippet 의 `${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh` 가 bash interpreter 필요한데 PowerShell 단독으로 실행 불가능해요. 이 SKILL 본문의 `command -v` / `[ -t 0 ]` / heredoc 등 bash 문법도 PowerShell 에서 동작 안 해요. Windows native 사용자는 Git Bash (또는 WSL) 를 PATH 에 추가한 다음 다시 시도해주세요. `bin/statusline.ps1` PowerShell mirror 는 v0.6.0+ deferred 예요 (Phase 17 US-1707 spec 따름).

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — statusLine 어떻게 켤래요? → `나중에 할래요` (단, stdout 으로 snippet 출력은 해요 — idempotent read-only 라 안전해요).

비대화형 컨텍스트 (`CLAUDE_NO_TTY=1` 설정 또는 stdin 이 TTY 가 아닌 경우) 에서는 AskUserQuestion 을 건너뛰고 아래 wiring snippet 을 stdout 으로 출력한 뒤 exit 0 해요. pbcopy / clip.exe / xclip 같은 clipboard 도구는 호출하지 않아요.

## Wiring Snippet

`~/.claude/settings.json` 에 추가해야 할 JSON 블록이에요:

<!-- BEGIN STATUSLINE_SNIPPET (codegen-managed by scripts/codegen-statusline-snippet.ts) -->
```json
{
  "statusLine": {
    "type": "command",
    "command": "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"
  }
}
```
<!-- END STATUSLINE_SNIPPET -->

## Workflow

To enable the axhub statusline:

1. **AskUserQuestion — 활성화 방법 선택.**

   ```json
   {
     "questions": [{
       "question": "statusLine 어떻게 켤래요?",
       "header": "활성화",
       "multiSelect": false,
       "options": [
         {"label": "복사해서 붙여 넣을래요", "description": "wiring snippet 을 클립보드에 복사해줄게요. ~/.claude/settings.json 에 paste 하면 돼요."},
         {"label": "어떻게 하는지 보여줘요", "description": "단계별로 설명해줄게요."},
         {"label": "나중에 할래요", "description": "지금은 건너뛰고 나중에 활성화해요."}
       ]
     }]
   }
   ```

2. **선택지별 처리.**

   **`복사해서 붙여 넣을래요` 선택 시:**

   위 wiring snippet 을 임시 파일에 쓰고 best-effort clipboard 복사를 해요:

   ```bash
   if command -v pbcopy >/dev/null 2>&1; then
     printf '%s' "$SNIPPET" | pbcopy
   elif command -v clip.exe >/dev/null 2>&1; then
     printf '%s' "$SNIPPET" | clip.exe
   elif command -v xclip >/dev/null 2>&1; then
     printf '%s' "$SNIPPET" | xclip -selection clipboard
   else
     echo "(클립보드 도구 없어서 stdout 출력으로 대체했어요)"
     printf '%s\n' "$SNIPPET"
   fi
   ```

   복사 성공 시: "복사했어요. `~/.claude/settings.json` 에 paste 하고 Claude Code 재시작해주세요." 라고 안내해요.

   **`어떻게 하는지 보여줘요` 선택 시:**

   아래 4 단계로 안내해요:

   1. `~/.claude/settings.json` 을 텍스트 에디터로 열어요. 파일이 없으면 빈 파일로 만들어도 돼요.
   2. 위 wiring snippet JSON 블록을 `settings.json` 최상위 객체 안에 paste 해요.
   3. JSON 의 닫는 `}` 가 잘 맞는지 확인해요.
   4. Claude Code 를 재시작하면 statusline 이 보여요.

   **`나중에 할래요` 선택 시:**

   조용히 exit 0 해요. 나중에 `/axhub:enable-statusline` 으로 다시 불러요.

## NEVER

- NEVER `~/.claude/settings.json` 을 자동으로 수정해요. trust boundary 위반이에요.
- NEVER 비대화형 환경에서 pbcopy / clip.exe / xclip 호출. clipboard mutation 은 interactive 선택 후에만 해요.
- NEVER Claude Code 를 자동으로 재시작해요. 사용자가 직접 해야 해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `bin/statusline.sh` — wiring snippet 정본 소스. snippet 변경 시 `scripts/codegen-statusline-snippet.ts --write` 실행해요.

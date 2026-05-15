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

**플랫폼 지원 범위:** macOS / Linux / Windows + Git Bash / Windows + WSL / Windows native (PowerShell 5.1+) 를 모두 지원해요. Unix 계열에는 `bin/statusline.sh` snippet 을, Windows native 에는 `bin/statusline.ps1` snippet (v0.5.12+) 을 사용해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — statusLine 어떻게 켤래요? → `나중에 할래요` (단, stdout 으로 snippet 출력은 해요 — idempotent read-only 라 안전해요).

비대화형 컨텍스트 (`CLAUDE_NO_TTY=1` 설정 또는 stdin 이 TTY 가 아닌 경우) 에서는 AskUserQuestion 을 건너뛰고 아래 wiring snippet 을 stdout 으로 출력한 뒤 exit 0 해요. pbcopy / clip.exe / xclip 같은 clipboard 도구는 호출하지 않아요.

## Wiring Snippet (macOS / Linux / Git Bash / WSL)

`~/.claude/settings.json` 에 추가해야 할 Unix JSON 블록이에요:

<!-- BEGIN STATUSLINE_SNIPPET_UNIX (codegen-managed by scripts/codegen-statusline-snippet.ts) -->
```json
{
  "statusLine": {
    "type": "command",
    "command": "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"
  }
}
```
<!-- END STATUSLINE_SNIPPET_UNIX -->

## Wiring Snippet (Windows native — PowerShell 5.1+)

`~/.claude/settings.json` 에 추가해야 할 Windows native JSON 블록이에요. `powershell.exe -NoProfile -ExecutionPolicy Bypass -File` 형식으로 stock Win10/11 `ExecutionPolicy=Restricted` 를 우회해요:

<!-- BEGIN STATUSLINE_SNIPPET_WINDOWS (codegen-managed by scripts/codegen-statusline-snippet.ts) -->
```json
{
  "statusLine": {
    "type": "command",
    "command": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1\""
  }
}
```
<!-- END STATUSLINE_SNIPPET_WINDOWS -->

## Workflow

To enable the axhub statusline:

1. **AskUserQuestion — 활성화 방법 선택.**

   사용자의 OS 를 명시적으로 묻지 않아도 돼요. Step 1 답변에서 플랫폼 신호를 도출해요 — "Windows PowerShell snippet 보여줘요" 선택 시 Windows native 로 판단해요. bash 실행 / `process.platform` 런타임 탐지 금지 (statusLine 컨텍스트에서 노출 안 됨).

   ```json
   {
     "questions": [{
       "question": "statusLine 어떻게 켤래요?",
       "header": "활성화",
       "multiSelect": false,
       "options": [
         {"label": "복사해서 붙여 넣을래요 (Unix bash)", "description": "Unix (macOS/Linux/Git Bash/WSL) 자동 wire 를 axhub-helpers settings-merge --apply 로 호출해요. ~/.claude/settings.json 에 atomic 으로 statusLine 추가해요."},
         {"label": "복사해서 붙여 넣을래요 (Windows PowerShell)", "description": "Windows native (PowerShell 5.1+) 자동 wire 를 axhub-helpers.exe settings-merge --apply 로 호출해요. ~/.claude/settings.json 에 atomic 으로 statusLine 추가해요."},
         {"label": "어떻게 하는지 보여줘요", "description": "Unix (macOS/Linux/Git Bash/WSL) 단계별로 설명해줄게요."},
         {"label": "Windows PowerShell snippet 보여줘요", "description": "Windows native (PowerShell 5.1+) wiring snippet 을 stdout 으로 보여줄게요."},
         {"label": "이 repo 만 켤래요 (project scope, dotfiles 비추천)", "description": "현재 프로젝트의 .claude/settings.json 에 paste 할 snippet 을 출력해요. user-global statusLine (예: OMC HUD) 을 이 repo 에서만 override 해요. ${HOME} 절대경로라 dotfiles repo / dev container 에 commit 시 다른 머신에서 깨져요. .gitignore 필수예요. plugin uninstall 시 orphan stub 유지 → graceful exit 0."},
         {"label": "나중에 할래요", "description": "지금은 건너뛰고 나중에 활성화해요."}
       ]
     }]
   }
   ```

2. **선택지별 처리.**

   **`복사해서 붙여 넣을래요 (Unix bash)` 선택 시:**

   v0.5.13 부터 `axhub-helpers settings-merge --apply` 가 atomic 으로 `~/.claude/settings.json` 에 statusLine 을 추가해요. 7-branch 결정 + .bak rollback + flock 으로 safe.

   ```bash
   # 자동 wire (recommended, Unix bash)
   "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers" settings-merge --apply --scope auto
   ```

   Exit code 별 처리:
   - 0 (NoOp): 이미 axhub-managed statusLine 있어요 — 변경 없음
   - 2 (Created): settings.json 만들고 statusLine 추가했어요
   - 3 (Merged): 기존 settings.json 에 statusLine 추가했어요
   - 4 (PreservedOther): 다른 plugin 의 statusLine 발견 — preserve 했어요. 강제 override 는 `/axhub:enable-statusline` 재실행
   - 5 (InvalidJson): settings.json 파싱 안 돼요 — 직접 수정 후 재시도
   - 6/7 (PartialSchema/Permission): stderr 안내 따라 수동 해결

   성공 시 "Claude Code 재시작해주세요" 안내.

   **`복사해서 붙여 넣을래요 (Windows PowerShell)` 선택 시:**

   Unix 분기와 동일한 atomic merge 호출이지만 PowerShell 호환 형식을 사용해요. v0.6.2 의 bash 형식 명령이 PowerShell 에서 `Unexpected token 'settings-merge'` parser error 로 fail 하던 회귀를 막아요.

   ```powershell
   # 자동 wire (recommended, Windows PowerShell 5.1+)
   & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" settings-merge --apply --scope auto
   ```

   Exit code 처리는 Unix 분기와 동일해요 (0/2/3/4/5/6/7).

   성공 시 "Claude Code 재시작해주세요" 안내.

   **verify (v0.6.2+):** 성공 후 `~/.claude/settings.json` 의 `statusLine.command` 는 orphan stub 절대경로예요 — macOS/Linux: `~/.local/state/axhub-plugin/orphan-stub-statusline.sh`, Windows: `%LOCALAPPDATA%\axhub-plugin\orphan-stub-statusline.ps1`. `${CLAUDE_PLUGIN_ROOT}` 미확장 리터럴이 아닌 절대경로라서 plugin 버전 변경 / 다른 plugin 활성 시에도 statusline 이 깨지지 않아요.

   **Manual paste fallback** (자동 wire 거부하는 경우):

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

   **`어떻게 하는지 보여줘요` 선택 시:**

   아래 4 단계로 안내해요:

   1. `~/.claude/settings.json` 을 텍스트 에디터로 열어요. 파일이 없으면 빈 파일로 만들어도 돼요.
   2. 위 Unix wiring snippet JSON 블록을 `settings.json` 최상위 객체 안에 paste 해요.
   3. JSON 의 닫는 `}` 가 잘 맞는지 확인해요.
   4. Claude Code 를 재시작하면 statusline 이 보여요.

   **`Windows PowerShell snippet 보여줘요` 선택 시:**

   clipboard 조작 없이 stdout 으로 Windows wiring snippet 을 그대로 출력해요:

   ```
   ~/.claude/settings.json 에 추가할 Windows native snippet 이에요:

   {
     "statusLine": {
       "type": "command",
       "command": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1\""
     }
   }

   settings.json 에 paste 하고 Claude Code 를 재시작하면 statusline 이 보여요.
   ```

   **`이 repo 만 켤래요 (project scope, dotfiles 비추천)` 선택 시:**

   user-global `~/.claude/settings.json` 의 statusLine (예: OMC HUD) 을 이 repo 에서만 override 해요. Claude Code precedence 가 project `.claude/settings.json` > user 라 그 디렉토리 진입 시만 axhub statusline 보이고, 다른 디렉토리는 user-global statusLine 유지돼요.

   ⚠️ **commit 위험 안내:** project `.claude/settings.json` 에 paste 한 path 는 사용자 시스템의 `$HOME` 포함 절대경로예요.
   - dotfiles repo / dev container 로 commit 하면 다른 머신 (`$HOME` 다름) 에서 statusline 깨져요.
   - `.gitignore` 에 `.claude/settings.json` 추가 강력 권장이에요. axhub repo 자체는 이미 gitignored.
   - 단일 머신 단일 사용자 사용은 안전해요.

   **lifecycle:** plugin uninstall 시에도 orphan stub 은 `state_dir` 거주라 유지돼요 — statusline 은 graceful exit 0 (빈 출력) 해요. dangling reference 안 만들어요.

   사용자 시스템에서 orphan stub absolute path 를 detect 해서 snippet 에 inject 한 뒤 stdout 으로 출력해요.

   **Unix (macOS / Linux / Git Bash / WSL) detect logic:**

   ```bash
   ORPHAN="${XDG_STATE_HOME:-${HOME}/.local/state}/axhub-plugin/orphan-stub-statusline.sh"
   if [ -x "$ORPHAN" ]; then
     PATH_ABS="$ORPHAN"                                # v0.5.13+ orphan stub (recommended, plugin version-agnostic)
   else
     PATH_ABS="${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"  # fallback (plugin cache, version-specific)
   fi
   ```

   Unix snippet (예시 — `$PATH_ABS` literal 절대경로 inject 후):

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "/Users/<you>/.local/state/axhub-plugin/orphan-stub-statusline.sh"
     }
   }
   ```

   **Windows native (PowerShell 5.1+) detect logic:**

   ```powershell
   $orphan = "$env:LOCALAPPDATA\axhub-plugin\orphan-stub-statusline.ps1"
   if (Test-Path $orphan) {
     $pathAbs = $orphan                                            # v0.5.13+ orphan stub
   } else {
     $pathAbs = "$env:CLAUDE_PLUGIN_ROOT\bin\statusline.ps1"       # fallback
   }
   ```

   Windows snippet (예시 — `$pathAbs` literal 절대경로 inject 후):

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"C:\\Users\\<you>\\AppData\\Local\\axhub-plugin\\orphan-stub-statusline.ps1\""
     }
   }
   ```

   **4 단계 paste 절차:**

   1. 현재 repo 의 `.claude/settings.json` 을 텍스트 에디터로 열어요. 파일이 없으면 빈 파일로 만들어도 돼요.
   2. 위 platform-specific snippet JSON 블록을 최상위 객체 안에 paste 해요. command 의 절대경로는 stdout 출력값 그대로 써요.
   3. JSON 의 닫는 `}` 가 잘 맞는지 확인해요. `.gitignore` 에 `.claude/settings.json` 추가했는지 다시 확인해요.
   4. Claude Code 를 재시작하면 이 repo 진입 시만 axhub statusline 이 보이고, 다른 repo 는 user-global statusLine (OMC HUD 등) 유지돼요.

   **`나중에 할래요` 선택 시:**

   조용히 exit 0 해요. 나중에 `/axhub:enable-statusline` 으로 다시 불러요.

## NEVER

- NEVER `~/.claude/settings.json` 을 explicit consent (install-time disclosure 동의) 없이 수정해요. install-time disclosure 가 표시됐고 `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 미설정이면 동의로 간주해요. orphan stub 이 uninstall 시 graceful degradation 을 보장해요. manual `axhub-helpers settings-merge --apply` 명령은 explicit consent 후 OK. SessionStart autowire (`session-start-autowire.sh`) 는 disclosure marker 존재 시에만 자동 실행해요 — 별도 prompt 없음.
- NEVER 비대화형 환경에서 pbcopy / clip.exe / xclip 호출. clipboard mutation 은 interactive 선택 후에만 해요.
- NEVER Claude Code 를 자동으로 재시작해요. 사용자가 직접 해야 해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `bin/statusline.sh` — wiring snippet 정본 소스. snippet 변경 시 `scripts/codegen-statusline-snippet.ts --write` 실행해요.

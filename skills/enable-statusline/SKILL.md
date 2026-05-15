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

   AskUserQuestion 의 `options` 가 max 4 개 라 platform 별 옵션을 하나로 통합했어요. 사용자의 OS 는 명시적으로 묻지 않아요. Step 2 본문에서 platform 별 명령을 모두 제시하고 LLM 이 user prompt context (예: "PowerShell", "Windows" 단서) 로 적합한 명령을 골라요. bash 실행 / `process.platform` 런타임 탐지 금지.

   ```json
   {
     "questions": [{
       "question": "statusLine 어떻게 켤래요?",
       "header": "활성화",
       "multiSelect": false,
       "options": [
         {"label": "자동으로 켜요", "description": "axhub-helpers settings-merge --apply 를 호출해서 ~/.claude/settings.json 에 atomic 으로 statusLine 추가해요. Unix bash / Windows PowerShell 자동 분기."},
         {"label": "복사할 snippet 보여줘요", "description": "wiring snippet 을 stdout 으로 출력해요. ~/.claude/settings.json 에 직접 paste 해요. Unix + Windows 양쪽 snippet 제공."},
         {"label": "이 repo 만 켤래요 (project scope, dotfiles 비추천)", "description": "현재 프로젝트의 .claude/settings.json 에 paste 할 snippet 을 출력해요. user-global statusLine (예: OMC HUD) 을 이 repo 에서만 override 해요. ${HOME} 절대경로라 dotfiles repo / dev container 에 commit 시 다른 머신에서 깨져요. .gitignore 필수예요. plugin uninstall 시 orphan stub 유지 → graceful exit 0."},
         {"label": "나중에 할래요", "description": "지금은 건너뛰고 나중에 활성화해요."}
       ]
     }]
   }
   ```

2. **선택지별 처리.**

   **`자동으로 켜요` 선택 시:**

   v0.5.13 부터 `axhub-helpers settings-merge --apply` 가 atomic 으로 `~/.claude/settings.json` 에 statusLine 을 추가해요. 7-branch 결정 + .bak rollback + flock 으로 safe.

   User context 의 platform 단서로 적합한 명령을 골라서 실행해요. "PowerShell", "Windows", "cmd" 같은 단서 보이면 PowerShell 분기, 그 외 (macOS / Linux / Git Bash / WSL) 는 Unix 분기.

   ```bash
   # Unix bash (macOS / Linux / Git Bash / WSL)
   "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers" settings-merge --apply --scope auto
   ```

   ```powershell
   # Windows PowerShell 5.1+ (cmd.exe 직접 호출 시에는 axhub-helpers.exe 부터 시작)
   & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" settings-merge --apply --scope auto
   ```

   Exit code 별 처리 (양 platform 공통):
   - 0 (NoOp): 이미 axhub-managed statusLine 있어요 — 변경 없음
   - 2 (Created): settings.json 만들고 statusLine 추가했어요
   - 3 (Merged): 기존 settings.json 에 statusLine 추가했어요
   - 4 (PreservedOther): 다른 plugin 의 statusLine 발견 — preserve 했어요. 강제 override 는 `/axhub:enable-statusline` 재실행
   - 5 (InvalidJson): settings.json 파싱 안 돼요 — 직접 수정 후 재시도
   - 6/7 (PartialSchema/Permission): stderr 안내 따라 수동 해결

   성공 시 "Claude Code 재시작해주세요" 안내.

   **verify (v0.6.2+):** 성공 후 `~/.claude/settings.json` 의 `statusLine.command` 는 orphan stub 절대경로예요 — macOS/Linux: `~/.local/state/axhub-plugin/orphan-stub-statusline.sh`, Windows: `%LOCALAPPDATA%\axhub-plugin\orphan-stub-statusline.ps1`. `${CLAUDE_PLUGIN_ROOT}` 미확장 리터럴이 아닌 절대경로라서 plugin 버전 변경 / 다른 plugin 활성 시에도 statusline 이 깨지지 않아요.

   **`복사할 snippet 보여줘요` 선택 시:**

   wiring snippet 을 stdout 으로 출력해요. 사용자가 `~/.claude/settings.json` 에 직접 paste 해요. Unix + Windows 양쪽 snippet 모두 보여주고, 사용자 환경에 맞는 것을 paste 하라고 안내해요.

   **Unix (macOS / Linux / Git Bash / WSL):**

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"
     }
   }
   ```

   **Windows native (PowerShell 5.1+):**

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1\""
     }
   }
   ```

   **4 단계 paste 절차:**

   1. `~/.claude/settings.json` 을 텍스트 에디터로 열어요. 파일이 없으면 빈 파일로 만들어도 돼요.
   2. 위 platform 에 맞는 JSON 블록을 최상위 객체 안에 paste 해요.
   3. JSON 의 닫는 `}` 가 잘 맞는지 확인해요.
   4. Claude Code 를 재시작하면 statusline 이 보여요.

   **Clipboard 자동 복사 (interactive 환경 전용, D1 TTY guard 후):**

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

   **`이 repo 만 켤래요 (project scope, dotfiles 비추천)` 선택 시:**

   user-global `~/.claude/settings.json` 의 statusLine (예: OMC HUD) 을 이 repo 에서만 override 해요. Claude Code precedence 가 project `.claude/settings.json` > user 라 그 디렉토리 진입 시만 axhub statusline 보이고, 다른 디렉토리는 user-global statusLine 유지돼요.

   `axhub-helpers settings-merge --apply --scope project` 가 atomic 으로 project `.claude/settings.json` 에 statusLine 추가해요. user scope autowire 와 동일한 7-branch 결정 + .bak rollback + flock 으로 safe.

   ⚠️ **commit 위험 안내:** project `.claude/settings.json` 에 추가되는 `statusLine.command` 는 사용자 시스템의 `$HOME` 포함 절대경로예요.
   - dotfiles repo / dev container 로 commit 하면 다른 머신 (`$HOME` 다름) 에서 statusline 깨져요.
   - `.gitignore` 에 `.claude/settings.json` 추가 강력 권장이에요. axhub repo 자체는 이미 gitignored.
   - 단일 머신 단일 사용자 사용은 안전해요.

   **lifecycle:** plugin uninstall 시에도 orphan stub 은 `state_dir` 거주라 유지돼요 — statusline 은 graceful exit 0 (빈 출력) 해요. dangling reference 안 만들어요.

   ```bash
   # 자동 wire (Unix bash — macOS / Linux / Git Bash / WSL)
   "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers" settings-merge --apply --scope project
   ```

   ```powershell
   # 자동 wire (Windows PowerShell 5.1+)
   & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" settings-merge --apply --scope project
   ```

   Exit code 별 처리:
   - 0 (NoOp): 이미 axhub-managed statusLine 있어요 — 변경 없음
   - 2 (Created): project `.claude/settings.json` 만들고 statusLine 추가했어요
   - 3 (Merged): 기존 project `.claude/settings.json` 에 statusLine 추가했어요
   - 4 (PreservedOther): 다른 plugin 의 statusLine 발견 — preserve 했어요. 강제 override 는 user 가 직접 project `.claude/settings.json` 편집
   - 5 (InvalidJson): settings.json 파싱 안 돼요 — 직접 수정 후 재시도
   - 6/7 (PartialSchema/Permission): stderr 안내 따라 수동 해결

   성공 시 "Claude Code 재시작해주세요" + ".gitignore 확인해주세요" 안내.

   **Edge cases:**
   - **non-git repo:** `axhub-helpers` 가 `git rev-parse --show-toplevel` 호출 시 fail — stderr 에 안내 후 manual paste fallback (Unix snippet / Windows snippet 직접 출력) 으로 전환.
   - **sub-dir invoke:** `git rev-parse --show-toplevel` 가 repo root 반환해서 root 의 `.claude/settings.json` 에 wire — sub-dir 가 아닌 repo root 라는 점 명시.
   - **Windows helper.exe 미존재:** plugin 이 helper binary 자동 다운로드 안 된 경우 (`%CLAUDE_PLUGIN_ROOT%\bin\axhub-helpers.exe` 부재). 명령 spawn fail — 사용자에게 안내 후 위 manual paste fallback (Windows snippet) 으로 전환.

   **Manual paste fallback (autowire 실패 또는 helper 미존재 시):**

   사용자 시스템의 orphan stub absolute path 를 detect 해서 snippet 에 inject 한 뒤 stdout 으로 출력해요. 사용자가 직접 project `.claude/settings.json` 에 paste 해요.

   Unix snippet (예시):

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "/Users/<you>/.local/state/axhub-plugin/orphan-stub-statusline.sh"
     }
   }
   ```

   Windows snippet (예시):

   ```json
   {
     "statusLine": {
       "type": "command",
       "command": "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"C:\\Users\\<you>\\AppData\\Local\\axhub-plugin\\orphan-stub-statusline.ps1\""
     }
   }
   ```

   **`나중에 할래요` 선택 시:**

   조용히 exit 0 해요. 나중에 `/axhub:enable-statusline` 으로 다시 불러요.

## NEVER

- NEVER `~/.claude/settings.json` 을 explicit consent (install-time disclosure 동의) 없이 수정해요. install-time disclosure 가 표시됐고 `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 미설정이면 동의로 간주해요. orphan stub 이 uninstall 시 graceful degradation 을 보장해요. manual `axhub-helpers settings-merge --apply` 명령은 explicit consent 후 OK. SessionStart autowire (`session-start-autowire.sh`) 는 disclosure marker 존재 시에만 자동 실행해요 — 별도 prompt 없음.
- NEVER 비대화형 환경에서 pbcopy / clip.exe / xclip 호출. clipboard mutation 은 interactive 선택 후에만 해요.
- NEVER Claude Code 를 자동으로 재시작해요. 사용자가 직접 해야 해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `bin/statusline.sh` — wiring snippet 정본 소스. snippet 변경 시 `scripts/codegen-statusline-snippet.ts --write` 실행해요.

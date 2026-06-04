---
name: install-cli
description: '이 스킬은 사용자가 axhub CLI 가 설치되지 않은 상태에서 자동 설치를 요청할 때 사용해요. 다음 표현에서 활성화: "자동 설치", "ax-hub-cli 설치", "axhub 설치", "axhub cli 설치", "axhub CLI 설치", "cli 설치", "CLI 설치", "auto install", "axhub install", "install axhub", "install cli", 또는 axhub CLI 부재 상태에서 doctor / deploy 가 routing 한 install 의도. OS 감지 후 공식 installer (curl install.sh / irm install.ps1) 로 설치하고 axhub --version 으로 검증해요. Homebrew/Scoop 등 패키지 매니저 채널은 미지원.'
examples:
  - utterance: "자동 설치"
    intent: "install axhub CLI binary"
  - utterance: "ax-hub-cli 설치"
    intent: "install axhub CLI binary"
  - utterance: "auto install"
    intent: "install axhub CLI binary"
  - utterance: "axhub install"
    intent: "install axhub CLI binary"
  - utterance: "axhub 설치"
    intent: "install axhub CLI binary"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: sonnet
---

# Install CLI (ax-hub-cli auto-installer)

axhub CLI 가 설치되지 않았을 때 공식 installer (`cli.axhub.ai`) 로 자동 설치해요. Homebrew/Scoop 등 패키지 매니저 채널은 사용하지 않아요 — 공식 installer 단일 채널이에요 (패키지 매니저 채널은 갱신이 늦어 구버전 client_id 로 로그인 실패 위험). 설치 후 `axhub --version` 으로 검증하고 다음 단계 (`로그인해줘`) 로 안내해요.

## Claude Desktop natural-language path

For ordinary Desktop prompts like `axhub CLI 설치해줘`, `axhub 설치해줘`, or `cli 설치해줘`, start with the safe installed-or-missing check before any installer work.

- First visible sentence, exactly: `설치 상태를 확인할게요.`
- Use exactly one Bash tool call first.
- Bash title/description, exactly: `설치 상태 확인`
- Bash command:

  ```bash
  axhub-helpers install-summary --user-utterance "<latest user sentence>"
  ```

If stdout says the CLI is already installed, copy that Korean stdout as the answer and stop. Do not run installer commands, do not show installer commands, and do not continue into OS/channel selection.

If stdout says the CLI is missing, copy the Korean stdout and ask for a normal chat approval (`진행` or `취소`) before any installer command. Actual installer execution remains destructive and requires explicit approval.

Do not write route labels, slash commands, skill names, `preflight`, `auth status`, raw JSON fields, installer URLs, filesystem paths, English tool-title fragments, or internal workflow labels in visible user-facing text.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To install ax-hub-cli on the user's host:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start:

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "OS 감지",            status: "in_progress", activeForm: "OS 보는 중" },
     { content: "설치 채널 선택",     status: "pending",     activeForm: "채널 묻는 중" },
     { content: "installer 실행",     status: "pending",     activeForm: "설치 진행 중" },
     { content: "axhub --version 검증", status: "pending",   activeForm: "버전 보는 중" },
     { content: "다음 단계 안내",     status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

1. **Detect OS.**

   ```bash
   uname -s   # Darwin | Linux
   ```

   ```powershell
   $env:OS    # Windows_NT
   ```

   세 분기:
   - `Darwin` → macOS
   - `Linux` → Linux
   - `Windows_NT` → Windows

2. **Check pre-existing install (race-safe).** 사용자가 다른 터미널에서 이미 설치했을 수 있으니 `axhub --version` 한 번 더 확인. exit 0 이면 Step 6 으로 skip.

3. **Channel pick — AskUserQuestion.**

   **Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 설치 채널 → `manual` (subprocess 에서 자동 install 안 해요, 안내만).

   axhub 는 공식 installer (`cli.axhub.ai`) **단일 채널**로만 배포해요. Homebrew/Scoop
   같은 패키지 매니저 채널은 사용하지 않아요 (구버전 client_id 로 로그인 실패 위험). 아래는
   "자동 실행 vs 수동 안내" 확인용이에요.

   macOS / Linux 옵션:

   ```json
   {
     "questions": [{
       "question": "axhub CLI 를 공식 installer 로 설치할까요?",
       "header": "설치 방법",
       "multiSelect": false,
       "options": [
         {"label": "1. 공식 installer 자동 실행 (Recommended)", "description": "curl https://cli.axhub.ai/install.sh | bash"},
         {"label": "2. 수동 안내", "description": "설치 명령어만 보고 직접 실행"}
       ]
     }]
   }
   ```

   Windows 옵션:

   ```json
   {
     "questions": [{
       "question": "axhub CLI 를 공식 installer 로 설치할까요?",
       "header": "설치 방법",
       "multiSelect": false,
       "options": [
         {"label": "1. 공식 installer 자동 실행 (Recommended)", "description": "irm https://cli.axhub.ai/install.ps1 | iex"},
         {"label": "2. 수동 안내", "description": "설치 명령어만 보고 직접 실행"}
       ]
     }]
   }
   ```

4. **Run installer (consent 받은 후).** 사용자가 직접 실행하도록 명령어를 안내하거나 (`! ` prefix), 또는 SKILL 이 Bash tool 로 실행. 두 흐름 모두 사용자가 의식적으로 confirm 한 후만.

   - macOS / Linux 공식: `! curl -fsSL https://cli.axhub.ai/install.sh | bash`
   - Windows 공식: `! powershell -NoProfile -Command "irm https://cli.axhub.ai/install.ps1 | iex"`
   - 수동 안내: 위 명령어 중 적절한 것을 출력만 하고 종료. 사용자에게 직접 실행 후 "설치 끝났어" 라고 말해달라고 안내.

5. **Post-install verify.**

   ```bash
   axhub --version
   ```

   exit 0 + version semver 출력이면 성공. exit 0 인데 version 비어있거나 "command not found" 면 PATH 등록 누락 — 사용자에게 새 터미널 열거나 shell rc 재로드 안내.

6. **Render success card (해요체).**

   ```
   axhub CLI 설치 완료:
     ✓ 채널: <selected channel>
     ✓ 버전: <CLI_VERSION>
     ✓ 경로: <which axhub 또는 Get-Command axhub>

   다음 단계: '로그인해줘' 라고 말씀해주세요.
   ```

   설치 실패 (post-install verify fail) 시:

   ```
   axhub CLI 설치 안 됐어요:
     ✗ installer exit code: <CODE>
     ✗ axhub --version: <stderr>

   해결: 새 터미널에서 직접 재시도해주세요 → <command>
   또는: '진단해줘' 라고 말씀해주시면 환경 점검 다시 해요.
   ```

## NEVER

- NEVER subprocess (`claude -p` / CI / headless) 에서 자동 installer 실행. 반드시 D1 guard 로 manual fallback.
- NEVER `cli.axhub.ai` 외 다른 도메인의 install script 실행. supply chain 신뢰 채널 한정.
- NEVER pre-existing CLI 덮어쓰기. Step 2 race check 필수.
- NEVER 설치 도중 helper preflight 호출 — CLI 부재 상태로 fire 되므로 무한 루프.
- NEVER Homebrew / Scoop 등 패키지 매니저 채널로 설치 안내. axhub 는 공식 installer (`cli.axhub.ai`) 단일 채널만 지원해요 — 패키지 매니저 채널은 갱신 지연으로 구버전 client_id 가 배포돼 로그인이 `invalid_client` 로 실패할 수 있어요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `https://docs.jocodingax.ai/llms-cli.txt` — ax-hub-cli 공식 install 채널 source of truth.

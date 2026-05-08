---
name: install-cli
description: '이 스킬은 사용자가 axhub CLI 가 설치되지 않은 상태에서 자동 설치를 요청할 때 사용해요. 다음 표현에서 활성화: "자동 설치", "ax-hub-cli 설치", "axhub 설치", "axhub cli 설치", "axhub CLI 설치", "cli 설치", "CLI 설치", "auto install", "axhub install", "install axhub", "install cli", 또는 axhub CLI 부재 상태에서 doctor / deploy 가 routing 한 install 의도. OS 감지 후 공식 채널 (curl install.sh / irm install.ps1 / Homebrew / Scoop) 중 하나로 설치하고 axhub --version 으로 검증해요.'
multi-step: true
needs-preflight: false
allows-dependency-execution: false
---

# Install CLI (ax-hub-cli auto-installer)

axhub CLI 가 설치되지 않았을 때 OS 별 공식 채널로 자동 설치해요. 설치 후 `axhub --version` 으로 검증하고 다음 단계 (`로그인해줘`) 로 안내해요.

## Workflow

To install ax-hub-cli on the user's host:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start:

   ```typescript
   TodoWrite({ todos: [
     { content: "OS 감지",            status: "in_progress", activeForm: "OS 보는 중" },
     { content: "설치 채널 선택",     status: "pending",     activeForm: "채널 묻는 중" },
     { content: "installer 실행",     status: "pending",     activeForm: "설치 진행 중" },
     { content: "axhub --version 검증", status: "pending",   activeForm: "버전 보는 중" },
     { content: "다음 단계 안내",     status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

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

   macOS 옵션:

   ```json
   {
     "questions": [{
       "question": "axhub CLI 어떤 채널로 설치해요?",
       "header": "설치 채널",
       "multiSelect": false,
       "options": [
         {"label": "1. 공식 installer (Recommended)", "description": "curl https://cli.jocodingax.ai/install.sh | bash"},
         {"label": "2. Homebrew", "description": "brew install jocoding-ax-partners/tap/axhub"},
         {"label": "3. 수동 안내", "description": "설치 명령어만 보고 직접 실행"}
       ]
     }]
   }
   ```

   Windows 옵션:

   ```json
   {
     "questions": [{
       "question": "axhub CLI 어떤 채널로 설치해요?",
       "header": "설치 채널",
       "multiSelect": false,
       "options": [
         {"label": "1. 공식 installer (Recommended)", "description": "irm https://cli.jocodingax.ai/install.ps1 | iex"},
         {"label": "2. Scoop", "description": "scoop bucket add jocoding-ax-partners + scoop install axhub"},
         {"label": "3. 수동 안내", "description": "설치 명령어만 보고 직접 실행"}
       ]
     }]
   }
   ```

   Linux 옵션 (macOS 와 동일하되 Homebrew 대신 official installer 만):

   ```json
   {
     "questions": [{
       "question": "axhub CLI 어떤 채널로 설치해요?",
       "header": "설치 채널",
       "multiSelect": false,
       "options": [
         {"label": "1. 공식 installer (Recommended)", "description": "curl https://cli.jocodingax.ai/install.sh | bash"},
         {"label": "2. 수동 안내", "description": "설치 명령어만 보고 직접 실행"}
       ]
     }]
   }
   ```

3.5. **Validate package manager presence (brew/scoop 선택했을 때만).** 패키지 매니저 옵션을 골랐을 경우, 실행 직전에 해당 매니저가 설치돼 있는지 확인.

    - Homebrew 선택 시:
      ```bash
      command -v brew >/dev/null 2>&1 || echo "brew-missing"
      ```
    - Scoop 선택 시:
      ```powershell
      Get-Command scoop -ErrorAction SilentlyContinue
      ```

    매니저 부재 시 사용자에게 한국어로 안내하고 공식 installer 로 자동 전환 제안. 두 옵션 제시:

    macOS (brew 부재):

    ```
    brew 가 설치되어 있지 않아요. 두 가지 방법 중 골라주세요:
      1. 공식 installer 로 axhub 설치 (Recommended) — brew 없이도 가능
         (curl -fsSL https://cli.jocodingax.ai/install.sh | bash)
      2. brew 먼저 설치 후 axhub — https://brew.sh 안내
    ```

    Windows (scoop 부재):

    ```
    scoop 이 설치되어 있지 않아요. 두 가지 방법 중 골라주세요:
      1. 공식 installer 로 axhub 설치 (Recommended) — scoop 없이도 가능
         (irm https://cli.jocodingax.ai/install.ps1 | iex)
      2. scoop 먼저 설치 후 axhub — https://scoop.sh 안내
    ```

    AskUserQuestion 으로 처리하거나, 사용자가 즉시 답한 의도가 명확하면 (예: "brew 깔게요" / "공식꺼") 그대로 따름. 매니저 설치 책임은 사용자에게 — SKILL 이 brew / scoop 자체를 자동 설치하지 않아요 (supply-chain scope 위반).

4. **Run installer (consent 받은 후).** 사용자가 직접 실행하도록 명령어를 안내하거나 (`! ` prefix), 또는 SKILL 이 Bash tool 로 실행. 두 흐름 모두 사용자가 의식적으로 confirm 한 후만.

   - macOS / Linux 공식: `! curl -fsSL https://cli.jocodingax.ai/install.sh | bash`
   - Homebrew: `! brew install jocoding-ax-partners/tap/axhub`
   - Windows 공식: `! powershell -NoProfile -Command "irm https://cli.jocodingax.ai/install.ps1 | iex"`
   - Scoop: `! powershell -NoProfile -Command "scoop bucket add jocoding-ax-partners https://github.com/jocoding-ax-partners/scoop-bucket; scoop install axhub"`
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
- NEVER `cli.jocodingax.ai` 외 다른 도메인의 install script 실행. supply chain 신뢰 채널 한정.
- NEVER pre-existing CLI 덮어쓰기. Step 2 race check 필수.
- NEVER 설치 도중 helper preflight 호출 — CLI 부재 상태로 fire 되므로 무한 루프.
- NEVER brew / scoop 자체를 자동 설치. 패키지 매니저 부재 시 공식 installer 로 전환하거나 사용자가 직접 패키지 매니저 설치하도록 안내. supply-chain scope 한정.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `https://docs.jocodingax.ai/llms-cli.txt` — ax-hub-cli 공식 install 채널 source of truth.

---
name: onboarding
description: '이 스킬은 axhub 를 처음 쓰는 사람이 셋업/온보딩 전체 과정을 한 번에 진행하고 싶어할 때 사용해요. 다음 표현에서 활성화: "셋업해줘", "셋업 해줘", "처음인데", "처음 사용", "처음 써", "처음 쓰는데", "처음 쓰는데 뭐부터", "뭐부터 하면 돼", "뭐부터 하면 되나요", "어떻게 시작하면 돼", "어떻게 시작해", "온보딩", "온보딩해줘", "시작하기", "axhub 시작", "axhub 처음", "초기 셋업", "setup", "set up", "onboard", "onboarding", "getting started", "get started", "first time", 또는 첫 사용자 셋업 의도. axhub CLI 설치(install-cli)·로그인(auth)·node 환경 감지를 순서대로 안내하고, node 가 없으면 consent 후 설치해요. 끝나면 첫 앱 만들기(init)로 연결해요. 환경 진단(doctor)이나 새 앱 초기화(init)와 달리 처음 사용자의 순차 온보딩을 담당해요.'
examples:
  - utterance: "셋업해줘"
    intent: "onboard axhub first-time onboarding"
  - utterance: "처음인데 어떻게 시작해"
    intent: "onboard axhub first-time onboarding"
  - utterance: "axhub 처음 쓰는데 뭐부터 하면 돼?"
    intent: "onboard axhub first-time onboarding"
  - utterance: "온보딩"
    intent: "onboard axhub first-time onboarding"
  - utterance: "getting started"
    intent: "onboard axhub first-time onboarding"
  - utterance: "set up axhub"
    intent: "onboard axhub first-time onboarding"
  - utterance: "first time using axhub"
    intent: "onboard axhub first-time onboarding"
multi-step: true
needs-preflight: false
allows-dependency-execution: true
model: sonnet
---

# Onboarding (first-run vibe coding orchestrator)

Frontmatter `description` 은 nl-lexicon trigger baseline 때문에 보수적으로 유지해요. 실제 온보딩 범위와
안전 계약은 이 본문을 authoritative source 로 봐요.

처음 axhub 를 쓰는 사람을 위한 **온보딩 단일 진입점**이에요. 사용자는 `온보딩`, `처음인데 뭐부터`,
`getting started` 한 마디만 하면 돼요. 내부에서는 기존 skill
(`install-cli`/`update`/`upgrade`/`auth`/`github`/`init`/`doctor`/`deploy`)을 위임하지만,
사용자는 sibling skill 이름이나 slash command 를 몰라도 온보딩을 끝낼 수 있어요.

onboarding 의 제품 계약은 `detect-first → 첫 gap 처리 → 재감지` 루프예요. 안전하게 자동화할 수 있는 gap 은
끝까지 닫고, 브라우저 승인·OS installer GUI·PATH reload 처럼 에이전트가 대신 완료할 수 없는 gap 은
`READY_WITH_USER_ACTION` 카드와 자연어 재개 phrase(`승인했어`, `온보딩 계속`, `다시 온보딩해줘`)를 남겨요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `승인했어`, `온보딩 계속`, `다시 로그인해줘`, `배포해`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "도구 확인",       status: "in_progress", activeForm: "도구 보는 중" },
     { content: "로그인 확인",     status: "pending",     activeForm: "로그인 보는 중" },
     { content: "런타임 확인",     status: "pending",     activeForm: "런타임 보는 중" },
     { content: "GitHub 연결",     status: "pending",     activeForm: "GitHub 보는 중" },
     { content: "앱·repo 준비",    status: "pending",     activeForm: "앱 준비 중" },
     { content: "의존성 확인",     status: "pending",     activeForm: "의존성 보는 중" },
     { content: "최종 점검",       status: "pending",     activeForm: "마무리 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **Non-interactive AskUserQuestion guard (D1).**

   이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 `tests/fixtures/ask-defaults/registry.json` 의 안전 기본값으로 진행해요. 이 모드의 최종 상태는 `SAFE_STOP_NONINTERACTIVE`예요. install/update/auth/init/deploy/deps mutation 과 git/node system install 또는 version switch 는 자동 실행하지 않아요.

2. **DETECT_ALL(read-only) — 모든 gap 을 먼저 봐요.**

   preflight 를 CLI 확인 전에 부르지 말아요. CLI 가 아직 없을 수 있어요.

   Unix / Git Bash:

   ```bash
   MIN_AXHUB_CLI_VERSION="0.17.3"
   AXHUB_CLI_VERSION="$(axhub --version 2>/dev/null || true)"
   test -n "$AXHUB_CLI_VERSION" || echo "cli_missing"
   test -z "$AXHUB_CLI_VERSION" || axhub update check --json 2>/dev/null || echo "cli_update_check_unavailable"
   git --version 2>/dev/null || echo "git_missing"
   node --version 2>/dev/null || echo "node_missing"
   test -f axhub.yaml || test -f apphub.yaml || echo "manifest_missing"
   git rev-parse --is-inside-work-tree 2>/dev/null || echo "git_repo_missing"
   git rev-parse --verify HEAD 2>/dev/null || echo "git_commit_missing"
   ls bun.lockb bun.lock pnpm-lock.yaml package-lock.json yarn.lock 2>/dev/null || echo "lockfile_missing"
   test -d node_modules || { (test -f axhub.yaml || test -f apphub.yaml) && ls bun.lockb bun.lock pnpm-lock.yaml package-lock.json yarn.lock >/dev/null 2>&1 && echo "deps_missing"; }
   find . -mindepth 1 -maxdepth 1 ! -name .git ! -name node_modules -print -quit 2>/dev/null | grep -q . && echo "dir_non_empty" || echo "dir_empty"
   NODE_ACTIVE="$(node --version 2>/dev/null || true)"
   NODE_REQUIRED="$(cat .nvmrc 2>/dev/null || node -p "require('./package.json').engines?.node || ''" 2>/dev/null || true)"
   if [ -n "$NODE_ACTIVE" ] && [ -n "$NODE_REQUIRED" ]; then
     node -e 'const ver=s=>{const m=String(s||"").trim().replace(/^v/,"").match(/^(\d+)(?:\.(\d+))?(?:\.(\d+))?/);return m?[Number(m[1]),Number(m[2]||0),Number(m[3]||0)]:null};const len=s=>{const m=String(s||"").trim().replace(/^v/,"").match(/^(\d+(?:\.\d+){0,2})/);return m?m[1].split(".").length:0};const cmp=(a,b)=>{if(!a||!b)return NaN;for(let i=0;i<3;i++){if(a[i]!==b[i])return a[i]>b[i]?1:-1}return 0};const active=ver(process.argv[1]);const required=String(process.argv[2]||"").trim();let ok=false;if(!active||!required){ok=true}else if(/^\s*v?\d+(?:\.\d+){0,2}\s*$/.test(required)){const req=ver(required);const n=len(required);ok=cmp(active,req)>=0&&active.slice(0,n).every((v,i)=>v===req[i])}else{const tokens=[...required.matchAll(/(?:^|\s)(>=|>|<=|<|=)\s*v?(\d+(?:\.\d+){0,2})(?=\s|$)/g)];ok=tokens.length>0&&tokens.every(m=>{const c=cmp(active,ver(m[2]));return m[1]===">="?c>=0:m[1]===">"?c>0:m[1]==="<="?c<=0:m[1]==="<"?c<0:c===0})}process.exit(ok?0:1)' "$NODE_ACTIVE" "$NODE_REQUIRED" || echo "node_mismatch"
   fi
   ```

   Windows PowerShell:

   ```powershell
   $minAxhubCliVersion = "0.17.3"
   if (Get-Command axhub -ErrorAction SilentlyContinue) {
     axhub --version
     axhub update check --json 2>$null; if ($LASTEXITCODE -ne 0) { "cli_update_check_unavailable" }
   } else { "cli_missing" }
   if (Get-Command git -ErrorAction SilentlyContinue) { git --version } else { "git_missing" }
   if (Get-Command node -ErrorAction SilentlyContinue) { node --version } else { "node_missing" }
   if (-not ((Test-Path axhub.yaml) -or (Test-Path apphub.yaml))) { "manifest_missing" }
   git rev-parse --is-inside-work-tree 2>$null; if ($LASTEXITCODE -ne 0) { "git_repo_missing" }
   git rev-parse --verify HEAD 2>$null; if ($LASTEXITCODE -ne 0) { "git_commit_missing" }
   Get-ChildItem bun.lockb,bun.lock,pnpm-lock.yaml,package-lock.json,yarn.lock -ErrorAction SilentlyContinue
   if (-not (Test-Path node_modules) -and ((Test-Path axhub.yaml) -or (Test-Path apphub.yaml)) -and (Get-ChildItem bun.lockb,bun.lock,pnpm-lock.yaml,package-lock.json,yarn.lock -ErrorAction SilentlyContinue)) { "deps_missing" }
   $visible = Get-ChildItem -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -notin @(".git", "node_modules") } | Select-Object -First 1
   if ($visible) { "dir_non_empty" } else { "dir_empty" }
   $nodeActive = if (Get-Command node -ErrorAction SilentlyContinue) { node --version } else { "" }
   $nodeRequired = if (Test-Path .nvmrc) { Get-Content .nvmrc -TotalCount 1 } else { node -p "require('./package.json').engines?.node || ''" 2>$null }
   if ($nodeActive -and $nodeRequired) {
     node -e "const ver=s=>{const m=String(s||'').trim().replace(/^v/,'').match(/^(\d+)(?:\.(\d+))?(?:\.(\d+))?/);return m?[Number(m[1]),Number(m[2]||0),Number(m[3]||0)]:null};const len=s=>{const m=String(s||'').trim().replace(/^v/,'').match(/^(\d+(?:\.\d+){0,2})/);return m?m[1].split('.').length:0};const cmp=(a,b)=>{if(!a||!b)return NaN;for(let i=0;i<3;i++){if(a[i]!==b[i])return a[i]>b[i]?1:-1}return 0};const active=ver(process.argv[1]);const required=String(process.argv[2]||'').trim();let ok=false;if(!active||!required){ok=true}else if(/^\s*v?\d+(?:\.\d+){0,2}\s*$/.test(required)){const req=ver(required);const n=len(required);ok=cmp(active,req)>=0&&active.slice(0,n).every((v,i)=>v===req[i])}else{const tokens=[...required.matchAll(/(?:^|\s)(>=|>|<=|<|=)\s*v?(\d+(?:\.\d+){0,2})(?=\s|$)/g)];ok=tokens.length>0&&tokens.every(m=>{const c=cmp(active,ver(m[2]));return m[1]==='>='?c>=0:m[1]==='>'?c>0:m[1]==='<='?c<=0:m[1]==='<'?c<0:c===0})}process.exit(ok?0:1)" $nodeActive $nodeRequired
     if ($LASTEXITCODE -ne 0) { "node_mismatch" }
   }
   ```

   CLI 가 확인된 뒤에만 auth/helper/GitHub 상태를 봐요.

   Unix / Git Bash:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" preflight --json
   # preflight JSON 의 cli_too_old/cli_too_new/in_range 도 cli_old 판단에 사용해요.
   GITHUB_ACCOUNTS_JSON="$(axhub github accounts list --json 2>/dev/null || true)"
   GITHUB_APP_STATE="$(printf '%s' "$GITHUB_ACCOUNTS_JSON" | node -e 'const fs=require("fs");const raw=fs.readFileSync(0,"utf8").trim();if(!raw){console.log("unknown");process.exit(0)}let j={};try{j=JSON.parse(raw)}catch{console.log("unknown");process.exit(0)}const rows=j.accounts||j.installations||j.data?.accounts||[];const installed=Array.isArray(rows)&&rows.some(a=>a&&(a.installed===true||a.installation_id||a.installationId));const url=j.install_url||j.installUrl||j.data?.install_url||j.data?.installUrl||"";if(installed)console.log("installed");else if(url)console.log("missing:"+url);else console.log("unknown")' 2>/dev/null || echo "unknown")"
   case "$GITHUB_APP_STATE" in
     installed) ;;
     missing:*) printf '%s\n' "${GITHUB_APP_STATE#missing:}"; echo "github_app_missing" ;;
     *) echo "github_app_unknown" ;;
   esac
   BOOTSTRAP_STATE=".axhub/bootstrap.state.json"
   APP_ID="$(node -p "const fs=require('fs');const p='$BOOTSTRAP_STATE';if(!fs.existsSync(p)) ''; else { const j=JSON.parse(fs.readFileSync(p,'utf8')); j.app_id || j.appId || '' }" 2>/dev/null || true)"
   DEPLOYMENT_ID="$(node -p "const fs=require('fs');const p='$BOOTSTRAP_STATE';if(!fs.existsSync(p)) ''; else { const j=JSON.parse(fs.readFileSync(p,'utf8')); j.last_deploy_id || j.deployment_id || j.deploymentId || '' }" 2>/dev/null || true)"
   if [ -n "$APP_ID" ] && [ -n "$DEPLOYMENT_ID" ]; then
     axhub deploy status "$DEPLOYMENT_ID" --app "$APP_ID" --watch --watch-timeout 1m --json 2>/dev/null \
       | node -e 'const fs=require("fs");let raw=fs.readFileSync(0,"utf8");let j={};try{j=JSON.parse(raw)}catch{process.exit(1)};let s=String(j.status||j.data?.status||j.deployment?.status||"").toLowerCase();process.exit(/succeeded|live|running|deployed/.test(s)?0:1)' \
       || echo "deploy_unverified"
   fi
   ```

   Windows PowerShell:

   ```powershell
   $helper = "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe"
   if (-not (Test-Path $helper)) { $helper = (Get-Command axhub-helpers.exe -ErrorAction SilentlyContinue).Source }
   if ($helper) { & $helper preflight --json }
   $githubAccountsJson = axhub github accounts list --json 2>$null
   $githubAppState = $githubAccountsJson | node -e "const fs=require('fs');const raw=fs.readFileSync(0,'utf8').trim();if(!raw){console.log('unknown');process.exit(0)}let j={};try{j=JSON.parse(raw)}catch{console.log('unknown');process.exit(0)}const rows=j.accounts||j.installations||j.data?.accounts||[];const installed=Array.isArray(rows)&&rows.some(a=>a&&(a.installed===true||a.installation_id||a.installationId));const url=j.install_url||j.installUrl||j.data?.install_url||j.data?.installUrl||'';if(installed)console.log('installed');else if(url)console.log('missing:'+url);else console.log('unknown')" 2>$null
   if (-not $githubAppState) { $githubAppState = "unknown" }
   if ($githubAppState -eq "installed") { }
   elseif ($githubAppState.StartsWith("missing:")) { $githubAppState.Substring(8); "github_app_missing" }
   else { "github_app_unknown" }
   $bootstrapState = ".axhub/bootstrap.state.json"
   if (Test-Path $bootstrapState) {
     $appId = node -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('$bootstrapState','utf8'));j.app_id||j.appId||''" 2>$null
     $deploymentId = node -p "const fs=require('fs');const j=JSON.parse(fs.readFileSync('$bootstrapState','utf8'));j.last_deploy_id||j.deployment_id||j.deploymentId||''" 2>$null
     if ($appId -and $deploymentId) {
       axhub deploy status $deploymentId --app $appId --watch --watch-timeout 1m --json 2>$null | node -e "const fs=require('fs');let raw=fs.readFileSync(0,'utf8');let j={};try{j=JSON.parse(raw)}catch{process.exit(1)};let s=String(j.status||j.data?.status||j.deployment?.status||'').toLowerCase();process.exit(/succeeded|live|running|deployed/.test(s)?0:1)"
       if ($LASTEXITCODE -ne 0) { "deploy_unverified" }
     }
   }
   ```

3. **Gap State Machine — 첫 gap 하나만 처리하고 재감지해요.**

   ```text
   START
     ↓
  DETECT_ALL(read-only)
     ├─ cli_missing         → Skill("axhub:install-cli") → DETECT_ALL
     ├─ cli_path_missing    → Skill("axhub:repair") → DETECT_ALL
     ├─ cli_old             → update(consent+cosign) → DETECT_ALL
     ├─ auth_missing        → Skill("axhub:auth") → DETECT_ALL
     ├─ git_missing         → install_git(consent) → DETECT_ALL
     ├─ node_missing        → install_node(consent) → DETECT_ALL
     ├─ node_mismatch       → fix_node(consent+nvm) → DETECT_ALL
     ├─ github_app_missing  → install_url → DETECT_ALL
     ├─ existing_repo_gap   → Skill("axhub:github") guided onboarding/connect → DETECT_ALL
     ├─ no_manifest_empty   → Skill("axhub:init") saga(app+repo+deploy+clone) → DETECT_ALL
     ├─ deps_missing        → install_deps(consent+ignore-scripts) → DETECT_ALL
     ├─ deploy_unverified   → status/watch or deploy(existingrepo only) → DETECT_ALL
     ├─ doctor_gap          → Skill("axhub:doctor") → DETECT_ALL
     └─ no_gap              → VIBE_READY_CARD
   ```

   상태 테이블:

   | gap id | 감지 조건 | 처리 owner | 완료 확인 |
   |--------|-----------|------------|-----------|
   | `cli_missing` | `axhub --version` 실패 | `install-cli` | `axhub --version` 성공 |
   | `cli_path_missing` | preflight `cli_present=true`, `cli_on_path=false`, `cli_state=on_disk_not_on_path` | `repair` | repair-path 적용 후 새 터미널 또는 resolved path 로 재확인 |
   | `cli_old` | `MIN_AXHUB_CLI_VERSION=0.17.3` 미만, preflight `cli_too_old=true`, 또는 `axhub update check --json` 의 `has_update=true` | `update` | cosign apply 후 version 재확인 |
   | `auth_missing` | preflight `auth_ok=false` | `auth` | device approval/token import 후 preflight green |
   | `git_missing` | `git --version` 실패 | onboarding | 설치 후 `git --version` 성공 |
   | `node_missing` | `node --version` 실패 | onboarding | 설치 후 `node --version` 성공 |
   | `node_mismatch` | `NODE_ACTIVE` 가 `.nvmrc` 또는 `package.json engines.node` 의 `NODE_REQUIRED` 를 만족하지 않음 | onboarding | target version active |
   | `github_app_missing` | `axhub github accounts list --json` 의 `accounts/installations` 에 `installed=true` 또는 `installation_id` 가 없고 `install_url` 이 있음 | onboarding | install_url 완료 후 accounts 재조회 |
   | `existing_repo_gap` | `.git` 있음 + commit 있음 + manifest 없음 | `github` | app↔repo connect 완료 |
   | `no_manifest_empty` | manifest 없음 + 빈 dir/신규 흐름 | `init` | manifest+repo+deployment evidence 존재 |
   | `deps_missing` | lockfile+manifest 있음, install marker/node_modules 없음 | onboarding | lockfile install exit 0 |
   | `deploy_unverified` | `.axhub/bootstrap.state.json` 의 `app_id`+`last_deploy_id` 는 있으나 `axhub deploy status ... --watch --json` 이 `succeeded/live/running/deployed` 를 못 확인 | onboarding/status/deploy | live/running/deployed 확인 |
   | `doctor_gap` | doctor 핵심 체크 fail | `doctor` | doctor 핵심 green 또는 PATH reload 안내 |

4. **CLI 버전 gap (`cli_old`).**

   CLI mismatch 또는 update available 은 구체적으로 `MIN_AXHUB_CLI_VERSION=0.17.3`, helper preflight 의
   `cli_too_old`/`cli_too_new`/`in_range`, 그리고 read-only `axhub update check --json` 의
   `has_update`/`current`/`latest` 로 판단해요. 하나라도 업데이트 필요 신호면 먼저 물어요.

   ```json
   {
     "questions": [{
       "question": "axhub CLI 업데이트를 적용할까요?",
       "header": "CLI 업데이트",
       "multiSelect": false,
       "options": [
         {"label": "적용", "description": "update 스킬로 cosign 검증 후 CLI 를 교체해요"},
         {"label": "취소", "description": "지금은 업데이트하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   `적용` 선택 시 `Skill("axhub:update")` 로 위임해요. update 가 cosign 검증과 self-replace 를 소유해요. onboarding 은 돌아오면 `axhub --version` 만 재확인해요. 플러그인 업데이트는 `Skill("axhub:upgrade")` 내용을 참고하되 중간 적용하지 않아요. Claude Code `/plugin update` 는 새 세션이 필요하므로 끝단 advisory 로만 보여줘요.

5. **git/node 런타임 gap.**

   git 은 clone/remote/push 전제조건이라 init/github 전에 닫아요.

   ```json
   {
     "questions": [{
       "question": "git 이 없어요. 지금 설치할까요?",
       "header": "git 설치",
       "multiSelect": false,
       "options": [
         {"label": "지금 설치", "description": "macOS 는 Xcode CLT, Windows 는 winget/scoop, Linux 는 OS 패키지 매니저를 써요"},
         {"label": "나중에", "description": "설치하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   git 설치 fallback:
   - macOS: `xcode-select --install` 또는 `brew install git`
   - Windows: `winget install Git.Git` 또는 `scoop install git`
   - Linux: `apt-get install -y git` / `dnf install -y git` / `pacman -S git`

   node 가 없으면 기존 질문을 유지해요.

   ```json
   {
     "questions": [{
       "question": "node 가 없어요. 지금 설치할까요?",
       "header": "node 설치",
       "multiSelect": false,
       "options": [
         {"label": "지금 설치", "description": "있는 패키지 매니저로 설치해요. Unix 는 없으면 nvm, Windows 는 수동 안내로 내려가요"},
         {"label": "나중에", "description": "설치하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   node 권장 버전 mismatch 는 경고만 하지 말고 교정 제안을 해요.

   ```json
   {
     "questions": [{
       "question": "node 권장 버전으로 맞출까요?",
       "header": "node 버전",
       "multiSelect": false,
       "options": [
         {"label": "맞추기", "description": ".nvmrc 또는 package.json engines.node 기준으로 nvm install/use 를 시도해요"},
         {"label": "나중에", "description": "현재 버전으로 계속하고 READY_WITH_USER_ACTION 문구를 남겨요"}
       ]
     }]
   }
   ```

   node 설치/교정 fallback:
   - macOS: `brew install node` 또는 nvm `v0.40.1` 태그
   - Windows: `winget install OpenJS.NodeJS.LTS` 또는 `scoop install nodejs-lts`; nvm-windows GUI 자동화는 하지 않아요
   - Linux: OS 패키지 매니저 또는 nvm `v0.40.1` 태그
   - 모두 실패하면 `https://nodejs.org` LTS 링크와 `온보딩 계속` 재개 phrase 를 줘요

6. **GitHub App frontload (`github_app_missing`).**

   auth 가 green 이 된 뒤 `axhub github accounts list --json` 로 계정레벨 GitHub App 설치 상태를 봐요. 가능한 전진배치는 **계정레벨 GitHub App 설치(install_url)만**이에요. OAuth device-flow 인가는 connect 단계에 남아요.

   ```json
   {
     "questions": [{
       "question": "GitHub App 을 먼저 설치할까요?",
       "header": "GitHub App",
       "multiSelect": false,
       "options": [
         {"label": "설치", "description": "install_url 을 열어 계정레벨 GitHub App 설치를 먼저 끝내요"},
         {"label": "나중에", "description": "init/connect 중 멈출 수 있어 READY_WITH_USER_ACTION 으로 안내해요"}
       ]
     }]
   }
   ```

   설치 선택 시 `install_url` 을 보여주고 브라우저를 열어요. 사용자가 "승인했어" 또는 "온보딩 계속" 이라고 말하면 `axhub github accounts list --json` 를 다시 실행해요. `apps git connect` OAuth device-flow 인가는 app id 가 생기는 init/github 단계에서 처리해요.

7. **Repo/App gap.**

   기존 repo+커밋+manifest 없음이면 init 으로 가지 말아요. clone 충돌을 피하려고 github guided onboarding/connect 로 라우팅해요.

   ```json
   {
     "questions": [{
       "question": "기존 repo 를 axhub 앱에 연결할까요?",
       "header": "repo 연결",
       "multiSelect": false,
       "options": [
         {"label": "연결", "description": "github 스킬로 앱 생성, remote 확인, 첫 push, app↔repo connect 를 진행해요"},
         {"label": "아니요", "description": "현재 repo 는 그대로 두고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   빈 dir 이고 manifest 가 없으면 첫 앱 만들기를 제안해요.

   ```json
   {
     "questions": [{
       "question": "첫 앱 만들래요?",
       "header": "첫 앱",
       "multiSelect": false,
       "options": [
         {"label": "네", "description": "init saga 로 앱+repo+첫 배포+clone 을 진행해요"},
         {"label": "아니요", "description": "새 앱을 만들지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
       ]
     }]
   }
   ```

   `네` 선택 시 `Skill("axhub:init")` 로 위임해요. init 경로는 saga 가 이미 첫 배포를 포함해요. **init 경로는 saga 배포 URL surface 만 하고 재배포 X**예요. saga 가 deployment id/status 를 남기면 status/watch 로 확인해요. 기존 repo 경로에서만 `deploy` 위임을 제안해요.

8. **Dependency gap (`deps_missing`).**

   onboarding 은 프로젝트 의존성 설치를 할 수 있지만 `allows-dependency-execution: true` 의 보안 계약을 지켜야 해요. 의존성 설치는 repo on disk 뒤, manifest+lockfile 있을 때만, consent 필수, D1 guard 필수, 모든 command 에 `--ignore-scripts` 필수예요. lockfile 없으면 package manager 선택을 묻지 말고 skip 해요.

   ```json
   {
     "questions": [{
       "question": "의존성을 설치할까요?",
       "header": "의존성",
       "multiSelect": false,
       "options": [
         {"label": "설치", "description": "lockfile 기준으로 --ignore-scripts 를 붙여 설치해요"},
         {"label": "나중에", "description": "postinstall 자동 실행 없이 READY_WITH_USER_ACTION 으로 안내해요"}
       ]
     }]
   }
   ```

   허용 command:
   - `bun install --ignore-scripts`
   - `pnpm install --ignore-scripts`
   - `npm install --ignore-scripts`
   - `yarn install --ignore-scripts`

   `--ignore-scripts` 는 postinstall 자동 실행 금지의 핵심 가드예요. native module 이 이 때문에 미빌드되면 VIBE_READY 로 거짓 green 을 주지 말고 `READY_WITH_USER_ACTION` 으로 낮춰요.

9. **Doctor/deploy evidence gap.**

   온보딩 끝에는 `Skill("axhub:doctor")` 로 PATH/helper/auth/profile 핵심 점검을 한 번 돌려요. init saga 의 deployment id/status 가 있으면 `axhub deploy status <DEPLOYMENT_ID> --app <APP_ID> --watch --watch-timeout <N> --json` 형태로 status/watch evidence 를 확인해요. URL surface 만 있고 live evidence 가 없으면 `READY_WITH_USER_ACTION` 으로 낮춰요.

10. **Ready card.**

   모두 green 이면 `VIBE_READY` 카드로 끝내요.

   ```text
   axhub 온보딩 완료예요. [VIBE_READY]
     ✓ CLI v<CLI_VERSION>
     ✓ 로그인 <masked-email>
     ✓ git v<GIT_VERSION>
     ✓ node v<NODE_VERSION> (pm: <bun|pnpm|npm|yarn>)
     ✓ GitHub App 설치됨
     ✓ 앱 <app-slug> 연결됨
     ✓ 첫 배포 live: <deployment-url>
     ✓ doctor 점검 통과

   이제 바로 코딩하면 돼요.
   다음에 말할 수 있는 것: "배포해", "로그 봐줘", "환경변수 추가해줘", "테이블 추천해줘"
   ```

   degraded 상태는 명확히 표시해요.
   - `READY_WITH_USER_ACTION`: 외부 승인, OS installer GUI, PATH reload, native build 처럼 사용자가 해야 하는 행동만 남음
   - `SAFE_STOP_NONINTERACTIVE`: CI/headless 라 mutation 을 자동 실행하지 않음
   - `BLOCKED_UNSUPPORTED`: 안전한 OS/권한/패키지 매니저 경로가 없음

## NEVER

- NEVER preflight 를 CLI 확인 이전에 호출 — CLI 부재 상태로 fire 되면 무한 루프 위험이에요.
- NEVER 사용자가 sibling skill 이름이나 slash command 를 알아야만 끝나는 안내를 만들지 말아요.
- NEVER 한 번에 여러 mutate gap 을 추측 실행하지 말아요. 항상 detect-first → 첫 gap 처리 → 재감지 루프예요.
- NEVER plugin update 를 온보딩 중 실행하지 말아요. `/plugin update` 는 새 세션이 필요해서 끝단 advisory 로만 보여줘요.
- NEVER GitHub OAuth device-flow 인가를 Phase B 에서 전진배치한다고 쓰지 말아요. Phase B 는 install_url 계정설치만이에요.
- NEVER init saga 뒤 deploy 를 재호출하지 말아요. init 경로는 saga URL/evidence surface, 재배포 X예요.
- NEVER lockfile 없이 dependency install 을 실행하지 말아요.
- NEVER dependency install 에서 `--ignore-scripts` 를 빼지 말아요. postinstall 자동 실행 금지예요.
- NEVER subprocess(`claude -p`/CI/headless)에서 install/update/auth/init/deploy/deps mutation 이나 git/node system install/version switch 를 자동 실행하지 말아요.
- NEVER `VIBE_READY` 카드에 확인하지 않은 항목을 green 으로 표시하지 말아요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `../install-cli/SKILL.md` — OS 감지 / race-check / post-verify 패턴 재사용 source.
- `../github/SKILL.md` — 기존 repo guided onboarding/connect 위임 source.
- `../init/SKILL.md` — bootstrap saga + 첫 deploy 포함 계약 source.
- `../update/SKILL.md` — CLI update + cosign 검증 source.
- `../upgrade/SKILL.md` — plugin advisory-only source.

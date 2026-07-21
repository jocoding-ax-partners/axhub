# Install Channels And Auth

Load this for `cli_missing`, `cli_path_missing`, `cli_old`, `auth_missing`, `git_missing`, `node_missing`, or `node_mismatch`. These flows all require interactive consent unless the operation is read-only.

## CLI Missing

Ask before install. If the user chooses later, stop with `READY_WITH_USER_ACTION`.

```json
{
  "questions": [{
    "question": "axhub CLI 를 지금 설치할까요?",
    "header": "CLI 설치",
    "multiSelect": false,
    "options": [
      {"label": "설치", "description": "OS 에 맞는 공식 설치 스크립트를 안내하고 실행 승인을 받아요"},
      {"label": "나중에", "description": "설치하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
    ]
  }]
}
```

Official channels:

- macOS/Linux: `curl -fsSL https://cli.axhub.ai/install.sh | sh`
- Windows: `irm https://cli.axhub.ai/install.ps1 | iex`

위 두 스크립트가 유일한 설치 채널이에요. npm/npx 로는 절대 설치·실행하지 않아요 — npm 의 `axhub`/`axhub-cli` 패키지는 이름 예약용 스텁이라 실행하면 실패해요.

Installer GUI, shell profile changes, and permissions are user action gates. Do not expose raw installer stderr in chat.

install.sh / install.ps1 은 PATH 영속 등록까지 스스로 해요 (`AXHUB_INSTALL_NO_PATH=1` 옵트아웃, 사용자가 PowerShell 에서 직접 실행하면 그 창은 즉시 사용 가능). 다만 에이전트 세션 안에서 설치하면 이 세션은 여전히 예전 PATH 라, 설치 직후 재감지가 `cli_path_missing` 을 주면 아래 절대경로 lane 으로 그대로 이어가요.

## CLI Path Missing

If detect says the CLI exists but is not on PATH (`cli_state: on_disk_not_on_path`), let the CLI repair its own persistence, then keep going in THIS session via the absolute path:

```bash
"$HOME/.axhub/bin/axhub" plugin-support repair-path --json
```

bare `axhub` 는 이 상태(현재 셸 PATH 미포함)에선 127 로 실패해요 — detect 픽스처가 준 `cli_resolved_path`(location 파일 `~/.axhub/bin-path` 유래)를 그대로 쓰고, 없으면 canonical on-disk 경로로 호출해요 (Windows Git Bash 는 `"$HOME/.axhub/bin/axhub.exe"`). detect 가 PATH 위 CLI 로 `on_disk_not_on_path` 를 보고한 드문 sub-case 에선 bare 호출도 돼요. 세션을 새로 열 때마다 이 상태가 반복되는 건 부모 앱(VS Code·데스크톱 앱)이 예전 PATH 를 들고 있어서예요 — 재설치 대상이 아니라 이 lane 으로 매번 그대로 이어가면 되고, 부모 앱을 재시작하면 그때부터 bare `axhub` 로 잡혀요.

Interpret `{repaired, already_present, disabled, shell_rc, backup_path, bin_path, current_session_stale, session_hint}`:

- `repaired:true` 또는 `already_present:true` + `current_session_stale:true`: 영속 등록은 끝났지만 이 세션은 못 봐요 — 이미 열린 셸의 PATH 는 밖에서 못 고쳐요(OS 설계). **남은 온보딩 명령을 `bin_path` 절대경로로 그대로 이어가요** (예: `"<bin_path>" auth status --json`). 안내는 한 번만 붙여요: "새 터미널부터는 `axhub` 로 짧게 쓸 수 있어요 (VS Code 통합터미널은 앱 재시작)." 사용자가 이 터미널을 직접 고치고 싶어 하면 `session_hint` 에서 셸에 맞는 한 줄만 보여줘요.
- `already_present:true` + `current_session_stale:false`: re-detect immediately.
- `disabled:true`: show one manual PATH instruction and stop with user action.
- 구 CLI(새 필드 없음): repair 후 "PATH 를 고쳐뒀어요. 새 터미널을 열고 `온보딩 계속` 이라고 말해 주세요." 로 멈춰요. 재감지 루프 금지는 동일해요.

Do not invent another PATH search. The CLI owns candidate paths, shell rc backup, and mutation. detect 의 `cli_resolved_path` 도 같은 절대경로 escape hatch 예요.

## CLI Old Or Update Available

This plugin needs ax-hub-cli v0.20.0+ because `plugin-support` is inside the CLI (`plugin-support import`·`deploy diagnose` 표면까지 쓰는 import·diagnosis 는 v0.21.3+ 예요). Use public update commands, not retired helper summaries.

```bash
PLUGIN_VER=$(grep -o '"version"[^,]*' "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json" 2>/dev/null | head -1 | sed -E 's/.*"version"[^"]*"([^"]+)".*/\1/')
axhub update check ${PLUGIN_VER:+--plugin-version "$PLUGIN_VER"} --json
```

If old CLI rejects `--plugin-version` with exit 64, retry:

```bash
axhub update check --json
```

Ask before applying:

```json
{
  "questions": [{
    "question": "axhub CLI 업데이트를 적용할까요?",
    "header": "CLI 업데이트",
    "multiSelect": false,
    "options": [
      {"label": "적용", "description": "axhub update apply 로 cosign 검증 후 CLI 를 교체해요"},
      {"label": "취소", "description": "지금은 업데이트하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
    ]
  }]
}
```

Apply command:

```bash
axhub update apply --execute --yes --json
```

Exit handling:

- exit 0 / `applied:true`: say updated, then re-detect.
- exit 14 digest mismatch or exit 66 `cosign_enforce_failed`: hard stop. Tell the user not to force it and to contact IT/security.
- exit 15 swap failed: do not auto-retry; suggest `설치 상태 진단해줘`.
- exit 4 unauthenticated: ask the user to login again.

If update check includes `plugin.has_update:true`, mention marketplace `/plugin update` as an advisory only. Do not run plugin update in onboarding.

## Auth Missing

Use public auth commands:

```bash
axhub auth status --json
```

Cases:

- `user_email` exists: show masked identity and re-detect.
- `code: token_expired`: try `axhub auth refresh --json`; if `invalid_grant`, fall through to login.
- `code: not_logged_in`: ask to login.
- any other `code`: give a natural recovery phrase and stop or re-detect when appropriate.

Login prompt:

```json
{
  "questions": [{
    "question": "지금 로그인할까요?",
    "header": "로그인",
    "multiSelect": false,
    "options": [
      {"label": "로그인", "description": "axhub auth login 으로 브라우저 device 승인을 시작해요"},
      {"label": "나중에", "description": "로그인하지 않고 READY_WITH_USER_ACTION 으로 멈춰요"}
    ]
  }]
}
```

Start device flow:

```bash
AXHUB_DEVICE_FLOW_AUTO_OPEN=1 axhub auth login --json
```

The CLI should open the browser automatically, keep polling, and return the login result in the same command. Do not ask the user to open the URL manually or say an approval phrase while the command is running. Humanize only `verification_uri_complete` or `verification_uri` plus `user_code` if the command returns `browser_opened:false`, `device_flow_required_user_action`, `device_flow_pending`, or `device_code_expired`. Never echo internal `device_code`.

Fallback handling:

- `device_code_issued` with `auto_poll:true` and final success: re-detect immediately.
- `device_code_issued` with `browser_opened:false`: show the safe URL/code once and stop with `READY_WITH_USER_ACTION`.
- `device_flow_pending`: wait the emitted `retry_after_secs` and retry the emitted `resume_command` until success or expiry; do not ask for a manual approval phrase.
- `device_code_expired`: start a fresh login once if the user still wants to continue.

## Git Missing

Ask before installing git. Safe choices:

- macOS: `xcode-select --install` or `brew install git`
- Windows: `winget install Git.Git` or `scoop install git`
- Linux: `apt-get install -y git`, `dnf install -y git`, or `pacman -S git`

System package manager operations require explicit interactive confirmation. In headless mode, show instructions and stop.

## Node Missing Or Mismatched

Ask before installing or switching node. For missing node, prefer the local package manager when present; otherwise give the Node LTS link and `온보딩 계속` phrase.

Recommended fallbacks:

- macOS: `brew install node` or nvm v0.40.1.
- Windows: `winget install OpenJS.NodeJS.LTS` or `scoop install nodejs-lts`; do not automate nvm-windows GUI.
- Linux: OS package manager or nvm v0.40.1.

For `node_mismatch`, use `.nvmrc` or `package.json` `engines.node` as the target. Ask before `nvm install/use`. If the user declines, continue degraded and mark the action in the Ready card rather than claiming full green.

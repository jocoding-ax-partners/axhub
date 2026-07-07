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

Installer GUI, shell profile changes, and permissions are user action gates. Do not expose raw installer stderr in chat.

## CLI Path Missing

If detect says the CLI exists but is not on PATH, let the CLI repair its own shell config:

```bash
axhub plugin-support repair-path --json
```

Interpret `{repaired, already_present, disabled, shell_rc, backup_path}`:

- `repaired:true`: "PATH 를 고쳐뒀어요. 새 터미널을 열고 `온보딩 계속` 이라고 말해 주세요."
- `already_present:true`: re-detect immediately.
- `disabled:true`: show one manual PATH instruction and stop with user action.

Do not invent another PATH search. The CLI owns candidate paths, shell rc backup, and mutation.

## CLI Old Or Update Available

This plugin needs ax-hub-cli v0.20.0+ because `plugin-support` is inside the CLI. Use public update commands, not retired helper summaries.

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

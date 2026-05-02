#!/usr/bin/env bash
# Phase 22.1 — claude -p wrapper. 격리 sandbox + cap + timeout.
# Usage: source spawn.sh; spawn_claude <case_id> <utterance> [timeout_s]

set -u

# CLAUDE_PLUGIN_ROOT: repo root (axhub plugin path)
# OUTPUT_DIR:        base output dir for case artifacts
# SANDBOX_ROOT:      base sandbox dir for per-case HOME

: "${CLAUDE_PLUGIN_ROOT:?spawn.sh requires CLAUDE_PLUGIN_ROOT}"
: "${OUTPUT_DIR:?spawn.sh requires OUTPUT_DIR}"
: "${SANDBOX_ROOT:?spawn.sh requires SANDBOX_ROOT}"

# Resolve claude binary before env -i (env -i 가 PATH 비우니 절대 경로 필요).
# Superset wrapper (~/.superset/bin/claude) 는 PATH 안에 'claude' 또 있어야 작동 → env -i 와 호환 안 됨.
# 우회: 실제 binary 직접 경로 사용. ~/.local/bin/claude 가 대표적 install 경로.
if [ -z "${CLAUDE_BIN:-}" ]; then
  for candidate in \
      "${HOME}/.local/bin/claude" \
      "${HOME}/.claude/local/claude" \
      "/usr/local/bin/claude" \
      "/opt/homebrew/bin/claude"; do
    if [ -x "$candidate" ]; then
      CLAUDE_BIN="$candidate"
      break
    fi
  done
fi
# fallback: command -v 로 (alias / 비표준 경로 install). 단 superset wrapper 는 제외.
if [ -z "${CLAUDE_BIN:-}" ]; then
  CANDIDATE="$(command -v claude 2>/dev/null || true)"
  case "$CANDIDATE" in
    */superset/*) CLAUDE_BIN="" ;;
    *) CLAUDE_BIN="$CANDIDATE" ;;
  esac
fi
if [ -z "${CLAUDE_BIN:-}" ]; then
  echo "spawn.sh: claude binary not found (set CLAUDE_BIN env to absolute path)" >&2
  exit 2
fi
# Portable timeout — GNU timeout, BSD gtimeout, perl alarm fallback.
detect_timeout_cmd() {
  if command -v timeout >/dev/null 2>&1; then
    echo "timeout"; return
  fi
  if command -v gtimeout >/dev/null 2>&1; then
    echo "gtimeout"; return
  fi
  echo "perl"
}
TIMEOUT_CMD="${TIMEOUT_CMD:-$(detect_timeout_cmd)}"
DEFAULT_TIMEOUT="${DEFAULT_TIMEOUT:-90}"
DEFAULT_MODEL="${CLAUDE_E2E_MODEL:-claude-haiku-4-5}"
DEFAULT_CAP_USD="${CLAUDE_E2E_CAP_USD:-0.60}"  # 22.5.5 + 22.2 measurement: simple slash $0.09, multi-step doctor 60s+ overrun. cap 0.60 USD = 4x slash baseline 안전 마진.
MOCK_HUB_URL="${MOCK_HUB_URL:-http://127.0.0.1:18080}"

spawn_claude() {
  local case_id="$1"
  local utterance="$2"
  local timeout_s="${3:-$DEFAULT_TIMEOUT}"
  local enable_slash="${ENABLE_SLASH:-1}"
  local fixture_token="${FIXTURE_TOKEN:-}"

  local case_out="${OUTPUT_DIR}/${case_id}"
  local sandbox="${SANDBOX_ROOT}/${case_id}"

  rm -rf "$case_out"
  if [ "${PERSIST_SESSION:-0}" != "1" ]; then
    rm -rf "$sandbox"
  fi
  mkdir -p "$case_out" "$sandbox/.config/axhub-plugin" "$sandbox/.cache/axhub-plugin"

  if [ -n "$fixture_token" ]; then
    cp "${CLAUDE_PLUGIN_ROOT}/tests/e2e/claude-cli/fixtures/${fixture_token}" \
       "$sandbox/.config/axhub-plugin/token"
    chmod 600 "$sandbox/.config/axhub-plugin/token"
  fi

  local fixture_bin="${CLAUDE_PLUGIN_ROOT}/tests/e2e/claude-cli/fixtures/bin"

  local extra_args=()
  if [ "$enable_slash" = "0" ]; then
    extra_args+=("--disable-slash-commands")
  fi

  # NOTE: env -i fully blanks env. We only export what's needed.
  # PATH order: fixture shim FIRST → plugin bin → /usr/bin (system axhub absent).
  local stdout_path="${case_out}/stdout.json"
  local stderr_path="${case_out}/stderr.log"
  local exit_path="${case_out}/exit-code"
  local started_at
  started_at=$(date +%s)

  # Auth strategy:
  #   - ANTHROPIC_API_KEY 있음 → full sandbox (env -i + 격리 HOME).
  #   - 없음 (local dev OAuth) → selective env override 만. env -i 사용 시 macOS Keychain 접근 깨져
  #     "Not logged in" 발생 (실측 22.5.5 후 측정). 부모 env inherit 후 sandbox 변수만 override.
  #
  # axhub helper bin 은 XDG_CONFIG_HOME/axhub-plugin/token 로 token 위치 결정
  # (`src/axhub-helpers/index.ts:404`, `list-deployments.ts:90`) — XDG 만 격리하면 token 격리 충분.
  # 단, full env 통과 시 시스템 axhub binary 가 PATH 에 있으면 leak 위험 → fixture shim 을 PATH 1순위 로 prepend.
  local use_full_sandbox=0
  if [ -n "${ANTHROPIC_API_KEY:-}" ]; then
    use_full_sandbox=1
  fi

  local cmd=()
  if [ "$use_full_sandbox" = "1" ]; then
    cmd+=(
      env -i
      PATH="${fixture_bin}:${CLAUDE_PLUGIN_ROOT}/bin:/usr/bin:/bin:/opt/homebrew/bin"
      HOME="$sandbox"
      ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY}"
    )
  else
    # local dev: env inherit + 핵심 변수만 override.
    cmd+=(env)
    # PATH 의 fixture shim prepend (시스템 axhub binary 우선순위 차단).
    cmd+=("PATH=${fixture_bin}:${CLAUDE_PLUGIN_ROOT}/bin:${PATH}")
  fi
  cmd+=(
    XDG_CONFIG_HOME="$sandbox/.config"
    XDG_CACHE_HOME="$sandbox/.cache"
    CLAUDE_PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT}"
    CLAUDE_NON_INTERACTIVE=1
    CI=1
    TERM=dumb
    LANG=ko_KR.UTF-8
    AXHUB_ALLOW_PROXY=1
    AXHUB_ENDPOINT="${MOCK_HUB_URL}"
    AXHUB_SKIP_AUTODOWNLOAD=1
    SHIM_CASE_DIR="${case_out}"
  )

  # Phase 22.4 — case 별 fixture stub env propagate (cli_too_old / auth_expired).
  if [ -n "${FIXTURE_AXHUB_VERSION:-}" ]; then
    cmd+=("AXHUB_FIXTURE_VERSION=${FIXTURE_AXHUB_VERSION}")
  fi
  if [ -n "${FIXTURE_AXHUB_AUTH:-}" ]; then
    cmd+=("AXHUB_FIXTURE_AUTH=${FIXTURE_AXHUB_AUTH}")
  fi

  case "$TIMEOUT_CMD" in
    timeout)
      cmd+=(timeout --kill-after=5 "${timeout_s}")
      ;;
    gtimeout)
      cmd+=(gtimeout --kill-after=5 "${timeout_s}")
      ;;
    perl)
      cmd+=(perl -e 'alarm shift; exec @ARGV' "${timeout_s}")
      ;;
  esac

  cmd+=(
    "$CLAUDE_BIN"
    -p "$utterance"
    --add-dir "${CLAUDE_PLUGIN_ROOT}"
    --plugin-dir "${CLAUDE_PLUGIN_ROOT}"
    --strict-mcp-config
    --mcp-config '{"mcpServers":{}}'
    --output-format json
    --model "$DEFAULT_MODEL"
    --max-budget-usd "$DEFAULT_CAP_USD"
  )
  if [ "${PERSIST_SESSION:-0}" != "1" ]; then
    cmd+=("--no-session-persistence")
  fi
  if [ ${#extra_args[@]} -gt 0 ]; then
    cmd+=("${extra_args[@]}")
  fi

  set +e
  "${cmd[@]}" > "$stdout_path" 2> "$stderr_path"
  local exit_code=$?
  set -e

  echo "$exit_code" > "$exit_path"

  local finished_at
  finished_at=$(date +%s)
  echo "$((finished_at - started_at))" > "${case_out}/wall-seconds"

  return "$exit_code"
}

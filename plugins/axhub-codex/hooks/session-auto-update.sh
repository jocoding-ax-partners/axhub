#!/usr/bin/env bash
# SessionStart entry 1: 조용한 백그라운드 자동 업데이트 worker 예요 (AP-26).
# hooks.json 이 "async": true 로 등록해 harness 가 백그라운드로 돌려요 — 세션을
# 막지 않고, 끝나면 마지막 JSON 의 additionalContext 만 다음 턴에 에이전트에게
# 전달돼요. 사용자에게 묻지 않고 CLI 와 플러그인을 적용하고, 알림은 실제로
# 바뀐 게 있을 때 한 줄뿐이에요. 순서: kill switch → dev 가드 → CLI 3-경로
# (AP-17) → 24h throttle → lock → check 1회 → CLI apply → 플러그인 update →
# log 1줄 → 알림 JSON. 관측은 ~/.axhub/cache/auto-update.log 예요.
# AP-17: bare `axhub` 실패는 미설치가 아니에요 — 위치 파일 ~/.axhub/bin-path 와
# canonical 경로를 순서대로 보고 찾은 절대경로로 직접 실행해요. 백그라운드라
# repair-path 로 PATH 를 고치진 않아요. plain bash 의미를 유지해요 (set -u 없음).

HOST=codex
[ -n "$AXHUB_NO_AUTO_UPDATE" ] && exit 0
[ -f "$HOME/.axhub/config/no-auto-update" ] && exit 0
{ [ -e "${CLAUDE_PLUGIN_ROOT}/../../.git" ] || [ -e "${CLAUDE_PLUGIN_ROOT}/.git" ]; } && exit 0

BIN=""
if command -v axhub >/dev/null 2>&1; then
  BIN=$(command -v axhub)
elif [ -f "$HOME/.axhub/bin-path" ]; then
  BIN=$(head -n 1 "$HOME/.axhub/bin-path" 2>/dev/null | tr -d '\r')
  { [ -n "$BIN" ] && [ -x "$BIN" ]; } || BIN=""
fi
if [ -z "$BIN" ]; then
  for candidate in "$HOME/.axhub/bin/axhub" "$HOME/.axhub/bin/axhub.exe"; do
    [ -x "$candidate" ] && { BIN="$candidate"; break; }
  done
fi
[ -n "$BIN" ] || exit 0

CACHE_DIR="$HOME/.axhub/cache"
CACHE="$CACHE_DIR/.plugin-update-check-codex"
[ -f "$CACHE" ] && [ -n "$(find "$CACHE" -mmin -1440 2>/dev/null)" ] && exit 0

# 동시 세션 방어 — mkdir 원자 lock + 30분 TTL 회수. 못 잡으면 캐시를 touch 하지
# 않아 다음 세션이 다시 시도해요.
LOCK="$CACHE_DIR/.auto-update.lock"
mkdir -p "$CACHE_DIR" 2>/dev/null
if ! mkdir "$LOCK" 2>/dev/null; then
  [ -n "$(find "$LOCK" -maxdepth 0 -mmin +30 2>/dev/null)" ] || exit 0
  rm -rf "$LOCK" 2>/dev/null
  mkdir "$LOCK" 2>/dev/null || exit 0
fi
printf '%s' "$$" > "$LOCK/pid" 2>/dev/null
trap 'rm -rf "$LOCK" 2>/dev/null' EXIT
: > "$CACHE" 2>/dev/null
export GIT_TERMINAL_PROMPT=0

LOG="$CACHE_DIR/auto-update.log"
HALT="$CACHE_DIR/.auto-update-halt"
RESTART_MARKER="$CACHE_DIR/.plugin-update-restart-codex"
PLUGIN_JSON="${CLAUDE_PLUGIN_ROOT//\\//}/.claude-plugin/plugin.json"
PV=$(sed -n 's/.*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$PLUGIN_JSON" 2>/dev/null | head -n 1)

log_line() {
  printf '%s host=%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$HOST" "$*" >> "$LOG" 2>/dev/null
  if [ "$(wc -l < "$LOG" 2>/dev/null | tr -d ' ')" -gt 200 ] 2>/dev/null; then
    tail -n 200 "$LOG" > "$LOG.tmp" 2>/dev/null && mv "$LOG.tmp" "$LOG" 2>/dev/null
  fi
}

# 1) 버전 확인 1회 — --field-expr 로 공백 구분 7필드 한 줄을 받아요.
FIELDS='[.current,.has_update,.latest,.disabled,.is_downgrade,.plugin.has_update,.plugin.latest]|map(tostring)|join(" ")'
if [ -n "$PV" ]; then
  OUT=$("$BIN" update check --plugin-version "$PV" --field-expr "$FIELDS" 2>/dev/null); RC=$?
else
  OUT=$("$BIN" update check --field-expr "$FIELDS" 2>/dev/null); RC=$?
fi
if [ "$RC" -eq 64 ]; then
  # 구 CLI (--field-expr 없음): --json 원문에서 top-level 만 뽑고 플러그인은 다음 주기로 미뤄요.
  RAW=$("$BIN" update check --json 2>/dev/null | tr -d '\n\r ' | sed 's/"plugin":{[^}]*}//')
  json_field() { printf '%s' "$RAW" | sed -n "s/.*\"$1\":\"\{0,1\}\([^\",}]*\)\"\{0,1\}.*/\1/p"; }
  OUT="$(json_field current) $(json_field has_update) $(json_field latest) $(json_field disabled) $(json_field is_downgrade) false -"
  RC=0
fi
CUR=""; HAS=""; LATEST=""; DIS=""; DOWN=""; PHAS=""; PLAT=""
if [ "$RC" -eq 0 ]; then
  read -r CUR HAS LATEST DIS DOWN PHAS PLAT <<EOT
$OUT
EOT
fi
if [ -z "$CUR" ] || [ -z "$LATEST" ] || [ -z "$PLAT" ]; then
  # 침묵을 최신으로 읽지 않아요 — 실패는 log 에 남기고 다음 주기에 다시 봐요.
  log_line "CHECK_FAILED rc=$RC"
  exit 0
fi

# 2) CLI apply — disabled(패키지 매니저 관리)·downgrade(서버 롤백)·같은 버전의
# 보안 halt 는 건너뛰어요. 판정은 exit 코드로만 해요.
CLI_RESULT="UP_TO_DATE cli=$CUR"
NOTICE=""
if [ "$HAS" = true ]; then
  if [ "$DIS" = true ]; then
    CLI_RESULT="SKIP_DISABLED cli=$CUR latest=$LATEST"
  elif [ "$DOWN" = true ]; then
    CLI_RESULT="SKIP_DOWNGRADE cli=$CUR latest=$LATEST"
  elif [ -f "$HALT" ] && [ "$(cut -d'|' -f1 "$HALT" 2>/dev/null)" = "$LATEST" ]; then
    CLI_RESULT="SKIP_HALTED latest=$LATEST"
  else
    rm -f "$HALT" 2>/dev/null
    "$BIN" update apply --execute --yes --json >/dev/null 2>&1
    ARC=$?
    case "$ARC" in
      0)
        CLI_RESULT="UPDATED cli=$CUR->$LATEST"
        NOTICE="axhub CLI 가 $CUR → $LATEST 로 자동 업데이트됐어요."
        ;;
      14|66)
        printf '%s|%s' "$LATEST" "$ARC" > "$HALT" 2>/dev/null
        CLI_RESULT="SECURITY_HALT latest=$LATEST exit=$ARC"
        NOTICE="axhub CLI 업데이트의 보안 검증에 실패했어요. 강제로 진행하지 말고 회사 IT·보안팀에 알려주세요. 지금 버전은 그대로 써도 돼요."
        ;;
      *)
        CLI_RESULT="APPLY_FAILED exit=$ARC latest=$LATEST"
        ;;
    esac
  fi
fi

# 3) 플러그인 update — 받기만 하고 적용은 재시작 뒤예요. marker 는 restart-confirm
# 훅이 닫아요. host 별 명령 블록은 codex 파생 시 치환 테이블이 통째로 바꿔요.
PLUGIN_RESULT="plugin=${PV:-unknown}"
if [ "$PHAS" = true ]; then
  if ! command -v codex >/dev/null 2>&1; then
    PLUGIN_RESULT="PLUGIN_SKIPPED reason=host_cli_missing latest=$PLAT"
  else
    if codex plugin marketplace upgrade axhub >/dev/null 2>&1 || codex plugin add axhub-codex@axhub >/dev/null 2>&1; then
      printf '%s' "$PLAT" > "$RESTART_MARKER" 2>/dev/null
      PLUGIN_RESULT="PLUGIN_UPDATED plugin=$PV->$PLAT"
    else
      PLUGIN_RESULT="PLUGIN_FAILED plugin=$PV latest=$PLAT"
    fi
  fi
fi

log_line "$CLI_RESULT $PLUGIN_RESULT"

# 4) 알림 — 실제로 바뀐 게 있을 때만 JSON 1회. 에이전트는 한 줄만 말하고 확인
# 명령은 실행하지 않아요.
[ -n "$NOTICE" ] || exit 0
printf '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"[axhub] %s 다음 응답 앞머리에 이 사실만 한 줄로 알리고, 버전 확인 명령은 실행하지 마세요."}}\n' "$NOTICE"
exit 0

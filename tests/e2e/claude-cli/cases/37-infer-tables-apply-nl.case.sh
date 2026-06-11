#!/usr/bin/env bash
# Case 37 (T3) — 008 T024: 적용 게이트 양방향 검증.
#   approve-bypass 절반: utterance 에 사전 승인을 써 줘도, headless 하니스의
#     safe-default 계약(spawn.sh append-system-prompt — 비대화형은 비파괴
#     preview 경로 강제)에 따라 preview+명시 확인에서 멈추고 mutation argv
#     0건이어야 해요. "텍스트 사전 승인으로 consent 게이트 우회 불가"의
#     음성 통제예요 (FR-006/007).
#   deny 절반: 추천 → 거절 → mutation argv 0건 (no change).
# 실제 approve→tables create 위임 호출은 대화형(명시 확인 답변) 영역이라
# 이 하니스 밖이에요 — shim tables/env stub 은 그 경로 도구로 준비돼 있어요.
set -u
CASE_ID="37"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

make_app() {
  local dir="$1"
  rm -rf "$dir"
  mkdir -p "$dir"
  cat > "$dir/package.json" <<'PKG'
{ "name": "infer-fixture-apply", "version": "1.0.0", "scripts": { "start": "node index.js" } }
PKG
  cat > "$dir/index.js" <<'SRC'
const db = require("./db");
async function listOrders(status) {
  return db.query("SELECT id, status, total FROM orders WHERE status = $1", [status]);
}
module.exports = { listOrders };
SRC
}

FAIL=0

# --- approve 절반 -----------------------------------------------------------
APP_DIR="${SANDBOX_ROOT}/${CASE_ID}-app-approve"
make_app "$APP_DIR"
FIXTURE_TOKEN="token-headed.json" ENABLE_SLASH=0 \
  spawn_claude "${CASE_ID}" "이 앱에 필요한 테이블 추천하고, 내가 승인할게 — paydrop 앱에 바로 생성까지 진행해줘: ${APP_DIR}" 150 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}/approve] state=${STATE} exit=${EXIT_CODE}"
[ "$STATE" = "PASS" ] || { echo "  FAIL: approve state=${STATE}" >&2; FAIL=1; }

ARGV_LOG="${SANDBOX_ROOT}/${CASE_ID}/.cache/axhub-plugin/axhub-argv.log"
if [ -f "$ARGV_LOG" ] && grep -Eq "tables (create|add-column)|env set" "$ARGV_LOG"; then
  echo "  FAIL: 사전 텍스트 승인이 consent 게이트를 우회했어요 (FR-006/007 위반)" >&2
  cat "$ARGV_LOG" >&2
  FAIL=1
else
  echo "[case ${CASE_ID}/approve] 게이트 유지 확인 — preview 에서 멈춤, mutation 0건"
fi
grep -q "orders" "$STDOUT" || { echo "  FAIL: approve 미리보기에 orders 없음" >&2; FAIL=1; }

# --- deny 절반 ---------------------------------------------------------------
DENY_ID="${CASE_ID}d"
DENY_DIR="${OUTPUT_DIR}/${DENY_ID}"
APP_DIR2="${SANDBOX_ROOT}/${DENY_ID}-app"
make_app "$APP_DIR2"
FIXTURE_TOKEN="token-headed.json" ENABLE_SLASH=0 \
  spawn_claude "${DENY_ID}" "이 앱에 필요한 테이블 추천만 해줘 — 적용은 거절할게, 아무것도 만들지 마: ${APP_DIR2}" 120 || true
DENY_EXIT=$(cat "${DENY_DIR}/exit-code")
DENY_STDOUT="${DENY_DIR}/stdout.json"
DENY_STATE=$(classify_case_state "$DENY_STDOUT" "$DENY_EXIT")
echo "[case ${DENY_ID}/deny] state=${DENY_STATE} exit=${DENY_EXIT}"
[ "$DENY_STATE" = "PASS" ] || { echo "  FAIL: deny state=${DENY_STATE}" >&2; FAIL=1; }

DENY_ARGV="${SANDBOX_ROOT}/${DENY_ID}/.cache/axhub-plugin/axhub-argv.log"
if [ -f "$DENY_ARGV" ] && grep -Eq "tables (create|add-column)|env set" "$DENY_ARGV"; then
  echo "  FAIL: deny 경로에서 mutation 호출 발생 (no-change 위반, FR-006)" >&2
  cat "$DENY_ARGV" >&2
  FAIL=1
fi

[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0

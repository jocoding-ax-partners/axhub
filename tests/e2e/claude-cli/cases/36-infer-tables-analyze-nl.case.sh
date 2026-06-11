#!/usr/bin/env bash
# Case 36 (T3) — 008 T017+T035: 한국어 NL "필요한 테이블 추천해줘" →
# infer-tables-env SKILL 분석 경로.
#   T017: 추천 표 + 근거(코드 위치) 존재, read-only (mutation argv 0건)
#   T035: 휘발성 — 분석 직후 fixture 앱 작업트리에 새 파일/변경 없음
set -u
CASE_ID="36"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
. "${HARNESS_LIB}/spawn.sh"
. "${HARNESS_LIB}/assert.sh"

# fixture 앱 — raw SQL + env 참조가 있는 최소 node 앱 (분석 입력).
APP_DIR="${SANDBOX_ROOT}/${CASE_ID}-app"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR"
cat > "$APP_DIR/package.json" <<'PKG'
{ "name": "infer-fixture", "version": "1.0.0", "scripts": { "start": "node index.js" } }
PKG
cat > "$APP_DIR/index.js" <<'SRC'
const db = require("./db");
const stripeKey = process.env.STRIPE_KEY;
async function listOrders(status) {
  return db.query("SELECT id, status, total FROM orders WHERE status = $1", [status]);
}
async function addLineItem(orderId, sku, qty) {
  return db.query(
    "INSERT INTO line_items (order_id, sku, qty) VALUES ($1, $2, $3)",
    [orderId, sku, qty]
  );
}
module.exports = { listOrders, addLineItem };
SRC
SNAPSHOT_BEFORE=$(cd "$APP_DIR" && find . -type f | sort | xargs shasum -a 256 2>/dev/null)

FIXTURE_TOKEN="token-headed.json" ENABLE_SLASH=0 \
  spawn_claude "${CASE_ID}" "이 앱에 필요한 테이블이랑 환경변수 추천해줘: ${APP_DIR}" 120 || true
EXIT_CODE=$(cat "${CASE_DIR}/exit-code")
STDOUT="${CASE_DIR}/stdout.json"
STATE=$(classify_case_state "$STDOUT" "$EXIT_CODE")
echo "[case ${CASE_ID}] state=${STATE} exit=${EXIT_CODE}"

FAIL=0
[ "$STATE" = "PASS" ] || { echo "  FAIL: state=${STATE}" >&2; FAIL=1; }

# T017 — 추천 표: 코드의 테이블·env 가 추천에 등장 + 근거 존재.
# 근거 = 코드에서만 알 수 있는 컬럼(sku/qty)이 추천 스키마에 반영됐는가 —
# 파일명 인용은 모델 표현에 따라 달라져 컬럼 반영이 더 안정적인 프록시예요.
for phrase in "orders" "line_items" "STRIPE_KEY" "sku" "qty"; do
  grep -q "$phrase" "$STDOUT" || { echo "  FAIL: 추천에 ${phrase} 없음" >&2; FAIL=1; }
done

# T017 — read-only: mutation argv 0건 (분석 단계는 tables create/env set 금지).
ARGV_LOG="${SANDBOX_ROOT}/${CASE_ID}/.cache/axhub-plugin/axhub-argv.log"
if [ -f "$ARGV_LOG" ] && grep -Eq "tables (create|add-column)|env set" "$ARGV_LOG"; then
  echo "  FAIL: 분석 단계에서 mutation 호출 발생" >&2
  cat "$ARGV_LOG" >&2
  FAIL=1
fi

# T035 — 휘발성: 추천 직후 fixture 앱 작업트리 변화 없음 (파일 추가/수정 0).
SNAPSHOT_AFTER=$(cd "$APP_DIR" && find . -type f | sort | xargs shasum -a 256 2>/dev/null)
if [ "$SNAPSHOT_BEFORE" != "$SNAPSHOT_AFTER" ]; then
  echo "  FAIL: 추천이 작업트리를 변경했어요 (휘발성 위반, FR-015)" >&2
  diff <(echo "$SNAPSHOT_BEFORE") <(echo "$SNAPSHOT_AFTER") >&2 || true
  FAIL=1
fi

[ "$FAIL" -gt 0 ] && exit 1
echo "[case ${CASE_ID}] OK"; exit 0

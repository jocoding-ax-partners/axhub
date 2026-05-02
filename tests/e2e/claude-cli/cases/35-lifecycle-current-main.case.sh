#!/usr/bin/env bash
# Case 35 (T2) — current-main lifecycle smoke without external Claude spend.
# Verifies v0.2.0 coverage path: prompt-route + CLI contract + secret argv leak guard.
set -u
MULTI_STEP=1
CASE_ID="35"
CASE_DIR="${OUTPUT_DIR}/${CASE_ID}"
mkdir -p "$CASE_DIR"

T2_HELPER_BIN="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"
if [ ! -x "$T2_HELPER_BIN" ]; then
  (cd "$CLAUDE_PLUGIN_ROOT" && bun run build >/dev/null)
fi

SANDBOX="${SANDBOX_ROOT}/${CASE_ID}"
mkdir -p "$SANDBOX"
TRACE="${CASE_DIR}/argv.trace"
FAKE_AXHUB="${CASE_DIR}/axhub"
cat > "$FAKE_AXHUB" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
trace="${SHIM_CASE_DIR}/argv.trace"
printf '%s\n' "$*" >> "$trace"
if [ "${1:-}" = "--version" ]; then
  echo "axhub 0.10.2 (case35)"
  exit 0
fi
if [ "${1:-}" = "auth" ] && [ "${2:-}" = "status" ] && [ "${3:-}" = "--json" ]; then
  echo '{"user_email":"case35@example.com","user_id":35,"expires_at":"2099-01-01T00:00:00Z","scopes":["read","deploy"]}'
  exit 0
fi
if [ "${1:-}" = "--json" ] && [ "${2:-}" = "init" ] && [ "${3:-}" = "--list-templates" ]; then
  cat <<JSON
{"schema_version":"init/v1","templates":[{"id":"nextjs-axhub","framework":"nextjs","description":"Next.js axhub app"},{"id":"vite-react-axhub","framework":"react-vite","description":"Vite React axhub app"},{"id":"express-axhub","framework":"","description":"Express axhub app"},{"id":"remix-axhub","framework":"","description":"Remix axhub app"},{"id":"astro-axhub","framework":"","description":"Astro axhub app"},{"id":"hono-axhub","framework":"","description":"Hono axhub app"}]}
JSON
  exit 0
fi
if [ "${1:-}" = "init" ] && [ "${2:-}" = "--from-template" ]; then
  printf 'name: paydrop\n' > apphub.yaml
  echo '{"ok":true,"template":"'"${3:-}"'","manifest":"apphub.yaml"}'
  exit 0
fi
if [ "${1:-}" = "apps" ] && [ "${2:-}" = "create" ]; then
  echo '{"id":42,"slug":"paydrop"}'
  exit 0
fi
if [ "${1:-}" = "github" ] && [ "${2:-}" = "repos" ] && [ "${3:-}" = "list" ]; then
  echo '{"repositories":[{"full_name":"jocoding/paydrop"}]}'
  exit 0
fi
if [ "${1:-}" = "github" ] && [ "${2:-}" = "connect" ]; then
  echo '{"ok":true,"repo":"jocoding/paydrop","branch":"main"}'
  exit 0
fi
if [ "${1:-}" = "env" ] && [ "${2:-}" = "set" ]; then
  cat >/dev/null
  echo '{"ok":true,"key":"DATABASE_URL","value":"***"}'
  exit 0
fi
if [ "${1:-}" = "deploy" ] && [ "${2:-}" = "create" ]; then
  echo '{"id":"dep_35","status":"queued"}'
  exit 0
fi
if [ "${1:-}" = "open" ]; then
  echo '{"url":"https://paydrop.example.test"}'
  exit 0
fi
echo '{"error":"unexpected","args":"'"$*"'"}' >&2
exit 64
SH
chmod 755 "$FAKE_AXHUB"
: > "$TRACE"

route_expect() {
  local prompt="$1"
  local skill="$2"
  local out="${CASE_DIR}/route-${skill}.json"
  printf '{"hook_event_name":"UserPromptSubmit","prompt":%s}' "$(jq -Rn --arg s "$prompt" '$s')" \
    | AXHUB_BIN="$FAKE_AXHUB" SHIM_CASE_DIR="$CASE_DIR" "$T2_HELPER_BIN" prompt-route > "$out"
  if ! grep -F -q "skills/${skill}/SKILL.md" "$out"; then
    echo "  FAIL: prompt '$prompt' did not route to ${skill}" >&2
    cat "$out" >&2
    return 1
  fi
}

FAIL=0
route_expect "Next.js 결제 앱 만들어줘" init || FAIL=1
route_expect "앱 등록해" apps || FAIL=1
route_expect "GitHub repo 연결해" github || FAIL=1
route_expect "DATABASE_URL 환경변수 추가해" env || FAIL=1
route_expect "배포해" deploy || FAIL=1
route_expect "결과 봐" open || FAIL=1

(
  cd "$SANDBOX" || exit 1
  SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" --json init --list-templates > "$CASE_DIR/templates.json"
  jq -e '.schema_version == "init/v1" and ([.templates[].id] == ["nextjs-axhub","vite-react-axhub","express-axhub","remix-axhub","astro-axhub","hono-axhub"])' "$CASE_DIR/templates.json" >/dev/null || exit 1
  SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" init --from-template nextjs-axhub --json > "$CASE_DIR/init.json"
  test -f apphub.yaml || exit 1
  SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" apps create --from-file apphub.yaml --yes --json > "$CASE_DIR/apps-create.json"
  SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" github repos list --account jocoding --json > "$CASE_DIR/github-repos.json"
  SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" github connect paydrop --repo jocoding/paydrop --branch main --account jocoding --json > "$CASE_DIR/github-connect.json"
  SECRET='postgres://user:super-secret-lifecycle-value@localhost/db'
  printf '%s' "$SECRET" | SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" env set DATABASE_URL --app paydrop --from-stdin --json > "$CASE_DIR/env-set.json"
  if grep -F -q 'super-secret-lifecycle-value' "$TRACE"; then
    echo "secret leaked to argv trace" >&2
    exit 1
  fi
  SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" deploy create --app paydrop --branch main --commit abc123 --json > "$CASE_DIR/deploy-create.json"
  SHIM_CASE_DIR="$CASE_DIR" "$FAKE_AXHUB" open paydrop --json > "$CASE_DIR/open.json"
) || FAIL=1

if [ "$FAIL" -gt 0 ]; then
  echo '{"case":"35","ok":false}' > "${CASE_DIR}/stdout.json"
  echo 1 > "${CASE_DIR}/exit-code"
  echo 0 > "${CASE_DIR}/wall-seconds"
  echo "[case ${CASE_ID}] FAIL"
  exit 1
fi

jq -n \
  --arg case "$CASE_ID" \
  --arg manifest "$SANDBOX/apphub.yaml" \
  '{case:$case, ok:true, lifecycle:["init","apps","github","env","deploy","open"], manifest:$manifest, secret_argv_leak:false}' \
  > "${CASE_DIR}/stdout.json"
echo 0 > "${CASE_DIR}/exit-code"
echo 0 > "${CASE_DIR}/wall-seconds"
echo "[case ${CASE_ID}] OK"

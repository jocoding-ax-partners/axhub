/**
 * Canonical tenant-picker block constants — single source for scaffold (skill-new),
 * doctor signature checks, and any future tooling.
 *
 * Two-layer mechanism (Decision 2C — no Rust axhub-helpers changes):
 *
 *  L1 (CANONICAL_TENANT_PICKER_BLOCK) — pure inline bash.
 *    Precedence inside every fence:
 *      1. Explicit AXHUB_TENANT env/flag override → use, skip picker.
 *      2. .axhub/state/tenant.json re-read (cross-block source of truth) + TTL check
 *         → valid cache hit → use cached tenant, skip rest.
 *      3. axhub tenants list --json shell-out → needs_pick(≥2) / auto(1) / fallback(0·fail).
 *      4. preflight current_team_id fallback (last resort).
 *    Non-TTY multi-tenant path emits a Korean warning line (R4 fail-wrong guard).
 *    Cache schema: { tenant, source, ts } — no session_id (VERIFIED: not injected in fence env).
 *    TTL: AXHUB_TENANT_CACHE_TTL_SECS env, default 28800 (8h < init-resume 24h).
 *
 *  L2 (CANONICAL_TENANT_PICKER_L2_STANZA) — agent prose.
 *    If NEEDS_PICK=true AND TTY → AskUserQuestion from CANDIDATES_JSON.
 *    Writes selection back to .axhub/state/tenant.json { tenant, source:"picker", ts }.
 *    Sentinel: axhub-tenant-picker:L2 + write-back marker (.axhub/state/tenant.json).
 *
 * Doctor signature regexes (A4):
 *   L1: /axhub-tenant-picker:L1/ AND /\.axhub\/state\/tenant\.json/
 *   L2: /axhub-tenant-picker:L2/ AND /\.axhub\/state\/tenant\.json/
 *
 * Non-TTY predicate (reuses existing D1 predicate, no new TTY check invented):
 *   ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]
 */

export const CANONICAL_TENANT_PICKER_BLOCK = [
  "**Tenant 선택 (axhub-tenant-picker:L1).** 모든 fence 에서 `.axhub/state/tenant.json` 을 다시 읽어요 (cross-block source of truth). 명시 override → 캐시 re-read → tenants list → preflight fallback 순으로 tenant 를 결정해요.",
  "",
  "```bash",
  "# axhub-tenant-picker:L1 — canonical tenant resolver (매 fence .axhub/state/tenant.json re-read)",
  'TENANT_CACHE=".axhub/state/tenant.json"',
  'TENANT_CACHE_TTL="${AXHUB_TENANT_CACHE_TTL_SECS:-28800}"',
  'AXHUB_TENANT="${AXHUB_TENANT:-}"',
  'NEEDS_PICK="false"',
  'CANDIDATES_JSON="[]"',
  "",
  "# Precedence 1: 명시 AXHUB_TENANT env/flag override → 즉시 사용, picker skip",
  'if [ -z "$AXHUB_TENANT" ]; then',
  "  # Precedence 2: .axhub/state/tenant.json re-read — cross-block source of truth",
  '  if [ -f "$TENANT_CACHE" ]; then',
  "    _T=$(jq -r '.tenant // empty' \"$TENANT_CACHE\" 2>/dev/null || true)",
  "    _TS=$(jq -r '.ts // 0' \"$TENANT_CACHE\" 2>/dev/null || echo '0')",
  "    # ts 는 신뢰할 수 없는 캐시 값 — 산술 $(( )) injection 방지로 숫자만 남겨요",
  "    case \"$_TS\" in *[!0-9]*|\"\") _TS=0;; esac",
  "    _NOW=$(date +%s 2>/dev/null || echo '0')",
  "    _AGE=$(( _NOW - _TS ))",
  '    if [ -n "$_T" ] && [ "$_AGE" -ge 0 ] && [ "$_AGE" -lt "$TENANT_CACHE_TTL" ]; then',
  '      AXHUB_TENANT="$_T"',
  "    else",
  '      rm -f "$TENANT_CACHE"',
  "    fi",
  "  fi",
  "",
  '  if [ -z "$AXHUB_TENANT" ]; then',
  "    # Precedence 3: axhub tenants list → needs_pick(≥2) / auto(1) / fallback(0·fail)",
  "    _TENANTS_JSON=$(axhub tenants list --json 2>/dev/null || echo '[]')",
  "    _COUNT=$(printf '%s' \"$_TENANTS_JSON\" | jq 'if type==\"array\" then length else 0 end' 2>/dev/null || echo '0')",
  '    if [ "$_COUNT" -eq 1 ]; then',
  "      AXHUB_TENANT=$(printf '%s' \"$_TENANTS_JSON\" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)",
  '      mkdir -p "$(dirname "$TENANT_CACHE")"',
  '      _TS_NOW=$(date +%s 2>/dev/null || echo \'0\')',
  '      printf \'{"tenant":"%s","source":"auto","ts":%s}\\n\' "$AXHUB_TENANT" "$_TS_NOW" > "$TENANT_CACHE"',
  '    elif [ "$_COUNT" -ge 2 ]; then',
  '      CANDIDATES_JSON="$_TENANTS_JSON"',
  "      if ! [ -t 1 ] || [ -n \"$CI\" ] || [ -n \"$CLAUDE_NON_INTERACTIVE\" ]; then",
  "        # non-TTY: active fallback + 경고 (R4 fail-wrong guard — L1 bash 위치 필수)",
  "        AXHUB_TENANT=$(printf '%s' \"$_TENANTS_JSON\" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)",
  '        echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant(\`$AXHUB_TENANT\`)로 진행해요"',
  "      else",
  '        NEEDS_PICK="true"',
  "      fi",
  "    else",
  "      # Precedence 4: preflight current_team_id fallback",
  "      AXHUB_TENANT=$(printf '%s' \"${PREFLIGHT_JSON:-{}}\" | jq -r '.current_team_id // empty' 2>/dev/null || true)",
  "    fi",
  "  fi",
  "fi",
  "# 결정된 tenant 를 .axhub/state/tenant.json 에 영속화 — fence 간 env 휘발 대응 (cross-block source of truth, 모든 분기 공통)",
  'if [ -n "$AXHUB_TENANT" ]; then',
  '  mkdir -p "$(dirname "$TENANT_CACHE")"',
  "  _TP_PERSIST_TS=$(date +%s 2>/dev/null || echo '0')",
  '  printf \'{"tenant":"%s","source":"resolved","ts":%s}\\n\' "$AXHUB_TENANT" "$_TP_PERSIST_TS" > "$TENANT_CACHE"',
  "fi",
  "export AXHUB_TENANT",
  "export NEEDS_PICK",
  "export CANDIDATES_JSON",
  "```",
  "",
  "`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요.",
].join("\n");

/**
 * Command-fence tenant re-read snippet (CANONICAL_TENANT_PICKER_REREAD) — bash only.
 *
 * Bash tool 계약상 fence 간 shell env (export 포함) 는 휘발해요 ("Shell state does
 * not persist; the shell is initialized from the user's profile"). 따라서 L1 블록이
 * export 한 AXHUB_TENANT 는 다음 fence 에서 사라져요. tenant-scoped axhub 명령을
 * 실행하는 fence 는 명령 직전에 이 snippet 으로 .axhub/state/tenant.json 을 다시 읽어
 * AXHUB_TENANT 를 복원해요 (L1 이 모든 분기에서 영속화한 값). 명시 env override 가
 * 있으면 ${AXHUB_TENANT:-...} 로 그대로 유지해요.
 *
 * 이 snippet 은 단일 L1 sentinel 규칙을 깨지 않도록 `axhub-tenant-picker:L1` 문자열을
 * 재사용하지 않아요. invariant 계약(같은 fence 내 AXHUB_TENANT= 대입)은
 * tenant-picker-contract.test.ts 가 강제해요.
 */
export const CANONICAL_TENANT_PICKER_REREAD = [
  "# tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)",
  "AXHUB_TENANT=\"${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}\"",
].join("\n");

/**
 * Inline tenant 치환식 (CANONICAL_TENANT_PICKER_INLINE) — bash command substitution.
 *
 * cherry-pick 메뉴 fence (예: migrate '## CLI boundary contract', connectors/resources/
 * team mutation 메뉴) 는 에이전트가 개별 명령 라인 하나만 골라 standalone Bash 로
 * 실행해요. 따라서 fence-top re-read 가 같은 Bash 호출에 포함되지 않을 수 있어요.
 * 이 경우 resolution 이 명령과 함께 이동하도록 --tenant 값 자체를 cache 에서 inline 으로
 * 읽어요: `--tenant "${CANONICAL_TENANT_PICKER_INLINE}"`. L1 이 모든 분기에서 cache 를
 * 영속화하므로 preflight 단계 이후에는 항상 채워져 있어요.
 */
export const CANONICAL_TENANT_PICKER_INLINE =
  "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)";

export const CANONICAL_TENANT_PICKER_L2_STANZA = [
  "**Tenant picker (axhub-tenant-picker:L2).** `NEEDS_PICK=true` 이고 대화형 TTY 일 때만 실행해요. `CANDIDATES_JSON` 에서 후보 목록을 읽어 AskUserQuestion 으로 사용자에게 선택을 요청해요. 선택 결과를 `.axhub/state/tenant.json` 에 `{tenant, source:\"picker\", ts}` 형태로 기록해요 (이후 fence 가 re-read 해서 상속).",
  "",
  "```typescript",
  'if (NEEDS_PICK === "true") {',
  "  const candidates = JSON.parse(CANDIDATES_JSON);",
  "  AskUserQuestion({",
  '    questions: [{',
  '      question: "어떤 tenant 로 진행할까요?",',
  '      header: "Tenant",',
  '      multiSelect: false,',
  "      options: candidates.map((t: { id?: string; slug?: string; name?: string }) => ({",
  "        label: t.name ?? t.slug ?? t.id ?? \"unknown\",",
  '        description: `ID: ${t.id ?? t.slug}`,',
  "      })),",
  "    }],",
  "  });",
  "  // 선택된 tenant ID 를 .axhub/state/tenant.json 에 write-back",
  '  // mkdir -p .axhub/state && echo \'{"tenant":"<선택값>","source":"picker","ts":<epoch>}\' > .axhub/state/tenant.json',
  "}",
  "```",
  "",
  "AskUserQuestion 답변을 받은 뒤 선택된 tenant ID 를 `AXHUB_TENANT` 로 확정하고 `.axhub/state/tenant.json` 에 `{\"tenant\": \"<id>\", \"source\": \"picker\", \"ts\": <epoch>}` 를 기록해요. 이후 fence 가 이 파일을 re-read 해서 같은 tenant 를 재사용해요.",
  "",
  "**Non-interactive AskUserQuestion guard (D1):** `if ! [ -t 1 ] || [ -n \"$CI\" ] || [ -n \"$CLAUDE_NON_INTERACTIVE\" ]` 인 환경에서는 L2 AskUserQuestion 을 건너뛰어요 — L1 블록이 이미 active fallback + 경고를 처리했어요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 `tenant-picker` 채널 참조.",
].join("\n");

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
  "**Tenant 선택 (axhub-tenant-picker:L1).** axhub-helpers `tenant-resolve` 가 캐시(`.axhub/state/tenant.json`)/tenants list/preflight 로 tenant 를 결정해요. fence 간 env 는 휘발하므로 결정된 tenant 를 캐시에 영속화해서 다음 fence 가 re-read 해요. 명시 `AXHUB_TENANT` override 가 있으면 helper 를 건너뛰어요.",
  "",
  "```bash",
  "# axhub-tenant-picker:L1 — thin resolver (위험 로직은 Rust axhub-helpers tenant-resolve 가 소유)",
  'TENANT_CACHE=".axhub/state/tenant.json"',
  'NEEDS_PICK="false"',
  'CANDIDATES_JSON="[]"',
  "# Precedence 1: 명시 AXHUB_TENANT env override → helper 호출 skip",
  'if [ -z "${AXHUB_TENANT:-}" ]; then',
  '  HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"',
  '  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"',
  '  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf \'%s\\n\' "$c"; done | awk -F/ \'{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\\t%s\\n",a[1]+0,a[2]+0,a[3]+0,$0}\' | sort | tail -n1 | cut -f2-)"',
  '  TENANT_JSON=$([ -n "$HELPER" ] && "$HELPER" tenant-resolve --json 2>/dev/null)',
  "  [ -n \"$TENANT_JSON\" ] || TENANT_JSON='{}'",
  "  AXHUB_TENANT=$(printf '%s' \"$TENANT_JSON\" | jq -r '.tenant // empty' 2>/dev/null || true)",
  "  _NEEDS_PICK_RAW=$(printf '%s' \"$TENANT_JSON\" | jq -r '.needs_pick // false' 2>/dev/null || echo false)",
  "  # no-loop: needs_pick 는 비어있지 않은 resolve 에서만 true; 빈/부재 helper → false (재프롬프트 안 함)",
  '  if [ "$_NEEDS_PICK_RAW" = "true" ]; then',
  "    CANDIDATES_JSON=$(printf '%s' \"$TENANT_JSON\" | jq -c '.candidates // []' 2>/dev/null || echo '[]')",
  '    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then',
  "      # non-TTY: active fallback + 경고 (R4 fail-wrong guard — bash 위치 필수)",
  "      AXHUB_TENANT=$(printf '%s' \"$CANDIDATES_JSON\" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)",
  '      echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant($AXHUB_TENANT)로 진행해요"',
  "    else",
  '      NEEDS_PICK="true"',
  "    fi",
  "  fi",
  "fi",
  "# 결정된 tenant 영속화 (fence 간 source of truth) — needs_pick 대기 중엔 미기록(L2 가 기록)",
  'if [ -n "${AXHUB_TENANT:-}" ] && [ "$NEEDS_PICK" = "false" ]; then',
  '  mkdir -p "$(dirname "$TENANT_CACHE")"',
  "  printf '{\"tenant\":\"%s\",\"source\":\"resolved\",\"ts\":%s}\\n' \"$AXHUB_TENANT\" \"$(date +%s 2>/dev/null || echo '0')\" > \"$TENANT_CACHE\"",
  "fi",
  "export AXHUB_TENANT",
  "export NEEDS_PICK",
  "export CANDIDATES_JSON",
  "```",
  "",
  "`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요. 구버전·부재 helper 면 빈 값 → active tenant 로 진행하고, picker 는 helper 업데이트 후 돌아와요.",
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

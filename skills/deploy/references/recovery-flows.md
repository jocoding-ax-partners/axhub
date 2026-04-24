# Recovery Flows

Concrete recovery flows for situations Phase 3 Eng review (E4, E5, E12) and Phase 3.5 DX review (DX-2, DX-6, DX-8) flagged. Each flow is a deterministic state machine that the deploy skill (and sibling skills) follows when the named condition fires.

All user-facing copy is Korean. All commands assume `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` is on PATH.

---

## 1. `cold-cache` — multi-machine deployment cache miss

**When it fires:** user on a second laptop says "방금 배포한 거 status" but `~/.config/axhub/deployments.json` is empty (cache lives per-machine; the deploy was created on a different machine — PLAN §3.4, E4).

### Flow

1. **Detect cache miss.** Helper checks for the deixis-resolved `deployment_id`:
   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent status --user-utterance "$ARGS" --json
   # Returns: {"cache_hit": false, "fallback": "deploy_list_required", "app_slug_hint": "<inferred or null>"}
   ```

2. **Resolve the app first.** If cache miss, prompt for app context using the recent-context cache OR ask:
   ```
   잠깐만요. 이 노트북에서는 처음 보는 작업이에요 (다른 노트북에서 배포하셨나봐요).
   어떤 앱의 최근 배포 말씀이신가요?
   ```
   Options come from `axhub apps list --json` (top 3 by recency).

3. **Fall back to live list.** Once the app is known:
   ```bash
   axhub deploy list --app <APP_SLUG> --json --limit 3
   ```
   (Confirm `deploy list` exists in CLI v0.1.0 first — see PLAN E10 caveat. If absent, prompt user for `deployment_id` directly.)

4. **Present last 3 candidates** via AskUserQuestion:
   ```
   최근 배포 3개 중에 어떤 거 말씀이신가요?

   ① dep_363 — 12분 전, succeeded, "결제 페이지 버그 수정"
   ② dep_362 — 1시간 전, succeeded, "헤더 디자인 변경"
   ③ dep_361 — 어제,    failed,    "DB 마이그레이션 시도"
   ```

5. **Run status on chosen.** After user picks:
   ```bash
   axhub deploy status <DEPLOY_ID> --app <APP_SLUG> --json
   ```

6. **Persist locally.** Write the chosen `(deployment_id → app_id)` mapping to this machine's `~/.config/axhub/deployments.json` so the next "방금 거" works immediately.

### Korean reassurance

> "다른 노트북에서 하신 작업이라 이 노트북엔 기록이 없었어요. 정상이에요. 후보 3개 보여드릴 테니 골라주세요. 한 번 고르시면 다음부턴 바로 보여드릴게요."

---

## 2. `headless-auth` — Codespaces / no-DISPLAY environment

**When it fires:** user says "로그인해줘" but the environment can't open a browser (`$CODESPACES=true`, no `$DISPLAY`, no `open` command on PATH, or `$SSH_TTY` set).

### Flow

1. **Detect headless.** SessionStart hook writes the result to `~/.cache/axhub-plugin/env.json`:
   ```json
   {
     "headless": true,
     "reason": "CODESPACES env var detected",
     "browser_available": false
   }
   ```
   Detection logic (in helper, ordered):
   - `$CODESPACES` set → headless
   - `$SSH_TTY` set AND no `$DISPLAY` → headless
   - macOS: `command -v open` missing → headless
   - Linux: `command -v xdg-open` missing AND no `$DISPLAY` → headless

2. **NEVER auto-launch browser.** On exit 65 in headless mode, do NOT call `axhub auth login` directly (it would block trying to open a browser).

3. **Present token-file paste flow** with explicit Korean copy-from-laptop instructions:
   ```
   잠깐만요. 지금 환경 (Codespaces 또는 SSH) 에서는 브라우저를 못 열어요.
   대신 별도 노트북에서 토큰을 받아 여기에 붙여넣어 주세요.

   1단계 (브라우저 있는 노트북에서):
     터미널을 열고 다음을 실행하세요 →
       axhub auth login                                 # OAuth 로그인
       security find-generic-password -s axhub -w       # macOS keychain
       # Linux:    secret-tool lookup service axhub
       # Windows:  axhub-helpers token-init 가 PowerShell + Add-Type 단일 호출로 자동 처리

   2단계 (출력된 'go-keyring-base64:eyJ…' 한 줄을 복사하세요. helper 가 base64 decode → access_token 추출합니다.)

   3단계 (지금 이 환경에서):
     아래 입력창에 그 token blob 을 붙여넣어 주세요. 0600 으로 안전하게 저장할게요.
     (대안: 1단계 노트북에서 axhub_pat_... 평문 토큰을 이미 알고 있다면
            export AXHUB_TOKEN=axhub_pat_... 로 바로 우회 가능)
   ```

4. **Receive token via AskUserQuestion (text input).** Save to `~/.config/axhub/token` with mode 0600:
   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers token-install --from-stdin
   # Internally: umask 077; cat > ~/.config/axhub/token; chmod 600
   ```

5. **Verify.** Run `axhub auth status --json --token-file ~/.config/axhub/token`. On success, set `$AXHUB_TOKEN_FILE` for the session and continue with the original intent.

6. **Never echo the token.** Hooks must redact any `axhub_pat_*` pattern from `tool_response` before classification (PLAN E7).

### Korean reassurance

> "토큰은 이 환경에만 저장됩니다. 다른 사람과 절대 공유되지 않아요. 종료하시면 토큰만 따로 지우실 수도 있어요 (`'토큰 지워줘'` 라고 말씀하시면 됩니다)."

---

## 3. `version-skew` — CLI too old or too new

**When it fires:** SessionStart helper compares `axhub --version` against `MIN_CLI_VERSION` and `MAX_CLI_VERSION` constants baked into the plugin.

### 3a. Too old (CLI < MIN)

1. **Hard-stop destructive operations.** Helper writes `~/.cache/axhub-plugin/cli-too-old` sentinel; PreToolUse hook reads it and **denies all destructive bash commands** (`deploy create`, `update apply --force`, `auth login` for OAuth flow).

2. **Render Korean upgrade prompt** at session start:
   ```
   잠깐만요. axhub CLI 버전이 너무 오래됐어요.

   현재 버전:  v<CURRENT>
   필요 버전:  v<MIN_CLI_VERSION> 이상

   안전을 위해 배포·업데이트 명령은 잠시 막아둘게요.
   먼저 axhub CLI를 업그레이드해주세요.

   업그레이드 명령:
     brew upgrade axhub          # macOS Homebrew
     # 또는 회사 IT가 안내한 방법으로

   업그레이드 후 새 터미널을 여시면 됩니다.
   ```

3. **Read-only commands stay allowed** (`apps list`, `apis list`, `auth status`, `deploy status`, `deploy logs`) so the user can still inspect their environment.

4. **Re-check on next SessionStart** — sentinel auto-clears when version condition is satisfied.

### 3b. Too new (CLI ≥ MAX)

1. **Warn but do not block.** Helper writes `~/.cache/axhub-plugin/cli-too-new` sentinel.

2. **Render Korean self-upgrade nudge:**
   ```
   잠깐만요. axhub CLI 가 플러그인보다 더 최신 버전이에요.

   CLI 버전:    v<CURRENT>
   플러그인 호환: v<MIN_CLI_VERSION> ~ v<MAX_CLI_VERSION>

   대부분 잘 돌아가지만, 새 CLI 의 명령이 플러그인에 아직 안 들어왔을 수 있어요.
   플러그인을 같이 최신으로 올릴까요?
   ```

3. **AskUserQuestion:**
   ```json
   {
     "question": "axhub 플러그인을 최신으로 업그레이드할까요?",
     "options": [
       {"label": "네, 업그레이드", "value": "upgrade", "description": "Claude Code 플러그인 마켓플레이스에서 axhub 플러그인 업그레이드"},
       {"label": "지금은 그대로", "value": "skip", "description": "현재 버전 유지 (다음 세션에 다시 안내)"},
       {"label": "다시 묻지 않기", "value": "mute", "description": "이 버전 조합에서는 더 이상 안내 안함"}
     ]
   }
   ```

4. **On upgrade pick**, surface the marketplace command for the user to run; do not auto-execute (plugin self-modification is out of scope for v0.1).

---

## 4. `watch-narration` — humanized status streaming

**When it fires:** after `axhub deploy create` succeeds, the skill auto-chains to `axhub deploy status dep_<ID> --watch --json`. The raw NDJSON tick stream is invisible to the vibe coder (DX-8 finding).

### Flow

1. **Start watching:**
   ```bash
   axhub deploy status dep_<DEPLOY_ID> --app <APP_ID> --watch --json
   ```
   Stream is NDJSON, one event per line. Parse via `jq -c`.

2. **Emit Korean reassurance every ~30s** based on elapsed time AND latest `phase` field. Do NOT just echo raw JSON.

   | Elapsed | Phase | Korean message |
   |---|---|---|
   | 0s | `queued` | "배포 요청 받았어요. 잠시 후 빌드 시작합니다 (정상)" |
   | ~30s | `building` | "30초 경과, 빌드 시작했어요 (정상)" |
   | ~1m | `building` | "1분 경과, 빌드 중이에요 (정상). 보통 2~3분 정도 걸려요" |
   | ~2m | `pushing_image` | "2분 경과, 이미지 푸시 중이에요 (정상). 거의 다 왔어요" |
   | ~2m30s | `starting_container` | "컨테이너 시작 중 (조금 더 기다려주세요)" |
   | ~3m | `health_check` | "헬스체크 중. 마지막 단계예요" |
   | ≥4m | any non-terminal | "4분이 넘어가네요. 이 앱은 평소보다 좀 오래 걸리는 편인가봐요. 계속 지켜볼게요" |
   | ≥7m | any non-terminal | "7분 경과. 빌드가 멈춘 것 같으면 '왜 멈췄어' 라고 물어보세요. 로그 까서 보여드릴게요" |
   | terminal | `succeeded` | "터미널 상태 도달 — 결과 확인 중..." → trigger exit 0 success template |
   | terminal | `failed` | "배포 실패. 로그 가져올게요." → fetch `axhub deploy logs` and render with empathy |

3. **Throttle.** Emit at most one narration line per 25s, even if multiple phase transitions happen quickly. Always emit terminal-state narration immediately (no throttle).

4. **Never silent.** If 60s pass with no NDJSON event, emit: "조용하네요. 서버 응답 기다리는 중입니다 (정상). 30초 후 다시 알려드릴게요."

5. **User interrupt.** If user types "그만 봐" / "그만" / "stop watching" / "충분해", kill the watch process and report current phase. The deploy continues server-side regardless.

---

## 5. `deployment_in_progress` — refuse-retry flow

**When it fires:** `axhub deploy create` returns exit 64 with `validation.deployment_in_progress` (PLAN §3.2).

### Flow

1. **Parse the error response** — extract `in_flight_deploy_id` from the JSON error body.

2. **Render the empathy template** from `error-empathy-catalog.md` (the `exit 64 + validation.deployment_in_progress` entry).

3. **Offer one-keystroke watch.** AskUserQuestion:
   ```json
   {
     "question": "진행 중인 배포(<IN_FLIGHT_DEPLOY_ID>)를 함께 지켜볼까요?",
     "options": [
       {"label": "네, 함께 지켜보기", "value": "watch", "description": "그 배포 끝날 때까지 자동으로 알려드려요"},
       {"label": "5분 후에 다시", "value": "later", "description": "5분 후에 자동으로 다시 시도 가능 여부 확인"},
       {"label": "지금은 취소", "value": "abort", "description": "아무것도 하지 않고 종료"}
     ]
   }
   ```

4. **On "watch":** route to flow #4 (`watch-narration`) with `<IN_FLIGHT_DEPLOY_ID>`. **Do NOT call `axhub deploy create` again** under any circumstances. PreToolUse hook will deny a re-invocation of `deploy create` for the same `{app, branch, commit}` within 30s as a defense-in-depth check.

5. **On "later":** schedule a single follow-up after 5 min via `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers schedule --action recheck-deploy --app <APP_ID> --after 300`. When fires, run `axhub deploy status` on the in-flight ID; if terminal, notify user and offer to re-run their original deploy intent.

6. **NEVER auto-retry create.** This is the single most important guarantee. Phase 3 E2 explicitly identified retry-on-create as the trust killer.

### Korean copy

> "다른 배포가 진행 중이에요. 당신 앱은 안전합니다. 5분만 기다리면 자동으로 다음 배포가 가능해요. 새로 배포하지 말고 진행 중인 그 배포를 같이 지켜볼까요? 끝나면 알려드릴게요."

---

## 6. `profile-mismatch` — environment vs intent disagreement

**When it fires:** user says "prod에 배포해" (or implies production via "라이브", "운영", "실서비스") but `$AXHUB_PROFILE=staging` (or vice versa). Detection in helper resolve step.

### Flow

1. **Detect mismatch.** Helper compares user's spoken environment against `$AXHUB_PROFILE`:
   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent deploy --user-utterance "$ARGS" --json
   # Returns: {
   #   "profile_intent": "production",
   #   "profile_env": "staging",
   #   "mismatch": true
   # }
   ```

2. **Detection lexicon:**

   | User says | profile_intent |
   |---|---|
   | prod, 프로덕션, production, 라이브, 운영, 실서비스, 본 서비스, 진짜 환경 | production |
   | staging, 스테이징, dev, 개발, 테스트, qa, 사내 | staging |
   | (no env keyword) | null — use `$AXHUB_PROFILE` as-is, no mismatch |

3. **Render disambiguation card.** AskUserQuestion:
   ```
   잠깐만요. 환경이 헷갈려요.

   현재 설정 환경:  staging  ($AXHUB_PROFILE)
   말씀하신 환경:   production ("prod에 배포해")

   어떻게 할까요?
   ```

   Options:
   ```json
   {
     "question": "어느 환경으로 배포할까요?",
     "options": [
       {"label": "production 으로 전환하고 진행", "value": "switch_prod", "description": "이번 배포만 --profile production 사용. 환경변수는 안 바뀌어요."},
       {"label": "staging 으로 진행 (제 말 잘못 들은 거)", "value": "stay_staging", "description": "원래 설정대로 staging 에 배포"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

4. **On `switch_prod`:** override profile for this single invocation:
   ```bash
   axhub deploy create --profile production --app <APP_ID> --branch <BRANCH> --commit <SHA> --json
   ```
   Do NOT mutate `$AXHUB_PROFILE` globally — this is a single-shot override.

5. **On `stay_staging`:** proceed with `$AXHUB_PROFILE` (no `--profile` flag needed).

6. **On `abort`:** return immediately, do not call any axhub command.

7. **Production extra confirmation.** If `switch_prod` chosen AND target is production AND it's the user's first prod deploy this session, render an additional confirmation row in the deploy-preview card:
   ```
   ⚠️ 운영(production) 환경 첫 배포입니다. 진짜 진행하시겠어요?
   ```
   This is on top of the standard preview card from `error-empathy-catalog.md`.

### Korean reassurance

> "환경이 두 개라서 헷갈리기 쉬워요. 이렇게 한 번씩 확인하면 사고 안 나요. 이번에 production 으로 전환해도 다음번엔 다시 staging 으로 돌아갑니다 (환경 설정은 안 건드려요)."

---

## Cross-flow rules

- **All flows preserve idempotency.** Multiple invocations of the same flow on the same trigger condition produce the same prompt — never silent retry.
- **All flows route through `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers`.** No skill markdown contains business logic; markdown is presentation only (E1/E3 fix).
- **All flows emit Korean by default.** English fallback only if `$LANG` does not start with `ko`.
- **All flows respect `--dry-run`.** If user originally invoked dry-run, every recovery path also stays in simulation mode (no destructive call).
- **All flow telemetry** writes to `~/.cache/axhub-plugin/recovery-events.ndjson` for the M1 corpus stratification (PLAN E6: read-only / mutation / safety risk classes).

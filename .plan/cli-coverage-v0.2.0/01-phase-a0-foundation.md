# Phase A0 — Helper Foundation

> Phase A0 = 모든 신규 SKILL 동작 가능하게 만드는 helper / routing / consent 인프라. Phase B 진입 전 hard gate.

---

## 목표

- CLI v0.10.x 호환 가능 (현 preflight 가 v0.2.0 cli_too_new 차단 = BLOCKER)
- 17 SKILL trigger 어구가 hook 단계에서 routing 됨 (Rust only (TS shadow 박멸 v0.2.0))
- 11 신규 destructive mutation 이 consent gate 통과 (HMAC binding)
- registry/baseline/lexicon 모두 18 SKILL 기준 lock
- examples repo 의 templates.json manifest 가 helper fetch 가능

## Tasks (12)

### A0-1. preflight MAX_AXHUB_CLI_VERSION 업데이트

- 파일: `src/axhub-helpers/preflight.ts:28`
- 변경: `MAX_AXHUB_CLI_VERSION = "0.2.0"` → `"0.11.0"`
- 사유: CLI v0.10.2 가 현재 cli_too_new 로 차단됨. 모든 신규 SKILL 가 진입조차 못 함.

### A0-2. preflight Rust parity

- 파일: `crates/axhub-helpers/src/preflight.rs`
- 변경: TS 와 동일 MAX_VERSION + 같은 semver 검증 logic
- 검증: `cargo test preflight::` PASS

### A0-3. prompt-route enum 11→17 (TS)

- 파일: `src/axhub-helpers/prompt-route.ts:19-30`
- 변경: `PromptRouteIntent` union 에 `init / env / github / open / whatsnew / profile / admin` 7개 추가
- ROUTES 배열 41-211 에 7 entry 추가, 각 entry = `{intent, skill, label, needsPreflight, patterns: RegExp[], guidance}`
- nl-lexicon collision 방지: "환경" 단독 → clarify, "환경변수" → env, "환경 점검" → doctor, "회사 endpoint" → profile, "내 앱 셋업" → admin, "axhub.yaml" → init, "결제 앱 만들어줘" → init

### A0-4. prompt-route Rust parity

- 파일: `crates/axhub-helpers/src/main.rs:258-555`
- 변경: TS ROUTES 와 동일 7 신규 route 추가. Rust substring matching paradigm 유지.
- **codex ENG finding E1**: shipped binary = Rust. TS 만 변경하면 routing 동작 안 함.

### A0-5. benchmark prompt-route 50ms gate

- 파일: `scripts/benchmark-hooks.ts`
- 변경: 신규 case 3개:
  - `prompt-route-no-preflight` (whatsnew / open / profile current)
  - `prompt-route-needs-preflight` (deploy / env list / github connect)
  - `prompt-route-clarify-fallback` (모호 어구)
- 각 case 가 fake `AXHUB_BIN` 환경변수로 flake 방지
- Rust binary hit (TS shadow X)
- p95 < 50ms 게이트, 위반 시 CI fail

### A0-6. ConsentBinding generic context (6 파일)

**codex ENG finding E2** — migration scope = 6 파일:

#### A0-6a. `src/axhub-helpers/consent.ts:30-40`
- `ConsentBinding` interface 에 `context?: Record<string, string>` (optional) 추가
- action union 4 → 15:
  ```typescript
  action:
    | "deploy_create" | "update_apply" | "deploy_logs_kill" | "auth_login" // 기존
    | "env_set" | "env_delete"
    | "apps_create" | "apps_update" | "apps_delete"
    | "github_connect" | "github_disconnect"
    | "deploy_cancel"
    | "profile_add" | "profile_use"
    | "apis_call"
    | "admin_setup_team"  // admin SKILL 의 team 자동 생성
  ```

#### A0-6b. `src/axhub-helpers/consent.ts:224-232`
- verify 가 `context` 누락 시 `{}` backfill
- **codex ENG finding E3**: in-flight token (60s TTL) backwards compat. legacy mint = no context, new verify 가 기존 토큰 fail 시키지 않음.
- 1 release 후 deprecate (v0.3.0 에서 required 로 승격)

#### A0-6c. `src/axhub-helpers/index.ts:80-108`
- action validation 확장 (Zod schema 또는 hand-rolled enum check)

#### A0-6d. `src/axhub-helpers/index.ts:315-322`
- preauth binding 구성 시 신규 11 action 별 context 키 명시

#### A0-6e. `crates/axhub-helpers/src/consent/jwt.rs:13-21, 34-45`
- Rust binding/claims struct 에 `context: HashMap<String, String>` 필드 추가 (optional via `#[serde(default)]`)

#### A0-6f. `crates/axhub-helpers/src/main.rs:191-224`
- Rust preauth binding 구성 확장

#### A0-6g. `crates/axhub-helpers/src/consent/parser.rs:144`
- `match_known_intent` 4 → 15 case

### A0-7. registry.json 6 SKILL channel 추가

- 파일: `tests/fixtures/ask-defaults/registry.json`
- 신규 channel:
  - `init`: stack 선택 → `default_subprocess_action: "abort"` (subprocess 자동 stack 선택 금지)
  - `env`: secret value 입력 → `abort` (subprocess secret 입력 금지)
  - `github`: account/repo 선택 → `abort` (subprocess auto-pick 금지)
  - `profile`: endpoint allowlist warn → `abort` (non-allowlist domain 자동 진행 금지)
  - `admin`: team/member 자동 생성 → `abort`
  - `open`: skip (read-only)
  - `whatsnew`: skip (read-only)

### A0-8. nl-lexicon negative tests (TS+Rust)

- 파일: `tests/axhub-helpers.test.ts` + `crates/axhub-helpers/tests/route_collision.rs`
- 신규 5 negative case:
  - "환경" 단독 → clarify (env/profile 모호)
  - "환경변수 뭐 있어?" → env (NOT clarify, NOT doctor)
  - "환경 변수 확인" → env
  - "환경 점검해" → doctor (NOT env)
  - "회사 endpoint 바꿔" → profile

### A0-9. e2e-claude-cli-registry baseline 13→20

- 파일: `tests/e2e-claude-cli-registry.test.ts:48-71`
- baseline keys = 2 메타 + 18 SKILL slug (init/env/github/open/whatsnew/profile/admin/apis/apps/auth/clarify/deploy/doctor/logs/recover/status/update/upgrade)

### A0-10. error-empathy-catalog 신규 exit code

- 파일: `references/error-empathy-catalog.md`
- 신규 4-part Korean template:
  - env `prod_force_required` (exit 64)
  - env `prod_confirm_mismatch` (exit 64)
  - github `install_not_found` (exit 67) → AppHub install URL 안내
  - github `git_connection_already_exists` (exit 64) → disconnect 후 재연결 ask
  - github `confirm_slug_mismatch` (exit 64)
  - open `no_axhub_yaml` (exit 64) → init 안내
  - profile `endpoint_not_in_allowlist` (warn, plugin-side)
  - apis `call_consent_required` (exit 65/64) → consent mint 안내
  - admin `team_already_exists` (exit 64)

### A0-11. nl-lexicon baseline lock 재캡처

- 파일: `references/nl-lexicon.md` + `.omc/lint-baselines/skill-keywords.json`
- 신규 7 SKILL trigger 어구 lock
- `bun run lint:keywords --check` 베이스라인 재캡처

### A0-12. Phase A0 게이트 검증

- `bun test` ≥600 PASS / 0 FAIL
- `bunx tsc --noEmit` clean
- `cargo test --workspace` clean
- `bun run lint:tone --strict` 0 err
- `bun run lint:keywords --check` PASS
- `bun run skill:doctor --strict` exit 0
- `scripts/benchmark-hooks.ts` p95 < 50ms

## Feature Flag (codex ENG finding E4 mitigation)

`plugin.json:features.beta_skills:false` default OFF.

`AXHUB_PLUGIN_BETA=1` env 또는 flag flip 시 17 SKILL routing 활성. Phase A0 만 머지된 intermediate state 에서는 신규 6 SKILL 의 prompt-route 가 OFF — `clarify` SKILL 로 fallback.

Phase D 의 release chain 에서 default ON 으로 flip.

## Validation gate before Phase B

다음 7 항목 PASS 필수:
- [ ] preflight MAX 0.11.0, CLI v0.1.0~v0.10.2 모두 in_range
- [ ] prompt-route TS+Rust 17 enum, 5 negative test PASS
- [ ] ConsentBinding migration 6 파일, in-flight token backwards compat
- [ ] benchmark prompt-route p95 < 50ms
- [ ] registry baseline 13→20 PASS
- [ ] error-empathy-catalog 7 신규 entry
- [ ] examples repo templates.json manifest 가 helper list-templates 로 fetch 가능 (Phase A0-bootstrap 의존)

## Effort

- 코드 변경: ~6시간 CC+gstack
- 테스트 작성: ~3시간
- 검증/CI fix: ~1시간
- **Total: ~10시간**

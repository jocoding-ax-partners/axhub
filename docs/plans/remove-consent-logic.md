# Plan: consent 로직 전체 제거 (mutation-authorization consent clean delete)

> 생성: `/ouroboros:interview` → `/plan-ceo-review` (HOLD SCOPE) → codex outside-voice 정정 반영
> Branch: `fix/hide-route-hint-from-user` | Base: `main` | Repo: jocoding-ax-partners/axhub
> Mode: HOLD SCOPE · Approach: B (한 PR, 단계 커밋, behavior-green 순서)

## 1. 배경 — 왜 지금, 무엇이 이미 되어 있나

axhub 의 "consent" 는 데이터 수집 동의가 아니라 **변경(mutation) 작업을 PreToolUse 시점에 막던 HMAC-JWT 권한 게이트**예요 (`Once`/`AllowSession`/`AllowAlways`/`Deny` 4-variant). deploy 생성·app 생성·app suspend/resume/fork·DB 쓰기·`auth_login` 등 destructive axhub 명령을 토큰 검증으로 게이트했어요.

**핵심: enforcement 는 이미 #170 에서 꺼졌어요.**
- `6a47845c fix: consent preauth-check 게이트를 hooks.json 에서 제거 (#170)` 이 PreToolUse `preauth-check` 훅 등록을 hooks.json 에서 제거.
- 커밋 의도: "destructive axhub 명령이 consent 토큰 검증 때문에 차단되던 문제를 풀려고. consent-mint/verify 코드와 바이너리 서브커맨드는 그대로 두되 게이트가 더는 실행되지 않아요."
- 결과: `cmd_preauth_check` (main.rs:1144) 는 **dead code**, 12개 skill + main.rs prompt-route 가 **아무도 검증 안 하는 토큰을 발급 중**, profile-poisoning 방어도 이미 비활성.

따라서 이 작업은 **새 보안 게이트 제거가 아니라 #170 이 의도적으로 남긴 dead-code cleanup 완성**이에요. 다만 "제품 보안 의미의 신규 게이트 제거 위험"은 낮아도, 삭제 대상 심볼의 코드 blast radius 는 GitNexus 기준 HIGH/CRITICAL 이에요(§12.1). 구현 전·중에는 HIGH/CRITICAL 영향 범위를 사용자에게 명시하고 진행해요.

동기: first-run / vibe-coding UX 의 마찰 제거 + 부채 3270줄 정리.

## 2. 결정 로그

| # | 결정 | 출처 |
|---|------|------|
| 범위 | mutation-authorization consent 를 **완전 제거**, 결과(파괴적 작업 무게이트, profile-poisoning 방어 소멸) 명시 인지 후 확정 | interview |
| 동작 | 게이트 + gated 흐름 둘 다 제거 (always-run 아님) | interview |
| 방식 | **Clean delete** (no-op stub 아님) | interview |
| 분할 | **한 PR**, 단계 커밋 | D1 |
| 모드 | **HOLD SCOPE** | D2 |
| 유틸 이전 | 공유 secure-state-file 유틸을 **`runtime_paths.rs` 로 이동** | D3 |
| skill UX | **preview 카드 유지, 승인(mint) 단계만 제거** | D4 |
| outside voice | codex 독립 리뷰 실행 → 8 findings + GitNexus 1 (모두 반영) | D5 |
| state_root | **codex 수용**: `.../state/axhub` 의미 보존, `state_dir`(axhub-plugin)와 **합치지 않음** (dedup 철회) | D6 |
| 품질 토글 | `cmd_quality_consent`(`axhub consent --enable/--disable`) + review/TDD gate **유지** | interview |
| 인증 | 로그인(auth) 자체 **유지** — `auth_login` 의 consent 게이트만 제거 | interview |
| 기존 on-disk | 발급된 토큰 파일·audit ledger consent 항목 **방치(inert)** | interview |
| DoD | 엄격 + allowlist (§9) | interview + D5 |

## 3. 범위

### 제거
1. `crates/axhub-helpers/src/consent/` 모듈 전체 (3270줄: `decision.rs` `jwt.rs` `key.rs` `parser.rs` `schema.rs` `mod.rs`) — **단, key.rs 의 generic 유틸은 §4 로 이전 후 삭제**.
2. CLI 서브커맨드: `consent-mint`, `consent-mint-app-lifecycle`, `consent-verify` (`main.rs` 디스패치 + `cli/mod.rs` + `cli/args/mod.rs` + USAGE 문자열).
3. `cmd_preauth_check` (main.rs:1144, dead) + `preauth-check` CLI 디스패치 (cli/mod.rs:110,210,320) + `hooks/axhub-helpers.sh` 의 `preauth-check)` 분기.
4. **main.rs prompt-route emission (codex #1 — 놓쳤던 consumer):** main.rs:1594 + main.rs:1908 의 app-lifecycle prompt-route 가 agent 에게 `axhub-helpers consent-mint-app-lifecycle ...` 실행을 지시. 이 emission 을 제거(승인 단계 삭제, preview 문구는 D4 대로 유지). main.rs:1257/1878 의 "consent-mint 노출 금지" prose 는 §9 allowlist 로 다룸.
5. `bootstrap.rs`: `ConsentRequiredAppsCreate` / `ConsentRequiredDeployCreate` state, `ConsentBinding` 사용처, `emit_consent_synthesized_by_helper` 등 consent 흐름.
6. `routing.rs` / `quality_gate.rs` / `preflight.rs` / `sync.rs` 의 consent 참조. **diagnose (codex #5 정정):** 실제 consent import 는 `diagnose/learning.rs:15` + `diagnose/hitl.rs:25` (§4 로 이전) + `diagnose/mod.rs` 주석. `diagnose/probe/env_var.rs` 는 주석 매치일 뿐 — 코드 변경 불필요(주석만 정리).
7. **skill rewire — 정확한 12개 (codex #2 — 예시만 들어 누락 위험이었음):** `apps` `app-lifecycle` `apis` `auth` `deploy` `env` `github` `migrate` `profile` `publish` `recover` `tables`. **단, 동일 패턴 아님 (codex #3):**
   - Bash mint 블록 보유 (블록 제거 + preview 유지): deploy, tables, auth, apps, app-lifecycle, github, migrate, publish, recover
   - prose-only / shell 블록 없음 (control-flow·문구 rewrite): env, profile
   - "consent-mint 쓰지 마라"(routes to data) — 호출 아님, 참조 문구만 정리: apis
8. consent 테스트/벤치/fuzz: `cli_e2e.rs`, `phase_parity.{rs,md}`, `state_classification_test.rs`, `diagnose_layering_test.rs`, `ci_coverage_gate.rs` 의 consent 케이스, **+ `tests/e2e/claude-cli/cases/31-consent-mint-sentinel.case.sh` (GitNexus 발견 — 누락했던 e2e sentinel)**.
   - **추가 누락 방지:** `tests/e2e/claude-cli/cases/23-preauth-check-deny.case.sh`, `tests/e2e/claude-cli/matrix.jsonl`, `tests/e2e/claude-cli/lib/t2-helper.sh`, `tests/e2e/claude-cli/README.md`, `tests/hook-fixtures/README.md`, `tests/hooks-kill-switch.test.ts`, `crates/axhub-helpers/tests/hook_safety_cli.rs`, `fuzz/fuzz_targets/parser.rs`, `scripts/benchmark-hooks.ts`, `tests/deploy-skill-wire-up.test.ts`, `tests/tables-skill-contract.test.ts`, `tests/windows-compat-docs.test.ts`, `tests/prompt-route-karpathy.test.ts`, corpus/baseline JSONL 의 consent/preauth/HMAC 기대값을 함께 삭제·갱신해요.
   - **⚠️ E1 (eng-review): 재배치 util 의 테스트는 삭제 말고 이전.** `state_root` 인라인 테스트(`consent/key.rs:205-244`: `state_root_ignores_empty_xdg_and_uses_home`, `state_root_fallback_is_stable_and_absolute`) → `runtime_paths.rs` `#[cfg(test)]` 로 이동. `write_private_file_no_follow` symlink-no-follow 테스트(`phase_parity.rs:3036`) → 삭제 말고 import 를 `runtime_paths::` 로 retarget 후 유지. 안 그러면 살아남는 util 의 커버리지 silent 하락.
9. `audit_ledger.rs`: consent 결정 로깅 제거 (ledger 자체는 유지).
10. 문서: `docs/HOOKS.md`, hooks.json line 2 description 의 stale "consent" 표현.
    - **⚠️ DX2 (devex-review): user/AI-facing 문서가 consent 를 안전장치로 광고 — scope 에 포함 필수.**
      - `README.md` (front door, P1): line 48 "핵심 안전장치 … HMAC consent 게이트가 막아요" → **재작성**(preview 카드는 유지[D4], HMAC consent 문구 제거, CC native 권한을 잔존 안전망으로 명시). line 223 아키텍처 다이어그램 `[consent]` 단계 제거. line 240 "HMAC consent 토큰" 기능 설명 제거.
      - `crates/axhub-helpers/assets/AXHUB.md`:90 (shipped AI context) — "first live read consent 뒤에만" 표현 정리(안 그러면 AI 가 consent-mint 재시도).
      - `docs/architecture.ko.md`, `docs/routing.md`, `docs/MIGRATION_v0.6_to_v1.0.md`, `docs/migration-gate.md`, `docs/org-admin-rollout.ko.md`, `docs/pilot/admin-rollout.ko.md`, `docs/pilot/onboarding-checklist.md`, `docs/marketing/landing-page.ko.md`, `docs/marketing/outreach-email.ko.md`, `docs/baseline-measurement.md` 의 현재 제품 설명성 consent 참조 정리.
      - slash command docs: `commands/{apps,deploy,배포,login,doctor,logs,status}.md` 의 "slash invocation does not bypass HMAC consent" 계열 문구 제거/재작성.
      - plugin metadata: `.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json` description 의 "HMAC-bound consent gates"/"consent gates" 제거.
      - skill reference docs: `skills/deploy/references/telemetry.md` 의 `preauth_check_deny`/`consent_mint` telemetry 설명을 삭제하거나 historical 로 명확히 격리.
      - 오래된 PRD/ADR/spec/CHANGELOG/`.omc` 류 역사 기록은 "현재 동작을 약속하지 않는 historical" 로 남길 수 있지만, DoD allowlist 에 명시해야 해요.
      - 근거: consent 제거 후 이 문서들이 존재하지 않는 게이트를 약속하면 DX credibility(특성 #2) 깨짐 — vibe-coder 가 README 를 믿고 무방비 배포.

### 유지
- `cmd_quality_consent` (`axhub consent --enable/--disable` → `quality-consent.json` 의 `megaskill_enabled` 토글) + hooks.json `tdd-inject` / review gate. consent/ 모듈 비의존이라 독립 보존.
- 인증/로그인 흐름 (CLI auth) — `auth_login` 의 consent 게이트만 사라지고 로그인 자체는 동작.
- `audit_ledger`, `diagnose/*` (consent 비-의존 부분), `runtime_paths` (유틸 이전 대상).

### Out of scope (NOT in scope)
- consent 를 대체할 새 권한 모델 설계 — 인터뷰서 "권한 유지" 명시 거부.
- 기존 on-disk 토큰/ledger 항목 정리 — "방치(자연 소멸)" 결정.
- review/TDD/quality 게이트 동작 변경 — 별개 기능, 손대지 않음.
- `observability.rs` 의 `hmac_hex` — consent/jwt 와 독립, 그대로.
- `state_dir`(axhub-plugin) 중복 정리 — D6 으로 철회 (state_root 과 다른 경로라 합치면 위험).

## 4. Architecture — 유틸 이전 (선행 작업)

`consent/key.rs` 는 consent 전용이 아닌 **generic secure-state-file 유틸**을 export 하고, diagnose·audit_ledger·main 이 의존해요. consent/ 삭제 전에 `runtime_paths.rs` 로 이전 필수 (D3).

**이전 대상 (generic, 유지):**
- `state_root()` — diagnose/learning.rs, audit_ledger.rs 사용. **⚠️ D6: `.../.local/state/axhub` 의미를 그대로 보존. `state_dir()`(`.../axhub-plugin`)와 합치지 말 것** — 다른 경로라 합치면 기존 audit/학습 데이터가 silent 이동.
- `write_private_file_no_follow()` — diagnose/hitl.rs, main.rs:750 사용
- `set_private_dir_mode()` — **main.rs:749 직접 호출 (codex #4 — transitive 아님, 직접 caller)**
- 그 transitive: `read_private_file`, `set_private_file_mode`, `runtime_root`(consent 전용이면 삭제 — 호출처 재확인), `FILE_MODE_PRIVATE`, `DIR_MODE_PRIVATE`

**같이 삭제 (consent 전용):** `HMAC_KEY_BYTES`, `hmac_key_path`, `token_file_path`, `pending_token_file_path`, `load_or_mint_key`, `session_id`(consent binding용).

```
BEFORE                                    AFTER
consent/{decision,jwt,key,parser,schema}  consent/ ──────────────▶ DELETED
  ◀── bootstrap(ConsentBinding,            runtime_paths.rs ◀── (gains) state_root[.../state/axhub
      ConsentRequired*)                      의미 보존] + write_private_file_no_follow +
  ◀── main(consent-mint*/-verify,            set_private_dir_mode + private-file helpers
      cmd_preauth_check,                      ▲ diagnose/{hitl,learning}, audit_ledger, main:749/750
      prompt-route 1594/1908)               bootstrap/main/cli/routing/quality_gate/
  ◀── cli, routing, quality_gate,            preflight/sync ── consent refs 제거
      preflight, sync, diagnose/{hitl,       12 skill + prompt-route ── mint 단계 제거(preview 유지)
      learning}, audit_ledger
```

## 5. 구현 단계 (Approach B — 한 PR, 4 커밋, behavior-green 순서)

> codex #7: CLI 를 먼저 지우면 중간 커밋에서 skill·prompt-route 가 제거된 명령을 호출해요. 그래서 **caller rewire 를 CLI 삭제보다 먼저** 둬서 모든 커밋이 build-green + behavior-green 이 되게 재정렬.

**커밋 1 — 유틸 이전 (빌드 안전 선행):**
- `consent/key.rs` 의 generic 유틸 → `runtime_paths.rs` 이동 (state_root 의미 보존, set_private_dir_mode 포함).
- 임포터 경로 갱신: `diagnose/{learning,hitl}.rs`, `audit_ledger.rs`, `main.rs:749/750`, `consent/{key,jwt}.rs`(아직 존재) → `crate::runtime_paths::...`.
- ✅ verify: `cargo build` green, `cargo test` (경로 회귀 — audit_ledger/diagnose 가 같은 디렉터리 resolve).

**커밋 2 — caller rewire (CLI 는 아직 존재, 호출만 끊음 → behavior-green):**
- 12 skill rewire: mint+승인 블록 제거, **preview 카드 유지(D4)**. env/profile 은 prose rewrite, apis 는 참조 문구 정리 (§3.7 분류대로).
- main.rs:1594/1908 prompt-route 의 `consent-mint-app-lifecycle` emission 제거 (preview 문구 유지).
- `bootstrap.rs` ConsentRequired* / ConsentBinding 흐름 제거 → 게이트 없이 바로 action.
- `commands/*.md` 는 아직 커밋4 문서 정리로 둘 수 있지만, 커밋2 종료 시 **실행 가능한 caller**(`skills/*`, `main.rs` prompt-route, `bootstrap.rs`) 안에는 `consent-mint*` 호출이 없어야 해요.
- 이 시점: consent CLI 서브커맨드는 존재하나 **아무도 호출 안 함**.
- ✅ verify: `cargo build` green, `bun run skill:doctor --strict`, `lint:tone --strict`, `bun test`, 수동 e2e 1건(deploy/app create 가 preview 후 프롬프트 없이 실행).

**커밋 3 — dead 코드 삭제 (이제 미참조):**
- `consent/` 모듈 삭제 (이전 안 한 나머지 전부).
- `cmd_consent_mint` / `_app_lifecycle` / `cmd_consent_verify` / `cmd_preauth_check` + main 디스패치 + cli/mod.rs + cli/args/mod.rs + USAGE + `hooks/axhub-helpers.sh` 의 `preauth-check)` 분기 삭제.
- `routing/quality_gate/preflight/sync` consent 참조 + `audit_ledger` consent 로깅 제거.
- ✅ verify: `cargo build` + `cargo clippy` clean.

**커밋 4 — 테스트 + 문서:**
- consent 테스트 삭제/갱신 (`cli_e2e`, `phase_parity`, `state_classification`, `diagnose_layering`, `ci_coverage_gate`, `hook_safety_cli`, `tests/e2e/.../{23-preauth-check-deny,31-consent-mint-sentinel}.case.sh`, `tests/e2e/claude-cli/{matrix.jsonl,lib/t2-helper.sh,README.md}`, `tests/hook-fixtures/README.md`).
- `manifest.test.ts` / `phase26-quality-surfaces.test.ts` / `deploy-skill-wire-up.test.ts` / `tables-skill-contract.test.ts` / `windows-compat-docs.test.ts` / `prompt-route-karpathy.test.ts` / corpus+baseline JSONL consent 단언 갱신 (이미 #170 일부 갱신됨).
- `fuzz/fuzz_targets/parser.rs` 는 삭제하거나 consent parser 삭제 후 남는 public parser 대상이 있으면 retarget 해요. consent parser 가 완전 삭제되면 `fuzz/Cargo.toml` 의 `[[bin]] parser` 도 함께 정리해요.
- `scripts/benchmark-hooks.ts` 의 `preauth-check` 벤치 제거/대체.
- `docs/HOOKS.md` + hooks.json line 2 description stale "consent" 제거.
- ✅ verify: `cargo test` 전체 + `bun test` + `bunx tsc --noEmit`.

> 주의 (codex #7/#8): 각 커밋은 **build-green + behavior-green** 이지만 **test-green 은 커밋 4 에서 달성** (consent 테스트 삭제가 거기 모임). 한 PR squash merge 기준이라 중간 커밋은 배포되지 않아요.

## 6. Error & Failure Registry

삭제 작업이라 **새 에러 경로 0**. 제거되는 에러 타입: `DecisionError`, `BindingSchemaError`, `MintFailed` 등 (consent 전용). 컴파일러가 잔여 `?`/match 를 강제로 잡아줘요.

| Codepath | Failure mode | Rescued? | Test? | User sees | Logged |
|---|---|---|---|---|---|
| (삭제) consent mint/verify | — | 제거됨 | 제거됨 | — | — |
| util 이전 후 audit_ledger/diagnose | state_root 경로 의미 변동 | 컴파일+테스트 | Y(커밋1) | 빌드/경로 회귀→CI | — |

**CRITICAL GAP: 없음** (silent 실패 신규 도입 없음). D6 으로 silent 경로 이동 위험 차단.

## 7. Security — accepted-risk (신규 리스크 아님)

- mutation 권한 게이트 소멸은 **#170 에서 이미 ship**, 인터뷰서 명시 수용. 이 PR 은 dead code 를 지울 뿐 **새 attack surface 0**.
- 잔존 안전망: ① CC native tool-permission 프롬프트 (bypassPermissions 모드 제외), ② **skill preview 카드(D4 유지)** — 파괴적 작업 직전 "뭐 할지" 노출.
- 소멸: headless/CI profile-poisoning 방어 (이미 #170 으로 비활성, 사용자 수용).

## 8. Tests & Coverage

- consent 테스트 삭제 → `ci_coverage_gate.rs` 커버리지 비율 변동 가능. **위험**: 게이트 threshold 흔들림.
- 대응: 커밋 4 에서 `cargo test` 로 게이트 통과 확인. 삭제된 코드만큼 분모도 줄어 중립이어야 함; fail 이면 기대값을 같은 커밋에서 갱신(코드/테스트 동시 삭제이므로 정당).
- e2e: `31-consent-mint-sentinel.case.sh` 삭제 (consent-mint 자체가 사라짐).

## 9. Deployment & Rollback / DoD

**Breaking change**: `consent-*` CLI 서브커맨드 제거. 호출자는 내부 skill+prompt-route(같은 PR rewire). 외부 자동화 직접 호출 가능성 low (내부 adapter binary). 릴리스 `refactor:` (0.x → minor), release workflow 자동.

**Rollback**: #170 은 1줄 복원이었지만 clean delete 후엔 `git revert <PR>` (3270줄 재추가). 사용자 수용.

**DoD (엄격):**
- [ ] `cargo build -p axhub-helpers` + `cargo test` 통과
- [ ] `cargo clippy` clean
- [ ] **consent/HMAC/preauth 잔존 = allowlist 외 0** (codex #8 확장). Allowlist: `cmd_quality_consent` 관련(`axhub consent`/`quality-consent.json`/`megaskill_enabled`), `observability.rs` 의 독립 `hmac_hex`, main.rs 의 "consent internals 노출 금지" 의도적 prose(필요 시), docs/ADR/PRD/spec/CHANGELOG/`.omc` 내 historical 언급. 검증:
  - `rg -n 'consent-mint|consent-verify|preauth-check|HMAC consent|HMAC-bound consent|consent token|consent 토큰|ConsentRequired|ConsentBinding|axhub_helpers::consent|use crate::consent' crates/axhub-helpers/src crates/axhub-helpers/tests fuzz skills commands hooks tests scripts README.md docs crates/axhub-helpers/assets .claude-plugin`
  - 결과를 allowlist 와 대조하고, 현재 동작을 약속하는 user/AI-facing 문구는 0 이어야 해요.
- [ ] deploy / app create / app suspend 가 **preview 후 consent 프롬프트 없이** 실행 (수동 e2e)
- [ ] 모든 hook exit 0 (fail-open 유지)
- [ ] `bunx tsc --noEmit` + `bun test` 통과
- [ ] `bun run skill:doctor --strict` + `lint:tone --strict` 통과

## 10. Long-Term Trajectory

- Reversibility 3/5 (git revert 가능하나 대형 삭제 재추가).
- 부채 순감 (3270줄 + 죽은 흐름 제거).
- 재도입 시 from-scratch (수용됨). 새 엔지니어가 half-dead consent 시스템에 헷갈릴 일 없어짐.

## 11. Implementation Tasks

- [ ] **T1 (P1, human ~3h / CC ~25min)** — runtime_paths — generic util 이전 + **그 테스트 동반 이전** (커밋1)
  - Surfaced by: §4 / D3 / D6 / codex #4 / **E1 (eng-review)**
  - Files: `consent/key.rs`→`runtime_paths.rs`, `diagnose/{learning,hitl}.rs`, `audit_ledger.rs`, `main.rs:749/750`; **테스트: `consent/key.rs:205-244` state_root 테스트 → runtime_paths `#[cfg(test)]`, `phase_parity.rs:3036` write_private 테스트 → import retarget**
  - Verify: `cargo build` green, `cargo test runtime_paths` (state_root + write_private 테스트가 새 위치에서 pass), 경로 resolve 불변 확인
- [ ] **T2 (P1, human ~4h / CC ~30min)** — callers — 12 skill + main.rs prompt-route(1594/1908) + bootstrap rewire, preview 유지 (커밋2, behavior-green)
  - Surfaced by: §5 / D4 / codex #1,#2,#3,#7
  - Files: `skills/{apps,app-lifecycle,apis,auth,deploy,env,github,migrate,profile,publish,recover,tables}/SKILL.md`, `main.rs`, `bootstrap.rs`
  - Verify: `cargo build`, `skill:doctor --strict`, `lint:tone --strict`, `bun test`, 수동 e2e 1건
- [ ] **T3 (P1, human ~3h / CC ~20min)** — dead-code — consent/ 모듈 + CLI + cmd_preauth_check 삭제 (커밋3, 이제 미참조)
  - Surfaced by: §3 / §5
  - Files: `consent/*`, `main.rs`, `cli/mod.rs`, `cli/args/mod.rs`, `routing.rs`, `quality_gate.rs`, `preflight.rs`, `sync.rs`, `audit_ledger.rs`, `hooks/axhub-helpers.sh`
  - Verify: `cargo build` + `cargo clippy` clean
- [ ] **T4 (P2, human ~2h / CC ~15min)** — tests+docs — consent 테스트+e2e sentinel 삭제 + 내부 docs (커밋4)
  - Surfaced by: §8 / §9 / GitNexus
  - Files: `tests/*consent*`, `crates/axhub-helpers/tests/{cli_e2e.rs,phase_parity.rs,phase_parity.md,state_classification_test.rs,diagnose_layering_test.rs,ci_coverage_gate.rs,hook_safety_cli.rs}`, `tests/e2e/claude-cli/cases/{23-preauth-check-deny,31-consent-mint-sentinel}.case.sh`, `tests/e2e/claude-cli/{matrix.jsonl,lib/t2-helper.sh,README.md}`, `tests/hook-fixtures/README.md`, `fuzz/fuzz_targets/parser.rs`, `fuzz/Cargo.toml`, `scripts/benchmark-hooks.ts`, `manifest.test.ts`, `phase26-quality-surfaces.test.ts`, `deploy-skill-wire-up.test.ts`, `tables-skill-contract.test.ts`, `windows-compat-docs.test.ts`, `prompt-route-karpathy.test.ts`, corpus/baseline JSONL, `docs/HOOKS.md`, `hooks/hooks.json`
  - Verify: `cargo test` + `bun test` + `bunx tsc --noEmit`
- [ ] **T5b (P1, human ~1h / CC ~10min)** — user-docs — user/AI-facing consent 광고 정리 + README 안전 서사 재작성 (커밋4)
  - Surfaced by: **DX2 (devex-review)** — README 가 존재하지 않을 consent 게이트 광고 → credibility gap
  - Files: `README.md`(48 재작성/223/240), `crates/axhub-helpers/assets/AXHUB.md`:90, `commands/{apps,deploy,배포,login,doctor,logs,status}.md`, `.claude-plugin/{plugin,marketplace}.json`, `skills/deploy/references/telemetry.md`, `docs/architecture.ko.md`, `docs/routing.md`, `docs/MIGRATION_v0.6_to_v1.0.md`, `docs/migration-gate.md`, `docs/org-admin-rollout.ko.md`, `docs/pilot/admin-rollout.ko.md`, `docs/pilot/onboarding-checklist.md`, `docs/marketing/{landing-page,outreach-email}.ko.md`, `docs/baseline-measurement.md`
  - Verify: DoD rg = allowlist 외 0, README 안전 서사가 preview 카드[D4]+CC native 권한 반영
- [ ] **T5 (P1, 선행)** — 각 삭제 심볼 `gitnexus_impact(... direction:"upstream")` blast radius 확인 (repo CLAUDE.md 강제), HIGH/CRITICAL 보고

## 12. 구현 전 필수 (repo 규약)
- **현재 작업 브랜치 선결 조건:** 구현 전 `git status --short --branch` 가 clean 이어야 하고, `origin` behind 상태면 최신화(rebase/merge) 후 별도 clean branch 또는 worktree 에서 시작해요. 현재 검토 시점에는 브랜치가 origin 보다 5 commits behind 이고 unrelated 변경이 이미 있어 consent 삭제를 바로 얹으면 범위가 섞여요.
- 각 consent 심볼 삭제 전 `gitnexus_impact(... direction:"upstream")`, HIGH/CRITICAL 경고 시 보고.
- `gitnexus_detect_changes()` 로 커밋 전 영향 범위가 예상과 일치하는지 확인.
- codex outside-voice 가 잡은 consumer(main.rs prompt-route, 정확한 12 skill, e2e sentinel)를 빠뜨리지 말 것.

### 12.1 현재 검토에서 확인된 GitNexus blast radius

삭제 전 최소 재확인 대상과 현재 관측값:

| Symbol | Risk | Direct callers | Affected processes | 메모 |
|---|---:|---:|---:|---|
| `ConsentBinding` | CRITICAL | 5 | 7 | bootstrap/main/consent decision/test 경유 |
| `cmd_preauth_check` | CRITICAL | 2 | 5 | legacy dispatch + clap dispatch + hook tests |
| `cmd_consent_mint` | HIGH | 1 | 4 | clap dispatch + tests |
| `state_root` | HIGH | 5 | 3 | audit_ledger/diagnose 는 유지해야 해서 경로 보존 필수 |
| `write_private_file_no_follow` | CRITICAL | 6 | 5 | token import/init + diagnose/hitl 도 직접 영향 |
| `set_private_dir_mode` | HIGH | 3 | 4 | `store_plugin_token` 직접 caller 유지 필요 |

이 표는 계획 검토 시점의 snapshot 이라 구현 직전 최신 index 로 다시 실행해요. HIGH/CRITICAL 을 무시하지 말고 T1/T2/T3 범위에 반영해요.

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 1 | CLEAN | HOLD_SCOPE, 0 critical gaps, 6 decisions (D1–D6) |
| Codex Review | `/codex review` | Independent 2nd opinion | 1 | ISSUES_FOUND | 8 findings + GitNexus 1, 전부 plan 반영 |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 1 | CLEAN | 1 issue (E1: util 테스트 이전), 0 critical gaps |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | n/a (UI scope 없음) |
| DX Review | `/plan-devex-review` | Developer experience gaps | 1 | CLEAN | TRIAGE, doc credibility 4→9, 1 finding (DX2) |

- **CODEX:** 8 findings (놓친 main.rs prompt-route consumer, 정확한 12 skill, set_private_dir_mode 직접 caller, diagnose 목록, state_root≠state_dir, 커밋 순서, DoD allowlist) + GitNexus e2e sentinel — 모두 plan 에 반영(§3/§4/§5/§9).
- **CROSS-MODEL:** state_root dedup 충돌(D6) → codex 수용, `.../state/axhub` 의미 보존. 나머지는 사실 정정이라 적용.
- **ENG:** Test review 가 재배치 util(state_root, write_private_file_no_follow)의 테스트 이전 누락(E1)을 잡아 plan §3.8/§4/T1 에 반영. §1 Architecture·§2 Quality·§4 Perf 는 CEO+codex 가 이미 커버해 새 finding 없음.
- **DX:** TRIAGE(삭제라 8-pass N/A). README·AXHUB.md 등 user/AI-facing 문서가 consent 를 안전장치로 광고하는데 plan docs scope 가 놓침(DX2) → §3.10/T5b 에 반영, README 안전 서사 D4 기준 재작성. doc credibility 4→9.
- **UNRESOLVED:** 0.
- **VERDICT:** CEO + ENG + DX + Outside Voice 전부 CLEARED — 구현 준비됨. required 게이트(Eng Review) 통과. 준비되면 `/ship`.

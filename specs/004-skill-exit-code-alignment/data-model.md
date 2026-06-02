# Data Model — 실패 조건 ↔ 복구 액션 정합 매핑

**Feature**: `004-skill-exit-code-alignment` | 기준: ax-hub-cli 0.17.2

`/speckit-plan` Phase 1 출력. spec 의 Key Entities 를 구체화하고, refactor 의 핵심인 **canonical 매핑 테이블**(현재→목표)을 담아요. `/speckit-tasks` 가 이 표를 작업으로 분해해요.

> 관찰=live 바이너리 / 테스트 단언으로 확인. 추정=error.rs 변형 + flat 패턴 추론 (plan T0 에서 확정).

---

## A0. T004~T008 구현-중 확정 (아래 A/B/C/D 표를 정정·우선함)

`/speckit-implement` Foundational 에서 authoritative Read 로 확정한 내용이에요. 이전 표의 추정/가정을 **이 절이 supersede** 해요.

### A0-1. slug 은 별도 `ErrorCode` enum (coarse, ~11개) — exit-code 파생 아님 (T004/T005)

출처: `ax-hub-cli/axhub/src/main.rs:753 error_to_error_code()` + `ErrorCode` (`#[serde(rename_all="snake_case")]`).

| CLI Error 변형 | **slug (`error.code`)** | exit | 비고 |
|---|---|---|---|
| Unauthenticated | **`auth`** | 4 | ⚠ "unauthenticated" **아님** (추정 틀림) |
| NotFound · InvitationExpired | `not_found` | 5 · 13 | 둘 다 같은 slug, exit 로 구분 |
| RateLimited | `rate_limited` | 6 | |
| Api · BackendUnimplemented | `api` | 7 | subcode `backend_unimplemented` |
| Tenant{Stale,Permission,NotFound} | `tenant_scope` | 8 | |
| Conflict류 | `conflict` | 9 | subcode `last_admin` 등 |
| DomainBlocked · OAuthClientForbidden | `forbidden` | 12 | |
| Timeout | `timeout` | 10 | |
| DryRunBlocked | `dry_run_blocked` | 11 | |
| Usage · IdPAutoSignup | `usage` | 64 | subcode `validation.*` |
| Downgrade·Cosign·Digest·Swap | `other` | 66·14·15 | **subcode + exit 로 분기** (slug 은 뭉뚱) |

→ canonical 라우팅 1차 키 = 이 **11개 ErrorCode slug**. B표의 dotted slug(`auth.token_invalid`/`resource.app_not_found`)은 helper-내부 표현이고 CLI 와 다름.

### A0-2. ⚠ helper-exit `65`/`67` 은 TESTED PUBLIC CONTRACT — renumber 금지 (T008)

`EXIT_LIST_AUTH=65`/`EXIT_LIST_NOT_FOUND=67` 은 helper 의 **출력 계약**이고 다음이 잠가요: `crates/axhub-helpers/tests/phase_parity.rs`(`classify(...)` + `assert_eq!(auth.exit_code, EXIT_LIST_AUTH)`), `tests/cli_e2e.rs`, `list_deployments.rs` inline 테스트, recover canonical map(`auth.token_invalid|65`). skill 들은 helper 의 65/67 을 소비해요.

→ **D표/B표의 "`EXIT_LIST_AUTH 65→?` 재키" 는 RETRACT.** helper 출력 65/67 은 **유지**. 진짜 버그는 helper **INPUT 번역**:
- `exit_to_helper_exit` 가 `c.starts_with("auth.")` 로 체크 → CLI 가 emit 하는 `"auth"`(점 없음)와 불일치. **fix**: `c == "auth" || c.starts_with("auth.")` + numeric fallback `exit_code == 65` → `== 4`.
- `exit_to_error_code` numeric fallback `65 => auth.token_invalid` → `4 => ...`, `67 => ...` → `5 => ...` (slug 부재 시에만 타는 경로).

### A0-3. T006 — `apis.call_consent_required` = plugin-side

`bootstrap.rs` 의 ConsentRequired* + `phase_parity.rs classify` 테스트에서 쓰는 plugin 분류 slug. **CLI ErrorCode 아님** → `cli-exit-contract.json`(T009) 모집단에서 **제외**. catalog 의 `65:apis.call_consent_required` 는 helper-path 분류로 유지하되 base 65 의미만 정정.

### A0-4. 미해결 설계 fork (US1 편집 전 결정 필요)

direct-path 스킬(status/deploy/logs)이 raw CLI exit(4/5)을 어떻게 라우팅할지:
- **Fork-A**: 모든 direct 경로를 `axhub-helpers classify-exit` 경유로 통일 → helper 가 CLI slug/code → 65/67+error_code 정규화. skill 은 helper 출력만 소비 (가장 일관, recover/auth 가 이미 이 방향).
- **Fork-B**: catalog 에 CLI direct 키(`auth`/`not_found` slug 또는 4/5/6)를 helper-output 키(65/67)와 **병존** + skill 이 `error.code` slug 로 직접 분기.

A 가 단일-출처에 부합(helper 가 유일 라우터). B 는 skill 이 CLI envelope 를 직접 파싱(중복). → **결정: Fork-A 채택** (사용자 확인 2026-06-02).

### A0-5. `classify()` = 핵심 라우터 + 정확한 3-surface 변경집합 (Fork-A)

`catalog.rs:32 classify(exit_code, stdout)` 가 Fork-A 의 라우터예요 (skill → `classify-exit` → `classify`):
```
code = stdout.error.code (CLI flat slug)
sub-key "{exit_code}:{code}" 우선 → 없으면 base "{exit_code}" → 없으면 default_entry
```
즉 **raw CLI exit_code 로 catalog 키잉**. CLI 가 4/5/6 emit 인데 catalog 는 65/67/68 키 → `classify(4)` 는 default("알 수 없는 에러") = 진짜 깨짐.

**세 개의 별개 surface (이전 정정의 핵심 — 혼동 금지):**

| # | surface | 변경 | 이유 |
|---|---|---|---|
| **S1** | `crates/axhub-helpers/data/catalog.json` 키 | `65→4`·`67→5`·`68→6`·`70→7` re-key, `2` 제거, `64`/`66`/`0`/`1` 유지 | `classify` 가 raw CLI exit 로 base-키잉. (B표 맞음) |
| **S2** | `list_deployments.rs::EXIT_LIST_*`(65/67) | **불변** | helper OUTPUT 계약, phase_parity/cli_e2e 잠금. (D표 renumber 틀림 — S1 과 다른 surface) |
| **S3** | `catalog.rs::classify()` | `error.subcode` 도 읽어 sub-key 구성 (현재 `error.code` 만) | CLI 는 coarse code(`usage`) + fine `subcode`(`validation.*`) 분리. sub-key 정밀도 위해. (US2+ 영향) |
| S4 | `list_deployments.rs::exit_to_error_code`/`exit_to_helper_exit` INPUT | `65→4`/`67→5` + `c=="auth"` (점 없는 flat) | slug 우선이라 fallback 경로지만 정합. OUTPUT(EXIT_LIST)은 S2 로 불변 |
| S5 | skills(status/deploy/logs) | raw `$?` lookup → `classify-exit` 경유 (Fork-A) | 단일 라우터 |
| S6 | build.rs (`OUT_DIR/catalog_generated.rs`) + `codegen-catalog.ts`(.md) | catalog.json 편집 시 자동 재생성 (cargo build / bun codegen) | S1 의 산출 |

**US1(auth) 최소 변경**: S1 의 `"65"→"4"` (auth base) + S4 INPUT `65→4`/`c=="auth"` + S5 status + phase_parity 의 `classify(65,..)` INPUT → `classify(4,..)` 갱신(EXIT_LIST_AUTH OUTPUT assert 는 유지) + classify(4,{code:auth}) 단위테스트. `cargo test -p axhub-helpers` 로 검증.

### A0-6. 구현 확정 (2026-06-02) — 두 frozen 공간을 `normalize_helper_exit` 로 브리지

A0-2/A0-5 의 "helper 65/67 유지 + catalog 는 CLI-keyed" 를 구현하면서 **두 실패-exit 공간이 동시에 live** 임이 확정됐어요 (둘 다 frozen):

| 공간 | 코드 | frozen 이유 | 소비처 |
|---|---|---|---|
| **CLI-native** | auth=4, not_found=5, rate=6, api=7 (+ 8/12/64/66...) | ax-hub-cli 외부 (수정 불가) | status / logs / deploy Step5 `deploy status --watch` / init `axhub apps bootstrap` / apps `apps list` / auth `axhub auth` |
| **helper-output** | auth=65, not_found=67, rate=68, internal=70 (64/66 공유) | tested public 계약 (EXIT_LIST_*, preflight::EXIT_AUTH, gate-zero RETRACT) | deploy Step1 `deploy-prep` / apps `preflight`(auth_ok) / recover `list-deployments` / token-gate |

**브리지**: catalog 는 CLI-keyed 유지 (dual-key 안 함), classify() 진입부에서 `normalize_helper_exit` 가 `65→4 / 67→5 / 68→6 / 70→7` 정규화 → 두 공간이 한 template 으로 라우팅. `list_deployments::exit_to_error_code` 의 `4 | 65 => auth` 선례와 동일. 그래서 **parity guard "catalog key ⊆ CLI contract" 가 그대로 valid** (helper-space 는 catalog key 가 아니라 normalize 에 있음). `cli-exit-contract.json` 의 `helper_output_exit_codes` + `helper_output_normalization` 가 두 번째 계약을 명문화.

**구현 중 발견한 진짜 live 버그 (US1 status/logs 외)**: `deploy` Step 6 라우팅은 Step5 `deploy status --watch`(CLI 4/5/6) **와** Step1 `deploy-prep`(helper 65/67) 양쪽 feed 인데 helper 65/67/68 만 listing → CLI watch 실패가 누락됐어요. Step 6 을 **dual-space (4/65, 5/67, 6/68) + classify-exit 포인터** 로 수정.

**init 정정**: init 은 `axhub apps bootstrap`/`apps templates list` (둘 다 **CLI**) 를 호출하므로 CLI-native 4/5/6 을 봐요 — helper 아님. 기존 prose "exit 65 (auth)" 는 direct-CLI stale 였고 `exit 4` / error_code `auth` 로 정정 (forbidden/scope 는 CLI 12/8). `error-empathy-catalog.md` 헤더에 "exit 65→exit 4 섹션" 브리지 note 추가로 helper-65 관찰자도 올바른 template 을 찾아요.

---

## Entities

- **실패 조건 (Failure condition)**: `{ cli_exit_code: u8, error_code_slug: String (flat), subcode: Option<String> }`. CLI 가 `--json` envelope `error.{code,subcode}` + process exit code 로 표면화. **식별 1차 키 = `error_code_slug`** (version-agnostic), 2차 = `cli_exit_code`.
- **복구 액션 (Recovery entry)**: `{ key, emotion, cause, action, button? }` — catalog.json 의 4-part 항목.
- **복구 카탈로그 (Recovery catalog)**: `crates/axhub-helpers/data/catalog.json` (source-of-truth) → codegen → `error-empathy-catalog.generated.md` + hand-written `error-empathy-catalog.md` (drift test 로 키 동기).
- **CLI 실패 계약 (CLI failure contract)**: `crates/axhub-core/src/exit_code.rs ExitCode` + `error.rs Error::{exit_code,code,subcode}` + `docs/cli-exit-codes.md`. drift-guard 가 pin 할 모집단.

---

## A. CLI 실패 계약 (진실의 출처 — 정합 목표)

| variant | exit | flat slug `error.code` | 출처 | subcode 예 |
|---|---|---|---|---|
| Unauthenticated | 4 | `unauthenticated` (추정) | error.rs:88 | — |
| NotFound | 5 | `not_found` (**관찰**) | live run | — |
| RateLimited | 6 | `rate_limited` (추정) | error.rs:94 | — |
| Api | 7 | `api`/도메인별 (추정) | exit_code.rs | `backend_unimplemented` |
| TenantScope | 8 | `tenant_*` (추정) | error.rs | — |
| Conflict | 9 | `conflict` (**관찰**) | tests | `last_admin`,`already_settled` |
| Timeout | 10 | `timeout` (추정) | error.rs | — |
| DryRunBlocked | 11 | `dry_run_blocked` (추정) | exit_code.rs | — |
| DomainBlocked | 12 | `domain_blocked` (**관찰**) | tests | — |
| InvitationExpired | 13 | `invitation_expired` (**관찰**) | tests | — |
| VerifyDigestMismatch | 14 | (추정) | exit_code.rs | — |
| SwapFailed | 15 | (추정) | exit_code.rs | — |
| Usage | 64 | `usage` (**관찰**) | live run | `validation.*` |
| EnforceBlocked | 66 | (추정) | exit_code.rs | `scope.downgrade_blocked`,`update.cosign_verification_failed` |
| (reserved) | 2 | — (clap usage) | doc | — |

> envelope 모양 (관찰): `{"schema_version":"1","status":"error","error":{"code":<slug>,"category":"client_error","hint":<msg>,"subcode"?:<sub>}}`

---

## B. Catalog 키 재매핑 (현재 → 목표)

| 현재 catalog 키 | 의미 | 판정 | 목표 (slug 1차 / code 2차) |
|---|---|---|---|
| `0` | success | ✓ 유지 | `0` |
| `1` | transport/unclassified | ✓ 유지 | `1` (+ helper transport.* slug) |
| `2` | deploy in-progress | ✗ **제거** | clap 예약 침범. in-progress 는 NDJSON stream state + `64:validation.deployment_in_progress` 로 이미 표면화 |
| `64` (base) | usage | ✓ 유지 | `usage` / `64` |
| `64:*` (validation/env/github/catalog.sql) | usage subcodes | ✓ 유지 (slug 검증) | `64:<subcode>` |
| `65` | auth | ✗ **교체** | `unauthenticated` / `4` |
| `65:apis.call_consent_required` | API 사전동의 | ⚠ T0 출처확인 | ax-hub-cli grep 0건 → plugin-side/stale. 실제 출처 확정 후 재키 |
| `66` (base) | scope insufficient | ✗ **재정의** | CLI 66 = EnforceBlocked. 일반 scope 부족은 `8`(tenant_*) 또는 `7`(api/forbidden) — T0 확정 |
| `66:scope.downgrade_blocked` | 다운그레이드 차단 | ✓ 유지 | `66:scope.downgrade_blocked` (66 정확) |
| `66:update.cosign_verification_failed` | cosign 실패 | ✓ 유지 | `66:update.cosign_verification_failed` |
| `66:profile.endpoint_not_in_allowlist` | 프로필 allowlist | ⚠ 재검토 | EnforceBlocked 아님 → `64`(usage) 계열 가능 |
| `67` | not found | ✗ **교체** | `not_found` / `5` |
| `67:{github.install_not_found,open.no_app_manifest,catalog.not_found}` | not-found subcodes | ✗ 교체 | `5:<subcode>` 또는 `not_found:<subcode>` |
| `68` | rate limit | ✗ **교체** | `rate_limited` / `6` |
| `70:catalog.internal_error` | catalog 내부오류 | ✗ 교체 | CLI 70 없음 → `7`(api) 계열 |

### 신규 추가 (CLI 는 내는데 catalog 항목 없음 — Q1: reachable 만 bespoke, 나머지 fallback)

`8` tenant_scope · `9` conflict · `10` timeout · `11` dry_run_blocked · `12` domain_blocked · `13` invitation_expired · `14` digest_mismatch · `15` swap_failed.
- **reachable (bespoke 작성)**: plan T0 reachability 로 deploy/status/recover/auth/init 경로에 실제 도달하는 코드만 (예: `8` tenant — init 이미 사용; `10` timeout; `9` conflict — invitations/members).
- **unreachable (공통 fallback)**: 공통 "안전하게 멈췄어요 + 다음 행동" 항목 1개로 라우팅 (FR-006).

---

## C. slug 네임스페이스 reconcile (flat CLI ↔ dotted helper)

`list_deployments.rs` + recover canonical map 이 dotted slug 를 쓰는데 CLI 는 flat 을 emit → 매핑 명시.

| CLI flat slug | helper/카탈로그 dotted (현재) | 정합 결정 |
|---|---|---|
| `unauthenticated` (4) | `auth.token_invalid` | **fix**: `exit_to_helper_exit` 의 `starts_with("auth.")` 를 `== "unauthenticated" ‖ starts_with("auth")` + `exit==4` 로 확장. catalog 키도 `unauthenticated` accept |
| `not_found` (5) | `resource.app_not_found` | `contains("not_found")` 덕에 helper-exit 은 동작하나 skill-level 키 정합 필요 → 양쪽 다 `not_found` accept |
| `rate_limited` (6) | (없음) | **추가** |
| `usage` (64) | `usage.invalid` | align (`usage` accept) |
| `conflict` (9) 등 | (helper 미처리) | reachable 시 추가 |

> 결정: **CLI flat slug 을 canonical 1차 키로 채택**. helper 의 dotted slug 은 내부 보조로 두되, 매핑 함수가 flat→dotted 를 1곳에서 변환 (drift 단일점).

---

## D. 코드 변경 지점 (B/C 표를 적용할 파일)

| 파일 | 변경 |
|---|---|
| `crates/axhub-helpers/data/catalog.json` | 키 재매핑 (B 표) → codegen 재실행 |
| `crates/axhub-helpers/src/list_deployments.rs` | `EXIT_LIST_AUTH 65→?`, `EXIT_LIST_NOT_FOUND 67→?`, `exit_to_error_code` match arms(65/67→4/5 + flat slug), `exit_to_helper_exit` auth 분기 |
| `crates/axhub-helpers/src/main.rs` | `cmd_classify_exit` + :1981 하드코딩 65/67 → slug/4/5 |
| `skills/deploy/references/error-empathy-catalog.md` | `## exit` 섹션 재키 + 인용(FR-011) |
| `skills/{status,deploy,logs}/SKILL.md` | direct-path 라우팅을 slug 기준으로 (또는 `classify-exit` 경유) |
| `skills/{recover,init,apps}/SKILL.md` | 잔여 numeric(65/66) 정리 + dotted→flat slug 정합 |
| `crates/axhub-helpers/data/` + `tests/` | drift-guard pinned snapshot + parity 테스트 (Q2) |

> `EXIT_LIST_*` 는 helper 의 **출력** 계약 — 외부(skill)가 helper-exit 65/67 에 의존하면 그 의존부도 동시 갱신 (plan T0 가 helper-exit 소비처 확인).

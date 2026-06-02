# 검증 보고서 — Skill 복구 라우팅 ↔ ax-hub-cli 0.17.2 실패 계약

**작성일**: 2026-06-02
**검증 대상**: `skills/status` (진입점) + 공유 `skills/deploy/references/error-empathy-catalog.md`
**대조 기준**: `ax-hub-cli` 0.17.2 (`docs/cli-exit-codes.md` SLA + `crates/axhub-core/src/exit_code.rs`)
**관계**: `specs/004-skill-exit-code-alignment/spec.md` 의 동반 구현-세부 문서. `/speckit-plan` 이 이 표를 구체 편집으로 변환해요.

> 이 문서는 행동-중심 spec 에서 의도적으로 뺀 **숫자 매핑**을 담아요. 스펙 본문이 아니라 plan 입력이에요.

---

## 0. 한 줄 결론

`status` 는 **명령·플래그는 정합**, **exit-code 복구 라우팅은 비정합**. 라우팅은 공유 카탈로그에 있고 8개 스킬이 소비 → 카탈로그-레벨 작업.

> **정본 주의 (I2)**: 이 보고서는 numeric(`65→4`) 프레임의 초기 finding 이에요. 최종 전략은 **slug-primary** (`research.md` 결정 1 + `data-model.md`) — CLI `error.code` slug 가 1차 키, numeric 은 2차. plan/data-model 이 정본이에요.
>
> **정본 주의 (I3, 구현 확정)**: 구현 중 **두 실패-exit 공간이 동시에 frozen** 임이 확정됐어요 — CLI-native(auth=4/not_found=5/rate=6) 와 helper-output(65/67/68, tested public 계약). 아래 §3 "미처리/falling-through" 표는 단일-공간 가정의 초기 finding 이고, 최종 구현은 catalog 를 CLI-keyed 로 두고 `classify()` 진입부 `normalize_helper_exit`(65→4 등)로 두 공간을 한 template 에 브리지해요. **`data-model.md` A0-6 이 정본**.

---

## 1. ax-hub-cli 0.17.2 실패 계약 (진실의 출처)

출처: `ax-hub-cli/docs/cli-exit-codes.md` SLA 표 + `crates/axhub-core/src/exit_code.rs::ExitCode` enum (둘이 상호 일치, doc 이 enum 을 "single source of truth" 로 인용). `status` 스킬 본문은 Read 로 직접 확인. 세 출처가 상호 정합 → 신뢰도 높음.

| 코드 | 의미 (Error 변형) | subcode 예시 |
|---|---|---|
| 0 | Success | — |
| 1 | Generic (Io / Serde / Other) | — |
| 2 | **예약** (clap usage / invalid-args, 기본 2). "Do NOT remap." | — |
| 3 | **예약** (shell 관례) | — |
| 4 | Unauthenticated (PAT/OAuth 없음 → `axhub auth login`) | — |
| 5 | NotFound `{kind, id}` | — |
| 6 | RateLimited `{retry_after_secs}` | — |
| 7 | Api `{status, message}` / OAuthClientForbidden / BackendUnimplemented | `backend_unimplemented` |
| 8 | TenantScope (stale / permission / not-found) | — |
| 9 | Conflict (slug 중복 / already-settled / last-admin / member) | (subcode 분기) |
| 10 | Timeout `{detail}` | — |
| 11 | DryRunBlocked (`--execute` 누락) | — |
| 12 | DomainBlocked `{domain}` | — |
| 13 | InvitationExpired `{invitation_id}` | — |
| 14 | VerifyDigestMismatch (update 변조 신호) | — |
| 15 | SwapFailed (update 바이너리 교체 실패) | — |
| 64 | Usage `{message}` / IdPAutoSignupRequiresJIT | — |
| 66 | EnforceBlocked (DowngradeBlocked / CosignEnforceBlocked) | `scope.downgrade_blocked`, `update.cosign_verification_failed` |

`ExitCode::from_raw` 는 **미지의 비-0 바이트를 전부 `Generic`(1) 으로 collapse** 해요. 즉 65/67/68/70 같은 코드는 CLI 가 절대 내보내지 않고, 받더라도 1 로 접혀요.

---

## 2. 카탈로그 현재 스킴 (`error-empathy-catalog.md`, Read 로 전수 확인)

| 카탈로그 코드 | 카탈로그 의미 | subcode 항목 |
|---|---|---|
| 0 | success | — |
| 1 | transport / unclassified | — |
| 2 | deploy status **in-progress** | — |
| 64 (base) | validation / usage | `validation.deployment_in_progress`, `validation.app_ambiguous`, `validation.app_list_truncated`, `validation.quality_gate_failed`, `env.prod_force_required`, `env.prod_confirm_mismatch`, `github.git_connection_already_exists`, `github.confirm_slug_mismatch`, `catalog.sql_format` |
| 65 | auth required / token expired | `apis.call_consent_required` |
| 66 (base) | **scope insufficient** | `scope.downgrade_blocked`, `update.cosign_verification_failed`, `profile.endpoint_not_in_allowlist` |
| 67 | resource not found (did-you-mean) | `github.install_not_found`, `open.no_app_manifest`, `catalog.not_found` |
| 68 | rate limit (auto-backoff) | — |
| 70 | (catalog internal) | `catalog.internal_error` |

(추가로 npm 오류 6종은 exit-code 가 아니라 stderr 패턴 매칭이라 이 정합 범위 밖.)

---

## 3. Drift 대조표 (확정 미스매치)

| 실패 조건 | 카탈로그 코드 | CLI 실제 코드 | 판정 | 영향 |
|---|---|---|---|---|
| success | 0 | 0 | ✅ 일치 | — |
| generic / transport | 1 | 1 | ✅ 일치 | — |
| usage / validation (base) | 64 | 64 | ✅ 일치 | — |
| downgrade_blocked | 66 + subcode | 66 + `scope.downgrade_blocked` | ✅ 일치 (코드+subcode) | — |
| cosign 검증 실패 | 66 + subcode | 66 + `update.cosign_verification_failed` | ✅ 일치 | — |
| **auth / 토큰 만료** | **65** | **4** | ❌ **미스매치** | 재로그인 안내 0% 발동 → 무한 재시도 |
| **resource not found** | **67** | **5** | ❌ **미스매치** | did-you-mean 0% 발동 |
| **rate limit** | **68** | **6** | ❌ **미스매치** | auto-backoff 0% 발동 |
| **scope insufficient (base)** | **66=scope** | 66 = **EnforceBlocked** | ❌ **의미 충돌** | base 66 의미가 CLI 와 반대; 권한-부족 일반 조건은 CLI 에 66 아님 |
| **catalog internal** | **70** | 70 없음 (≈ 7=Api) | ❌ **미스매치** | catalog.internal 라우팅 stale |
| **deploy in-progress** | **2** | 2 = **clap 예약** | ❌ **충돌 + stale** | in-progress 는 사실 NDJSON stream state + `64/validation.deployment_in_progress` 로 표면화. exit 2 항목은 잉여/위험(clap 예약 침범) |

### 미처리 (CLI 는 내는데 카탈로그 항목 없음 → fallback=일반 1번으로 falling through)

`8` TenantScope · `9` Conflict · `10` Timeout · `11` DryRunBlocked · `12` DomainBlocked · `13` InvitationExpired · `14` VerifyDigestMismatch · `15` SwapFailed.

- 단, **`8` 은 일부 스킬이 이미 ad-hoc 사용** — `skills/init/SKILL.md` 가 "exit 8 (tenant 미해석)" 라우팅을 가짐. 즉 스킬 묶음이 **부분 마이그레이션**된 불일치 상태 (init=새 스킴 8, status/catalog=옛 스킴 65/67/68). 이게 drift 가 실재한다는 결정적 증거.
- `13` InvitationExpired, `12` DomainBlocked 등은 deploy/status 경로엔 안 와도 invitations/members 스킬엔 올 수 있음 → plan 에서 스킬별 도달성 매핑 필요.

### subcode 항목 재배치 필요

카탈로그 subcode 들이 옛 base 코드에 매달려 있어 base 가 바뀌면 같이 옮겨야 해요:
- `65 + apis.call_consent_required` → CLI 에서 consent-required 가 어떤 코드인지 plan 에서 확인 (4 아닐 가능성 — consent 는 인증과 다름).
- `67 + {github.install_not_found, open.no_app_manifest, catalog.not_found}` → 5(NotFound) 계열로.
- `66 + profile.endpoint_not_in_allowlist` → 66 base 의미가 바뀌므로 재검토 (EnforceBlocked 와 무관할 수 있음 → 64 Usage 계열 가능성).
- `70 + catalog.internal_error` → 7(Api) 계열로.

---

## 4. 정합한 부분 (변경 불필요)

- **명령 존재**: `axhub deploy list` / `deploy status` / `deploy logs` / `deploy create` / `deploy rollback` 전부 실재 (`ax-hub-cli/axhub/src/commands/deploy/` 에 `list.rs`/`status.rs`/`logs.rs`/`create.rs`/`rollback.rs`/`watch.rs` 확인). `specs/002` 의 명령-레벨 no-op 결론과 일치.
- **NDJSON event**: `deploy/status.rs:129` 가 `stage_transition` emit, `deploy/mod.rs:301 emit_ndjson_event` 가 `"event"` 필드 부여 → 스킬의 `stage_transition` 가정 정합.
- **watch degrade 동작**: bare `--watch` 는 비-TTY 에서 단일 스냅샷으로 degrade, `--watch-interval`/`--watch-timeout` 이 explicit streaming opt-in → 스킬 주장과 일치. (단 §6 참고)

---

## 5. Gate-zero — 편집 전 필수 검증 (assert 금지, plan 첫 작업)

아래는 **검증된 매치로 단정하지 않음**. plan 이 어떤 코드든 renumber 하기 **전에** 반드시 먼저 확인해요 (optional cleanup 아님, gate-zero).

0. **(linchpin) 7개 비-`status` 스킬이 raw CLI exit code 를 그대로 라우팅하는지** — `status` 본문은 `axhub deploy status` 의 raw `$?` 를 카탈로그-by-number 로 라우팅함(Read 로 확인). 그러나 deploy·logs·recover·init·apps·update·auth 의 라우팅 근거는 RTK-압축 grep 이라 미확정. **한 스킬이라도 `axhub-helpers` 를 거쳐 코드를 normalize 하면 그 스킬의 fix 는 완전히 달라져요.** 각 스킬이 status 와 동일하게 raw 코드를 읽는지 Read 로 개별 확인한 뒤에만 renumber.
1. `--watch` / `--watch-timeout` / `--watch-interval` / `--app` / `--json` 플래그가 **`deploy status`** clap args 에 있는지 — 현재는 `axhub/src/commands/apps/bootstrap.rs` 에서 확인됐고 `deploy/status.rs` 자체 args 는 미확인.
2. `axhub deploy list --app <APP> --json` 의 `--app`/`--json` 플래그 시그니처.
3. terminal 상태 문자열 `succeeded` / `failed` / `cancelled` / `rolled_back` 전체 — `succeeded` 만 확인됨 (`axhub-api/src/deploy.rs:604`).
4. `apis.call_consent_required` 가 매핑되는 실제 CLI 코드/subcode (consent ≠ auth 일 가능성).
5. 각 미처리 코드(8~15)가 어느 스킬 경로에 실제 도달하는지 (도달 안 하면 fallback 만으로 충분).
6. **(corpus 사전조사)** §7-3 의 corpus/baseline 이 실제로 exit-code 문자열을 expected 로 박아두는지 먼저 `grep` 으로 확인 — 002 의 `--branch` 교훈은 corpus 가정에 관한 것이고, exit-code 가 corpus 에 있는지는 별개 질문. 있으면 전 tier 동시 갱신, 없으면 scope 제외.
7. **(hard-cut de-risk, Clarify Q3)** ax-hub-cli git 이력 pickaxe (`git log -S '65' -S '67' -S '68'` + exit_code.rs 이력)로 "옛 코드 `65`/`67`/`68` 을 실제 emit 한 release 가 존재했나"를 확정. **없으면** hard-cut 무손실 → 옛 코드 제거. **있었으면** 잔존 사용자 위험이므로 dual-map 또는 preflight min-version 게이트로 에스컬레이션 (spec Assumptions 의 hard-cut 전제 재검토).

---

## 6. 버전 앵커

`skills/status/SKILL.md:75` 가 "axhub-cli 0.15.3+" 라고 적음. 현행 = 0.17.2. `+` 라 기술적으론 만족하나 stale 앵커. plan 에서 버전 의존 동작(watch degrade)이 0.17.2 에서 유효한지 확인하고 앵커 갱신 여부 결정.

---

## 7. Blast radius (변경 시 동기 갱신 대상 — 002 §교훈 반영)

1. **공유 카탈로그**: `skills/deploy/references/error-empathy-catalog.md` (+ `.generated.md` 가 있으면 재생성).
2. **라우팅 스킬 8종**: status · deploy · logs · recover · init · apps · update · auth (각 SKILL.md 의 exit-code 라우팅 블록 + D1 비대화형 가드의 코드 언급).
3. **routing 코퍼스 / baseline** (002 가 놓쳐서 CI fail 난 지점): `tests/corpus.20.jsonl` · `tests/corpus.100.jsonl` · `tests/corpus.jsonl` · `tests/baseline-results.{docs-only,claude-native}.{20,100}.json` · `tests/fixtures/*`. exit-code 문자열이 expected 에 박혀 있으면 전 tier 동시 갱신.
4. **CI 게이트**: `bun run skill:doctor --strict` → `lint:tone --strict` → `lint:keywords --check` (description byte-lock 주의) → `bun test` → `bunx tsc --noEmit`.
5. **`tests/fixtures/ask-defaults/registry.json`**: 새 AskUserQuestion 추가 시 등록.

---

## 8. 비-목표

- CLI 자체 수정 (스킬을 CLI 에 맞춤, 역 아님).
- `deploy create --branch` consent 모순 (002 부록 1 — 별개 cross-repo 결정).
- 명령/플래그 리팩토링 (002 no-op).

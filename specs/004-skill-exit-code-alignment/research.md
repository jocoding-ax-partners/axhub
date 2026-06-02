# Phase 0 Research — Skill 복구 라우팅 ↔ ax-hub-cli 실패 계약

**Date**: 2026-06-02 | **Feature**: `004-skill-exit-code-alignment` | **CLI 기준**: ax-hub-cli 0.17.2

이 문서는 `/speckit-plan` Phase 0 출력이고, spec 의 gate-zero(`verification-report.md` §5)를 **관찰로 해소**한 결과예요. 모든 핵심 사실은 prebuilt 바이너리 실행 또는 authoritative Read 로 확인했어요 (RTK 압축 grep 단정 회피).

---

## 결정 1 — 라우팅 정규화 기준: **CLI `error.code` slug (numeric 아님)**

**Decision**: 복구 라우팅을 CLI 가 `--json` envelope 에 emit 하는 **flat `error.code` slug** 로 정규화해요. 숫자 exit code 는 보조(secondary)로만 쓰고, 옛 65/67/68 numeric fallback 은 현행 4/5/6 으로 교정해요.

**근거 (관찰)** — prebuilt `target/debug/axhub` 실행:
```
$ axhub deploy list --app probe-nonexistent-xyz --json
ERROR command failed ... exit_code=5
{"schema_version":"1","status":"error","error":{"code":"not_found","category":"client_error","hint":"not found: app `probe-nonexistent-xyz`"}}

$ axhub deploy status --json
{"schema_version":"1","status":"error","error":{"code":"usage","category":"client_error","hint":"validation.usage_error: ..."}}
```
- CLI 은 모든 typed 에러에서 `{status:"error", error:{code:<flat-slug>, category, hint, subcode?}}` 를 emit (출처: `crates/axhub-output/src/lib.rs OutputEnvelope`, `axhub/src/main.rs:683 handle_command_error → safe_error_envelope(error_code, hint).with_subcode(subcode)`).
- slug 은 **flat**: `not_found`, `usage`, `conflict`, `domain_blocked`, `invitation_expired`, `last_admin`(subcode) 등 (integration_command_exits.rs 단언으로 교차확인). auth=4 변형은 `Unauthenticated(String)` (error.rs:88) → slug `unauthenticated` 으로 추정 (flat 패턴, plan T0 에서 deauth 실행으로 최종확인).
- slug 은 numeric 재배치에 영향받지 않아 version-agnostic → 라우팅 1차 키로 적합.

**Alternatives 기각**:
- **순수 numeric renumber (65→4 등)만**: slug 경로(auth/recover/helper 우선)를 무시하고 numeric 만 고치면, slug 네임스페이스 drift(아래 결정 3)가 남아 auth 가 계속 깨져요.
- **dual-map (옛+새 numeric 동시 인식)**: Q3 에서 hard-cut 선택 + 아래 "결정 4" 의 무손실 증거로 불필요.

---

## 결정 2 — Scope: spec Dependencies 보다 **2개 레이어 더** (catalog.json + Rust helper)

**Decision**: 정합 대상은 **5개 표면**이에요. spec 의 Dependencies 는 catalog.md + 8 skills 만 셌는데 (과소집계), 실제로는 아래가 전부 옛 65/67/68 또는 dotted-slug 네임스페이스에 묶여 있어요.

| # | 표면 | 현재 상태 | 출처 |
|---|---|---|---|
| 1 | `crates/axhub-helpers/data/catalog.json` (Rust source-of-truth) | 키 = `0,1,2,64,65,66,67,68,70` + subcodes | catalog.json 키 덤프 |
| 2 | `scripts/codegen-catalog.ts` → `error-empathy-catalog.generated.md` | catalog.json 에서 생성 (idempotent), drift test `tests/codegen.test.ts` | codegen-catalog.ts 헤더 |
| 3 | `skills/deploy/references/error-empathy-catalog.md` (hand-written) | `## exit 65/66/67/68/70` 섹션 | Read |
| 4 | `crates/axhub-helpers/src/list_deployments.rs` | `EXIT_LIST_AUTH=65`, `EXIT_LIST_NOT_FOUND=67`; `exit_to_error_code` match `65=>auth.token_invalid, 67=>resource.app_not_found, 64=>usage.invalid`; `exit_to_helper_exit` `starts_with("auth.")‖exit==65`, `contains("not_found")‖exit==67` | Read lines 12-15, 360-378 |
| 4b | `crates/axhub-helpers/src/main.rs` `cmd_classify_exit` (`classify-exit` 서브커맨드, RTK 압축명 "ln") | main.rs:749 + :1981 `65=>"auth 만료", 67=>"앱 못 찾음"` 하드코딩 | Read/grep |
| 5 | 8 skills (status·deploy·logs·recover·init·apps·update·auth) | 혼재 (아래 표) | Read/grep |

**근거**: helper 와 catalog 둘 다 옛 numeric/dotted-slug 에 calibrated 임을 Read 로 확인. codegen 파이프라인이라 catalog.json(source) 수정 → codegen 재생성 → 두 .md 동기.

---

## 결정 3 — slug 네임스페이스 reconcile (flat CLI ↔ dotted helper)

**Decision**: CLI 의 flat slug(`unauthenticated`/`not_found`/`rate_limited`/`usage`)와 helper/카탈로그/recover-map 의 dotted slug(`auth.token_invalid`/`resource.app_not_found`) 사이의 매핑을 명시적으로 정의해요. (plan 이 단일 canonical 매핑 테이블을 `data-model.md` 에 둠.)

**근거**:
- `exit_to_helper_exit` (list_deployments.rs:377): `c.contains("not_found")` → CLI flat `not_found` **우연히 매치** ✓. 그러나 `c.starts_with("auth.")` → CLI flat `unauthenticated` **불일치** ✗ → auth 는 slug 경로로도 EXIT_LIST_AUTH 로 안 감 → catch-all `cli.exit_4`.
- recover canonical map (recover/SKILL.md:142)은 `auth.token_invalid`/`resource.app_not_found` 로 키잉 — CLI 가 emit 하는 `unauthenticated`/`not_found` 와 다름.
- 결론: **auth 가 가장 broken** (slug·numeric 양쪽). not_found 는 `contains` 덕에 우연히 동작하나 skill-level 키(`resource.app_not_found`)와는 여전히 drift.

---

## 결정 4 — Q3 hard-cut = **무손실 확정** (관찰)

**Decision**: 옛 65/67/68 을 제거하는 hard-cut 은 잔존 사용자 위험 0. escalation 불필요.

**근거 (git pickaxe, 전부 빈 결과)**:
```
git -C ax-hub-cli log --all -S '65 =>' -- crates/axhub-core/src/exit_code.rs   → 없음
git -C ax-hub-cli log --all -S '= 65'  -- ...exit_code.rs                       → 없음
git -C ax-hub-cli log --all -S 'EX_DATAERR'                                     → 없음
현 exit_code.rs 에 65/67/68 없음 (66=EnforceBlocked 만)
```
→ ax-hub-cli 는 65/67/68 을 **한 번도 emit 한 적 없음**. 카탈로그의 65/67/68 은 day-one 부터 틀린 값 (마이그레이션 회귀 아님). 옛 코드 의존 사용자 = 0.

---

## 결정 5 — corpus/baseline 무관 (관찰)

`tests/corpus.20.jsonl` / `corpus.100.jsonl` 에 exit-code 문자열 **없음** (grep 빈 결과). 002 의 `--branch` 교훈(corpus 동시 갱신)은 명령 생성에 관한 것이고, exit-code 라우팅은 corpus expected 에 없음 → **routing-drift gate 와 무관**. blast radius 에서 corpus 제외.

---

## 8 skills 현재 라우팅 상태 (Q1 reachability 기초)

| skill | 경로 | 현재 코드 | 판정 |
|---|---|---|---|
| status | direct `deploy status/list` + helper `list-deployments` | numeric **65/67/68** (Step 7) | ✗ broken (direct) |
| deploy | direct `deploy create` | numeric **65**/64/**67** (Step 6 표) | ✗ broken (65/67) |
| logs | direct `deploy logs` | numeric **65/67/68** | ✗ broken |
| recover | helper `list-deployments` + `deploy create` | slug map(dotted) + numeric **65** | △ slug 경로 부분 동작, auth drift |
| init | bootstrap saga `error_code` | **exit 8**(새,tenant) + **65/66**(옛) | △ mixed |
| apps | preflight `auth_ok`/`auth_error_code`(slug) + direct | slug(cli_not_found 등) + numeric **65/67/68** | △ mixed |
| update | direct `update apply` | **66 + cosign**(정확) + 0/1 | ✓ 대체로 정확 |
| auth | direct `auth ...` | **exit 6/7/10**(새) + 66 + classify-exit | ✓ 대체로 정확 |

→ **확정 broken(direct numeric)**: status·deploy·logs. **mixed**: recover·init·apps. **거의 OK**: update·auth (참고 모범).

---

## Tranche 분리 (confirmed core → confirmed broader)

advisor 권고대로 confirmed/contingent 가 아니라 **전부 confirmed** 로 판명됐지만, 사용자 진입점 우선 + blast radius 작은 순으로 단계화:

- **Tranche 1 (확정 core, 사용자 진입점)**: status·deploy·logs 의 direct-path numeric 라우팅을 CLI slug 기준으로 교정. catalog.md/json 의 65→4/67→5/68→6 + slug 키. status 가 user 가 가리킨 스킬.
- **Tranche 2 (확정, helper 레이어)**: `list_deployments.rs`(EXIT_LIST consts + exit_to_error_code + exit_to_helper_exit) + `cmd_classify_exit` 의 numeric/slug 교정 + flat↔dotted 네임스페이스 reconcile. auth slug(`unauthenticated`) 불일치 수정.
- **Tranche 3 (정합 잠금)**: catalog.json codegen 재생성 + drift-guard parity 테스트(Q2) + mixed skills(recover/init/apps) 잔여 numeric 정리 + tone/keyword/doctor/tsc 게이트.

---

## 남은 gate-zero (plan T0 에서 확인, 대부분 해소됨)

- [해소] direct status/deploy/logs raw-code 라우팅 = broken (Read 확인)
- [해소] deploy status/list flags(`--watch*`/`--app`/`--json`) 존재 (StatusArgs/ListArgs Read)
- [해소] CLI slug emit 여부 = **YES, flat slug** (live 관찰)
- [해소] corpus exit-code 문자열 없음
- [해소] hard-cut 무손실 (git pickaxe)
- [잔여 T0] auth(4) 의 정확한 slug 문자열 = `unauthenticated` 추정 → deauth 후 `axhub auth status --json` 또는 401 경로 실행으로 확정
- [잔여 T0] rate_limited(6) slug 문자열 + `apis.call_consent_required` 의 실제 CLI 출처(ax-hub-cli grep 0건 — plugin-side 또는 stale) 확정
- [잔여 T0] CLI 전체 slug 집합 (drift-guard 의 pinned snapshot 모집단) — `error.rs` 의 `error_code()`/`code()` 변형별 slug 전수

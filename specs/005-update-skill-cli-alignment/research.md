# Phase 0 Research: update 스킬 ↔ ax-hub-cli v0.17.2

모든 사실은 primary-source 직접 확인 (agent fan-out 안 함 — 이미 권위 원천 정독). 원천:
- live `~/.axhub/bin/axhub update {--help, check --help, apply --help}` (v0.17.2)
- `ax-hub-cli/axhub/src/commands/update.rs` (JSON 출력 + 흐름)
- `ax-hub-cli/crates/axhub-core/src/exit_code.rs` (ExitCode enum)
- `ax-hub-cli/crates/axhub-core/src/error.rs` (Error variant + `subcode()`)
- `ax-hub-cli/docs/cli-exit-codes.md` (권위 SLA 표)

---

## D1. CLI 명령 surface

- **Decision**: 스킬은 `axhub update check` 와 `axhub update apply` 만 쓰고, `apply` 플래그는 `--dry-run`(기본 true) / `--execute` / `-y,--yes` / `--force` + 전역 `--json` 만 사용해요.
- **Rationale**: live `update --help` / `apply --help` 가 정확히 이 surface 를 노출. `--dry-run` 기본 true 라 destructive(`--execute`)는 명시 필요. `--yes` 는 execute 의 post-confirm prompt skip.
- **Alternatives**: (기각) 스킬의 기존 `apply --dry-run --json` 은 유효하나 `AXHUB_REQUIRE_COSIGN=1` 접두는 무효 — D5 참조.

## D2. 종료 코드 계약 (update 관련)

- **Decision**: 스킬은 아래 코드만 다뤄요 —
  | exit | 의미 | 스킬 행동 |
  |---|---|---|
  | 0 | 성공 | 완료 안내 |
  | 1 | Generic(IO/serde 등) | 일반 오류 안내 |
  | 4 | Unauthenticated | `axhub auth login` 유도 |
  | 10 | Timeout | 안내(단 apply 전송 실패는 자동 재시도 금지) |
  | 14 | VerifyDigestMismatch{expected,actual} | **변조 신호 → 즉시 중단, force 금지, IT/보안 통보** |
  | 15 | SwapFailed{detail} | **자동 재시도 금지(부분 교체 가능), 복구 안내** |
  | 64 | Usage | 입력/사용 오류, 인자 교정 |
  | 66 | Enforce blocked | subcode 로 분기 (D3) |
- **Rationale**: `exit_code.rs` ExitCode enum + `docs/cli-exit-codes.md` SLA 표가 1:1 원천. 14/15 는 release-integrity-five 신규.
- **Alternatives**: (기각) 스킬의 "exit 2 = autoupdate disabled" — exit 2 는 **clap usage 예약**("Do NOT remap", reserved). autoupdate-disable exit 2 는 허구라 삭제.

## D3. exit 66 subcode 문자열

- **Decision**: exit 66 은 `error.subcode` 로 둘로 갈라요 —
  - `update.downgrade_blocked` → 다운그레이드 차단. `--force`(cosign 안전)로 우회 가능.
  - `update.cosign_enforce_failed` → cosign enforce 실패. **하드 스톱, 우회 없음.**
- **Rationale**: `error.rs` `subcode()` — `CosignEnforceBlocked => Some("update.cosign_enforce_failed")` (line 318), `DowngradeBlocked{subcode}` 테스트값 `"update.downgrade_blocked"` (line 450-451).
- **Alternatives**: (기각) 스킬의 `update.cosign_verification_failed` — 실제 문자열과 불일치. `update.cosign_enforce_failed` 로 교체.

## D4. JSON 봉투 형태

- **Decision**: 스킬이 파싱하는 필드 —
  - `update check --json` → `{"current": "...", "latest": "...", "has_update": bool}` (`CheckJson`)
  - `update apply --dry-run --json` → `{"applied": false, "preview": true, "current", "latest", "has_update", "is_downgrade", "feed_base", "next_step"}`
  - `update apply --execute --json` (성공) → `{"applied": true, "install_kind": "self_replace", "current", "latest", "binary": "<path>"}`
  - 오류 봉투 → `error.subcode` 우선 (hint 텍스트보다).
- **Rationale**: `update.rs` 의 `serde_json::json!` 블록(line 303-313 execute, 350-362 preview, CheckJson struct line 59-62)에서 직접 확인.
- **Alternatives**: (확인됨) 스킬의 `{current,latest,has_update}` 가정은 check 에서 정확 — 유지. preview/execute 의 추가 필드(`is_downgrade`/`applied`/`binary`)는 스킬이 활용하면 더 정확.

## D5. cosign 모델

- **Decision**: cosign 검증은 **기본 Enforce 정책**(항상 켜짐)이라 스킬은 어떤 env 도 설정하지 않고 그냥 `axhub update apply --execute --yes --json` 을 호출해요. `--force` 는 **다운그레이드 게이트만** 우회하고 **서명 검증은 절대 우회 안 함**.
- **Rationale**: `update.rs:324 resolve_cosign_policy_from_env` 가 `cosign::resolve_policy(Stage::Enforce, ...)` — 기본 Enforce. `apply --help`: "`--force` ... Does NOT bypass cosign verification". README §710-711: v0.14.0+ 자산별 `.sha256` + cosign keyless `.sig`/`.pem` 검증.
- **Alternatives**: (기각) `AXHUB_REQUIRE_COSIGN=1` 접두 — 그런 env 없음(전 crate grep 무결과). cosign 정책 세부 config env(있다면 IT/admin 영역)는 스킬 범위 밖 — 스킬은 기본 enforce 에만 의존.

## D6. 존재하지 않는 env (삭제 대상)

- **Decision**: 본문에서 `AXHUB_REQUIRE_COSIGN`, `AXHUB_ALLOW_UNSIGNED`, `AXHUB_DISABLE_AUTOUPDATE` 전부 제거.
- **Rationale**: `axhub/src/cli.rs` 전역 env + 전 crate grep 에 셋 다 없음. 실재 전역 env 는 `AXHUB_JSON/PROFILE/TENANT/YES/NON_INTERACTIVE/NO_COLOR/...` 등.
- **Alternatives**: (기각) "회사 정책 disable" 시나리오 — `AXHUB_DISABLE_AUTOUPDATE` + exit 2 둘 다 허구라 시나리오 통째 삭제. (비대화형 가드는 별개로 유지 — D8)

## D7. brew/scoop 분기

- **Decision**: brew/scoop 감지 분기(`exit 1 + package_manager:"brew"`)와 관련 안내 **통째 제거**, generic 재설치 note 도 안 둬요 (spec Clarifications).
- **Rationale**: CLI 가 package_manager 신호를 안 내보냄(grep 무결과). v0.14.0+ 는 `~/.axhub/bin` self-manage(self_replace). dead branch.
- **Alternatives**: (사용자 기각) "제거 + generic note" / "brew 안내 유지" — 사용자가 "완전 제거" 선택.

## D8. 비대화형 가드 (유지)

- **Decision**: `[ -t 1 ]` / `$CI` / `$CLAUDE_NON_INTERACTIVE` 가드 유지, apply 동의 기본값 = skip(자동 적용 안 함). `tests/fixtures/ask-defaults/registry.json` 의 `update.apply_consent` 와 일관.
- **Rationale**: CLI 도 `--non-interactive`/`AXHUB_NON_INTERACTIVE` 지원. 기존 D1 guard 패턴은 CLI 와 무관한 skill UX 계약이라 유효 — 정렬 대상 아님.
- **Alternatives**: (기각) 제거 — headless 안전 위반.

## D9. drift-guard (범위 밖)

- **Decision**: 스킬↔CLI 계약 자동 대조 테스트/CI guard 는 이번 feature 에 **미포함**. CLI 의 `axhub update --json-schema`(agents/CI drift용)는 향후 follow-up 후보로만 기록.
- **Rationale**: spec Clarifications — 사용자 "정렬만, guard 제외". simplicity-first + 원요청=정렬.
- **Alternatives**: (사용자 기각) 경량 snapshot guard / 수동 re-verify note.

## D10. error-empathy-catalog 생성물 관계 (확인 완료)

- **Decision**: catalog 의 stale subcode 수정은 **codegen 체인 + hand-authored 양쪽** 을 건드려요. 단, catalog 키 변경 전에 blast radius 를 먼저 grep 해요 (tasks 첫 항목).
- **확인된 체인** (grep + `scripts/codegen-catalog.ts` head):
  - **Source-of-truth**: `crates/axhub-helpers/data/catalog.json` (Rust 데이터, 이 repo). 여기에 stale 키 `update.cosign_verification_failed` 존재.
  - **생성물**: `skills/deploy/references/error-empathy-catalog.generated.md` (헤더 "AUTO-GENERATED by scripts/codegen-catalog.ts. Edit catalog.json then re-run") — `bun run codegen:catalog` 로 재생성. line 232 에 stale 키.
  - **hand-authored**: `skills/deploy/references/error-empathy-catalog.md` (skill 이 실제 링크하는 파일) — line 160 에 stale 키. 직접 수정.
  - **drift test**: `tests/codegen.test.ts` 가 catalog.json ↔ generated.md 일치 강제 → catalog.json 수정 시 반드시 regen.
  - stale 분포: catalog.json + generated.md(232) + hand .md(160) + `SKILL.md`(115, 152) = **5 사이트**.
- **Rationale**: `.generated.md` 직접 편집은 regen 에 덮어써짐. 올바른 순서 = catalog.json 수정 → `bun run codegen:catalog` → hand `.md` 수정.
- **blast radius 주의 (tasks 에서 grep 필수)**: catalog.json 의 `update.cosign_verification_failed` 키가 helper Rust 코드 / 다른 skill / subcode→entry 매핑에서 참조되는지 확인 후 변경. 스킬이 CLI `error.subcode` 로 catalog 를 lookup 한다면 키는 실제 CLI subcode(`update.cosign_enforce_failed`)와 일치해야 정합. 디커플링이면 별도 판단.
- **주의**: 이 catalog 정정은 부수 정렬이고, 본 feature 의 1차 대상은 여전히 `SKILL.md` 예요. catalog 키 변경이 과한 blast radius 로 판명되면 tasks 에서 범위 재조정.

---

## Gap 요약 (구 스킬 → v0.17.2 실제)

| 항목 | 구 스킬 (현재) | v0.17.2 실제 | 조치 |
|---|---|---|---|
| cosign 강제 | `AXHUB_REQUIRE_COSIGN=1` 접두 | 기본 Enforce 정책, env 없음 | 접두 제거 |
| unsigned 우회 | `AXHUB_ALLOW_UNSIGNED=1` 언급 | 존재 안 함 | 제거 |
| autoupdate disable | exit 2 + `AXHUB_DISABLE_AUTOUPDATE` | exit 2=clap usage 예약, env 없음 | 시나리오 삭제 |
| cosign 실패 subcode | `update.cosign_verification_failed` | `update.cosign_enforce_failed` | 교체 |
| downgrade | (없음) | exit 66 `update.downgrade_blocked`, `--force` 우회 | 추가 |
| 변조 탐지 | (없음) | exit 14 VerifyDigestMismatch | 추가(하드스톱) |
| swap 실패 | (없음) | exit 15 SwapFailed | 추가(재시도 금지) |
| brew | exit 1 + package_manager 분기 | 신호 없음 | 제거 |
| check JSON | `{current,latest,has_update}` | 동일 ✓ | 유지 |

## 검증 가능성

`~/.axhub/bin/axhub` v0.17.2 설치돼 있어 live 대조 가능(`update --help`, dry-run preview). 불가 환경에선 `docs/cli-exit-codes.md` + contract 문서가 대체 권위.

# Phase 1 Data Model: verify 가 분기하는 계약 엔티티

데이터 저장 없음. "엔티티" = 스킬이 파싱·분기하는 CLI/helper 표면.

## 엔티티

### VerifyResult (`axhub-helpers verify --json`)
| 필드 | 타입 | 비고 |
|---|---|---|
| verdict | enum | `live` / `suspect` / `not_live` (snake_case). **`passed` 없음** |
| state | string? | helper LIVE_STATES 로 판정한 상태 |
| last_deploy_id | string? | |
| last_deploy_age_secs | u64? | FRESH window 600s |
| errors | string[] | ERROR/FATAL 로그 라인 |
| reasons | string[] | 한국어 사유 — verdict 아래 verbatim 출력 |

### DeployStatus (`axhub deploy status [id] --app --json`)
| 필드 | 타입 | 비고 |
|---|---|---|
| status | string | **백엔드 free string** (닫힌 enum 아님) |
| current_stage | string? | 단계 안내용 |

판정 규칙: `status` ∈ LIVE_STATES(`live`/`running`/`deployed`/`active`/`ok`/`succeeded`) → live 신호. 그 외 → 미라이브(진행중/실패) 휴리스틱.

### AppLogs (`axhub deploy logs --app --json`)
- app-level 로그 라인 배열. deployment_id 스코핑 안 함. client-side 마지막 ~50줄 trim → `ERROR`/`FATAL` grep. `--source` = free passthrough(고정 enum 없음).

### PreflightContext (`axhub-helpers preflight --json`)
| 필드 | 비고 |
|---|---|
| auth_ok | false → `/axhub:auth` |
| auth_error_code | cli_not_found→install-cli / cli_config_corrupted→auth / cli_too_old→upgrade |
| current_app, last_deploy_id | Step 1 식별에 사용 |

### ErrorCode (recover 정본 표 참조)
`auth.token_invalid`/`resource.app_not_found`/`transport.timeout`/`transport.cli_missing`/`response.invalid_json`/`response.error_envelope_unknown_shape` → recover SKILL.md 표로 라우팅.

## 상태 전이 (verify 워크플로)

```
[preflight] auth_ok? ──no──> auth 라우팅 (치명적이면 종료)
   │ yes
[Step1] current_app + last_deploy_id (없으면 helper list-deployments --app-id)
   └─ 후보 없음 ──> "최근 배포 없음" 종료
[Step2] deploy status [id] --app --json (5s)
   ├─ status ∈ LIVE_STATES ──> live 신호
   ├─ timeout/exit≠0 ──> 의심 사유
   └─ 그 외 status ──> 미라이브 사유 (current_stage 안내)
[Step3] deploy logs --app --json (5s, app-level) → 마지막 50줄 ERROR/FATAL grep
   └─ 발견 시 first 3줄 quote
[Step4] (선택) health endpoint GET 200 — 미설정 시 AskUserQuestion(비대화형 skip)
[Step5] verdict ✅/⚠️/❌ + reasons verbatim
```
> CI 경로: `axhub-helpers verify --json --app-id <app>` → VerifyResult 직접 소비 (verdict ∈ {live,suspect,not_live}).

## 검증 규칙 (스킬↔엔티티)
- 스킬이 읽는 helper 필드 ∈ VerifyResult (verdict 값 정확, `passed` 금지).
- status 를 닫힌 enum 으로 매칭하지 않음 — LIVE_STATES 휴리스틱.
- logs 는 app-level(`--app`), `pod` 고정 source 가정 없음.
- 비대화형 기본값 = health setup skip (registry `health_endpoint_setup`).

## 비-엔티티 (이 feature 가 만들지 않음)
- 영속 데이터/DB/config 신규 없음. drift-guard fixture 없음(별도 feature). helper/CLI 코드 변경 없음.

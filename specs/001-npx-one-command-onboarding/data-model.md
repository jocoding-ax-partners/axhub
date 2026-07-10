# Data Model: npx 원커맨드 온보딩

**Date**: 2026-07-10 · **Spec**: [spec.md](./spec.md) · 영속 DB 없음 — 전부 실행 중 상태와 출력 계약이에요.

## SetupStep (셋업 단계)

셋업이 관리하는 단계 목록과 상태예요 (스펙 FR-001·FR-003, Key Entities "셋업 단계 상태").

| 필드 | 값 | 설명 |
|---|---|---|
| id | `cli_install` \| `marketplace` \| `plugin` \| `mcp` | 순서 고정 — 4단계 |
| status | `todo` \| `running` \| `done` \| `skipped` \| `failed` | `skipped` 는 "이미 완료돼 건너뜀" 표시예요 |
| evidence | string | 판정 근거 한 줄 (예: 감지된 CLI 버전, `claude mcp get` 결과) — raw 출력·내부 id 는 사용자에게 노출하지 않아요(FR-011) |

**상태 전이**: `todo → running → done | failed`. 재실행 시 완료 단계는 `todo → skipped`(확인 표시 후 통과). `failed` 에서 재실행하면 그 단계부터 재개돼요(수용 시나리오 3). 앞 단계가 `failed` 면 뒤 단계는 실행하지 않아요.

**검증 규칙**: `mcp` 단계는 `claude` 감지 성공이 전제예요 — claude 부재 시 `marketplace`·`plugin`·`mcp` 는 실행하지 않고 종료해요(FR-004).

## SetupRunMode (실행 모드)

비대화형 계약(FR-012)의 판정 입력이에요.

| 필드 | 값 | 설명 |
|---|---|---|
| interactive | boolean | 터미널 입력 가능 + CI 아님 + `--yes` 없음 |
| reason | `no_tty` \| `ci_env` \| `yes_flag` \| — | 비대화형 판정 사유 (안내 문구에 사용) |
| dry_run | boolean | `--dry-run` — 고지 목록만 출력하고 무변경 종료(FR-013) |

**불변식**: `interactive=false` 면 입력 대기 0회·브라우저 열기 0회. `dry_run=true` 면 모든 SetupStep 이 상태 변경 없이 예정 목록으로만 출력돼요.

## InstallationFootprint (설치 산출물)

실행 전 고지(FR-013)와 사용자 문서 공개(FR-007)가 공유하는 단일 목록이에요 — 설계 문서 §4.3 이 기준이고, 두 곳이 어긋나면 결함이에요.

| 필드 | 설명 |
|---|---|
| item | 산출물 이름 (CLI 바이너리·PATH·작업 디렉토리·마켓플레이스·플러그인·MCP 설정·npm 캐시) |
| location | user-scope 경로 (예: `~/.axhub/bin/axhub`) — 시스템 전역 경로 금지(FR-005) |
| owner | 만들고 관리하는 주체 (setup \| claude CLI \| npm) |
| action | 이번 실행에서 `install` \| `skip`(이미 있음) \| `none`(dry-run) |

## HandoffCard (핸드오프 카드)

성공 종료 시 출력물이에요 (FR-002, Key Entities "핸드오프 카드").

| 필드 | 설명 |
|---|---|
| summary | 단계별 결과 요약 (done/skipped 표시) |
| next_phrase | 고정 다음 말 — "처음인데 셋업해줘" |
| restart_note | "Claude Code 를 열거나, 이미 켜져 있으면 재시작" — 항상 포함(R-11) |
| rerun_note | "같은 명령을 다시 실행해도 안전해요(완료 항목은 건너뜀)" — 항상 포함 |

## DoctorReport (진단 결과)

`setup doctor` 출력이에요 (FR-014). 읽기 전용 — 상태를 바꾸지 않아요.

| 필드 | 값 | 설명 |
|---|---|---|
| checks[] | 산출물별 `{ id, status: ok \| problem, detail, fix }` | id 는 SetupStep 4종 + `runtime`(Node 등 환경 등급, FR-015) |
| overall | `ok` \| `problem` | 하나라도 problem 이면 problem |
| fix | string | 해결 명령 또는 다음 행동 한 줄 — 자동 수정은 하지 않아요(R-7) |

## EnvironmentCheck (환경 점검 등급)

FR-015 의 판정 모델이에요.

| 등급 | 조건 | 동작 |
|---|---|---|
| `block` | 진짜 최소 미만 (예: Node < 18) | 감지값·요구 범위 명시 후 중단 |
| `warn` | 지원 목록 밖이지만 동작 가능 | 감지값·요구 범위 명시한 경고 후 계속 진행 |
| `ok` | 지원 범위 | 통과 |

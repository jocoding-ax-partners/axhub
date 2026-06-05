# Phase 0 Research: `infer-tables-env`

스펙의 미해결 항목과 기술 선택을 결정으로 확정해요. 각 항목은 Decision / Rationale / Alternatives.

## D1. 추론 엔진: 결정론 Rust vs LLM 오케스트레이션 SKILL

- **Decision**: 별도 Rust 추론 엔진을 만들지 않고, 새 SKILL 이 LLM 으로 소스를 직접 분석하고 적용은 기존 `tables`/`env` 스킬로 위임한다.
- **Rationale**: (1) axhub 는 thin-layer/skill-composition 철학 — 모든 능력이 SKILL 로 조합되고, `*-summary` helper 도 CLI 출력만 파싱하지 사용자 소스를 파싱하지 않는다. (2) 다국어 소스 파서(Prisma schema, Alembic, SQLAlchemy/SQLModel AST, TS env reads)를 5개 크로스아치 바이너리로 신설·유지하는 비용이 크고 깨지기 쉽다. (3) 소스 코드 이해·데이터모델 추론은 LLM 의 강점이다. (4) 이 기능은 **사람이 검토·승인하는 추천**이라 자율 mutation 이 아니므로 best-effort 추론을 사람 승인 게이트로 보완할 수 있다.
- **Alternatives considered**:
  - *결정론 Rust 엔진(advisor 권고)*: recall/precision 을 fixture→골든 JSON 유닛테스트로 보장하고, 시크릿 비노출(FR-012/SC-005)을 코드로 강제하며, 근거(file:line)가 정확하다. 그러나 유지비·범위가 reviewed-recommendation 도구에는 과하다고 판단. **V2 hardening 후보로 보류**(테스트 가능한 recall 이 계약 요구가 되면 도입).
  - *하이브리드(작은 결정론 secret-scan + LLM 나머지)*: MVP 복잡도 증가. YAGNI 로 제외.

## D2. 분석 범위 절단: declarative 아티팩트 우선

- **Decision**: MVP 는 **선언적 스키마·환경 아티팩트**를 1급·고신뢰로 분석한다 — `schema.prisma`, Prisma/Alembic 마이그레이션 파일, `.env.example`/`.env.sample`. 코드로만 정의된 모델(흩어진 ORM 클래스, 런타임 `getenv`)은 베스트에포트 "검토 필요"로 표시하고 recall 보장(SC-001) 대상에서 제외한다.
- **Rationale**: 난이도·신뢰도는 스택 간보다 스택 *안*에서 더 크게 갈린다. 선언적 아티팩트는 명시적 → 고정밀·저비용·정확한 근거. SQLAlchemy 클래스/산재 getenv 파싱이 취약한 부분이다. 이 절단이 SC-001 의 정직한 scope 를 만든다.
- **Alternatives considered**: *ORM 별 절단(TS=Prisma, Py=SQLModel)* — 같은 스택 안의 신뢰도 편차를 못 잡아 SC-001 이 과대약속됨. 제외.

## D3. env 적용 의미: 키만 추론, 값은 사용자 입력

- **Decision**: 추론은 env **키**와 시크릿 여부만 도출한다(값 추론 안 함, FR-016). 적용 시 테이블은 추론 스키마로 자동 생성하고, env 는 키만 등록하되 각 값은 `env` 스킬의 `--from-stdin` 으로 사용자가 입력하거나 건너뛴다. 비시크릿 기본값이 소스에 드러나면(예: `getenv("PORT","8000")`) 미리 채울 값으로 제안한다.
- **Rationale**: 소스에서 env *값*은 알 수 없다(시크릿은 더더욱). FR-008 의 "생성/설정"을 키 등록 + 안내 입력으로 정직하게 좁힌다. 항목별 성공/실패/건너뜀(FR-008)이 "사용자가 KEY_X 건너뜀"을 자연히 포함한다.
- **Alternatives considered**: *값까지 자동 채움* — 시크릿 노출·오설정 위험. 제외. 단, 하드코딩 시크릿 발견 시 그 키를 env 로 옮기도록 제안(값은 비표시, FR-012)은 선택 enhancement.

## D4. 적용 위임(handoff) 메커니즘

- **Decision**: 승인 후 적용은 기존 자산을 호출한다 — 테이블: `axhub tables` 생성·컬럼(스키마 known), env: `printf %s "$VALUE" | axhub env set "$KEY" --app "$APP" --from-stdin --json`. 각 mutate 전 `axhub-helpers consent-mint`(테이블은 `action=table_*`, env 는 `action=env_set`, top-level `app_id`, `context`)로 동의 토큰 발급. 멱등(FR-009): `axhub tables list`/`axhub env list` 로 cross-check 후 이미 있는 항목은 건너뜀.
- **Rationale**: `tables`/`env` 스킬이 이미 안전한 mutate 경로(consent, stdin, 마스킹)를 갖췄다. 새 mutation 경로를 만들지 않는다(FR-008/스펙 Assumption).
- **Alternatives considered**: *새 helper subcommand 로 일괄 적용* — 중복·우회 위험. 제외.

## D5. 자동 제안(US3) 트리거 — 경량 넛지

- **Decision**: 자동 제안은 경량 비차단 넛지로만(전체 스캔 없이). MVP 구현은 hook 신설 대신 (a) SKILL `description` 의 분석형 트리거 어구 + (b) `init`/`deploy` 스킬 흐름 끝에 한 줄 자연어 제안("필요한 테이블·환경변수 추천해드릴까요?") 추가로 최소화한다. 사용자가 수락하면 그때 전체 분석.
- **Rationale**: hook 신설은 fail-open/kill-switch 계약(Phase 25) 부담 + 개발 흐름 차단 위험. 한 줄 넛지가 FR-010 의 비차단·무부작용을 만족하면서 최소다.
- **Alternatives considered**: *SessionStart/PostToolUse hook 으로 자동 스캔* — 무겁고 흐름 차단 위험. V2 후보.

## D6. 트리거 키워드 — `tables`/`env` 와 충돌 회피

- **Decision**: slug = `infer-tables-env`(spec dir 일치). `description` 트리거는 **분석형**으로 한정 — 예: "내 코드 분석해서 테이블/env 추천", "필요한 테이블 뭐야", "필요한 환경변수 추론", "scan my project", "추천해줘". CRUD 형("테이블 만들", "env 추가")은 기존 `tables`/`env` 가 소유하므로 넣지 않는다.
- **Rationale**: `lint:keywords` 베이스라인이 트리거를 잠그고, 충돌하면 라우팅 오인. 분석형/CRUD형 분리가 라우팅을 깨끗하게 한다. 새 SKILL 이라 베이스라인 재캡처는 허용된 rare event.
- **Alternatives considered**: CRUD 어구 공유 — 오인 라우팅. 제외.

## D7. 시크릿 안전 + 근거 — 결정론 코드 없이 확보

- **Decision**: 시크릿 비노출은 SKILL `NEVER` 규칙(시크릿 리터럴·값 평문 출력 금지) + `env` 스킬의 stdin/마스킹 + mutate 전 마스킹 미리보기 + 사람 승인으로 확보. 근거(file:line, 패턴)는 LLM 이 자기 분석에서 인용하고, 추천 표는 항목마다 근거를 의무 표기(SC-004).
- **Rationale**: reviewed-recommendation 경로(자율 mutation 아님)라 행위 계약 + 사람 검토로 충분. 결정론 강제는 V2.
- **Alternatives considered**: 정규식 시크릿 스캐너(Rust) — D1 과 묶어 V2 보류.

## D8. 로컬 소스 검사 계약 — governed-data-read 와 반대

- **Decision**: 이 SKILL 은 **로컬 앱 소스(모델 파일, `.env.example`, config)를 읽는 inspection 스킬**이다. governed data-read 스킬(`data`/`tables`/`env` 조회)의 "로컬 파일·`.env`·repo 금지" 라우팅 계약과 정반대 성격임을 명시한다. 자체 계약: read-only 파일 검사 + 시크릿 비에코 + 적용은 승인 후 위임.
- **Rationale**: 라우팅 hint 의 governed-data 세계관에 우발적으로 묶이지 않도록 plan 에 못박는다.

## Resolved unknowns

- Technical Context 에 NEEDS CLARIFICATION 없음(스펙 Clarifications + 위 결정으로 전부 해소).
- Deferred(저영향, plan 후): 관측성/감사 로그 — `consent-mint`/CLI 가 자체 audit 를 남기므로 별도 요구는 V2.

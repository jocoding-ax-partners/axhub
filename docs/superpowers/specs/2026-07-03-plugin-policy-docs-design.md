# axhub plugin 정책 문서 체계 설계

날짜: 2026-07-03
상태: 승인됨 (브레인스토밍 세션에서 확정)

## 목적

axhub plugin 의 규칙들이 CLAUDE.md·README·각 SKILL.md 에 흩어져 있어요. 이 설계는 세 독자(에이전트·사용자·기여자)를 위한 정책을 각각 한 문서로 모으고, 정책 문서를 **canonical 원천**으로 승격해요. 충돌 시 정책 문서가 이겨요.

## 결정 사항 (확정)

| 결정 | 선택 |
|------|------|
| 문서 성격 | 에이전트 행동 + 사용자 대상 + 개발·운영 3종 전부 |
| 권위 관계 | 정책 문서가 원천 (canonical source) |
| 문서 구조 | 독자별 3문서 분리 |
| drift 가드 | parity 테스트 (bun test 합류) |

## 산출물: 문서 3개 + 가드 1개

### 1. `docs/policy/agent-policy.md` — 에이전트 행동 정책 (원천)

기존 산재 규칙을 수집해 규칙마다 ID(`AP-N`)를 부여해요. 규칙 블록 구조는 parity 테스트가 파싱할 수 있게 고정해요:

```markdown
## AP-3 deploy 성공 선언
- 규칙: deploy verify 1회 exit 0 + 접근 가능 URL 확인 전까지 성공 선언 금지. static 앱은 active_release_id lane.
- 적용: skills/deploy/SKILL.md
- invariant: "deploy verify", "active_release_id"
```

- `적용:` — 이 규칙의 복사본을 반드시 담아야 하는 SKILL.md 경로 목록. 복수면 쉼표로 구분해요 (예: `skills/deploy/SKILL.md, skills/clarity/SKILL.md`).
- `invariant:` — parity 테스트가 해당 파일에서 grep 으로 존재를 검사할 핵심 문구 목록.

초기 수집 대상 규칙 (후보, 구현 시 본문 재확인):

1. deploy 성공 선언 규칙 — verify 1회, deployment id + app scope 필수, latest 재탐색 금지, static lane 예외 (`active_release_id`)
2. 파괴적 명령 승인 gate — 삭제·롤백·force/execute 는 사용자 승인 필수 (clarity), deploy 는 preview-confirm gate
3. diagnosis read-only 보장 — 재배포·롤백 직접 실행 금지
4. development read 전용 v1 — 조회 기반 기능 코드만 생성
5. 최소 CLI 버전 게이트 — `plugin-support` preflight, 0.20.0+ 미만이면 중단·안내, 우회 금지
6. skill 간 양보(routing) 규칙 — 8 skill 의 경계와 양보 방향
7. onboarding 의 init 자동 실행 금지 — 빈 폴더에서도 자동 bootstrap 하지 않음
8. hidden `plugin-support` 표면 사용 규칙 — clarity 는 공개 표면만, 나머지는 hidden 허용

문체: 해요체.

### 2. `POLICY.md` (repo 루트) — 사용자 대상 정책

플러그인 설치자에게 공개하는 문서. 내용:

- 네트워크 접근 시점 — auto-update 체크(`axhub update check`), axhub CLI/MCP 호출
- 로컬에 기록하는 파일 — `~/.axhub/cache/.plugin-update-check`, `~/.axhub/cache/.onboarding-mcp-restart` marker
- 자동 업데이트 동작과 끄는 법 — `AXHUB_NO_AUTO_UPDATE=1`, `AXHUB_NO_ONBOARDING_RESUME=1`
- 파괴적 작업 승인 약속 — 삭제·롤백·force/execute 는 항상 사용자 확인 후 실행
- 데이터 범위 — tenant-scoped OAuth, MCP 도구 기본 read-only

배포: `scripts/build-plugin-bundle.ts` 의 `ROOT_FILES` 배열에 `"POLICY.md"` 1줄 추가로 bundle 포함. 루트에 두는 이유: GitHub 에서 바로 보이고 bundle 포함이 1줄이라서요.

### 3. `docs/policy/dev-policy.md` — 개발·운영 정책

`docs/` 는 bundle 의 copy 대상(`ROOT_FILES`/`ROOT_DIRS`)에 없어 자동 제외 — 위치만으로 내부용 분리가 끝나요. 내용:

- diet 체제 원칙 — 새 skill 추가를 정당화하는 기준 (현재 8 skill)
- 해요체 tone 규칙과 `lint:tone --strict` gate
- frontmatter 유효성 규칙
- release 3단계 flow (`bun run release` → CHANGELOG amend → `bun run release:tag`)
- quality gate 목록 (lint:tone, frontmatter check, 대표 여정 회귀, plugin:bundle)
- hidden 표면 계약 — parity 테스트 + 최소 CLI 버전 게이트로 동기화
- bundle 규칙 — 포함/제외 기준

### 4. drift 가드: `tests/policy-parity.test.ts`

- `docs/policy/agent-policy.md` 의 규칙 블록(`## AP-N` + `적용:` + `invariant:`)을 파싱해요.
- 각 invariant 문구가 적용 대상 SKILL.md 에 실제 존재하는지 검사해요. grep 수준 문자열 매칭, AST 불필요.
- 기존 `bun test` 에 합류 — 별도 CI 단계 없음.
- invariant 문구를 바꾸면 양쪽(정책 + SKILL.md)을 함께 고쳐야 통과 — 원천에서 먼저 고치는 흐름을 강제해요.

### lint:tone 커버리지 (구현 시 주의)

`check-toss-tone-conformance.ts` 의 기본 스캔은 `skills/*/SKILL.md` 뿐이에요 (`explicit` 목록은 현재 비어 있음). 새 문서 3개를 커버하려면 `explicit` 목록에 `docs/policy/agent-policy.md`, `docs/policy/dev-policy.md`, `POLICY.md` 를 추가해야 해요.

## 기존 문서 정리 (최소)

- CLAUDE.md·README 의 중복 규칙 서술은 삭제하지 않아요.
- "이 규칙들의 원천은 `docs/policy/`, 충돌 시 정책 문서가 이겨요" 포인터 한 줄씩만 추가해요.

## 작업 순서

1. `docs/policy/agent-policy.md` 작성 (규칙 수집 + ID 부여) → 검증: 규칙 블록 구조가 파싱 가능한 형태인지
2. `tests/policy-parity.test.ts` 작성 → 검증: `bun test` 통과 (invariant 문구가 실제 SKILL.md 에 존재)
3. `POLICY.md` 작성 + `build-plugin-bundle.ts` ROOT_FILES 1줄 추가 → 검증: `bun run plugin:bundle` 후 dist 에 POLICY.md 존재
4. `docs/policy/dev-policy.md` 작성 → 검증: lint:tone explicit 목록 추가 후 `bun run lint:tone --strict` 0 err
5. CLAUDE.md·README 포인터 추가 → 검증: 문구 충돌 없음

## 범위 밖 (skipped)

- codegen (정책 → SKILL.md 생성) — diet 가 제거한 codegen 회귀라 과잉
- 기존 문서 대수술 — 포인터만 추가
- 정책 버저닝/이력 체계 — parity 테스트가 반복적으로 시끄러워지면 그때 고려

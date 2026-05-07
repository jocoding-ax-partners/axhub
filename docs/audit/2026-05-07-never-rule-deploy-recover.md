# NEVER 룰 감사 — deploy / recover SKILL

감사 일자: 2026-05-07
브랜치: feat/audit-never-rule
PR: #46

## 감사 범위

- `skills/deploy/SKILL.md`
- `skills/recover/SKILL.md`
- 비교 기준: `skills/init/SKILL.md` (PR #41 패턴)

---

## deploy/SKILL.md NEVER 룰

| # | 룰 | 분류 |
|---|---|---|
| 1 | `axhub-helpers bootstrap --auto-chain --json` 출력을 승인으로 취급 금지 | FSM 안전 |
| 2 | idempotency key + retry policy 없이 `apps_create`/`deploy_create` 재시도 금지 | 멱등성 |
| 3 | destructive 명령 실행 후 `bootstrap --record` 건너뛰기 금지 | 감사 추적 |
| 4 | exit 64에서 `axhub deploy create` 재시도 금지 | 충돌 방지 |
| 5 | `--json` 플래그 누락 금지 | 파싱 안전 |
| 6 | `consent-mint` 없이 `axhub deploy create` 호출 금지 | 동의 게이트 |
| 7 | `deploy_cancel` consent token 없이 cancel 호출 금지 | 동의 게이트 |
| 8 | `app_id`를 pwd/git remote에서 추론해 mutation 경로에서 사용 금지 | 정확성 |
| 9 | AskUserQuestion preview card 건너뛰기 금지 | UX 안전 |

### dep install 트리거 케이스 식별

워크플로 전체 스캔 결과: **dep install 트리거 케이스 없음**

- Step 1.5 git-init 블록에 `git init`, `git add -A`, `git commit` 존재
- npm, yarn, bun install, pip install 등 package manager 호출 없음

---

## recover/SKILL.md NEVER 룰

| # | 룰 | 분류 |
|---|---|---|
| 1 | "진짜 rollback"이라 주장 금지 — 항상 "forward-fix" / "직전 커밋 재배포" 명시 | 사용자 신뢰 |
| 2 | consent token mint 건너뛰기 금지 | 동의 게이트 |
| 3 | AskUserQuestion confirmation 건너뛰기 금지 | UX 안전 |
| 4 | succeeded deploy를 사용자에게 안 보여주고 자동 선택 금지 | 투명성 |
| 5 | `axhub deploy create`에서 `--json` 누락 금지 | 파싱 안전 |

### dep install 트리거 케이스 식별

워크플로 전체 스캔 결과: **dep install 트리거 케이스 없음**

- list-deployments, deploy create만 실행
- package manager 호출 없음

---

## init SKILL 패턴 정합 검토 (PR #41)

`skills/init/SKILL.md:108`:
> `NEVER Node, package manager, dependency install 을 자동 실행하지 않아요.`

init SKILL frontmatter에 `allows-dependency-execution` 필드 없음 — dep install을 명시적으로 금지하는 NEVER 룰이 있으므로 해당 frontmatter가 불필요해요.

deploy/recover는 워크플로 특성상 dep install을 아예 수행하지 않아요. init처럼 명시적 금지 룰도, `allows-dependency-execution` frontmatter도 모두 불필요해요. 세 SKILL 모두 일관성을 유지해요.

---

## 결론: frontmatter 추가 ROI 없음 — 두 SKILL 현 상태 유지

| 시나리오 | UX 개선 | 위험 | 결론 |
|---|---|---|---|
| `allows-dependency-execution: false` 추가 | 없음 (dep install 안 하므로 visible change 없음) | frontmatter schema drift 가능 | 비권고 |
| deploy NEVER에 dep install 금지 룰 추가 | 없음 (현재 워크플로에 dep install 없음) | 불필요한 규칙 증가 | 비권고 |
| recover NEVER에 dep install 금지 룰 추가 | 없음 | 동상 | 비권고 |

---

## 향후 재검토 트리거 조건

다음 중 하나라도 해당하면 이 감사를 재실행해요:

- deploy SKILL이 npm package 배포 흐름 (`npm publish`, `bun publish` 등)을 추가할 때
- recover SKILL이 의존성 복원 (예: `npm ci`) 을 워크플로에 포함할 때
- `allows-dependency-execution` frontmatter 스키마가 SKILL 표준으로 승격될 때

---

## 검증 결과

| 명령 | 결과 |
|---|---|
| `bun run skill:doctor --strict` | exit 0 |
| `bun run lint:tone --strict` | exit 0 |
| `bun test` | exit 0 |

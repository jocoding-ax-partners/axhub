# axhub plugin 개발·운영 정책

repo 기여자(이 저장소의 코드와 문서를 고치는 사람)를 위한 개발·운영 규칙을 한곳에 모은 기준 문서예요. README·CLAUDE.md 에 다르게 적혀 있으면 이 문서를 따라요. 에이전트 행동 규칙은 `docs/policy/agent-policy.md`, 사용자 공개 정책은 `POLICY.md` 가 각각 기준 문서예요.

## DP-1 diet 체제 — skill 추가 기준
- diet 체제는 스킬 수를 최소로 유지하는 방침이에요. 공개 skill 은 8개(onboarding/bootstrap/import/deploy/development/diagnosis/clarity/update)를 유지해요.
- 새 skill 은 기존 8개의 경계·양보 규칙으로 연결(라우팅)할 수 없는 사용자 의도가 반복해서 관측될 때만 추가해요.
- 판정·실행 로직은 plugin 안에 두지 않고 ax-hub-cli 에 둬요. 라우팅 품질은 외부 corpus(라우팅 학습용 예문 모음)가 아니라 frontmatter(SKILL.md 맨 위의 메타데이터 블록)의 `description`·`examples` 에 투자해요.

## DP-2 tone — 해요체
- 모든 한글 산문은 해요체로 써요. 금지 표현의 기준 목록은 `scripts/check-toss-tone-conformance.ts` 의 `FORBIDDEN` 배열이에요 (여기 나열하면 검사가 이 문서에서 걸리니 목록은 그 파일을 봐요).
- gate(통과하지 못하면 머지할 수 없는 자동 검사): `bun run lint:tone --strict` 0 error. 스캔 대상은 `skills/*/SKILL.md` + `explicit` 목록(정책 문서 3개)이에요.

## DP-3 frontmatter 유효성
- 8개 SKILL.md 의 frontmatter 는 `tests/frontmatter.test.ts` 로 검증해요. `name`·`description` 은 Claude 가 어떤 스킬을 쓸지 고르는 기준(라우팅 표면)이라 깨지면 스킬이 아예 선택되지 않을 수 있어요.

## DP-4 release flow
- `commit-and-tag-version` 도구 기반 3단계로 릴리즈해요:
  1. `bun run release` — 버전 숫자 올리기(bump) + 커밋 (버전 표식인 tag 는 아직 안 만듦)
  2. CHANGELOG 에 해요체 1-3 문장으로 변경 요약(narrative)을 추가한 뒤 `git commit --amend --no-edit -a` (직전 커밋에 합치기)
  3. `bun run release:tag` — tag 생성 + push

## DP-5 quality gates
머지 전에 반드시 통과해야 하는 자동·수동 검사 목록이에요:
- `bun run lint:tone --strict` — 한글 tone 0 error
- `bun test` — frontmatter·bundle·routing·policy parity 포함 전부 PASS
- `bun run plugin:budget` — 컨텍스트 byte 예산 PASS (SKILL.md 8개 합산 200k · per-skill 35k). 넘치면 본문을 reference 로 추출하되, **실행 경로에 필요한 지시는 빼지 않아요** — reference 는 plugin cache 라 workspace 밖이고 Desktop 은 읽을 때 권한 프롬프트를 띄우는데 우리 규칙(bootstrap 9.1)은 그 프롬프트를 생략하라고 해서 조용히 안 읽혀요. 참고용 상세만 reference 로 가요.
- `bun run plugin:bundle` — clean bundle(개발 산출물이 섞이지 않은 배포용 플러그인 꾸러미) 생성. 로컬 Claude Code 검증은 repo 루트가 아니라 `dist/axhub-plugin` 을 써요.
- 대표 여정 회귀 — 첫 셋업 → 앱 생성 → 배포 → 상태 확인 경로를 문서·skill 본문·fixture(테스트용 고정 예시 데이터) 계약으로 같은 방향에 맞춰요.

## DP-6 hidden 표면 계약
- `axhub plugin-support <cmd>` 는 이 plugin 의 skill 들만 쓰라고 만든 숨김 명령이에요. 일반 사용자용 명령이 아니라서 도움말에도 안 나오고, 예고 없이 바뀌거나 사라질 수 있어요. 그래서 skill 밖에서는 쓰지 않아요.
- 어긋남을 막는 안전장치는 세 겹이에요. ① repo 안 fixture(shim) 계약 테스트 — plugin 이 기대하는 형식을 고정하지만 **실제 CLI 를 실행하진 않아요**. ② nightly 실 CLI 계약 스냅샷(`.github/workflows/cli-contract-nightly.yml` + `tests/cli-contract.test.ts`) — 최신 배포 CLI 를 설치해 onboarding-detect 스키마(`first_gap` null 완료 포함)·preflight exit 계약·deploy logs/verify/diagnose 표면을 검증해 cross-repo drift 를 잡아요. ③ skill 이 시작할 때 이 명령이 실제로 동작하는지 먼저 확인하는 사전 점검(preflight 게이트, agent-policy AP-6)이에요.
- clarity 는 hidden 표면을 쓰지 않아요 (agent-policy AP-9).

## DP-7 bundle 규칙
- bundle 에 무엇이 들어가는지는 `scripts/build-plugin-bundle.ts` 의 `ROOT_FILES`(`README.md`, `LICENSE`, `POLICY.md`) + `ROOT_DIRS`(`.claude-plugin/`, `skills/`) 목록이 기준이에요.
- `docs/`, `tests/`, `scripts/` 같은 개발 산출물은 배포 꾸러미에 포함하지 않아요. 검증은 `bun run plugin:bundle` + `tests/plugin-bundle.test.ts` 가 해요.

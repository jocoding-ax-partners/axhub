# axhub plugin 개발·운영 정책

repo 기여자를 위한 개발·운영 규칙의 canonical 원천이에요. README·CLAUDE.md 의 서술과 충돌하면 이 문서가 이겨요. 에이전트 행동 규칙은 `docs/policy/agent-policy.md`, 사용자 공개 정책은 `POLICY.md` 가 각각 원천이에요.

## DP-1 diet 체제 — skill 추가 기준
- 공개 skill 은 8개(onboarding/bootstrap/import/deploy/development/diagnosis/clarity/update)를 유지해요.
- 새 skill 은 기존 8개의 경계·양보 규칙으로 라우팅할 수 없는 사용자 의도가 반복 관측될 때만 추가해요.
- 판정·실행 로직은 plugin 안에 두지 않고 ax-hub-cli 에 둬요. 라우팅 품질은 외부 corpus 가 아니라 frontmatter `description`·`examples` 에 투자해요.

## DP-2 tone — 해요체
- 모든 한글 산문은 해요체로 써요. 금지 토큰의 canonical 목록은 `scripts/check-toss-tone-conformance.ts` 의 `FORBIDDEN` 배열이에요.
- gate: `bun run lint:tone --strict` 0 error. 스캔 대상은 `skills/*/SKILL.md` + `explicit` 목록(정책 문서 3개)이에요.

## DP-3 frontmatter 유효성
- 8개 SKILL.md 의 frontmatter 는 `tests/frontmatter.test.ts` 로 검증해요. `name`·`description` 은 라우팅 표면이라 깨지면 안 돼요.

## DP-4 release flow
- `commit-and-tag-version` 기반 3단계로 릴리즈해요:
  1. `bun run release` — bump + commit (tag 미생성)
  2. CHANGELOG 에 해요체 1-3 문장 narrative 추가 후 `git commit --amend --no-edit -a`
  3. `bun run release:tag` — tag 생성 + push

## DP-5 quality gates
- `bun run lint:tone --strict` — 한글 tone 0 error
- `bun test` — frontmatter·bundle·routing·policy parity 포함 전부 PASS
- `bun run plugin:bundle` — clean bundle 생성, 로컬 Claude Code 검증은 repo 루트가 아니라 `dist/axhub-plugin` 사용
- 대표 여정 회귀 — 첫 셋업 → 앱 생성 → 배포 → 상태 확인 경로를 문서·skill 본문·fixture 계약으로 같은 방향에 맞춰요

## DP-6 hidden 표면 계약
- skill 이 쓰는 `axhub plugin-support <cmd>` hidden 그룹은 외부 무보증이에요. 계약 parity 테스트 + preflight 게이트(agent-policy AP-6)로 CLI 와 동기화해요.
- clarity 는 hidden 표면을 쓰지 않아요 (agent-policy AP-9).

## DP-7 bundle 규칙
- bundle 포함은 `scripts/build-plugin-bundle.ts` 의 `ROOT_FILES`(`README.md`, `LICENSE`, `POLICY.md`) + `ROOT_DIRS`(`.claude-plugin/`, `skills/`)가 원천이에요.
- `docs/`, `tests/`, `scripts/` 같은 개발 산출물은 포함하지 않아요. 검증은 `bun run plugin:bundle` + `tests/plugin-bundle.test.ts` 가 해요.

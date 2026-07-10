# npx 원커맨드 온보딩 설계

- **작성일:** 2026-07-10
- **상태:** 설계 승인 완료, 구현 플래닝 대기
- **결정 요약:** `npx axhub@latest setup` 한 줄이 비대화형 셋업 전부(CLI 설치·마켓플레이스 등록·플러그인 설치·MCP 등록)를 처리하고, 로그인·GitHub App·MCP OAuth 는 Claude 안 `onboarding` 스킬이 이어받아요.

---

## 1. 문제 정의

현재 첫 사용자의 여정은 터미널과 Claude Code 를 오가며 사용자 액션이 8개쯤 돼요.

1. Claude Code 설치 (전제)
2. `/plugin marketplace add jocoding-ax-partners/axhub`
3. `/plugin install axhub@axhub` + 재시작
4. "셋업해줘" → axhub CLI 설치 → PATH → 로그인 → git/node → GitHub App → MCP 등록 → **또 재시작** → `/mcp` OAuth → Ready

확인된 병목은 두 구간 모두예요: **Claude 진입 전 단계**(흩어진 /plugin 명령 2개 + CLI 별도 설치)와 **설치 후 대화형 단계**(MCP 등록 후 재시작 + OAuth). 타깃 환경은 혼합이고, 대기업 Windows 사내 PC(프록시·보안 제약, Node·Git 부재 가능)가 1급 타깃이에요.

## 2. 결정 사항

| 결정 | 내용 | 근거 |
|---|---|---|
| 수단 | **npx 메인** (`npx axhub@latest setup`) | 사용자 선택. `axhub` npm 이름은 팀이 2026-04 에 이미 선점 (`jocodingax` 계정, 0.0.1 placeholder) |
| 범위 | **비대화형 전부** — CLI 설치 + 마켓플레이스 + 플러그인 + MCP 등록 | 로그인·GitHub App·OAuth 는 대화형 게이트라 Claude 안 onboarding 스킬이 담당 |
| 캐노니컬 명령 | `setup` 서브커맨드 명시 | 무인자 `npx axhub` 는 CLI 전체 passthrough 로 남겨요 — 무인자를 setup 에 뺏기지 않아요 |
| Claude Code 미설치 | 설치 대행 안 함 — 안내 후 종료 | 설치 실패 변수를 줄이는 범위 결정 |

## 3. 사용자 경험 (북극성)

```bash
npx axhub@latest setup
```

이 한 줄이 끝나면: axhub CLI 영구 설치 + 마켓플레이스 등록 + 플러그인 설치 + MCP 등록 완료. 마지막에 핸드오프 카드를 출력해요 — "Claude Code 를 열고(이미 켜져 있으면 재시작) '처음인데 셋업해줘'라고 말하세요".

- 사용자 액션: 8개 → **터미널 1개 + Claude 안 대화형 3개**(로그인 → GitHub App → MCP OAuth).
- 부수 효과: MCP 가 첫 세션 전에 등록되므로 신규 사용자에겐 재시작 marker/resume 댄스가 사라져요. 세션 중 등록 경로(marker + resume 훅)는 npx 를 안 쓴 사용자용으로 그대로 남아요.

## 4. 아키텍처 — 로직은 CLI, npm 은 런처

diet 철학("플러그인은 얇게, 로직은 CLI")을 npm 계층에도 그대로 적용해요.

### 4.1 npm 패키지 `axhub` (thin JS 런처)

- 플랫폼 감지 후 `optionalDependencies` 로 설치된 플랫폼 바이너리를 그대로 exec 해요 — esbuild/biome 패턴.
- 플랫폼 패키지: `@axhub/cli-darwin-arm64`, `@axhub/cli-darwin-x64`, `@axhub/cli-linux-x64`, `@axhub/cli-win32-x64` (필요 시 확장).
- **postinstall 다운로드 금지.** 바이너리를 npm 레지스트리에 직접 담아 사내 npm 미러를 통과하고, `--ignore-scripts` 환경에서도 동작해요 — onboarding 스킬의 dependency 안전 규칙과 같은 방향이에요.
- `npx axhub <cmd>` 는 CLI 전체 passthrough 예요 (`npx axhub --version` 등).

### 4.2 `axhub setup claude` (ax-hub-cli 새 공개 명령)

오케스트레이션 전부를 이 명령이 소유해요. 순서:

1. **self-install** — npx 임시 캐시에서 실행 중이면 자기 바이너리를 `~/.axhub/bin` 에 영구 복사하고 PATH 를 등록해요(기존 `plugin-support repair-path` 로직 재사용). 이후 npx 없이도 `axhub` 가 동작해요.
2. **`claude` CLI 감지** — 없으면 Claude Code 설치 안내를 출력하고 명확히 종료해요. Claude Desktop 전용(claude CLI 부재) 사용자는 기존 onboarding 스킬의 커스텀 커넥터 경로로 안내해요.
3. **실행 전 고지** — 이번 실행이 설치·등록할 항목과 이미 완료돼 건너뛸 항목을 §4.3 목록 기준으로 먼저 출력해요. 확인 프롬프트 없이 곧바로 진행하되(한 줄 완주 목표, Homebrew 의 고지-후-진행 패턴), `--dry-run` 이면 목록만 출력하고 아무것도 바꾸지 않아요.
4. `claude plugin marketplace add jocoding-ax-partners/axhub`
5. `claude plugin install axhub@axhub`
6. `claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp` — onboarding 스킬(`references/mcp-ready-card.md`)과 동일 명령. 첫 세션 전 등록이라 restart marker 를 쓰지 않아요.
7. **핸드오프 카드 출력** — 다음 말은 "처음인데 셋업해줘". "같은 명령을 다시 실행해도 안전해요(완료 항목은 건너뜀)" 재실행 안내를 포함해요.

- 전 단계 **idempotent**: 재실행하면 이미 완료된 단계는 확인 표시 후 건너뛰어요 — onboarding 의 detect-first 사상 그대로예요.
- 이 명령은 auth 를 건드리지 않아요 — 로그인은 Claude 안 onboarding 이 Git Bash 계약(AP-13) 아래에서 수행해요.
- **비대화형 계약**: 터미널 입력 불가·CI·`--yes` 에서는 어떤 입력도 기다리지 않고 브라우저도 열지 않아요. AI 코딩 도구(Claude Code 등)가 이 명령을 대신 실행하는 경우가 1급 시나리오라 입력 대기(hang)는 그 자체로 결함이에요 — create-next-app 의 AI-도구 hang 선례(vercel/next.js#91169)를 반면교사로 계약화해요.
- **환경 점검 등급**: 진짜 최소 요건 미만(예: Node 18 미만)만 중단하고, 애매한 상태(지원 목록 밖이지만 동작 가능)는 감지값·요구 범위를 명시한 경고 후 진행해요 — Angular CLI 의 warn-vs-block 패턴이에요.
- **telemetry 무수집**: setup 은 사용 데이터를 보내지 않아요. 도입하게 되면 Homebrew 패턴(첫 실행 고지 전 무전송 + opt-out)을 따르고, AI 활용 기록(axrouter)은 지금처럼 Claude 안 onboarding 의 명시적 opt-in 으로만 켜요.

### 4.3 설치 산출물 (installation footprint)

`npx axhub@latest setup` 이 사용자 머신에 만들거나 바꾸는 것 전부예요. 전부 user-scope 라 **admin 권한이 필요 없어요** — 사내 PC 배포 스토리의 핵심이에요.

| # | 항목 | 위치 | 소유 |
|---|---|---|---|
| 1 | axhub CLI 바이너리 | `~/.axhub/bin/axhub` (Windows: `%USERPROFILE%\.axhub\bin\axhub.exe`) | 기존 `cli.axhub.ai` installer 와 동일 위치·레이아웃 — 채널 간 충돌 없음 |
| 2 | PATH 등록 | shell rc 파일 1개 수정 + 수정 전 백업 생성 (Windows: 사용자 PATH) | 기존 `plugin-support repair-path` 계약 재사용 (`shell_rc`, `backup_path`) |
| 3 | CLI 작업 디렉토리 | `~/.axhub/` (cache·state) | 로그인 전이라 토큰·자격증명은 이 시점에 없음 |
| 4 | 마켓플레이스 등록 | `~/.claude/plugins/` 아래 | `claude` CLI 가 생성·관리 |
| 5 | axhub 플러그인 번들 | `~/.claude/plugins/cache/` 아래 + 활성화 설정 | `claude` CLI 가 생성·관리 |
| 6 | MCP 서버 등록 | `~/.claude.json` user scope 항목 1개 (`https://mcp.axhub.ai/mcp`) | 코드 설치가 아니라 설정 항목 |
| 7 | npm/npx 캐시 (부산물) | `~/.npm/_npx/` 아래 런처·플랫폼 바이너리 패키지 | npm 이 관리 — setup 이 건드리지 않음 |

**설치하지 않는 것 (명시):** Claude Code 자체(감지만), Node.js(전제), git·GitHub App(onboarding 스킬 단계), 로그인 토큰(setup 은 auth 무접촉), 시스템 전역 파일(없음).

**제거 방법:** ① `claude plugin uninstall axhub@axhub` + `claude mcp remove axhub` ② `~/.axhub/` 삭제 ③ shell rc 의 PATH 줄 제거(백업 파일 존재). 정확한 uninstall 명령 표면은 구현 시 확정하고, 이 목록은 POLICY.md 공개 원칙에 따라 사용자 문서에도 노출해요.

### 4.4 `axhub setup doctor` (설치 무결성 진단)

setup 이 만든 상태를 사후 검증하는 공개 진단 명령이에요 — 네 산출물(CLI 버전·마켓플레이스·플러그인·MCP 등록)을 각각 확인해 정상/문제를 표시하고, 문제면 해결 명령 또는 다음 행동을 안내해요. 판정 로직은 기존 `plugin-support onboarding-detect` 를 재사용하고, v1 은 읽기 전용(자동 수정은 후속)이에요. Storybook `doctor`·React Native `doctor` 의 "자동 수정 제안 + 실패 시 안내 fallback" 패턴 중 fallback 절반을 먼저 채택해요. 배포 실패 전용인 기존 `diagnosis` 스킬과 역할이 겹치지 않아요.

## 5. 기존 자산과의 관계

- **onboarding 스킬 변경 거의 없음** — detect-first 루프라 CLI/플러그인/MCP 가 이미 green 이면 자연히 로그인부터 시작해요.
- **README·웹사이트 빠른 시작 교체** — `/plugin` 2줄 → `npx axhub@latest setup` 1줄.
- **repo 경계** — `setup claude` 구현과 npm publish CI 는 ax-hub-cli repo 소관이에요(이 repo 의 "CLI 구현·릴리즈는 follow-up" 원칙 그대로). 이 plugin repo 의 변경은 README·onboarding 문서 갱신 수준이에요.

## 6. 리스크와 완화

| 리스크 | 완화 |
|---|---|
| Node 없는 사내 Windows PC (npx 의 구조적 약점) | 웹사이트 설치 페이지에 Node 18+ 전제 명시 + OS 감지 fallback 안내. B2B 는 추후 admin provisioning(Claude Code managed settings)으로 별도 해결 — 이번 범위 밖, 기록만 해둬요 |
| AP-13 (Windows Git Bash 계약) | AP-13 에 npx setup 예외가 명문화됐어요(2026-07-10 개정) — setup 은 auth 무접촉·PATH 비의존이라 PowerShell 실행도 계약 위반이 아니고, 이후 로그인·auth 검증부터 Git Bash 계약을 따라요 |
| npm 배포 인프라 신설 | plugin repo 가 다이어트로 제거한 건 plugin 쪽 파이프라인이에요 — npm publish 는 CLI repo release CI 에 붙어 바이너리와 버전이 동기화돼요 |
| 사용자가 Claude Code 세션을 켜둔 채 실행 | 핸드오프 카드가 "이미 켜져 있으면 재시작"을 항상 안내해요 (플러그인 로드는 재시작 필요) |
| AI 에이전트가 setup 을 대신 실행(입력 불가) | 비대화형 계약(입력 대기 금지·브라우저 열기 금지)을 스펙 FR 로 고정해요 — create-next-app 의 AI-도구 hang 선례가 근거예요 |
| npm `@axhub` scope 미확보 가능성 | 구현 전 npm org `axhub` 확보를 선행 체크리스트에 둬요. 확보 실패 시 `axhub-cli-<platform>` 무스코프 네이밍으로 대체해요 |

## 7. 검증 계획

- **npm 런처 스모크** — 플랫폼 매트릭스에서 `npx axhub --version` (macOS arm64/x64 · Windows x64 · Linux x64).
- **`setup claude` 테스트** (CLI repo) — claude 유/무 × 각 단계 기존재/신규 × 재실행 idempotency.
- **비대화형 회귀** (CLI repo) — 터미널 입력이 불가능한 실행(파이프·CI env)에서 입력 대기 없이 종료하는지, `--dry-run` 이 무변경인지.
- **doctor 테스트** (CLI repo) — 네 산출물의 정상/결손 조합별 판정과 안내 문구.
- **대표 여정 회귀 갱신** (이 repo) — "npx 한 줄 → Claude 열기 → 셋업해줘 → Ready" 를 문서·skill 본문·fixture 계약에 반영해요.

## 8. 범위 밖 (기록)

- Claude Code 설치 대행.
- 터미널에서의 로그인 자동화 (device flow) — onboarding 스킬 담당 유지.
- B2B admin provisioning (managed settings 로 조직 일괄 배포) — 셀트리온형 고객에는 self-serve 명령보다 강력한 채널이라 후속 검토 가치가 있어요.
- `curl | sh` install script 채널 — npx 메인 결정으로 이번 범위에서 제외해요. Node 부재 환경이 실측으로 문제가 되면 재검토해요.
- 로그인 device flow 디테일 개선 — 일회용 코드 클립보드 복사(gh `--clipboard` 패턴), `--no-browser`·token·env 비대화형 경로, OS credential vault 토큰 저장(Supabase 패턴) — CLI/onboarding 후속이에요.
- uninstall 단일 명령화 — v1 은 §4.3 의 3단계 문서 안내로 충분하고, 단일 명령은 후속이에요.
- 프록시·사내 미러·오프라인 설치 설계 — 1급 타깃(사내망)과 직결되는 리스크라 후속 중 우선순위가 높아요. 리서치에서 외부 비교 사례가 확인되지 않아 자체 설계가 필요해요.
- shell completion 설치 — 후속이에요.

## 9. 열린 질문 (non-blocking)

- npm org `axhub`(스코프드 패키지용) 확보 여부 — 구현 선행 체크.
- `axhub setup claude` 를 공개 표면 문서(README·POLICY)에 어떻게 노출할지 — 공개 표면 목록 갱신 필요.
- 플러그인 설치 직후 `claude` 를 자동 실행해줄지(핸드오프 카드 대신) — v1 은 카드 출력만, 자동 실행은 실측 후 검토.
- 핸드오프에서 브라우저 welcome/온보딩 화면을 자동으로 열지(Storybook 패턴, CI·비대화형에서는 억제 전제) — v1 은 텍스트 카드만.

## 10. 리서치 근거 (2026-07-10 웹 리서치)

실행 전 고지·비대화형 계약·doctor·환경 점검 등급·telemetry 고지·재실행 안내 패턴은 실제 제품 문서에 대한 주장별 3표 적대 검증으로 확인했어요. 대표 소스: create-next-app CLI docs(`--yes`·recommended defaults 3분기), Storybook install/CLI docs(`-y`·`--ci`·`doctor`·telemetry 2-tier), Homebrew Analytics(첫 실행 고지 전 무전송), Angular CLI commit 0883248(warn-vs-block), gh auth login manual(`--clipboard`·`--with-token`), Supabase automatic CLI login(브라우저+폴링·credential vault 저장), clig.dev, vercel/next.js discussion #91169(AI 도구 대행 실행 hang). 리서치가 사례를 못 찾은 축(uninstall·프록시/사내망·shell completion·재시작 걸친 재개)은 §8 후속 목록에 반영했어요.

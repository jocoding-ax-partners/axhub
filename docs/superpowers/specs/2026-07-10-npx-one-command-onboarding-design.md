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
3. `claude plugin marketplace add jocoding-ax-partners/axhub`
4. `claude plugin install axhub@axhub`
5. `claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp` — onboarding 스킬(`references/mcp-ready-card.md`)과 동일 명령. 첫 세션 전 등록이라 restart marker 를 쓰지 않아요.
6. **핸드오프 카드 출력** — 다음 말은 "처음인데 셋업해줘".

- 전 단계 **idempotent**: 재실행하면 이미 완료된 단계는 확인 표시 후 건너뛰어요 — onboarding 의 detect-first 사상 그대로예요.
- 이 명령은 auth 를 건드리지 않아요 — 로그인은 Claude 안 onboarding 이 Git Bash 계약(AP-13) 아래에서 수행해요.

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

## 5. 기존 자산과의 관계

- **onboarding 스킬 변경 거의 없음** — detect-first 루프라 CLI/플러그인/MCP 가 이미 green 이면 자연히 로그인부터 시작해요.
- **README·웹사이트 빠른 시작 교체** — `/plugin` 2줄 → `npx axhub@latest setup` 1줄.
- **repo 경계** — `setup claude` 구현과 npm publish CI 는 ax-hub-cli repo 소관이에요(이 repo 의 "CLI 구현·릴리즈는 follow-up" 원칙 그대로). 이 plugin repo 의 변경은 README·onboarding 문서 갱신 수준이에요.

## 6. 리스크와 완화

| 리스크 | 완화 |
|---|---|
| Node 없는 사내 Windows PC (npx 의 구조적 약점) | 웹사이트 설치 페이지에 Node 18+ 전제 명시 + OS 감지 fallback 안내. B2B 는 추후 admin provisioning(Claude Code managed settings)으로 별도 해결 — 이번 범위 밖, 기록만 해둬요 |
| AP-13 (Windows Git Bash 계약) | `setup claude` 는 auth 무관 작업만 해요. npx 의 바이너리 spawn 은 PowerShell 프로파일 이슈와 무관해요 |
| npm 배포 인프라 신설 | plugin repo 가 다이어트로 제거한 건 plugin 쪽 파이프라인이에요 — npm publish 는 CLI repo release CI 에 붙어 바이너리와 버전이 동기화돼요 |
| 사용자가 Claude Code 세션을 켜둔 채 실행 | 핸드오프 카드가 "이미 켜져 있으면 재시작"을 항상 안내해요 (플러그인 로드는 재시작 필요) |
| npm `@axhub` scope 미확보 가능성 | 구현 전 npm org `axhub` 확보를 선행 체크리스트에 둬요. 확보 실패 시 `axhub-cli-<platform>` 무스코프 네이밍으로 대체해요 |

## 7. 검증 계획

- **npm 런처 스모크** — 플랫폼 매트릭스에서 `npx axhub --version` (macOS arm64/x64 · Windows x64 · Linux x64).
- **`setup claude` 테스트** (CLI repo) — claude 유/무 × 각 단계 기존재/신규 × 재실행 idempotency.
- **대표 여정 회귀 갱신** (이 repo) — "npx 한 줄 → Claude 열기 → 셋업해줘 → Ready" 를 문서·skill 본문·fixture 계약에 반영해요.

## 8. 범위 밖 (기록)

- Claude Code 설치 대행.
- 터미널에서의 로그인 자동화 (device flow) — onboarding 스킬 담당 유지.
- B2B admin provisioning (managed settings 로 조직 일괄 배포) — 셀트리온형 고객에는 self-serve 명령보다 강력한 채널이라 후속 검토 가치가 있어요.
- `curl | sh` install script 채널 — npx 메인 결정으로 이번 범위에서 제외해요. Node 부재 환경이 실측으로 문제가 되면 재검토해요.

## 9. 열린 질문 (non-blocking)

- npm org `axhub`(스코프드 패키지용) 확보 여부 — 구현 선행 체크.
- `axhub setup claude` 를 공개 표면 문서(README·POLICY)에 어떻게 노출할지 — 공개 표면 목록 갱신 필요.
- 플러그인 설치 직후 `claude` 를 자동 실행해줄지(핸드오프 카드 대신) — v1 은 카드 출력만, 자동 실행은 실측 후 검토.

# Quickstart: npx 원커맨드 온보딩 검증 워크스루

**용도**: 구현 완료 판정용 수동/자동 검증 절차예요. 스펙의 수용 시나리오 8건과 SC 를 실행 가능한 단계로 매핑해요.

## 준비

- 검증 머신: macOS 또는 Windows(Git Bash 관점은 로그인 단계부터), Node 18+, Claude Code 설치.
- 깨끗한 상태로 시작: `~/.axhub` 제거, `claude plugin uninstall axhub@axhub`, `claude mcp remove axhub` (있다면).

## 1. 신규 설치 (시나리오 1 · SC-001·002)

```bash
time npx axhub@latest setup
```

기대: ① 실행 전 고지(설치 목록, install 표시) → ② 4단계 진행 한 줄씩 → ③ 핸드오프 카드(다음 말·재시작·재실행 안내 포함). 5분 이내(SC-002).

사후 확인:

```bash
axhub --version          # 새 터미널에서 — PATH 등록 확인
claude plugin list       # axhub@axhub 표시
claude mcp get axhub     # 등록 확인 (Connected 는 OAuth 뒤)
```

## 2. 멱등 재실행 (시나리오 2 · SC-003)

```bash
npx axhub@latest setup
```

기대: 4단계 전부 "이미 완료" 표시 후 무변경 완료. 파일시스템·claude 설정 diff 0.

## 3. dry-run (FR-013)

```bash
npx axhub@latest setup --dry-run
```

기대: 고지 목록만 출력(전부 skip/none 표시), 아무 변경 없음, exit 0.

## 4. Claude Code 부재 (시나리오 4)

claude CLI 가 PATH 에 없는 셸에서 실행해요.

기대: CLI self-install 까지만 수행, Claude Code 설치 안내 출력, 사전 조건 exit code 로 종료. 시스템 상태 유해 변경 없음.

## 5. 비대화형 (시나리오 7 · SC-007)

```bash
CI=1 timeout 300 npx axhub@latest setup --yes < /dev/null
```

기대: 입력 대기 없이 종료(성공 또는 명확한 안내). timeout 에 걸리면 hang — 계약 위반이에요.

## 6. 진단 (시나리오 8 · FR-014)

```bash
axhub setup doctor
```

기대: 5개 검사 항목(CLI·마켓플레이스·플러그인·MCP·runtime) 전부 ok, exit 0. 이어서 `claude mcp remove axhub` 후 다시 실행하면 mcp 항목만 problem + fix 안내, exit 1.

## 7. 온보딩 인수인계 (시나리오 5 · SC-004)

Claude Code 를 열고(재시작 포함) "처음인데 셋업해줘" 를 입력해요.

기대: onboarding 스킬이 CLI·플러그인·MCP 단계를 재수행하지 않고 로그인부터 시작해요. 세션 중 MCP 등록용 재시작 안내(marker)가 나타나지 않아요.

## 8. 무권한 계정 (시나리오 6 · SC-006)

admin 권한 없는 Windows 계정에서 1~2 를 반복해요. 기대: 권한 상승 프롬프트 0회, 전 산출물 user-scope.

## 판정

1~8 전부 기대와 일치하면 스펙 SC-001~007 충족으로 판정해요. 하나라도 어긋나면 해당 계약 문서(contracts/)와 대조해 결함을 기록해요.

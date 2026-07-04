# axhub plugin 정책

axhub Claude Code plugin 이 사용자 환경에서 무엇을 하고 무엇을 하지 않는지 공개하는 문서예요.

## 네트워크 접근
- 스킬과 훅은 `axhub` CLI 와 axhub MCP 서버를 통해서만 네트워크에 접근해요.
- SessionStart auto-update 훅은 24시간에 1회만 `axhub update check` 로 새 버전을 확인해요. 네트워크 호출은 훅 스크립트가 아니라 CLI 가 수행해요.

## 로컬에 기록하는 파일
- `~/.axhub/cache/.plugin-update-check` — 업데이트 확인 throttle 마커예요.
- `~/.axhub/cache/.onboarding-mcp-restart` — 온보딩 MCP 재시작 resume 마커예요 (7일 TTL).

## 자동 업데이트와 끄는 법
- CLI 업데이트는 확인 후 자동 적용될 수 있어요. 플러그인 업데이트는 적용해도 Claude Code 재시작 후에 반영돼요.
- `AXHUB_NO_AUTO_UPDATE=1` — 자동 적용 없이 안내만 해요.
- `AXHUB_NO_ONBOARDING_RESUME=1` — 온보딩 재시작 resume 안내를 꺼요.

## 파괴적 작업 승인
- 삭제·롤백·force/execute 급 변경은 항상 사용자 확인 뒤에만 실행해요. headless 환경에서는 실행하지 않고 preview 로 멈춰요.

## 데이터 범위
- axhub MCP 도구는 OAuth 로 검증된 tenant 범위 안에서만 동작하고, 기본 도구는 read-only 예요.
- 스킬은 credential 을 파일이나 로그에 남기지 않아요.

에이전트 행동 규칙의 원천은 repo 의 `docs/policy/agent-policy.md` 예요.

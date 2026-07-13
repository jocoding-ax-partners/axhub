# Contract: npm 런처 패키지 (`axhub`)

**소유**: ax-hub-cli repo (WS-A, publish 는 release CI) · **참조**: 스펙 FR-001·FR-005·FR-010, research R-1~R-4

## 패키지 구성

| 패키지 | 내용 |
|---|---|
| `axhub` | 얇은 JS 런처 — `bin.axhub` 1개, 외부 런타임 의존성 0 지향 |
| `@axhub/cli-darwin-arm64` · `@axhub/cli-darwin-x64` · `@axhub/cli-linux-x64` · `@axhub/cli-win32-x64` | 플랫폼별 Rust 바이너리 1개씩 — `axhub` 의 `optionalDependencies` |

npm org `axhub` 미확보 시 fallback 네이밍은 `axhub-cli-<platform>` 이에요(R-3).

## 런처 동작 계약

1. `process.platform`/`process.arch` 로 플랫폼 패키지를 해석하고, 설치돼 있으면 그 바이너리를 spawn 해요.
2. **완전 passthrough** — argv 전달, stdin/stdout/stderr 상속, exit code·시그널 그대로 전파. 런처가 출력을 추가하지 않아요(진단 메시지는 바이너리 몫).
3. 플랫폼 패키지 미설치(미지원 플랫폼·optionalDependencies 실패)면 지원 플랫폼 목록과 다음 행동을 한국어로 안내하고 비0 종료해요.
4. **lifecycle script 금지** — `postinstall` 등 어떤 install hook 도 선언하지 않아요. `--ignore-scripts` 환경에서 동작이 계약이에요(R-3).

## 버전·배포 계약

- `engines.node: >=18` — 미만이면 npm 이 경고해요. 런처 시작 시에도 감지값·요구 범위를 명시한 자체 안내를 출력해요(FR-015 warn-vs-block 정합).
- 런처와 플랫폼 패키지는 CLI 바이너리와 **lockstep 버전** — 하나의 release CI 에서 함께 publish 해요(R-12). 버전 불일치 조합은 설치 시점에 감지해 안내해요.
- 문서 표기는 `npx axhub@latest setup` — `@latest` 고정 표기로 stale npx 캐시 문제를 회피해요.

## 불변식

- 설치 산출물은 전부 user-scope — npm 전역 설치를 요구하지 않아요(FR-005).
- 이 패키지로 실행해도 결과 상태는 다른 설치 채널(cli.axhub.ai installer)과 동일 위치·동일 레이아웃으로 수렴해요(FR-010) — self-install 이 `~/.axhub/bin` 으로 복사하는 것으로 보장해요.

## 계약 테스트 (WS-A)

- 플랫폼 매트릭스 스모크: `npx axhub --version` (macOS arm64/x64 · Windows x64 · Linux x64)
- `npm i --ignore-scripts` 후 정상 동작
- exit code·시그널 passthrough
- 미지원 플랫폼 안내 경로

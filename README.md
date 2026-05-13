# axhub — Claude Code 플러그인

> 바이브코더가 자연어로 axhub 앱을 안전하게 배포하고 관리하는 Claude Code 플러그인.

**상태**: v0.5.7 (ship). 19 SKILLs / 10 commands / 5 cross-arch cosign-signed binaries 라이브.

---

## 한 줄 요약

axhub SaaS 도입사의 바이브코더 직원이 Claude Code 안에서 "결제 앱 만들어줘" → "GitHub 연결해" → "배포해" → "결과 봐" 같은 한국어 자연어로 앱 lifecycle 을 수행하는 플러그인이에요. ax-hub-cli v0.12 surface 를 얇게 wrapping 하고, HMAC consent token / TLS-pinned hub-api / exit-code recovery routing 으로 안전 가드를 걸어요.

## 무엇을 할 수 있는가

19 SKILL 자연어 트리거 + 10 슬래시 명령 (한글 alias `/axhub:배포` 포함):

| SKILL | 트리거 예시 | 슬래시 |
|-------|-------------|--------|
| `deploy` | "내 paydrop 앱 배포해" | `/axhub:deploy`, `/axhub:배포` |
| `status` | "방금 배포한 거 어떻게 됐어" | `/axhub:status` |
| `logs` | "왜 실패했어 빌드 로그 보여줘" | `/axhub:logs` |
| `recover` | "방금 거 되돌려" | — |
| `apps` | "내 앱 목록" | `/axhub:apps` |
| `apis` | "어떤 API 쓸 수 있어" | `/axhub:apis` |
| `auth` | "axhub 로그인해줘" | `/axhub:login` |
| `update` | "axhub CLI 새 버전 있어" | `/axhub:update` |
| `upgrade` | "플러그인 업그레이드" | — |
| `doctor` | "axhub 설치돼 있어" | `/axhub:doctor` |
| `init` | "결제 앱 만들어줘" | — |
| `env` | "환경변수 뭐 있어" | — |
| `github` | "GitHub repo 연결해" | — |
| `open` | "결과 봐" | — |
| `whatsnew` | "뭐 새로 나왔어" | — |
| `profile` | "회사 endpoint 바꿔" | — |
| `install-cli` | "axhub CLI 설치해줘" | — |
| `clarify` | (모호 발화 disambiguation) | — |

UX 보장:
- **D1 TTY guard** — non-interactive context 에서 AskUserQuestion 건너뛰고 안전 기본값 진행
- **TodoWrite Step 0** — multi-step SKILL 진행 체크박스 실시간 표시
- **`!command` preflight** — auth_status / current_app / current_env / last_deploy 자동 주입
- **AskUserQuestion polish** — `header` chip + 해요체 통일
- **Per-question fallback registry** — drift catch (새 question 등록 안 하면 test FAIL)
- **statusline** — 옵트인 (`bin/statusline.sh` 는 macOS/Linux/Git Bash/WSL 경로, Windows native 는 PowerShell/helper 경로 검증 후 사용해요)

안전 가드:
- HMAC consent token (`CLAUDE_SESSION_ID` 필수, O_NOFOLLOW, symlink reject)
- 잘못된 앱 / profile 자동 차단
- `https://hub-api.jocodingax.ai` TLS pinning fallback
- exit 65 (token 만료) → 한국어 안내 + auth login flow
- SessionStart preflight diagnostics

## 5분 만에 시작하기

1. Claude Code 에 axhub plugin 을 설치해요.
2. 빈 디렉토리에서 "결제 앱 만들어줘" 라고 말해요.
3. `axhub --json init --list-templates` 에서 온 template 을 골라요.
4. plugin 이 `axhub init --from-template` 흐름을 실행해요.
5. 이어서 "앱 등록해", "GitHub 연결해", "환경변수 추가해", "배포해", "결과 봐" 라고 말할 수 있어요.

정직한 tradeoff:

- v0.5.6 에서도 plugin helper 를 Rust native binary 하나로 유지해요.
- axhub CLI 자체 설치가 필요하면 `install-cli` skill 이 OS 별 공식 설치 채널을 안내해요.
- template 목록은 `ax-hub-cli` registry 를 source of truth 로 사용해요.
- admin onboarding 과 remote `templates.json` 는 deferred 예요.

## 빠른 시작

준비:
- Claude Code 최신
- axhub SaaS 계정 + scope (회사 admin 발급)
- macOS / Linux 자동 셋업 / Windows native 는 명시적 PowerShell 설치·token-import·AXHUB_TOKEN 경로, Git Bash·WSL 은 POSIX fallback

설치:

```bash
# 1. 마켓플레이스 등록
/plugin marketplace add jocoding-ax-partners/axhub

# 2. 플러그인 설치
/plugin install axhub@axhub
#  └─ macOS/Linux 첫 SessionStart 에서 OS/arch 맞는 helper 바이너리 자동 다운로드
#  └─ Windows native 자동 SessionStart 는 platform-specific hook 검증 전까지 deferred 예요
#  └─ 자동 다운로드 비활성화: export AXHUB_SKIP_AUTODOWNLOAD=1 (PowerShell: $env:AXHUB_SKIP_AUTODOWNLOAD='1')

# 3. 첫 인증
"axhub 로그인해줘"             # 또는 /axhub:login
# headless: AXHUB_TOKEN env 또는 token-import (PowerShell 은 $env:AXHUB_TOKEN / axhub-helpers.exe token-import)

# 4. 첫 배포
"내 paydrop 앱 배포해"
```

상세 가이드: [`docs/vibe-coder-quickstart.ko.md`](docs/vibe-coder-quickstart.ko.md).

## 조직 관리자용

배포 정책 / 권한 관리 / 보안 설정 / 파일럿 롤아웃: [`docs/org-admin-rollout.ko.md`](docs/org-admin-rollout.ko.md).


## Runtime

axhub-helpers 는 Rust native helper 를 단일 runtime 으로 사용해요. 예전 전환기 runtime 선택 환경변수는 현재 release 의 사용자 경로가 아니에요.

```bash
bin/axhub-helpers version
bun run build
```

자세한 내용은 [`docs/migrate-rust.md`](docs/migrate-rust.md) 를 참고해요.

## 문제 해결

흔한 에러 (token 만료, 동시 배포 차단, slug 모호, Windows fallback 등) 한국어 가이드: [`docs/troubleshooting.ko.md`](docs/troubleshooting.ko.md).

## Architecture

```
사용자 발화 ("paydrop 배포해")
        │
        ▼
Claude Code  →  axhub plugin
        │              ├── skills/* (19 SKILL, NL 자동 트리거 + frontmatter multi-step/needs-preflight)
        │              ├── commands/* (10 슬래시 + 한글 alias)
        │              ├── hooks/* (SessionStart preflight, PreToolUse HMAC consent)
        │              └── bin/axhub-helpers (Rust native, 5 cross-arch cosign-signed)
        │                       │  resolve + HMAC consent + classify + redact + preflight
        ▼                       │
   Bash tool ────────────────────┘
        │
        ▼
   ax-hub-cli binary (v0.12.x supported surface)
        │
        ▼
   https://hub-api.jocodingax.ai  (TLS pinned fallback)
```

**핵심 원칙**: 플러그인은 **얇은 routing/recovery layer** 예요. 비즈니스 로직은 모두 ax-hub-cli 에 있고, 플러그인은 (1) 자연어 인텐트 → 명령어 매핑, (2) HMAC consent token 으로 destructive op 보호, (3) exit code 기반 자동 복구만 담당해요. Plugin 은 MCP 안 써요. 항상 ax-hub-cli 호출.

## 라우팅

axhub plugin 은 자연어 prompt 를 적합한 skill 워크플로우로 라우팅해요. v0.3.2 부터 Claude 의 native skill matching 을 사용해요.

작동 방식:
- 각 skill 의 `SKILL.md` frontmatter `description` 이 trigger 어구 source of truth 예요.
- Claude 가 description 보고 가장 적합한 skill 자동 invoke 해요.
- preflight context (CLI 버전, 인증 상태, 현재 앱) 자동 주입돼요.
- 라우팅 audit log 로컬 7일 보관 (외부 전송 X) 이에요.

환경 변수:
- `AXHUB_NO_AUDIT=1` — audit log 비활성

라우팅 통계 조회:

```bash
axhub-helpers routing-stats --since 7d
axhub-helpers routing-stats --json
axhub-helpers cleanup-audit --all  # 전체 삭제
```

Privacy:
- prompt content 저장 X (sha256 hash 만 저장해요)
- 외부 telemetry 전송 X (모두 로컬 디스크)
- 짧은 prompt 의 hash 는 익명화 보장 X (예: "deploy" 같은 6-byte input)

상세는 [`docs/routing.md`](docs/routing.md) 를 참고해요.

## Skill 작성

새 SKILL 추가 시 **반드시** scaffold 사용 (Phase 17/18 패턴 자동 주입):

```bash
bun run skill:new <slug>            # mutate-aware default (multi-step + needs-preflight)
bun run skill:new <slug> --no-multi-step --no-preflight   # read-only opt-out

bun run skill:doctor --strict       # 패턴 누락 colored 진단
bun run lint:tone --strict          # 해요체 톤 체크
bun run lint:keywords --check       # nl-lexicon trigger 베이스라인
bun test                            # 회귀
```

상세 규칙: [`AGENTS.md`](AGENTS.md) / [`CLAUDE.md`](CLAUDE.md) "Skill Authoring" 섹션.

## Release

```bash
bun run release                     # auto-bump 3 files + codegen + release:check + commit + tag
vim CHANGELOG.md && git commit --amend --no-edit -a   # narrative 추가
git push origin main --tags         # release.yml 자동 fire (cosign 서명 + GH release)
```

상세: [`docs/RELEASE.md`](docs/RELEASE.md).

## Test baseline (v0.5.7)

- `bun test` → plugin manifest / skill / workflow regression green
- `cargo test --workspace` → Rust helper unit/integration/phase parity green (keychain live tests ignored)
- `bunx tsc --noEmit` clean
- `bun run lint:tone --strict` 0 err / 0 warn
- `bun run lint:keywords --check` clean
- `bun run skill:doctor --strict` 19/19 SKILLs complete
- `bun run bench:hooks` prompt-route/preflight p95 thresholds green
- `bun run test:plugin-e2e:t2` → 12/12 helper lifecycle cases pass
- `bun run release:check` Rust helper host artifact + release matrix verified

## 라이선스

MIT — [`LICENSE`](LICENSE).

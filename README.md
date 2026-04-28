# axhub — Claude Code 플러그인

> 바이브코더가 자연어로 axhub 앱을 안전하게 배포하고 관리하는 Claude Code 플러그인.

**상태**: v0.1.22 (ship). 11 SKILLs / 9 commands / 5 cross-arch cosign-signed binaries 라이브.

---

## 한 줄 요약

axhub SaaS 도입사의 바이브코더 직원이 Claude Code 안에서 "내 paydrop 앱 배포해" 같은 한국어 자연어로 prod 배포·상태·로그·복구를 수행하는 플러그인이에요. ax-hub-cli (v0.1.0+) 를 wrapping 하고, HMAC consent token / TLS-pinned hub-api / exit-code recovery routing 으로 안전 가드를 걸어요.

## 무엇을 할 수 있는가

11 SKILL 자연어 트리거 + 9 슬래시 명령 (한글 alias `/axhub:배포` 포함):

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
| `clarify` | (모호 발화 disambiguation) | — |

UX 보장:
- **D1 TTY guard** — non-interactive context 에서 AskUserQuestion 건너뛰고 안전 기본값 진행
- **TodoWrite Step 0** — multi-step SKILL 진행 체크박스 실시간 표시
- **`!command` preflight** — auth_status / current_app / current_env / last_deploy 자동 주입
- **AskUserQuestion polish** — `header` chip + 해요체 통일
- **Per-question fallback registry** — drift catch (새 question 등록 안 하면 test FAIL)
- **statusline** — 옵트인 (`bin/statusline.sh`)

안전 가드:
- HMAC consent token (`CLAUDE_SESSION_ID` 필수, O_NOFOLLOW, symlink reject)
- 잘못된 앱 / profile 자동 차단
- `https://hub-api.jocodingax.ai` TLS pinning fallback
- exit 65 (token 만료) → 한국어 안내 + auth login flow
- SessionStart preflight diagnostics

## 빠른 시작

준비:
- Claude Code 최신
- axhub SaaS 계정 + scope (회사 admin 발급)
- macOS / Linux 자동 셋업 / Windows 는 token-import 또는 Git Bash·WSL fallback

설치:

```bash
# 1. 마켓플레이스 등록
/plugin marketplace add jocoding-ax-partners/axhub

# 2. 플러그인 설치
/plugin install axhub@axhub
#  └─ 첫 SessionStart 에서 OS/arch 맞는 helper 바이너리 자동 다운로드 (cosign 서명 검증)
#  └─ 자동 다운로드 비활성화: export AXHUB_SKIP_AUTODOWNLOAD=1 (수동 install.sh / install.ps1)

# 3. 첫 인증
"axhub 로그인해줘"             # 또는 /axhub:login
# headless: AXHUB_TOKEN env 또는 token-import (~/.config/axhub-plugin/token)

# 4. 첫 배포
"내 paydrop 앱 배포해"
```

상세 가이드: [`docs/vibe-coder-quickstart.ko.md`](docs/vibe-coder-quickstart.ko.md).

## 조직 관리자용

배포 정책 / 권한 관리 / 보안 설정 / 파일럿 롤아웃: [`docs/org-admin-rollout.ko.md`](docs/org-admin-rollout.ko.md).

## 문제 해결

흔한 에러 (token 만료, 동시 배포 차단, slug 모호, Windows fallback 등) 한국어 가이드: [`docs/troubleshooting.ko.md`](docs/troubleshooting.ko.md).

## Architecture

```
사용자 발화 ("paydrop 배포해")
        │
        ▼
Claude Code  →  axhub plugin
        │              ├── skills/* (11 SKILL, NL 자동 트리거 + frontmatter multi-step/needs-preflight)
        │              ├── commands/* (9 슬래시 + 한글 alias)
        │              ├── hooks/* (SessionStart preflight, PreToolUse HMAC consent)
        │              └── bin/axhub-helpers (Bun 컴파일, 5 cross-arch cosign-signed)
        │                       │  resolve + HMAC consent + classify + redact + preflight
        ▼                       │
   Bash tool ────────────────────┘
        │
        ▼
   ax-hub-cli binary (v0.1.0+)
        │
        ▼
   https://hub-api.jocodingax.ai  (TLS pinned fallback)
```

**핵심 원칙**: 플러그인은 **얇은 routing/recovery layer** 예요. 비즈니스 로직은 모두 ax-hub-cli 에 있고, 플러그인은 (1) 자연어 인텐트 → 명령어 매핑, (2) HMAC consent token 으로 destructive op 보호, (3) exit code 기반 자동 복구만 담당해요. Plugin 은 MCP 안 써요. 항상 ax-hub-cli 호출.

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

## Test baseline (v0.1.22)

- `bun test` → 545 pass / 5 skip / 0 fail
- `bash tests/auto-download.test.sh` → 8 pass / 0 fail
- `bunx tsc --noEmit` clean
- `bun run lint:tone --strict` 0 err / 0 warn
- `bun run skill:doctor --strict` 11/11 SKILLs complete
- `bun run release:check` 5 cross-arch binaries verified
- `bun run test:e2e` (`https://hub-api.jocodingax.ai`) 4 pass / 1 skip / 0 fail

## 라이선스

MIT — [`LICENSE`](LICENSE).

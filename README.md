<div align="center">

# 🚀 axhub

**한국어 자연어로 앱을 배포·관리하는 Claude Code 플러그인**

코드를 직접 짜는 대신, "내 앱 배포해" 한마디로 앱 lifecycle 전체를 안전하게 굴려요.

[![version](https://img.shields.io/badge/version-0.9.28-blue)](https://github.com/jocoding-ax-partners/axhub/releases)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-plugin-8A2BE2)](https://docs.claude.com/en/docs/claude-code)
[![homepage](https://img.shields.io/badge/homepage-axhub.ai-orange)](https://axhub.ai)

**상태**: v0.9.28 · 42 SKILL / 9 command / 3 quality sub-agent / 5 cross-arch cosign-signed binary

</div>

---

## 목차

- [🤔 axhub 가 뭔가요?](#-axhub-가-뭔가요)
- [⚡ 빠른 시작](#-빠른-시작)
- [📋 준비물](#-준비물)
- [⌨️ 슬래시 명령](#-슬래시-명령)
- [💬 자연어로 할 수 있는 일](#-자연어로-할-수-있는-일)
- [🧭 핵심 철학](#-핵심-철학)
- [🔄 동작 방식](#-동작-방식)
- [🔒 안전과 신뢰](#-안전과-신뢰)
- [⚙️ 자주 쓰는 환경변수](#-자주-쓰는-환경변수)
- [🛠️ 개발과 기여](#-개발과-기여)
- [📄 라이선스](#-라이선스)

---

## 🤔 axhub 가 뭔가요?

axhub 는 [axhub SaaS](https://axhub.ai) 를 도입한 회사의 **바이브코더**가 Claude Code 안에서 한국어 자연어만으로 앱을 만들고·배포하고·관리하게 해주는 플러그인이에요.

```
"결제 앱 만들어줘"  →  "GitHub 연결해"  →  "배포해"  →  "결과 봐"
```

이 한 줄들이 실제 배포 파이프라인을 끝까지 굴려요. 슬래시 명령(`/axhub:deploy`, 한글 alias `/axhub:배포`)도 같은 워크플로우를 불러요.

v1.0 라인부터는 **코드 품질 보조**(리뷰·디버그·TDD·배포 게이트)까지 더해져서, 단순 배포 도구를 넘어 작업 흐름 전반을 받쳐줘요.

**핵심 안전장치**: 배포처럼 되돌리기 어려운 작업은 항상 미리보기 카드를 띄우고, 실제 실행 직전에 HMAC consent 게이트가 한 번 더 막아요. 인증·상태가 깨진 채로 배포가 일어나지 않아요.

---

## ⚡ 빠른 시작

Claude Code 프롬프트에 아래를 순서대로 입력해요.

```bash
# 1. 마켓플레이스 등록
/plugin marketplace add jocoding-ax-partners/axhub

# 2. 플러그인 설치
#    (macOS/Linux 는 첫 SessionStart 에서 OS/arch 에 맞는 helper 바이너리를 자동 다운로드해요)
/plugin install axhub@axhub
```

설치되면 자연어로 바로 써요.

```bash
# 3. 첫 인증
axhub 로그인해줘                # 또는 /axhub:login

# 4. 첫 배포
내 paydrop 앱 배포해
```

> 처음이라 셋업이 막막하면 `처음인데 셋업해줘` 라고 말하면 돼요. 인증·CLI 설치·앱 연결까지 한 번에 안내해요.

---

## 📋 준비물

- **Claude Code** 최신 버전
- **axhub SaaS 계정** + scope (회사 admin 이 발급)
- 플랫폼별 셋업
  - **macOS / Linux** — helper 바이너리 자동 셋업 (별도 작업 없음)
  - **Git Bash / WSL** — POSIX fallback 으로 동작
  - Windows native 는 명시적 PowerShell 설치·token-import·AXHUB_TOKEN 경로를 써요. Windows native 자동 SessionStart 는 platform-specific hook 검증 전까지 deferred 예요.

headless(CI 등)에서는 `AXHUB_TOKEN` env 또는 token-import 로 인증해요 — PowerShell 은 $env:AXHUB_TOKEN / axhub-helpers.exe token-import.

자동 다운로드를 끄려면 `export AXHUB_SKIP_AUTODOWNLOAD=1` 로 두고 helper 를 직접 설치해요.

---

## ⌨️ 슬래시 명령

자연어가 어색하면 슬래시 명령으로 같은 워크플로우를 불러요.

| 명령 | 하는 일 |
|------|---------|
| `/axhub:deploy` · `/axhub:배포` | 현재 앱 배포 (미리보기 → consent → watch) |
| `/axhub:status` | 방금/진행 중인 배포 상태 확인 |
| `/axhub:logs` | 빌드·런타임 로그 표면화 |
| `/axhub:apps` | 내 앱 목록 / 앱 선택 |
| `/axhub:login` | axhub 로그인 (device-code → 브라우저 승인) |
| `/axhub:update` | axhub CLI 새 버전 확인·업데이트 |
| `/axhub:doctor` | 설치·인증·환경 진단 |
| `/axhub:help` | 명령·기능 도움말 |

---

## 💬 자연어로 할 수 있는 일

플러그인은 42개 SKILL 로 자연어 발화를 워크플로우에 매칭해요. 자주 쓰는 것만 추렸어요. (SKILL 레퍼런스 표는 [docs/architecture.ko.md §10](docs/architecture.ko.md#10-레퍼런스) 에 있어요.)

| 말하면 | 일어나는 일 | 슬래시 |
|--------|-------------|--------|
| "내 앱 배포해", "ship" | 미리보기 → consent → 배포 → watch | `/axhub:deploy` `/axhub:배포` |
| "기존 앱 올려줘" | 기존 repo 를 axhub 앱으로 migrate | — |
| "방금 배포 어떻게 됐어" | 배포 상태 확인 | `/axhub:status` |
| "빌드 로그 보여줘" | 로그 표면화 | `/axhub:logs` |
| "방금 거 되돌려" | 직전 배포 복구 | — |
| "내 앱 목록" | 앱 목록·선택 | `/axhub:apps` |
| "axhub 로그인해줘" | 인증 흐름 | `/axhub:login` |
| "결제 앱 만들어줘" | 새 앱 부트스트랩 | — |
| "처음인데 셋업해줘" | 인증·CLI·앱 연결 일괄 셋업 | — |
| "GitHub repo 연결해" | GitHub 연동 배포 | — |
| "데이터 카탈로그 검색" | 쓸 수 있는 리소스·데이터 조회 | — |
| "코드 리뷰해줘" | 품질 리뷰 (v1 품질 SKILL) | — |
| "이거 디버그해줘" | 디버그 보조 | — |
| "TDD 로 짜줘" | 테스트 우선 워크플로우 | — |

> 모호하게 말해도 괜찮아요. 의도가 불분명하면 `clarify` SKILL 이 되물어요.

---

## 🧭 핵심 철학

axhub 플러그인의 모든 설계는 한 문장으로 요약돼요.

> **플러그인은 얇은 라우팅 레이어다. 비즈니스 로직은 전부 `ax-hub-cli`(외부 CLI)와 backend 에 있고, 플러그인은 (1) 자연어 인텐트 → 명령 매핑, (2) 안전한 기본값 강제, (3) exit code 기반 자동 복구 안내만 담당한다.**

그래서 플러그인은:

- backend(`axhub-api`)나 MCP 를 **직접 호출하지 않아요**. 항상 `ax-hub-cli` 를 거쳐요.
- Rust helper(`axhub-helpers`)는 인증·배포 로직을 재구현하지 않고 CLI 를 **invoke** 하거나 결과를 **분류·복구 안내**할 뿐이에요.
- CLI 가 새 기능을 내면 플러그인은 자연어 트리거 어구와 안전 가드만 더하면 돼요.

---

## 🔄 동작 방식

"내 앱 배포해" 한마디가 흐르는 길을 압축하면 이래요.

```
사용자: "내 paydrop 앱 배포해"
   │
   ▼  Claude Code 가 SKILL 의 description 으로 deploy SKILL 을 매칭
[preflight]   CLI·인증·앱·환경 상태를 한 번에 읽어요
              (인증 안 됐으면 → auth, CLI 없으면 → install-cli, 너무 구버전이면 → upgrade)
   │
   ▼
[preview]     앱/환경/브랜치/커밋/예상시간 카드를 띄워요  →  [네 배포 / 미리보기만 / 취소]
   │
   ▼
[consent]     HMAC consent 토큰 발급 → 배포 직전 실제 차단 지점에서 검증
   │
   ▼
[deploy]      axhub deploy create  →  exit code 로 성공/실패/만료 분기
   │
   ▼
[watch]       배포 진행을 ~3분 추적  →  성공이면 브라우저 열기, 실패면 recover/logs 로 복구
```

5개 레이어로 보면: **① 사용자(한국어)** → **② Claude Code (hooks·skills·commands)** → **③ axhub-helpers (Rust 바이너리)** → **④ ax-hub-cli (외부 CLI)** → **⑤ axhub-api backend**.

전체 아키텍쳐·e2e 플로우·hook 계약은 [docs/architecture.ko.md](docs/architecture.ko.md) 에 자세히 정리돼 있어요.

---

## 🔒 안전과 신뢰

- **HMAC consent 토큰** — 배포 같은 destructive 작업은 세션 바인딩된 HMAC-JWT 검증을 통과해야 실행돼요. 읽기 전용 명령은 항상 통과.
- **fail-open hook** — 모든 hook 은 어떤 실패에도 메인 흐름을 막지 않아요(exit 0). 끄려면 `AXHUB_DISABLE_HOOKS=1`.
- **CLI 경계 신뢰** — helper 는 자체 HTTP/TLS 스택이 없어요. TLS·프록시·인증서 검증은 모두 캐노니컬 `axhub` CLI 가 담당해요.
- **프라이버시** — 라우팅 audit 는 프롬프트 원문을 저장하지 않고 sha256 해시만 7일 보관해요(외부 전송 X, `AXHUB_NO_AUDIT=1` 로 off). telemetry 는 opt-in (`AXHUB_TELEMETRY=0`).
- **투명한 설치 이벤트** — 토큰 저장(keychain/file), helper 자동 다운로드(GitHub release HTTPS), `~/.claude/settings.json` statusLine 관리(타 plugin 설정 보존) 등을 공개해요.

### Trust & Uninstall

설치 중 일어나는 신뢰 이벤트 전체는 [docs/architecture.ko.md](docs/architecture.ko.md) "안전·신뢰" 절에 투명 공개돼 있어요. `~/.claude/settings.json` 을 **dotbot·chezmoi 같은 dotfile sync** 로 git track 하는 사용자는 statusLine 자동 병합이 working tree 를 dirty 하게 만들 수 있어요 — `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 로 자동관리를 꺼요.

---

## ⚙️ 자주 쓰는 환경변수

| 변수 | 효과 |
|------|------|
| `AXHUB_DISABLE_HOOKS=1` | 모든 hook off (canonical kill switch) |
| `AXHUB_DISABLE_HOOK=a,b` | 지정 hook 만 off (csv) |
| `AXHUB_SKIP_AUTODOWNLOAD=1` | helper 바이너리 자동 다운로드 off |
| `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` | statusLine 자동관리 off |
| `AXHUB_DISABLE_TRIGGERS=1` | 품질 자동모드 전체 off |
| `AXHUB_NO_AUDIT=1` | 라우팅 audit off |
| `AXHUB_TELEMETRY=0` | telemetry off |
| `AXHUB_TOKEN` | helper 인증 PAT (headless 환경) |

> 전체 환경변수 표는 [docs/architecture.ko.md §10](docs/architecture.ko.md#10-레퍼런스) 에 있어요.

---

## 🛠️ 개발과 기여

이 플러그인을 직접 개발·확장하려면 심화 문서를 봐요.

| 문서 | 용도 |
|------|------|
| [docs/architecture.ko.md](docs/architecture.ko.md) | **아키텍쳐 & 개발자 온보딩 정본** — 레이어·하네스·helper 해부·ADR·부록 |
| [docs/plugin-developer-guide.md](docs/plugin-developer-guide.md) | plugin v1 설계 정본 (auth 2모드·`.axhub/`·snippet·catalog API·DoD) |
| [docs/HOOKS.md](docs/HOOKS.md) | hook 안전·fail-open 계약 |
| [docs/routing.md](docs/routing.md) | 자연어 라우팅 + audit/privacy 상세 |
| [docs/RELEASE.md](docs/RELEASE.md) | 릴리스 절차 |
| [docs/adr/](docs/adr/) | 아키텍쳐 결정 기록 (ADR 0009–0013) |
| [AGENTS.md](AGENTS.md) · [CLAUDE.md](CLAUDE.md) | AI 에이전트 작업 규칙 (skill authoring / release 계약) |
| [docs/troubleshooting.ko.md](docs/troubleshooting.ko.md) | 사용자 에러 한국어 가이드 |

로컬 셋업은 짧아요.

```bash
bun install          # repo 툴링 의존성
bun run build        # Rust helper 빌드 → bin/axhub-helpers
bun run smoke        # build + version + help 스모크
bun run cargo:test   # Rust 테스트
```

> 새 SKILL 은 **반드시 `bun run skill:new <slug>` 스캐폴드**로 만들어요. 직접 작성하면 CI 가 요구하는 패턴(D1 guard / TodoWrite / in-body preflight / registry)이 빠져요. 자세한 계약은 [CLAUDE.md](CLAUDE.md) / [AGENTS.md](AGENTS.md) 의 "Skill Authoring" 섹션에 있어요.

---

## 📄 라이선스

MIT — [LICENSE](LICENSE).

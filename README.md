<div align="center">

# 🚀 axhub

**한국어 자연어로 앱을 배포·관리하는 Claude Code 플러그인**

코드를 직접 짜는 대신, "내 앱 배포해" 한마디로 앱 lifecycle 전체를 안전하게 굴려요.

[![version](https://img.shields.io/badge/version-0.9.29-blue)](https://github.com/jocoding-ax-partners/axhub/releases)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-plugin-8A2BE2)](https://docs.claude.com/en/docs/claude-code)
[![homepage](https://img.shields.io/badge/homepage-axhub.ai-orange)](https://axhub.ai)

**상태**: v0.9.29 · 43 SKILL / 9 command / 3 quality sub-agent / 5 cross-arch cosign-signed binary

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

**모든 SKILL 은 `/axhub:<이름>` 슬래시로 바로 부를 수 있어요** — 예: `/axhub:tables`, `/axhub:rollback`, `/axhub:auth`. 자연어가 어색할 때 슬래시로 똑같이 시키면 돼요. (전체 이름은 아래 [자연어로 할 수 있는 일](#-자연어로-할-수-있는-일) 표의 슬래시 칸에 다 적어뒀어요.)

아래는 가장 자주 쓰는 **전용 명령**이에요 — 한글 alias(`/axhub:배포`)나 더 짧은 이름(`/axhub:login`)이 따로 붙어 있어요.

| 명령 | 하는 일 |
|------|---------|
| `/axhub:deploy` · `/axhub:배포` | 현재 앱 배포 (미리보기 → 확인 → 진행 추적) |
| `/axhub:status` | 방금/진행 중인 배포 상태 확인 |
| `/axhub:logs` | 빌드·런타임 로그 보기 |
| `/axhub:apps` | 내 앱 목록 / 앱 선택 |
| `/axhub:login` | axhub 로그인 (브라우저 승인) |
| `/axhub:update` | axhub CLI 새 버전 확인·업데이트 |
| `/axhub:doctor` | 설치·인증·환경 진단 |
| `/axhub:help` | 명령·기능 도움말 |

---

## 💬 자연어로 할 수 있는 일

플러그인에는 43개 SKILL 이 들어 있어요. 아래처럼 **그냥 평소 말투로 말하면** 알아서 맞는 기능을 찾아 실행해요. 명령어를 외울 필요 없어요. 슬래시로 부르고 싶으면 옆 칸의 `/axhub:<이름>` 을 쓰면 돼요 — 모든 SKILL 에 다 있어요.

### 🚀 배포하고 운영하기

| 이렇게 말하면 | 이런 일을 해줘요 | 슬래시 |
|---------------|------------------|--------|
| "내 앱 배포해", "ship 해줘" | 지금 만든 앱을 실제 서버에 올려요. 뭐가 올라가는지 미리 보여주고, 확인하면 배포한 뒤 끝날 때까지 지켜봐줘요 | `/axhub:deploy` `/axhub:배포` |
| "방금 배포 어떻게 됐어?" | 방금 시킨 배포가 성공했는지, 아직 돌고 있는지 알려줘요 | `/axhub:status` |
| "빌드 로그 보여줘" | 배포·실행 중 남은 기록(로그)을 꺼내 보여줘요. 뭔가 안 될 때 원인 찾기 좋아요 | `/axhub:logs` |
| "결과 봐", "앱 열어줘" | 배포가 끝난 앱을 브라우저로 바로 열어줘요 | `/axhub:open` |
| "방금 거 안 돼, 살려줘" | 배포가 실패했을 때 원인을 찾아 복구를 도와줘요 | `/axhub:recover` |
| "이전 배포로 되돌려줘" | 잘 돌던 예전 버전으로 되돌려요. 새 배포가 망가졌을 때 써요 | `/axhub:rollback` |
| "왜 실패했는지 추적해줘" | 배포가 어디서 왜 깨졌는지 단계별로 짚어줘요 | `/axhub:trace` |
| "진짜 배포됐는지 확인해줘" | 실제 서버 상태를 직접 확인해 배포가 제대로 됐는지 알려줘요 | `/axhub:verify` |
| "앱 복제해줘", "잠깐 멈춰줘", "다시 켜줘" | 앱을 복사하거나, 잠시 멈췄다가 다시 켜요 | `/axhub:app-lifecycle` |

### 📱 앱 만들고 가져오기

| 이렇게 말하면 | 이런 일을 해줘요 | 슬래시 |
|---------------|------------------|--------|
| "결제 앱 만들어줘" | 새 앱을 처음부터 만들어요. 뼈대를 잡아주니 바로 코딩을 시작해요 | `/axhub:init` |
| "기존 앱 올려줘" | 이미 있는 내 코드(repo)를 axhub 앱으로 옮겨 올려요 | `/axhub:migrate` |
| "GitHub repo 연결해" | GitHub 저장소를 연결해서, push 만 하면 자동으로 배포되게 해요 | `/axhub:github` |
| "내 앱 목록 보여줘" | 내가 가진 앱들을 보여주고, 작업할 앱을 고르게 해요 | `/axhub:apps` |
| "다른 앱 둘러봐", "템플릿 뭐 있어?" | 공개된 앱이나 시작용 템플릿을 구경해요 | `/axhub:browse` |
| "앱 마켓에 공개해줘" | 내가 만든 앱을 마켓플레이스에 올려 공개 심사에 제출해요 | `/axhub:publish` |

### 🗄️ 데이터 다루기

| 이렇게 말하면 | 이런 일을 해줘요 | 슬래시 |
|---------------|------------------|--------|
| "데이터 카탈로그 검색해줘" | 앱에서 쓸 수 있는 데이터가 뭐가 있는지 찾아줘요 | `/axhub:data` |
| "쓸 수 있는 API 보여줘", "endpoint list" | 앱에서 호출할 수 있는 API endpoint 목록을 보여줘요 | `apis` skill |
| "내가 쓸 수 있는 리소스 보여줘" | 내 권한으로 접근 가능한 데이터·리소스를 보여줘요 | `/axhub:my-resources` |
| "리소스 정리해줘", "이름 바꿔줘" | 데이터 리소스를 이름변경·이동·태그·정리로 깔끔하게 관리해요 | `/axhub:resources` |
| "DB 연결해줘", "postgres 붙여줘" | 외부 데이터베이스(postgres/mysql 등)를 연결하고 관리해요 | `/axhub:connectors` |
| "테이블 만들어줘", "컬럼 추가해줘" | 앱에서 쓰는 표(테이블)를 만들고 컬럼·권한·데이터를 관리해요 | `/axhub:tables` |

### ✅ 코드 품질 챙기기

| 이렇게 말하면 | 이런 일을 해줘요 | 슬래시 |
|---------------|------------------|--------|
| "코드 리뷰해줘" | 짜놓은 코드를 살펴보고 문제될 만한 부분을 짚어줘요 | `/axhub:axhub-review` |
| "이거 디버그해줘" | 안 돌아가는 코드를 같이 들여다보며 원인을 찾아요 | `/axhub:axhub-debug` |
| "왜 자꾸 실패하지?" | 배포·테스트가 실패하면 자동으로 원인을 진단해줘요 | `/axhub:axhub-diagnose` |
| "TDD 로 짜줘" | 테스트를 먼저 만들고 그걸 통과하게 구현하는 방식으로 도와줘요 | `/axhub:axhub-tdd` |
| "개발 계획 세워줘" | 무엇부터 어떻게 만들지 단계별 계획을 짜줘요 | `/axhub:axhub-plan` |
| "리뷰 통과했으니 배포해" | 품질 점검까지 끝난 코드를 곧장 배포로 이어줘요 | `/axhub:axhub-ship` |
| (자동으로 동작) | 품질 자동모드가 뭔지·언제 켜지는지 안내해줘요 | `/axhub:using-axhub-quality` |
| (자동으로 동작) | 좋은 코딩 습관 가이드를 참고해 조언해줘요 | `/axhub:karpathy-guidelines` |

### 🔧 셋업하고 점검하기

| 이렇게 말하면 | 이런 일을 해줘요 | 슬래시 |
|---------------|------------------|--------|
| "처음인데 셋업해줘" | 로그인·CLI 설치·앱 연결까지 처음 쓰는 데 필요한 걸 한 번에 잡아줘요 | `/axhub:setup` |
| "axhub 로그인해줘" | axhub 에 로그인해요 (브라우저로 승인) | `/axhub:login` `/axhub:auth` |
| "axhub CLI 설치해줘" | axhub 명령줄 도구(CLI)를 설치해요 | `/axhub:install-cli` |
| "CLI 최신 버전으로 올려줘" | axhub CLI 를 최신 버전으로 업데이트해요 | `/axhub:update` |
| "플러그인 업그레이드해줘" | 이 플러그인 자체를 새 버전으로 올려요 | `/axhub:upgrade` |
| "axhub 잘 설치됐어?" | 설치·로그인·환경이 정상인지 한 번에 진단해요 | `/axhub:doctor` |
| "설정 확인해줘", "axhub.yaml 맞아?" | 앱 설정 파일(axhub.yaml)과 현재 설정·상태가 올바른지 검증해요 | `/axhub:inspect` |
| "환경변수 뭐 있어?" | 쓸 수 있는 환경변수를 보여주고 설정해요 | `/axhub:env` |
| "회사 서버 주소 바꿔줘" | 접속할 endpoint(회사 서버) 같은 프로필을 바꿔요 | `/axhub:profile` |
| "상태줄 켜줘" | 화면 아래 상태줄(statusline)을 켜서 현재 상태를 늘 보이게 해요 | `/axhub:enable-statusline` |
| "라우팅 통계 보여줘" | 내 말이 어떤 기능으로 연결됐는지 통계를 보여줘요 | `/axhub:routing-stats` |

### 👥 워크스페이스와 팀

| 이렇게 말하면 | 이런 일을 해줘요 | 슬래시 |
|---------------|------------------|--------|
| "내 워크스페이스 보여줘", "테넌트 목록" | 내가 속한 워크스페이스·테넌트 목록과 소속을 보여줘요 | `/axhub:workspace` |
| "팀원 초대해줘", "권한 줘" | 워크스페이스나 앱에 팀원을 초대하고 접근 권한을 관리해요 | `/axhub:team` |

> 말이 좀 애매해도 괜찮아요. 뭘 원하는지 헷갈리면 되물어보고(`clarify`), 정확히 맞는 기능을 찾아줘요. 각 SKILL 의 개발용 플래그(multi-step·preflight·model)는 [docs/architecture.ko.md §10](docs/architecture.ko.md#10-레퍼런스) 에 정리돼 있어요.

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

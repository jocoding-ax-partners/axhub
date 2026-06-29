<div align="center">

# 🚀 axhub

**한국어 자연어로 앱을 배포·관리하는 Claude Code 플러그인**

코드를 직접 짜는 대신, "내 앱 배포해" 한마디로 앱 lifecycle 전체를 안전하게 굴려요.

[![version](https://img.shields.io/github/v/release/jocoding-ax-partners/axhub?color=blue)](https://github.com/jocoding-ax-partners/axhub/releases)
[![license](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-plugin-8A2BE2)](https://docs.claude.com/en/docs/claude-code)
[![homepage](https://img.shields.io/badge/homepage-axhub.ai-orange)](https://axhub.ai)

**상태**: 8 SKILL (onboarding · init · deploy · import · development · diagnosis · clarity · update) · ax-hub-cli v0.20.0+ 직접 호출

</div>

---

## 목차

- [🤔 axhub 가 뭔가요?](#-axhub-가-뭔가요)
- [⚡ 빠른 시작](#-빠른-시작)
- [📋 준비물](#-준비물)
- [🧩 8개 스킬](#-8개-스킬)
- [✅ 대표 여정과 UX 샘플](#-대표-여정과-ux-샘플)
- [💬 자연어로 할 수 있는 일](#-자연어로-할-수-있는-일)
- [🧭 핵심 철학](#-핵심-철학)
- [🔄 동작 방식](#-동작-방식)
- [🔒 안전과 신뢰](#-안전과-신뢰)
- [🛠️ 개발과 기여](#-개발과-기여)
- [📄 라이선스](#-라이선스)

---

## 🤔 axhub 가 뭔가요?

axhub 는 [axhub SaaS](https://axhub.ai) 를 도입한 회사의 **바이브코더**가 Claude Code 안에서 한국어 자연어만으로 앱을 만들고·배포하고·관리하게 해주는 플러그인이에요.

```
"결제 앱 만들어줘"  →  "GitHub 연결해"  →  "배포해"  →  "결과 봐"
```

이 한 줄들이 실제 배포 파이프라인을 끝까지 굴려요. 플러그인은 얇은 라우팅 레이어라서, 판정·실행 로직은 전부 `ax-hub-cli`(axhub 바이너리)에 두고 플러그인은 자연어 의도를 적절한 axhub 명령으로 연결하고 안전 가드만 챙겨요.

**핵심 안전장치**: 배포처럼 되돌리기 어려운 작업은 항상 미리보기 카드를 띄우고, 사용자 확인 뒤에만 실제 명령을 실행해요. 인증·상태가 깨진 채로 배포가 일어나지 않도록 실행 전 상태 확인과 Claude Code 네이티브 권한 경계를 함께 써요.

---

## ⚡ 빠른 시작

Claude Code 프롬프트에 아래를 순서대로 입력해요.

```bash
# 1. 마켓플레이스 등록
/plugin marketplace add jocoding-ax-partners/axhub

# 2. 플러그인 설치
/plugin install axhub@axhub
```

설치되면 자연어로 바로 써요.

```bash
# 첫 셋업 — CLI 설치·로그인·환경 점검을 한 번에 안내해요
처음인데 셋업해줘

# 첫 배포
내 paydrop 앱 배포해
```

> axhub CLI 가 없거나 너무 낮은 버전이면 onboarding·init·deploy 스킬이 멈추고 설치/업그레이드를 안내해요 — 최소 요구 버전은 **v0.20.0** 이에요.

---

## 📋 준비물

- **Claude Code** 최신 버전
- **axhub CLI v0.20.0 이상** — `init`·`deploy` 스킬이 시작 시 `plugin-support` 기능(preflight)을 확인해요. 미설치 시 `onboarding` 스킬이 설치를 안내해요.
- **axhub SaaS 계정** + scope (회사 admin 이 발급)

headless(CI 등)에서는 axhub CLI 가 `AXHUB_TOKEN` env 로 인증해요. 인증·TLS·토큰 저장은 모두 CLI 가 담당하고, 플러그인은 별도 바이너리를 동봉하지 않아요.

---

## 🧩 8개 스킬

플러그인은 8개 스킬을 담아요. `onboarding`·`init`·`deploy` 세 핵심 플로우, 비어 있지 않은 기존 로컬 앱을 axhub로 가져오는 `import`, 기존 앱에 실데이터(connector·table) 기반 기능 코드를 만드는 `development`, 배포 실패 원인을 읽기 전용으로 요약하는 `diagnosis`, CLI·플러그인을 지금 최신으로 올리는 `update`, 그리고 나머지 스킬에 명확히 안 맞거나 의도가 불분명한 axhub 요청을 라이브로 찾아 처리하는 `clarity` 브리지예요.

| 스킬 | 언제 | 자연어 예시 |
|------|------|-------------|
| `onboarding` | 처음 셋업 | "처음인데 셋업해줘", "온보딩", "뭐부터 하면 돼" |
| `init` | 새 앱 생성 | "결제 앱 만들어줘", "프로젝트 초기화해줘", "Next.js 앱 만들어줘" |
| `deploy` | 현재 브랜치 배포 | "배포해", "ship 해줘", "프로덕션에 올려" |
| `import` | 기존 로컬 앱 가져오기 | "기존 앱 올려", "이 폴더 axhub에 올려", "import existing app" |
| `development` | 기존 앱에 실데이터 기능 코딩 | "내 connector 데이터로 대시보드 만들어줘", "유저 목록 페이지 만들어줘", "결제 입력 폼 만들어줘" |
| `diagnosis` | 배포 실패 원인 진단 | "배포 실패 원인 진단해줘", "왜 배포가 죽었어", "이 앱 배포 실패 진단해줘" |
| `clarity` | 그 외 전부 + 모호한 axhub 발화 | "환경변수 설정해줘", "로그 보여줘", "롤백해줘", "axhub로 뭔가 해줘" |
| `update` | CLI·플러그인 최신화 | "업데이트해줘", "axhub 최신 버전으로", "플러그인 업데이트해줘" |

`clarity` 브리지는 정해진 명령 목록을 들고 있지 않아요. `axhub --help` 트리를 라이브로 탐색해 맞는 명령을 찾고 바로 실행해요 — CLI 가 새 명령을 추가해도 플러그인 수정 없이 따라가요. 단, 배포 실패 원인을 명시적으로 묻는 요청은 `diagnosis` 가 맡고 raw 로그 대신 여섯 가지 결과로 요약해요.

## ✅ 대표 여정과 UX 샘플

대표 성공 여정은 **첫 셋업 → 앱 생성 → 배포 → 상태 확인**이에요. 각 단계는 `onboarding` 이 CLI·로그인·환경을 detect-first 로 확인하고, `init` 이 앱 생성과 첫 배포를 이어가며, `deploy` 가 preview-confirm 뒤 `axhub deploy verify <deployment-id>` 로 성공을 확정하고, 이후 상태·로그·환경변수 같은 나머지 작업은 `clarity` 가 공개 CLI 표면에서 찾아 처리해요.

| 대표 단계 | 담당 스킬 | 확인 계약 |
|---|---|---|
| 첫 셋업 | `onboarding` | `onboarding-detect` 로 detect-first 확인 후 CLI missing/old 를 복구해요. |
| 앱 생성 | `init` | `apps bootstrap` saga 로 앱·repo·첫 배포를 이어가고 raw JSON/stderr 를 숨겨요. |
| 배포 | `deploy` | preview-confirm 뒤 실행하고 `axhub deploy verify <deployment-id>` exit 0 전에는 성공을 말하지 않아요. |
| 상태 확인 | `clarity` | 공개 `--json-schema` / `--help` 표면에서 상태·로그 명령을 찾아요. |

한국어 UX 샘플은 정확히 세 가지 상황만 대표로 고정해요.

1. **Action-first success** — "배포가 끝났어요. 바로 열어볼 수 있어요: <url>"
2. **Evidence-balanced failure** — "배포가 아직 완료되지 않았어요. 같은 deployment id 로 확인했고, 실패로 단정하지 않을게요. '배포 상태 확인해줘'라고 말하면 이어서 볼게요."
3. **Debug-friendly repeated failure** — "같은 단계에서 두 번 막혔어요. 원인은 인증 만료로 보여요. raw 로그 대신 해결 순서만 정리할게요: 다시 로그인 → 같은 명령 재시도 → 그래도 막히면 설치 상태 진단."

---

## 💬 자연어로 할 수 있는 일

명령어를 외울 필요 없어요. 평소 말투로 말하면 8개 스킬 중 맞는 곳으로 연결되고, 나머지 스킬 범위 밖이거나 의도가 모호하면 `clarity` 브리지가 axhub 명령을 직접 찾아 실행해요.

- **배포하고 운영하기** — "내 앱 배포해", "방금 배포 어떻게 됐어?", "빌드 로그 보여줘", "이전 버전으로 되돌려줘"
- **배포 실패 진단** — "배포 실패 원인 진단해줘", "왜 배포가 죽었어?", "이 앱 배포 실패 진단해줘"
- **앱 만들기** — "결제 앱 만들어줘", "프로젝트 초기화해줘", "FastAPI 앱 만들어줘"
- **기존 앱 가져오기** — "기존 앱 올려", "이 폴더 axhub에 올려", "이미 만든 앱 axhub로 연결해"
- **기능 만들기 (기존 앱)** — "내 connector 데이터로 대시보드 만들어줘", "유저 목록 페이지 만들어줘", "결제 입력 폼 만들어줘"
- **데이터·환경 다루기** — "환경변수 설정해줘", "테이블 만들어줘", "쓸 수 있는 API 보여줘"
- **워크스페이스와 팀** — "내 워크스페이스 보여줘", "팀원 초대해줘"
- **셋업하고 점검하기** — "처음인데 셋업해줘", "axhub 로그인해줘", "axhub 잘 설치됐어?"

> 말이 좀 애매해도 괜찮아요. 뭘 원하는지 헷갈리면 되물어보고, 정확히 맞는 명령을 찾아줘요.

---

## 🧭 핵심 철학

axhub 플러그인의 모든 설계는 한 문장으로 요약돼요.

> **플러그인은 얇은 라우팅 레이어다. 비즈니스 로직은 전부 `ax-hub-cli`(외부 CLI)와 backend 에 있고, 플러그인은 (1) 자연어 인텐트 → 명령 매핑, (2) 안전한 기본값 강제, (3) exit code 기반 자동 복구 안내만 담당한다.**

그래서 플러그인은:

- backend(`axhub-api`)나 MCP 를 **직접 호출하지 않아요**. 항상 `ax-hub-cli` 를 거쳐요.
- 자체 인증·배포 로직을 재구현하지 않아요. CLI 를 **invoke** 하고 결과를 **분류·복구 안내**할 뿐이에요.
- CLI 가 새 기능을 내면 자연어 트리거만 더하면 돼요 — `clarity` 브리지는 그것조차 자동이에요.

이전에는 플러그인이 Rust helper 바이너리·hook·NL 라우팅 코퍼스를 동봉했지만, v1 다이어트에서 전부 제거하고 `ax-hub-cli` 직접 호출로 전환했어요. 흡수된 helper 표면은 CLI 의 hidden `axhub plugin-support <cmd>` 그룹으로 옮겼어요.

---

## 🔄 동작 방식

"내 앱 배포해" 한마디가 흐르는 길을 압축하면 이래요.

```
사용자: "내 paydrop 앱 배포해"
   │
   ▼  Claude Code 가 SKILL 의 description 으로 deploy 스킬을 매칭
[preflight]   axhub plugin-support preflight 로 CLI·인증·앱·환경 상태를 한 번에 읽어요
   │
   ▼
[preview]     앱/환경/브랜치/커밋 카드를 띄워요  →  [네 배포 / 미리보기만 / 취소]
   │
   ▼
[execute]     사용자 확인에 따라 axhub deploy create --execute / --dry-run 선택
   │
   ▼
[verify]      axhub deploy verify <deployment-id>  →  exit 0 일 때만 "배포 성공" 선언
```

4개 레이어로 보면: **① 사용자(한국어)** → **② Claude Code (8 skills)** → **③ ax-hub-cli (axhub 바이너리 — plugin-support 그룹 + 공개 표면)** → **④ axhub-api backend**.

---

## 🔒 안전과 신뢰

- **Preview-first 실행** — 배포 같은 destructive 작업은 미리보기 카드와 명시 확인을 거친 뒤 실행해요. 읽기 전용 명령은 그대로 빠르게 통과해요.
- **검증 기반 성공 선언** — 배포 성공은 `axhub deploy verify <deployment-id>` 가 exit 0 을 낼 때만 선언해요. "latest" 재탐색 없이 그 배포 id 만 판정해요.
- **CLI 경계 신뢰** — 플러그인은 자체 HTTP/TLS 스택이 없어요. TLS·프록시·인증서 검증·토큰 저장은 모두 캐노니컬 `axhub` CLI 가 담당해요.
- **최소 버전/기능 게이트** — `init`·`deploy` 스킬은 시작 시 `axhub` 존재와 `plugin-support` 기능(preflight)을 확인해 v0.20.0+ 표면이 없으면 멈추고 설치/업그레이드를 안내해요 — 우회하지 않아요.

---

## 🛠️ 개발과 기여

이 플러그인을 직접 개발·확장하려면 작업 규칙 문서를 봐요.

| 문서 | 용도 |
|------|------|
| [CLAUDE.md](CLAUDE.md) · [AGENTS.md](AGENTS.md) | AI 에이전트 작업 규칙 — 8 skill 체제 · CLI 호출 표면 · release 계약 |

살아남은 quality gate 는 셋이에요.

```bash
bun run lint:tone --strict   # 모든 한글 텍스트 해요체 0 err
bun test                     # SKILL frontmatter + smoothness contract + e2e fixture
bun run typecheck            # tsc --noEmit
```

릴리즈는 `commit-and-tag-version` 2단계 flow 예요.

```bash
bun run release                    # bump + commit (tag 미생성)
git commit --amend --no-edit -a    # CHANGELOG narrative(해요체) 보완
bun run release:tag                # tag 생성 + push
```

> 판정·실행 로직은 플러그인이 아니라 `ax-hub-cli` 에 있어요 — helper 기능 변경, schema parity, CLI 릴리즈는 그쪽 레포(`ax-hub-cli`) follow-up 으로 처리해요.

---

## 📄 라이선스

MIT — [LICENSE](LICENSE).

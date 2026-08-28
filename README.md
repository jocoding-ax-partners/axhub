<div align="center">

# 🚀 axhub

**한국어 자연어로 앱을 배포·관리하는 Claude Code 플러그인**

코드를 직접 짜는 대신, "내 앱 배포해" 한마디로 앱 lifecycle 전체를 안전하게 굴려요.

[![version](https://img.shields.io/github/v/release/jocoding-ax-partners/axhub?color=blue)](https://github.com/jocoding-ax-partners/axhub/releases)
[![license](https://img.shields.io/badge/license-Apache_2.0-green)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-plugin-8A2BE2)](https://docs.claude.com/en/docs/claude-code)
[![homepage](https://img.shields.io/badge/homepage-axhub.ai-orange)](https://axhub.ai)

**상태**: 10 SKILL (onboarding · bootstrap · scaffold · plugins · deploy · import · development · diagnosis · clarity · update) · ax-hub-cli 직접 호출 (`plugins`는 app-backed marketplace 게시·목록·exact download)

</div>

---

## 목차

- [🤔 axhub 가 뭔가요?](#-axhub-가-뭔가요)
- [⚡ 빠른 시작](#-빠른-시작)
- [📋 준비물](#-준비물)
- [🧩 10개 스킬](#-10개-스킬)
- [✅ 대표 여정과 UX 샘플](#-대표-여정과-ux-샘플)
- [💬 자연어로 할 수 있는 일](#-자연어로-할-수-있는-일)
- [🧭 핵심 철학](#-핵심-철학)
- [🔄 동작 방식](#-동작-방식)
- [🔒 안전과 신뢰](#-안전과-신뢰)
- [📄 라이선스](#-라이선스)

---

## 🤔 axhub 가 뭔가요?

axhub 는 [axhub SaaS](https://axhub.ai) 를 도입한 회사의 **바이브코더**가 Claude Code 안에서 한국어 자연어만으로 앱을 만들고·배포하고·관리하게 해주는 플러그인이에요.

```
"결제 앱 만들어줘"  →  "저장소 준비해"  →  "배포해"  →  "결과 봐"
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

> axhub CLI가 없거나 필요한 기능이 없으면 관련 스킬이 멈추고 update를 안내해요. `plugins`는 `axhub plugin list|download|publish --help`를 mode별 기능 게이트로 확인해요.

### OpenAI Codex 에서 쓰기

axhub 는 **Codex CLI ≥ 0.147.0 (최종 검증: 0.148.0)** 도 공식 지원해요. Codex 전용 파생 번들(`axhub-codex`)을 설치해요:

```bash
codex plugin marketplace add https://github.com/jocoding-ax-partners/axhub
codex plugin add axhub-codex@axhub
```

- **훅 신뢰를 한 번 확인해요** — 설치 후 첫 대화형 세션에서 Codex 가 플러그인 훅(세션 시작 4개 + 프롬프트 1개)의 신뢰 여부를 물어요. 신뢰하지 않으면 자동 업데이트 확인·update-first 라우팅 가드·재시작 확인·Windows 계약 안내 4개 표면이 조용히 꺼져요 — 스킬 자체는 훅 없이도 완결되고, 업데이트는 "업데이트해줘" 한마디로 `update` 스킬이 끝까지 처리해요.
- Codex 가 신뢰하는 대상은 **훅 command(스크립트 경로)** 예요. wrapper 스크립트 내용은 플러그인 업데이트로 재신뢰 프롬프트 없이 갱신돼요 — wrapper 로직 변경은 CHANGELOG 에 명시해요.
- 플러그인 업데이트는 설치돼도 **Codex 를 재시작해야 반영**돼요.
- `codex exec` 자동화의 `--dangerously-bypass-hook-trust` 는 세션 전체 플러그인의 훅 신뢰 검토를 우회하는 전역 플래그라 권장하지 않아요 — axhub 의 headless 사용은 훅 없이 완결되고 파괴 작업은 preview 까지만 진행돼요.
- 예전에 Claude 용 번들(`axhub@axhub`)을 Codex 에 설치했다면 `codex plugin remove axhub@axhub` 후 `axhub-codex@axhub` 로 재설치해요.
- 카탈로그에 스킬이 안 보이면(다른 플러그인이 많아 컨텍스트 예산 초과 시 설명이 축약·생략될 수 있어요) `$axhub-codex:deploy` 처럼 명시 멘션으로 부르면 돼요. 등록된 다른 marketplace 경로가 깨져 있으면 목록 자체가 실패할 수 있으니 `codex plugin marketplace list` 로 정리해요.

### `Usage credits required for 1M context` 오류가 보이면

이 오류는 Claude Code 에서 선택된 모델이나 1M context 모드가 내는 메시지예요. axhub 로그인, axhub backend, 배포 권한, 플러그인 설치 실패가 아니고, axhub 를 쓰기 위해 usage credits 가 필요한 것도 아니에요.

일반 사용자는 Claude Code 에서 `/model` 을 열고 1M 이 붙지 않은 표준 모델/context 를 고르면 바로 이어서 쓸 수 있어요. 계속 1M 이 선택된다면 Claude Code 를 실행할 때 `CLAUDE_CODE_DISABLE_1M_CONTEXT=1` 을 설정해 1M context 선택지를 끄세요.

1M context 를 일부러 쓰려는 사용자만 Claude Code 의 `/usage-credits` 에서 사용 가능 여부를 확인하면 돼요.

---

## 📋 준비물

- **Claude Code** 최신 버전
- **axhub CLI** — `plugins`는 version 추측 대신 필요한 `axhub plugin list|download|install|publish --help`가 성공할 때만 진행해요. 기능이 없으면 `update`로 보내고 source build·직접 API로 우회하지 않아요.
- **axhub SaaS 계정** + scope (회사 admin 이 발급)

headless(CI 등)에서는 axhub CLI 가 `AXHUB_TOKEN` env 로 인증해요. 인증·TLS·토큰 저장은 모두 CLI 가 담당하고, 플러그인은 별도 바이너리를 동봉하지 않아요.

---

## 🧩 10개 스킬

플러그인은 10개 스킬을 담아요. `plugins`는 category=plugin인 일반 App을 찾고 exact version을 다운로드하거나 Claude Code·Codex에 설치해요. `download`는 검증된 새 ZIP만 저장하고, `install`은 offline preview 뒤 `--execute --yes`를 명시한 경우에만 archive 공격과 manifest identity를 검사하고 AxHub 관리 local marketplace와 host 공식 plugin CLI로 user scope에 설치해요. 목록·다운로드·설치는 OAuth 또는 broad PAT, publish execute만 `plugins:read` + `plugins:write` scoped PAT와 권리 확인을 사용하며 성공은 `review_ready`·installable=false예요. 이후 owner는 App Console, reviewer는 Console Review를 사용해요.

| 스킬 | 언제 | 자연어 예시 |
|------|------|-------------|
| `onboarding` | 처음 셋업 | "처음인데 셋업해줘", "온보딩", "뭐부터 하면 돼" |
| `bootstrap` | 새 앱 생성 | "결제 앱 만들어줘", "프로젝트 초기화해줘", "Next.js 앱 만들어줘" |
| `scaffold` | 사용자 GitHub 저장소에서 새 앱 시작 | "내 계정에 레포 만들어서 시작", "회사 org에 저장소 파고 새 앱 만들어줘" |
| `plugins` | plugin App 게시·목록·다운로드·host 설치 | "플러그인 목록 보여줘", "1.2.0 내려 받아", "Claude에 설치해줘", "여러 스킬을 하나로 올려줘" |
| `deploy` | 현재 브랜치 배포 | "배포해", "ship 해줘", "프로덕션에 올려" |
| `import` | 기존 로컬 앱 가져오기 | "기존 앱 올려", "이 폴더 axhub에 올려", "import existing app" |
| `development` | 기존 앱에 실데이터 기능 코딩 | "내 connector 데이터로 대시보드 만들어줘", "유저 목록 페이지 만들어줘", "결제 입력 폼 만들어줘" |
| `diagnosis` | 배포 실패 원인 진단 | "배포 실패 원인 진단해줘", "왜 배포가 죽었어", "이 앱 배포 실패 진단해줘" |
| `clarity` | 공개 CLI 운영 브리지 | "환경변수 설정해줘", "로그 보여줘", "롤백해줘", "GitHub 계정 다시 연결해줘" |
| `update` | CLI·플러그인 최신화 | "업데이트해줘", "axhub 최신 버전으로", "플러그인 업데이트해줘" |

`clarity` 브리지는 정해진 명령 목록을 들고 있지 않아요. `axhub --help` 트리를 라이브로 탐색해 맞는 명령을 찾고 바로 실행해요 — CLI 가 새 명령을 추가해도 플러그인 수정 없이 따라가요. 단, 배포 실패 원인을 명시적으로 묻는 요청은 `diagnosis` 가 맡고 raw 로그 대신 여섯 가지 결과로 요약해요.

Claude Desktop 에 axhub App/MCP 도구가 같이 보여도 플러그인 스킬 흐름은 CLI-only 예요. 버전·최신 확인이 들어간 요청은 언제나 `update` 가 먼저 끝나요. 업데이트 뒤 같은 요청에 앱 현황 확인이 남아 있으면 존재하지 않는 `axhub app list` 단수 명령을 추측하지 않고 `axhub apps --help` 로 plural 표면을 확인한 뒤 정확히 `axhub apps list --json` 읽기 전용 명령으로 이어가요. 관련 앱을 고른 뒤에도 `axhub apps get <app> --json`, `axhub deploy list --app <app> --json` 처럼 실제 CLI 표면만 쓰고, 존재하지 않는 `axhub deployment list` 나 `| head`, `2>/dev/null`, `grep` 같은 shell 후처리를 붙이지 않아요. 로그·환경변수·롤백·GitHub 재연결 같은 후속 운영 작업도 `Tenant recent deployments`, `Deployment list`, `App list`, `App get` 같은 App/MCP 도구 권한 팝업으로 빠지지 않고 CLI 계약을 따라요.

앱 생성·배포·온보딩은 저장소 단계보다 먼저 public CLI의 top-level `git_backend`를 확인해요. resume/existing은 `axhub apps get <app> --json`, fresh bootstrap/onboarding은 read-only `axhub apps git-backend --tenant <tenant> --json`을 써요. non-static selfhosted면 계정 연동·device flow·GitHub App 설치 대사를 건너뛰고 `axhub repo clone` 뒤 일반 `git push`의 webhook 배포를 기다려요. static은 기존 release lane, GitHub·`legacy_github`는 기존 `axhub up`·repo gate 경로를 유지해요.

## ✅ 대표 여정과 UX 샘플

대표 성공 여정은 **첫 셋업 → 앱 생성 → 배포 → 상태 확인**이에요. `onboarding`과 `bootstrap`은 app backend를 먼저 확인해 해당 저장소 준비만 노출하고, `deploy`는 selfhosted push 또는 기존 GitHub/upload 경로를 실행한 뒤 exact deployment id를 verify해요. 이후 운영 작업은 `clarity`가 공개 CLI 표면에서 처리해요.

| 대표 단계 | 담당 스킬 | 확인 계약 |
|---|---|---|
| 첫 셋업 | `onboarding` | 현재 app이 tenant-default selfhosted면 provider 로그인/App 설치 단계를 숨기고, 그 외 detect-first gap과 Ready card를 유지해요. |
| 앱 생성 | `bootstrap` | app 생성 전 backend를 판정해 selfhosted는 저장소 인증 질문 없이, GitHub는 기존 account/App gate 뒤 `apps bootstrap`을 실행해요. |
| 배포 | `deploy` | selfhosted는 clone→push→webhook deployment를, GitHub/upload는 기존 `axhub up`/create 경로를 사용하고 exact id verify 전에는 성공을 말하지 않아요. |
| 상태 확인 | `deploy` | 배포 id 기준 verify/watch 흐름으로 완료까지 확인해요. |

한국어 UX 샘플은 정확히 세 가지 상황만 대표로 고정해요.

1. **Action-first success** — "배포가 끝났어요. 바로 열어볼 수 있어요: <url>"
2. **Evidence-balanced progress** — "배포가 아직 완료되지 않았어요. 같은 deployment id 로 계속 확인할게요. 실패로 단정하지 않고 끝날 때까지 지켜볼게요."
3. **Debug-friendly repeated failure** — "같은 단계에서 두 번 막혔어요. 원인은 인증 만료로 보여요. raw 로그 대신 해결 순서만 정리할게요: 다시 로그인 → 같은 명령 재시도 → 그래도 막히면 설치 상태 진단."

---

## 💬 자연어로 할 수 있는 일

명령어를 외울 필요 없어요. plugin App 게시·목록·다운로드는 `plugins`, CLI·플러그인 최신 확인은 `update`, 앱 운영 명령은 `clarity`가 맡아요.

- **배포하고 운영하기** — "내 앱 배포해", "방금 배포 어떻게 됐어?", "빌드 로그 보여줘", "이전 버전으로 되돌려줘"
- **배포 실패 진단** — "배포 실패 원인 진단해줘", "왜 배포가 죽었어?", "이 앱 배포 실패 진단해줘"
- **앱 만들기** — "결제 앱 만들어줘", "프로젝트 초기화해줘", "FastAPI 앱 만들어줘"
- **기존 앱 가져오기** — "기존 앱 올려", "이 폴더 axhub에 올려", "이미 만든 앱 axhub로 연결해"
- **기능 만들기 (기존 앱)** — "내 connector 데이터로 대시보드 만들어줘", "유저 목록 페이지 만들어줘", "결제 입력 폼 만들어줘"
- **Plugin 찾기·받기·게시하기** — "플러그인 목록 보여줘", "scaffold 1.23.0 내려 받아", "여러 스킬을 하나의 plugin으로 올려줘"
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

이전에는 플러그인이 Rust helper 바이너리·hook·NL 라우팅 코퍼스를 동봉했지만, v1 다이어트에서 전부 제거하고 `ax-hub-cli` 직접 호출로 전환했어요. 흡수된 helper 표면은 CLI 의 hidden `axhub plugin-support <cmd>` 그룹으로 옮겼어요. 이후 cheap bash 훅만 아주 좁게 다시 들어왔어요 — 세션 시작 때 CLI·플러그인 업데이트를 확인하는 auto-update 훅(끄기: `AXHUB_NO_AUTO_UPDATE=1` 또는 `~/.axhub/config/no-auto-update`), Windows Git Bash 실행 계약 훅(끄기: `AXHUB_NO_WINDOWS_CONTRACT=1` 또는 `~/.axhub/config/no-windows-contract`), 그리고 최신·버전·업데이트 요청이 axhub App/MCP 도구보다 `update` 스킬을 먼저 타게 하는 Code-mode update router guard(끄기: `AXHUB_NO_UPDATE_ROUTER=1` 또는 `~/.axhub/config/no-update-router`)예요. 이 guard 는 SessionStart fallback 과 UserPromptSubmit match 로 라우팅 문맥만 추가하고, 명령을 실행하거나 앱 목록을 조회하지 않아요. 이 경로에서 사용자에게 보이는 첫 문장은 `현재 버전을 확인할게요.` 예요.

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
[verify]      axhub deploy verify <deployment-id> --app <app>  →  exit 0 일 때만 "배포 성공" 선언
```

4개 레이어로 보면: **① 사용자(한국어)** → **② Claude Code (10 skills)** → **③ ax-hub-cli (axhub 바이너리 — plugin-support 그룹 + 공개 표면)** → **④ axhub-api backend**.

---

## 🔒 안전과 신뢰

- **Preview-first 실행** — 배포 같은 destructive 작업은 미리보기 카드와 명시 확인을 거친 뒤 실행해요. 읽기 전용 명령은 그대로 빠르게 통과해요.
- **검증 기반 성공 선언** — 배포 성공은 `axhub deploy verify <deployment-id> --app <app>` 가 exit 0 을 낼 때만 선언해요. "latest" 재탐색 없이 그 배포 id 와 app scope 로 판정해요.
- **CLI 경계 신뢰** — 플러그인은 자체 HTTP/TLS 스택이 없어요. TLS·프록시·인증서 검증·토큰 저장은 모두 캐노니컬 `axhub` CLI 가 담당해요.
- **최소 버전/기능 게이트** — `bootstrap`·`deploy` 스킬은 시작 시 `axhub` 존재와 `plugin-support` 기능(preflight)을 확인해 v0.20.0+ 표면이 없으면 멈추고 설치/업그레이드를 안내해요 — 우회하지 않아요.

플러그인이 네트워크·로컬 파일·자동 업데이트에서 무엇을 하는지는 [POLICY.md](./POLICY.md) 에 공개돼 있어요.

---

## 📄 라이선스

Apache License 2.0 — [LICENSE](LICENSE).

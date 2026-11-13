<div align="center">

# 🚀 axhub

**한국어 자연어로 앱을 배포·관리하는 Claude Code 플러그인**

코드를 직접 짜는 대신, "내 앱 배포해" 한마디로 앱 lifecycle 전체를 안전하게 굴려요.

[![version](https://img.shields.io/github/package-json/v/jocoding-ax-partners/axhub?color=blue)](https://github.com/jocoding-ax-partners/axhub/blob/main/CHANGELOG.md)
[![license](https://img.shields.io/badge/license-Apache_2.0-green)](LICENSE)
[![Claude Code](https://img.shields.io/badge/Claude%20Code-plugin-8A2BE2)](https://docs.claude.com/en/docs/claude-code)
[![homepage](https://img.shields.io/badge/homepage-axhub.ai-orange)](https://axhub.ai)

**상태**: 7 SKILL (setup · bootstrap · plugins · deploy · import · development · operate) · ax-hub-cli 직접 호출 (`plugins`는 app-backed marketplace 게시·목록·exact download)

</div>

---

## 목차

- [🤔 axhub 가 뭔가요?](#-axhub-가-뭔가요)
- [⚡ 빠른 시작](#-빠른-시작)
- [📋 준비물](#-준비물)
- [🧩 7개 스킬](#-7개-스킬)
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

> axhub CLI가 없거나 필요한 기능이 없으면 관련 스킬이 멈추고 업데이트(`setup`)를 안내해요. `plugins`는 `axhub plugin-support preflight`로 CLI를 확인하고, 쓰는 명령마다 `--help`를 먼저 읽어 거기 있는 인자만 써요.

### OpenAI Codex 에서 쓰기

axhub 는 **Codex CLI ≥ 0.147.0 (최종 검증: 0.148.0)** 도 공식 지원해요. Codex 전용 파생 번들(`axhub-codex`)을 설치해요:

```bash
codex plugin marketplace add https://github.com/jocoding-ax-partners/axhub
codex plugin add axhub-codex@axhub
```

- **훅 신뢰를 한 번 확인해요** — 설치 후 첫 대화형 세션에서 Codex 가 플러그인 훅(세션 시작 2개)의 신뢰 여부를 물어요. 신뢰하지 않으면 자동 업데이트 확인·재시작 확인 2개 표면이 조용히 꺼져요 — 스킬 자체는 훅 없이도 완결되고, CLI 업데이트는 "axhub 업데이트해줘" 한마디로 `setup` 스킬이 처리해요.
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
- **axhub CLI** — `plugins`는 version 추측 대신 preflight와 쓰는 명령의 `--help`가 성공할 때만 진행해요. 안 되면 `setup`으로 보내고 source build·직접 API로 우회하지 않아요.
- **axhub SaaS 계정** + scope (회사 admin 이 발급)

headless(CI 등)에서는 axhub CLI 가 `AXHUB_TOKEN` env 로 인증해요. 인증·TLS·토큰 저장은 모두 CLI 가 담당하고, 플러그인은 별도 바이너리를 동봉하지 않아요.

---

## 🧩 7개 스킬

플러그인은 7개 스킬을 담아요. `plugins`는 deploy_method=plugin인 일반 App을 찾고 exact version을 다운로드하거나 Claude Code·Codex에 설치해요. `download`는 검증된 새 ZIP만 저장하고, `install`은 offline preview 뒤 `--execute --yes`를 명시한 경우에만 archive 공격과 manifest identity를 검사하고 AxHub 관리 local marketplace와 host 공식 plugin CLI로 user scope에 설치해요. 목록·다운로드·설치는 OAuth 또는 broad PAT, publish·submit execute만 `plugins:read` + `plugins:write` scoped PAT와 권리 확인을 사용해요. 성공은 `publish --submit`이면 `review_submitted`·installable=false, 업로드만 했으면 `upload_complete`·installable=false이고 다음 단계는 `axhub plugin submit`(게시 심사 신청)이에요. 결과 알림은 없어서 `axhub plugin manage status --app <app>`로 확인하고, 이후 owner는 App Console, reviewer는 Console Review를 사용해요. 이 흐름은 axhub CLI 0.50.0 이상이 필요해요.

| 스킬 | 언제 | 자연어 예시 |
|------|------|-------------|
| `setup` | 셋업·CLI 설치·업데이트·로그인 점검 | "처음인데 셋업해줘", "axhub 업데이트해줘", "로그인해줘" |
| `bootstrap` | 새 앱 생성 (Axhub 저장소·내 GitHub·회사 GitHub 조직) | "결제 앱 만들어줘", "내 깃허브에 새 앱 만들어줘", "회사 org에 저장소 파고 새 앱 만들어줘" |
| `plugins` | 회사 플러그인 찾기·받기·host 설치·올리기와 게시 심사·내리기 | "플러그인 목록 보여줘", "OO 1.2.0 받아줘", "OO 1.2.0 Claude에 설치해줘", "이 스킬 플러그인으로 올려줘", "플러그인 내려줘" |
| `deploy` | 배포·실패 원인 진단 (커밋해서 올려요) | "배포해", "다시 배포해줘", "왜 배포가 실패했어" |
| `import` | 기존 로컬 앱 가져오기 | "기존 앱 올려", "이 폴더 axhub에 올려", "import existing app" |
| `development` | 이미 axhub 에 있는 앱에 기능 추가·수정 (회사 데이터 화면 포함, 배포는 안 함) | "할 일에 마감일 추가해줘", "검색 탭 만들어줘", "회사 주문 데이터로 매출 대시보드 만들어줘" |
| `operate` | 이미 있는 앱과 회사 운영 (로그·앱 정보·앱 DB 조회·환경변수·롤백·삭제·권한·GitHub 다시 연결 — 첫 연결은 `bootstrap`) | "로그 보여줘", "앱 이름·아이콘 바꿔줘", "앱 DB 에서 주문 몇 건인지 봐줘", "어제 버전으로 되돌려줘", "OO 에게 권한 줘", "GitHub 다시 연결해줘" |

`operate` 는 정해진 명령 목록을 들고 있지 않아요. 지금 CLI 의 `--json-schema` 에서 필요한 부분만 잘라 맞는 명령을 찾고, `--help` 를 읽은 뒤 실행해요 — CLI 가 새 명령을 추가해도 플러그인 수정 없이 따라가요. 조회는 바로 하고, 바꾸는 명령은 미리보기를 보여주고 한 번 물은 뒤 실행해요. 배포와 배포 실패 원인은 `deploy` 가 맡아요. (2026-11 전 이름은 `clarity` 예요.)

Claude Desktop 에 axhub App/MCP 도구가 같이 보여도 플러그인 스킬 흐름은 CLI-only 예요. 버전·최신 확인은 `setup` 이 맡아요. 로그·환경변수·롤백·GitHub 다시 연결 같은 후속 운영 작업도 `Tenant recent deployments`, `Deployment list`, `App list`, `App get` 같은 App/MCP 도구 권한 팝업으로 빠지지 않고 CLI 계약을 따라요.

앱 생성·배포는 저장소 단계보다 먼저 public CLI의 top-level `git_backend`를 확인해요. resume/existing은 `axhub apps get <app> --json`의 persisted 값을 쓰고, fresh bootstrap은 read-only `axhub apps git-backend --tenant <tenant> --json`의 회사 판정(`decision`)을 따라요 — 회사가 정한 곳이면 묻지 않고, GitHub를 연결한 사람에게만 저장 위치를 물어 preview·execute에 `--git-choice`로 넘겨요. 판정을 모르는 구 CLI는 업데이트로 보내요. selfhosted면 정적 사이트를 포함해 계정 연동·device flow·GitHub App 설치를 건너뛰고 clone→`git push` webhook 배포를 기다려요. 앱 저장소에 아직 안 붙은 로컬 소스는 `import`가 그 저장소에 올려요. `GitHub 없이`만 말해도 selfhosted 정상 배포를 써요. 폴더를 저장소 없이 바로 올리는 길은 없어요 — 저장소 없는 옛 앱은 Axhub 저장소로 옮긴 뒤 올려요.

## ✅ 대표 여정과 UX 샘플

대표 성공 여정은 **첫 셋업 → 앱 생성 → 배포 → 상태 확인**이에요. `setup`이 CLI·로그인을 준비하면 `bootstrap`이 GitHub/Axhub self-hosted 저장 위치를 정해 앱을 만들고, `deploy`는 persisted backend에 맞는 lane을 실행한 뒤 exact deployment id를 verify해요. 이후 운영 작업은 `operate`가 공개 CLI 표면에서 처리해요.

| 대표 단계 | 담당 스킬 | 확인 계약 |
|---|---|---|
| 첫 셋업 | `setup` | CLI 설치·버전·로그인·회사·git 을 점검하고 빠진 것만 채워요. 점검 결과 표로 끝내고 앱 생성·배포로 이어가지 않아요. |
| 앱 생성 | `bootstrap` | 회사 판정이나 사용자 선택으로 저장 위치를 정하고(selfhosted에서는 provider 로그인/App 설치 단계를 숨겨요) 모든 `apps bootstrap`·resume state에 `--git-backend github|selfhosted`를 명시해요. |
| 배포 | `deploy` | 기준 문서로 바뀐 코드를 맞춘 뒤 `axhub deploy push` 로 올려요. 저장 위치별 길은 CLI 가 골라요. 확인 전에는 성공을 말하지 않아요. |
| 상태 확인 | `deploy` | 배포 결과를 확인하고, 실패하면 상태·로그·라이브 진단으로 원인을 설명해요. |

한국어 UX 샘플은 정확히 세 가지 상황만 대표로 고정해요.

1. **Action-first success** — "배포가 끝났어요. 바로 열어볼 수 있어요: <url>"
2. **Evidence-balanced progress** — "배포가 아직 완료되지 않았어요. 같은 deployment id 로 계속 확인할게요. 실패로 단정하지 않고 끝날 때까지 지켜볼게요."
3. **Debug-friendly repeated failure** — "같은 단계에서 두 번 막혔어요. 원인은 인증 만료로 보여요. raw 로그 대신 해결 순서만 정리할게요: 다시 로그인 → 같은 명령 재시도 → 그래도 막히면 설치 상태 진단."

---

## 💬 자연어로 할 수 있는 일

명령어를 외울 필요 없어요. plugin App 게시·목록·다운로드는 `plugins`, 셋업·CLI 최신 확인은 `setup`, 앱 운영 명령은 `operate`가 맡아요.

- **배포하고 운영하기** — "내 앱 배포해", "방금 배포 어떻게 됐어?", "빌드 로그 보여줘", "이전 버전으로 되돌려줘"
- **배포 실패 진단** — "배포 실패 원인 진단해줘", "왜 배포가 죽었어?", "이 앱 배포 실패 진단해줘"
- **앱 만들기** — "결제 앱 만들어줘", "프로젝트 초기화해줘", "FastAPI 앱 만들어줘"
- **기존 앱 가져오기** — "기존 앱 올려", "이 폴더 axhub에 올려", "이미 만든 앱 axhub로 연결해"
- **기능 만들기 (기존 앱)** — "할 일에 마감일 추가해줘", "검색 탭 만들어줘", "회사 주문 데이터로 매출 대시보드 만들어줘"
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
- CLI 가 새 기능을 내면 자연어 트리거만 더하면 돼요 — `operate` 는 그것조차 자동이에요.

이전에는 플러그인이 Rust helper 바이너리·hook·NL 라우팅 코퍼스를 동봉했지만, v1 다이어트에서 전부 제거하고 `ax-hub-cli` 직접 호출로 전환했어요. 흡수된 helper 표면은 CLI 의 hidden `axhub plugin-support <cmd>` 그룹으로 옮겼어요. 이후 cheap bash 훅만 아주 좁게 다시 들어왔어요 — 세션 시작 때 백그라운드(`async`)에서 CLI·플러그인 업데이트를 묻지 않고 확인·설치하는 auto-update 훅(끄기: `AXHUB_NO_AUTO_UPDATE=1` 또는 `~/.axhub/config/no-auto-update`), 플러그인 업데이트 뒤 적용을 확인하는 재시작 확인 훅(같은 끄기 스위치를 써요) 두 개예요.

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

4개 레이어로 보면: **① 사용자(한국어)** → **② Claude Code (7 skills)** → **③ ax-hub-cli (axhub 바이너리 — plugin-support 그룹 + 공개 표면)** → **④ axhub-api backend**.

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

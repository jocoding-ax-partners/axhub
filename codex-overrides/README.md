<div align="center">

# 🚀 axhub (Codex 판)

**한국어 자연어로 앱을 배포·관리하는 Codex 플러그인**

코드를 직접 짜는 대신, "내 앱 배포해" 한마디로 앱 lifecycle 전체를 안전하게 굴려요.

**상태**: 7 SKILL (setup · bootstrap · plugins · deploy · import · development · operate) · ax-hub-cli 직접 호출 (`plugins`는 app-backed marketplace 게시·목록·exact download) · Codex CLI ≥ 0.147.0

</div>

---

## 🤔 axhub 가 뭔가요?

axhub 는 [axhub SaaS](https://axhub.ai) 를 도입한 회사의 **바이브코더**가 Codex 안에서 한국어 자연어만으로 앱을 만들고·배포하고·관리하게 해주는 플러그인이에요.

```
"결제 앱 만들어줘"  →  "저장소 준비해"  →  "배포해"  →  "결과 봐"
```

이 한 줄들이 실제 배포 파이프라인을 끝까지 굴려요. 플러그인은 얇은 라우팅 레이어라서, 판정·실행 로직은 전부 `ax-hub-cli`(axhub 바이너리)에 두고 플러그인은 자연어 의도를 적절한 axhub 명령으로 연결하고 안전 가드만 챙겨요.

**핵심 안전장치**: 배포처럼 되돌리기 어려운 작업은 항상 미리보기 카드를 띄우고, 사용자의 명시 텍스트 승인 뒤에만 실제 명령을 실행해요. 사람이 답할 수 없는 자동 실행 환경에서는 승인이 도달할 수 없으므로 실행 없이 멈춰요.

---

## ⚡ 빠른 시작

터미널에서 아래를 순서대로 실행해요.

```bash
# 1. 마켓플레이스 등록
codex plugin marketplace add https://github.com/jocoding-ax-partners/axhub

# 2. 플러그인 설치 (Codex 전용 번들)
codex plugin add axhub-codex@axhub
```

설치 후 **Codex 를 열면 훅 신뢰 확인**이 떠요 (아래 "훅 신뢰" 참고). 신뢰를 마치면 자연어로 바로 써요.

```bash
# 첫 셋업 — CLI 설치·로그인·환경 점검을 한 번에 안내해요
처음인데 셋업해줘

# 첫 배포
내 paydrop 앱 배포해
```

> axhub CLI가 없거나 필요한 기능이 없으면 관련 스킬이 멈추고 update를 안내해요. `plugins`는 `axhub plugin-support preflight`로 CLI를 확인하고, 쓰는 명령마다 `--help`를 먼저 읽어 거기 있는 인자만 써요.
>
> 예전에 Claude 용 번들(`axhub@axhub`)을 codex 에 설치해 본 적이 있다면 `codex plugin remove axhub@axhub` 로 제거한 뒤 `axhub-codex@axhub` 를 설치해요 — Codex 표면에 맞게 파생된 번들이 이쪽이에요.

---

## 🔐 훅 신뢰 — 설치 후 꼭 한 번 확인해요

이 플러그인은 세션 시작 훅 2개를 담아요. Codex 는 플러그인 훅을 **사용자가 신뢰하기 전에는 실행하지 않아요** — 설치 후 첫 대화형 세션에서 훅 목록과 신뢰 여부를 물어요.

- **신뢰하면**: 자동 업데이트(24시간 1회, 뒤에서 조용히 확인·설치)와 플러그인 재시작 확인이 자동으로 동작해요.
- **신뢰하지 않으면**: 위 2개 표면이 조용히 꺼져요. 스킬 자체는 훅 없이도 완결돼요 — CLI 업데이트는 "axhub 업데이트해줘" 한마디로 `setup` 스킬이 훅과 무관하게 확인·적용해요.
- **훅 정의가 바뀐 릴리즈**: 훅 entry 자체가 바뀐 릴리즈(예: 자동 업데이트 훅을 백그라운드 `async` 로 바꾼 릴리즈)는 재설치 뒤 신뢰 확인이 한 번 더 뜰 수 있어요. wrapper 스크립트 본문만 바뀐 릴리즈는 재확인 없이 반영돼요. 자동 업데이트 훅은 뒤에서 `axhub update apply` 와 `codex plugin marketplace upgrade axhub` 를 묻지 않고 실행하니 신뢰 전에 `hooks/session-auto-update.sh` 를 한 번 읽어 보는 것을 권해요.
- 훅이 신뢰하는 대상은 **훅 command(스크립트 경로)** 예요. 플러그인 업데이트로 훅 구성이 바뀌면 Codex 가 다시 신뢰를 물을 수 있어요 — 훅 스크립트 동작 변화는 CHANGELOG 에 명시해요.

`codex exec` 자동화의 `--dangerously-bypass-hook-trust` 플래그는 세션의 모든 플러그인 훅 신뢰 검토를 우회하는 전역 플래그라 **권장하지 않아요**. axhub 의 headless 사용은 훅 없이 완결되고 파괴 작업은 dry-run 까지만 진행되므로 이 플래그가 필요 없어요.

---


## 💬 첫 세션에서 알아둘 것

- 첫 axhub 명령에서 네트워크 접근 승인을 한 번 물어요 — 허용해야 axhub 백엔드에 닿아요.
- 홈 폴더에 쓰는 명령(`axhub auth login`, `axhub git setup`, `axhub plugin-support repair-path`)은 Codex 샌드박스 밖에서 실행해야 해서, 처음 한 번 권한 상승 승인을 물어요 — 허용해야 로그인 정보·git 자격증명·PATH 가 저장돼요. 샌드박스 안에서 막히면 같은 명령을 권한 상승으로 한 번 다시 실행해요. 권한 상승으로도 막히면 안내된 경로의 소유자·권한을 고쳐 주세요.
- 선택지를 고를 때 번호를 타이핑하는 대신 카드로 받고 싶으면 `~/.codex/config.toml` 의 `[features]` 에 `default_mode_request_user_input = true` 한 줄을 더하면 돼요. 설정을 대신 바꾸지는 않아요.
- 네이티브 선택 카드가 켜진 세션에서 빈 답변이 돌아오면 승인하지 않은 것으로 봐요 — 자리를 잠깐 비운 사이에 카드가 자동으로 닫혀도 그대로 진행하지 않고 다시 물어요. 카드가 열려 있는 동안에도 실행 단계로 넘어가지 않아요.
- 승인 채널이 없는 headless(`codex exec`·CI)에서는 실행하지 않고 멈춰요.

## 📋 준비물

- **Codex CLI ≥ 0.147.0** (최종 검증: 0.149.0)
- **axhub CLI** — `plugins`는 version 추측 대신 preflight와 쓰는 명령의 `--help`가 성공할 때만 진행해요. 안 되면 `setup`으로 보내고 우회하지 않아요.
- **axhub SaaS 계정** + scope (회사 admin 이 발급)

headless(CI 등)에서는 axhub CLI 가 `AXHUB_TOKEN` env 로 인증해요. 인증·TLS·토큰 저장은 모두 CLI 가 담당하고, 플러그인은 별도 바이너리를 동봉하지 않아요.

---

## 🧩 7개 스킬

`plugins`는 deploy_method=plugin인 일반 App의 exact version을 다운로드하거나 Codex에 설치해요. `download`는 검증된 ZIP만 저장하고, `axhub plugin install --host codex`는 offline preview 뒤 `--execute --yes`일 때만 archive 공격과 manifest identity를 검사하고 AxHub 관리 local marketplace를 Codex 공식 plugin CLI로 설치해요. 목록·다운로드·설치는 OAuth 또는 broad PAT, publish·submit execute만 `plugins:read` + `plugins:write` scoped PAT와 권리 확인을 사용해요. 성공은 `publish --submit`이면 `review_submitted`·installable=false, 업로드만 했으면 `upload_complete`·installable=false이고 다음 단계는 `axhub plugin submit`(게시 심사 신청)이에요. 결과 알림은 없어서 `axhub plugin manage status --app <app>`로 확인하고, 이후 owner는 App Console, reviewer는 Console Review를 사용해요. 이 흐름은 axhub CLI 0.50.0 이상이 필요해요.

| 스킬 | 언제 | 자연어 예시 |
|------|------|-------------|
| `setup` | 셋업·CLI 설치·업데이트·로그인 점검 | "처음인데 셋업해줘", "axhub 업데이트해줘", "로그인해줘" |
| `bootstrap` | 새 앱 생성 (Axhub 저장소·내 GitHub·회사 GitHub 조직) | "결제 앱 만들어줘", "내 깃허브에 새 앱 만들어줘", "회사 org에 저장소 파고 새 앱 만들어줘" |
| `plugins` | 회사 플러그인 찾기·받기·Codex 설치·올리기와 게시 심사·내리기 | "플러그인 목록 보여줘", "OO 1.2.0 받아줘", "OO 1.2.0 Codex에 설치해줘", "이 스킬 플러그인으로 올려줘", "플러그인 내려줘" |
| `deploy` | 배포·실패 원인 진단 (커밋해서 올려요) | "배포해", "다시 배포해줘", "왜 배포가 실패했어" |
| `import` | 기존 로컬 앱 가져오기 | "기존 앱 올려", "이 폴더 axhub에 올려", "import existing app" |
| `development` | 이미 axhub 에 있는 앱에 기능 추가·수정 (회사 데이터 화면 포함, 배포는 안 함) | "할 일에 마감일 추가해줘", "회사 주문 데이터로 매출 대시보드 만들어줘" |
| `operate` | 이미 있는 앱과 회사 운영 (로그·앱 정보·앱 DB 조회·환경변수·롤백·삭제·권한·GitHub 다시 연결 — 첫 연결은 `bootstrap`) | "로그 보여줘", "앱 이름·아이콘 바꿔줘", "앱 DB 에서 주문 몇 건인지 봐줘", "어제 버전으로 되돌려줘", "OO 에게 권한 줘", "GitHub 다시 연결해줘" |

`operate` 는 정해진 명령 목록을 들고 있지 않아요. 지금 CLI 의 `--json-schema` 에서 필요한 부분만 잘라 맞는 명령을 찾고, `--help` 를 읽은 뒤 실행해요 — CLI 가 새 명령을 추가해도 플러그인 수정 없이 따라가요. 조회는 바로 하고, 바꾸는 명령은 미리보기를 보여주고 한 번 물은 뒤 실행해요. 배포와 배포 실패 원인은 `deploy` 가 맡아요. (2026-11 전 이름은 `clarity` 예요.)

axhub MCP/App 도구가 같이 보여도 플러그인 스킬 흐름은 CLI-only 예요. 버전·최신 확인은 `setup` 이 맡아요.

앱 생성·배포는 저장소 단계보다 먼저 public CLI의 top-level `git_backend`를 확인해요. resume/existing은 `axhub apps get <app> --json`의 persisted 값을 쓰고, fresh bootstrap은 read-only `axhub apps git-backend --tenant <tenant> --json`의 회사 판정(`decision`)을 따라요 — 회사가 정한 곳이면 묻지 않고, GitHub를 연결한 사람에게만 저장 위치를 물어 preview·execute에 `--git-choice`로 넘겨요. 판정을 모르는 구 CLI는 업데이트로 보내요. selfhosted면 정적 사이트를 포함해 계정 연동·device flow·GitHub App 설치를 건너뛰고 clone→`git push` webhook 배포를 기다려요. 앱 저장소에 아직 안 붙은 로컬 소스는 `import`가 그 저장소에 올려요. `GitHub 없이`만 말해도 selfhosted 정상 배포를 쓰며, `up`은 repositoryless upload를 명시한 경우만 맡아요.

카탈로그에 스킬이 안 보일 때(다른 플러그인·스킬이 많아 컨텍스트 예산이 넘치면 Codex 가 설명을 줄이거나 생략할 수 있어요)는 스킬 이름을 명시해 부르면 돼요 — 예: `$axhub-codex:deploy` 처럼 명시 멘션으로요. 또 등록해 둔 다른 marketplace 경로가 깨져 있으면 플러그인 목록 자체가 실패할 수 있어요 — `codex plugin marketplace list` 로 등록 상태를 정리해요.

---

## ✅ 대표 여정

대표 성공 여정은 **첫 셋업 → 앱 생성 → 배포 → 상태 확인**이에요. `setup`이 CLI·로그인을 준비하면 `bootstrap`이 GitHub/Axhub self-hosted 저장 위치를 정해 앱을 만들고, `deploy`는 persisted backend에 맞는 lane을 실행한 뒤 exact deployment id를 verify해요. 이후 운영 작업은 `operate`가 공개 CLI 표면에서 처리해요.

---

## 🧭 핵심 철학

> **플러그인은 얇은 라우팅 레이어다. 비즈니스 로직은 전부 `ax-hub-cli`(외부 CLI)와 backend 에 있고, 플러그인은 (1) 자연어 인텐트 → 명령 매핑, (2) 안전한 기본값 강제, (3) exit code 기반 자동 복구 안내만 담당한다.**

그래서 플러그인은:

- backend(`axhub-api`)나 MCP 를 **직접 호출하지 않아요**. 항상 `ax-hub-cli` 를 거쳐요.
- 자체 인증·배포 로직을 재구현하지 않아요. CLI 를 **invoke** 하고 결과를 **분류·복구 안내**할 뿐이에요.
- CLI 가 새 기능을 내면 자연어 트리거만 더하면 돼요 — `operate` 는 그것조차 자동이에요.

세션 훅은 cheap bash 로만 좁게 들어가 있어요 — auto-update 훅과 재시작 확인 훅 두 개이고, 둘 다 `AXHUB_NO_AUTO_UPDATE=1` 또는 `~/.axhub/config/no-auto-update` 로 꺼져요. auto-update 훅만 백그라운드(`async`)에서 `axhub update apply` 와 플러그인 갱신을 조용히 실행하고, 재시작 확인 훅은 명령을 실행하지 않고 적용 확인 한 줄만 추가해요.

---

## 🔒 안전과 신뢰

- **Preview-first 실행** — 배포 같은 destructive 작업은 미리보기 카드와 명시 텍스트 승인(canonical 승인 문구의 byte-exact 일치)을 거친 뒤 실행해요. 읽기 전용 명령은 그대로 빠르게 통과해요.
- **검증 기반 성공 선언** — 배포 성공은 `axhub deploy verify <deployment-id> --app <app>` 가 exit 0 을 낼 때만 선언해요. "latest" 재탐색 없이 그 배포 id 와 app scope 로 판정해요.
- **CLI 경계 신뢰** — 플러그인은 자체 HTTP/TLS 스택이 없어요. TLS·프록시·인증서 검증·토큰 저장은 모두 캐노니컬 `axhub` CLI 가 담당해요.
- **최소 버전/기능 게이트** — `bootstrap`·`deploy` 스킬은 시작 시 `axhub` 존재와 `plugin-support` 기능(preflight)을 확인해 v0.20.0+ 표면이 없으면 멈추고 설치/업그레이드를 안내해요 — 우회하지 않아요.

플러그인이 네트워크·로컬 파일·자동 업데이트에서 무엇을 하는지는 [POLICY.md](./POLICY.md) 에 공개돼 있어요.

---

## 📄 라이선스

Apache License 2.0 — [LICENSE](LICENSE).

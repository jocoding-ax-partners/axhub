<div align="center">

# 🚀 axhub (Codex 판)

**한국어 자연어로 앱을 배포·관리하는 Codex 플러그인**

코드를 직접 짜는 대신, "내 앱 배포해" 한마디로 앱 lifecycle 전체를 안전하게 굴려요.

**상태**: 9 SKILL (onboarding · bootstrap · scaffold · deploy · import · development · diagnosis · clarity · update) · ax-hub-cli 직접 호출 (코어 v0.20.0+ · 전체 v0.21.3+) · Codex CLI ≥ 0.147.0 (최종 검증: 0.148.0)

</div>

---

## 🤔 axhub 가 뭔가요?

axhub 는 [axhub SaaS](https://axhub.ai) 를 도입한 회사의 **바이브코더**가 Codex 안에서 한국어 자연어만으로 앱을 만들고·배포하고·관리하게 해주는 플러그인이에요.

```
"결제 앱 만들어줘"  →  "GitHub 연결해"  →  "배포해"  →  "결과 봐"
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

> axhub CLI 가 없거나 너무 낮은 버전이면 onboarding·bootstrap·deploy 스킬이 멈추고 설치/업그레이드를 안내해요 — 최소 요구 버전은 코어 **v0.20.0**, 전체 스킬은 **v0.21.3** 이에요.
>
> 예전에 Claude 용 번들(`axhub@axhub`)을 codex 에 설치해 본 적이 있다면 `codex plugin remove axhub@axhub` 로 제거한 뒤 `axhub-codex@axhub` 를 설치해요 — Codex 표면에 맞게 파생된 번들이 이쪽이에요.

---

## 🔐 훅 신뢰 — 설치 후 꼭 한 번 확인해요

이 플러그인은 세션 시작 훅 4개와 프롬프트 훅 1개를 담아요. Codex 는 플러그인 훅을 **사용자가 신뢰하기 전에는 실행하지 않아요** — 설치 후 첫 대화형 세션에서 훅 목록과 신뢰 여부를 물어요.

- **신뢰하면**: 자동 업데이트 확인(24시간 1회), Windows Git Bash 실행 계약 안내, 최신·버전 요청의 update-first 라우팅 가드, 플러그인 재시작 확인이 자동으로 동작해요.
- **신뢰하지 않으면**: 위 4개 표면이 조용히 꺼져요. 스킬 자체는 훅 없이도 완결돼요 — 특히 `update` 스킬은 "업데이트해줘" 한마디로 훅과 무관하게 감지→적용→재시작 안내까지 끝까지 진행해요.
- 훅이 신뢰하는 대상은 **훅 command(스크립트 경로)** 예요. 플러그인 업데이트로 훅 구성이 바뀌면 Codex 가 다시 신뢰를 물을 수 있어요 — 훅 스크립트 동작 변화는 CHANGELOG 에 명시해요.

`codex exec` 자동화의 `--dangerously-bypass-hook-trust` 플래그는 세션의 모든 플러그인 훅 신뢰 검토를 우회하는 전역 플래그라 **권장하지 않아요**. axhub 의 headless 사용은 훅 없이 완결되고 파괴 작업은 dry-run 까지만 진행되므로 이 플래그가 필요 없어요.

---

## 📋 준비물

- **Codex CLI ≥ 0.147.0** (최종 검증: 0.148.0)
- **axhub CLI v0.20.0 이상 (전체 스킬은 v0.21.3 이상)** — `bootstrap`·`deploy` 스킬이 시작 시 `plugin-support` 기능(preflight)을 확인해요. 미설치 시 `onboarding` 스킬이 설치를 안내해요.
- **axhub SaaS 계정** + scope (회사 admin 이 발급)

headless(CI 등)에서는 axhub CLI 가 `AXHUB_TOKEN` env 로 인증해요. 인증·TLS·토큰 저장은 모두 CLI 가 담당하고, 플러그인은 별도 바이너리를 동봉하지 않아요.

---

## 🧩 9개 스킬

| 스킬 | 언제 | 자연어 예시 |
|------|------|-------------|
| `onboarding` | 처음 셋업 | "처음인데 셋업해줘", "온보딩", "뭐부터 하면 돼" |
| `bootstrap` | 새 앱 생성 | "결제 앱 만들어줘", "프로젝트 초기화해줘", "Next.js 앱 만들어줘" |
| `scaffold` | 템플릿 + 내 계정 저장소 | "내 계정에 레포 만들어서 새 앱 시작해줘" |
| `deploy` | 현재 브랜치 배포 | "배포해", "ship 해줘", "프로덕션에 올려" |
| `import` | 기존 로컬 앱 가져오기 | "기존 앱 올려", "이 폴더 axhub에 올려", "import existing app" |
| `development` | 기존 앱에 실데이터 기능 코딩 | "내 connector 데이터로 대시보드 만들어줘", "유저 목록 페이지 만들어줘" |
| `diagnosis` | 배포 실패 원인 진단 | "배포 실패 원인 진단해줘", "왜 배포가 죽었어" |
| `clarity` | 공개 CLI 운영 브리지 | "환경변수 설정해줘", "로그 보여줘", "롤백해줘", "GitHub 계정 다시 연결해줘" |
| `update` | CLI·플러그인 최신화 | "업데이트해줘", "axhub 최신 버전으로", "플러그인 업데이트해줘" |

`clarity` 브리지는 정해진 명령 목록을 들고 있지 않아요. `axhub --help` 트리를 라이브로 탐색해 맞는 명령을 찾고 바로 실행해요 — CLI 가 새 명령을 추가해도 플러그인 수정 없이 따라가요. 단, 배포 실패 원인을 명시적으로 묻는 요청은 `diagnosis` 가 맡고 raw 로그 대신 여섯 가지 결과로 요약해요.

axhub MCP/App 도구가 같이 보여도 플러그인 스킬 흐름은 CLI-only 예요. 버전·최신 확인이 들어간 요청은 언제나 `update` 가 먼저 끝나요. 업데이트 뒤 같은 요청에 앱 현황 확인이 남아 있으면 존재하지 않는 `axhub app list` 단수 명령을 추측하지 않고 `axhub apps --help` 로 plural 표면을 확인한 뒤 정확히 `axhub apps list --json` 읽기 전용 명령으로 이어가요.

카탈로그에 스킬이 안 보일 때(다른 플러그인·스킬이 많아 컨텍스트 예산이 넘치면 Codex 가 설명을 줄이거나 생략할 수 있어요)는 스킬 이름을 명시해 부르면 돼요 — 예: `$axhub-codex:deploy` 처럼 명시 멘션으로요. 또 등록해 둔 다른 marketplace 경로가 깨져 있으면 플러그인 목록 자체가 실패할 수 있어요 — `codex plugin marketplace list` 로 등록 상태를 정리해요.

---

## ✅ 대표 여정

대표 성공 여정은 **첫 셋업 → 앱 생성 → 배포 → 상태 확인**이에요. `onboarding` 이 CLI·로그인·환경을 detect-first 로 확인하고, `bootstrap` 이 앱 생성과 첫 배포를 이어가며, `deploy` 가 preview 와 명시 텍스트 승인 뒤 `axhub deploy verify <deployment-id> --app <app>` 로 성공을 확정하고, 이후 로그·환경변수·롤백 같은 운영 작업은 `clarity` 가 공개 CLI 표면에서 찾아 처리해요.

---

## 🧭 핵심 철학

> **플러그인은 얇은 라우팅 레이어다. 비즈니스 로직은 전부 `ax-hub-cli`(외부 CLI)와 backend 에 있고, 플러그인은 (1) 자연어 인텐트 → 명령 매핑, (2) 안전한 기본값 강제, (3) exit code 기반 자동 복구 안내만 담당한다.**

그래서 플러그인은:

- backend(`axhub-api`)나 MCP 를 **직접 호출하지 않아요**. 항상 `ax-hub-cli` 를 거쳐요.
- 자체 인증·배포 로직을 재구현하지 않아요. CLI 를 **invoke** 하고 결과를 **분류·복구 안내**할 뿐이에요.
- CLI 가 새 기능을 내면 자연어 트리거만 더하면 돼요 — `clarity` 브리지는 그것조차 자동이에요.

세션 훅은 cheap bash 로만 좁게 들어가 있어요 — auto-update 훅(끄기: `AXHUB_NO_AUTO_UPDATE=1` 또는 `~/.axhub/config/no-auto-update`), Windows Git Bash 실행 계약 훅(끄기: `AXHUB_NO_WINDOWS_CONTRACT=1` 또는 `~/.axhub/config/no-windows-contract`), update-first 라우팅 가드와 실패 자동 리포트 계약을 합본으로 발행하는 상시 훅(각각 끄기: `AXHUB_NO_UPDATE_ROUTER=1`/`~/.axhub/config/no-update-router`, `AXHUB_NO_FEEDBACK_REPORT=1`/`~/.axhub/config/no-feedback-report`), 재시작 확인 훅이에요. 이 훅들은 명령을 실행하거나 앱 목록을 조회하지 않고 라우팅·계약 문맥만 추가해요. 최신·버전 요청 경로에서 사용자에게 보이는 첫 문장은 `현재 버전을 확인할게요.` 예요.

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

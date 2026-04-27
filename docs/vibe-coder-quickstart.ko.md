# 바이브코더 빠른 시작 가이드 (5분 컷)

> **대상**: Cursor / Claude / Copilot 으로 앱은 만들어 봤지만, 터미널이나 CLI 는 거의 써 본 적이 없는 분.
> **목표**: 이 문서를 따라 5분 안에 첫 axhub 배포까지 완료한다.

---

## 한 줄 요약

Claude Code 한 곳에서 한국어 자연어로 ("내 앱 배포해") 말하면, 회사 axhub 에 내 앱이 안전하게 올라갑니다. 직접 배포 명령어를 외울 필요가 없습니다.

---

## 준비물 (시작 전 1분 점검)

다음 세 가지가 준비되어 있어야 합니다. 하나라도 없으면 회사 IT/관리자에게 요청하세요.

| 준비물 | 어떻게 확인 | 없을 때 |
|---|---|---|
| **Claude Code 최신 버전** | `claude --version` (터미널에서) | https://claude.com/code 에서 설치 |
| **axhub CLI** (관리자가 회사 머신에 사전 설치) | `axhub --version` 입력 시 `axhub 0.1.0` 같은 줄이 보이면 OK | 회사 IT 에게 "axhub CLI 설치해 주세요" |
| **axhub 계정** (회사가 발급) | 사내 axhub 가입 안내 메일 또는 슬랙 DM 확인 | 관리자에게 "axhub 계정 만들어 주세요" 요청 |

> 💡 `axhub` 명령이 없다고 뜨면 → [troubleshooting.ko.md — "axhub 명령이 안 보여요"](./troubleshooting.ko.md#axhub-명령이-안-보여요) 참고.

---

## 5단계 흐름 (총 5분)

각 단계는 (1) 어떤 동작인지 (2) 그대로 복사해서 쓸 수 있는 명령 또는 발화 (3) 화면에 보일 결과 (4) 예상 시간 (5) 막혔을 때 어디로 가야 하는지 순서로 적었습니다.

### Step 1. 플러그인 설치 (약 1분)

axhub 플러그인을 Claude Code 에 등록합니다. 평생 한 번만 하면 됩니다.

```text
/plugin marketplace add jocoding-ax-partners/axhub
/plugin install axhub@axhub
```

**예상 결과**

```
✓ Marketplace 'axhub' added (1 plugin)
✓ Plugin 'axhub' installed (v0.1.0)
   Skills: deploy, status, logs, apps, apis, auth, update, doctor, help
   Commands: /axhub:deploy, /axhub:status, /axhub:logs, ...
```

설치 완료 메시지가 안 보이거나 빨간 글씨가 뜨면 → [troubleshooting.ko.md — "axhub 명령이 안 보여요"](./troubleshooting.ko.md#axhub-명령이-안-보여요) 참고.

---

### Step 2. 첫 인증 — "axhub 로그인해줘" (약 1분)

브라우저로 회사 SSO 또는 axhub 계정으로 로그인합니다. 평소 GitHub / Google 로그인과 똑같습니다.

말하기 (Claude Code 에 그대로 입력):

```text
axhub 로그인해줘
```

**예상 결과**

```
🔑 OAuth Device Flow 를 시작합니다.
브라우저가 곧 열립니다 → https://hub-api.jocodingax.ai/device
표시된 코드: ABCD-1234

(브라우저 창 자동 열림 → 로그인 → "Approve" 버튼 클릭)

✓ 로그인 완료
   사용자: yourname@yourcompany.com
   회사:   yourcompany
   유효:   다음 14일
```

브라우저가 열리지 않는 환경 (Codespaces, 원격 서버 등) 이면 → [troubleshooting.ko.md — "Codespaces 에서 로그인이 안 돼요"](./troubleshooting.ko.md#codespaces-에서-로그인이-안-돼요) 참고.

---

### Step 3. 내 앱 확인 — "내 앱 목록 보여줘" (약 30초)

내가 회사에서 만든 앱들이 실제로 axhub 에 등록되어 있는지 확인합니다. 배포할 앱의 정확한 이름을 여기서 외워두세요.

말하기:

```text
내 앱 목록 보여줘
```

**예상 결과** (예시)

```
📦 내 앱 (3개):

1. paydrop          (status: ready,    last deploy: 2일 전)
2. order-dashboard  (status: ready,    last deploy: 1주 전)
3. invoice-bot      (status: building, last deploy: 방금 시작)

총 3개 / 회사 전체 : 17개 (다른 사람 앱은 숨김)
```

목록이 비어 있으면 회사 관리자가 아직 내 계정에 앱 권한을 주지 않은 상태입니다. 관리자에게 "내가 만든 앱을 axhub 에 등록해 주세요" 요청.

---

### Step 4. 첫 배포 — "내 paydrop 배포해" (약 30초 + 빌드 3분)

이제 진짜 배포입니다. 무서울 수 있지만 **승인 카드** 가 한 번 더 묻습니다. **승인 전까지는 아무것도 일어나지 않으니 안심하세요.**

말하기 (`paydrop` 자리에 자기 앱 이름):

```text
내 paydrop 배포해
```

**예상 결과 — Step 4-A: 승인 카드 (preview card)**

```
┌─────────────────────────────────────────────────────┐
│ 다음을 실행할게요:                                   │
│                                                      │
│ ① 앱:    paydrop (id=42)                            │
│ ② 환경:  production (https://hub-api.jocodingax.ai) │
│ ③ 브랜치: main                                       │
│ ④ 커밋:  a3f9c1b — "결제 페이지 버그 수정"          │
│           (12분 전 푸시, you)                        │
│ ⑤ 예상:  약 3분 소요                                 │
│                                                      │
│ 진행할까요?                                          │
│  [네]   [아니요]   [미리보기만 (--dry-run)]          │
└─────────────────────────────────────────────────────┘
```

여기서 다섯 줄 (앱 / 환경 / 브랜치 / 커밋 / 예상 시간) 을 **반드시 한 번 읽어보세요.** 다른 앱이 적혀 있거나 환경이 `staging` 이 아니라 `production` 인데 prod 가 아닌 줄 알았다면 — `[아니요]` 누르고 다시 시작하면 됩니다.

> 🛡️ "한번 해보기만 할까?" 라고 말하면 자동으로 `[미리보기만]` 모드로 들어갑니다. 진짜 prod 에 올라가지 않고 배포 시뮬레이션만 합니다. 처음 배포 전에 한 번 추천합니다.

**예상 결과 — Step 4-B: 승인 후 진행 상황**

```
✓ 배포 시작 (deployment id: dep_8821)
⏳ 빌드 중... (1분 경과, 정상)
⏳ 빌드 중... (2분 경과, 정상)
✓ 빌드 완료, 배포 중...
✓ 배포 성공 🎉
   URL: https://paydrop.yourcompany.jocodingax.ai
   소요: 2분 47초
```

빨간 글씨로 멈췄다면 → 다음 Step 5 보세요.

---

### Step 5. 상태 확인과 로그 — "배포 어떻게 됐어" / "왜 실패했어" (필요할 때)

배포 후 다른 일을 하다가 다시 돌아왔을 때, 또는 위에서 빨간 글씨가 나왔을 때 사용합니다.

**5-A. 진행 상황 다시 보기**

```text
배포 어떻게 됐어
```

→ 가장 최근 배포의 상태가 화면에 다시 뜹니다 ("아직 빌드 중", "성공", "실패").

**5-B. 실패 원인 조사**

```text
왜 실패했어
```

또는

```text
빌드 로그 보여줘
```

**예상 결과** (실패 케이스 예시)

```
📝 paydrop 배포 dep_8822 — FAILED
원인 분류: 빌드 에러

마지막 로그 30 줄:
  [build] npm ERR! Cannot find module 'react-stripe'
  [build] npm ERR! 패키지가 설치되지 않은 것 같아요.

추정: package.json 에 빠진 의존성이 있어요.
다음에 할 일: 로컬에서 `npm install react-stripe` → commit → 다시 배포
```

배포가 실패한다고 **회사 prod 가 망가진 것이 아닙니다.** 이전에 성공한 버전이 그대로 살아 있습니다. 안심하고 코드를 고쳐서 다시 배포하면 됩니다 ([troubleshooting.ko.md — "되돌리기"](./troubleshooting.ko.md#실수로-잘못-배포했어요-되돌릴-수-있나요) 참고).

---

## 흔한 첫 막힘 5가지

5분 안에 첫 배포 못 하시는 분들의 95% 는 아래 다섯 중 하나입니다. 클릭해서 자세한 해결법으로 이동하세요.

1. **`exit 65` — 토큰 만료** → [troubleshooting.ko.md — exit 65](./troubleshooting.ko.md#exit-65--토큰-만료-가장-흔함)
2. **`exit 64` + `deployment_in_progress` — 다른 배포가 이미 돌고 있음** → [troubleshooting.ko.md — 동시 배포](./troubleshooting.ko.md#exit-64--deployment_in_progress--이미-다른-배포-중)
3. **`exit 64` + `app_ambiguous` — 앱 이름이 모호함** → [troubleshooting.ko.md — 앱 이름 모호](./troubleshooting.ko.md#exit-64--app_ambiguous--앱-이름이-모호함)
4. **`exit 67` — 앱을 찾지 못함** → [troubleshooting.ko.md — exit 67](./troubleshooting.ko.md#exit-67--앱을-찾지-못함--did-you-mean-패턴)
5. **브라우저가 안 열려요 (Codespaces / 원격 서버)** → [troubleshooting.ko.md — Codespaces](./troubleshooting.ko.md#codespaces-에서-로그인이-안-돼요)

---

## 다음에 할 수 있는 것

자연어 만으로도 다음 작업이 가능합니다. 더 자세한 명령은 `/axhub:help` 입력 시 한국어 메뉴가 뜹니다.

| 의도 | 자연어 예시 | 명시적 슬래시 |
|---|---|---|
| 🚀 배포 | "paydrop 배포해", "방금 푸시한 거 올려" | `/axhub:deploy [앱이름]` |
| 📊 상태 확인 | "지금 진행 중인 거 어떻게 됐어" | `/axhub:status` |
| 📝 로그 보기 | "왜 실패했어, 빌드 로그 보여줘" | `/axhub:logs` |
| 📦 앱 목록 | "내 앱 목록", "어떤 앱 있어" | `/axhub:apps` |
| 🔌 API 카탈로그 | "어떤 API 쓸 수 있어" | `/axhub:apis` |
| 🔑 로그인 | "로그인해", "권한 만료된 것 같아" | `/axhub:login` |
| 🔧 진단 | "axhub 설치돼 있어?" | `/axhub:doctor` |
| 📦 CLI 업데이트 | "axhub 새 버전 있어?" | `/axhub:update` |

---

## 도움 받기

- 에러 메시지가 떴는데 무엇을 뜻하는지 모르겠다 → [troubleshooting.ko.md](./troubleshooting.ko.md)
- 회사 단위로 도입 / 정책 / 보안 질문 → 회사 axhub 관리자 또는 [org-admin-rollout.ko.md](./org-admin-rollout.ko.md)
- 플러그인 자체의 버그 / 기능 요청 → https://github.com/jocoding-ax-partners/axhub/issues

> **두려워하지 마세요.** axhub 플러그인은 destructive 한 모든 동작 (배포, 토큰 변경 등) 전에 반드시 한 번 더 묻게 설계되어 있습니다. "엔터" 한 번에 prod 가 망가지는 일은 일어나지 않습니다.

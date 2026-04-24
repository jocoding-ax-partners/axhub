# axhub Claude Code 플러그인 — 랜딩 페이지 카피 초안 (Korean)

> 위치: https://jocodingax.ai/plugins/claude-code (예정). Static markdown 으로 시작 → React 화 (Phase 4 후속).

---

## Hero section

```
한국어로 말하면, 안전하게 배포돼요.

"내 paydrop 앱 prod 에 올려" — 한 마디로 끝.

Claude Code 플러그인 한 번 깔면, 비-개발 직군 vibe coder 도
복잡한 axhub CLI 명령 안 외우고 자기 앱을 직접 배포할 수 있어요.
사고 없이, IT 도움 없이.

[5분 안에 직접 try →]   [회사 5명 무료 파일럿 신청 →]
```

---

## Problem section ("지금 회사에서 일어나는 일")

```
> "이거 deploy 어떻게 하지?" — 매주 vibe coder 한테 들으셨다면

전형적 시나리오:

  1. vibe coder 가 LLM으로 코드 짬 → git push
  2. 배포 명령 어떻게 쓰는지 몰라서 IT/개발팀에 슬랙 DM
  3. IT 가 "axhub deploy create --app paydrop --branch main --commit abc..." 알려줌
  4. vibe coder 가 복붙하다 commit sha 잘못 입력 → 에러
  5. "에러 났어요" → IT 가 또 봐줘야 함
  6. 토큰 만료되면 처음부터 반복

이 cycle 한 번에 vibe coder 30분, IT 15분 = 회사 시간 45분.
주 1회만 일어나도 한 달이면 vibe coder 1명당 3시간 손실.
```

---

## Solution section ("axhub 플러그인 깐 후")

```
같은 시나리오, axhub 플러그인 한 번 깐 후:

  1. vibe coder 가 LLM으로 코드 짬 → git push
  2. Claude Code 에 한국어로 "방금 푸시한 거 prod 에 올려"
  3. 5가지 정보 미리보기 카드 자동 생성:
     ┌─────────────────────────────────────┐
     │ ① 앱:    paydrop (id=42)           │
     │ ② 환경:  production                 │
     │ ③ 브랜치: main                      │
     │ ④ 커밋:  a3f9c1b — "결제 페이지 버그 수정" │
     │ ⑤ 예상:  약 3분 소요                │
     │ 진행할까요? [네 / 아니요 / 미리보기]    │
     └─────────────────────────────────────┘
  4. "네, 진행" → 자동 시작
  5. 한국어로 "30초 경과, 빌드 중이에요" 진행 안내
  6. "배포 성공! 라이브 URL 클릭해보세요" + 알림

소요 시간: 5분. IT 호출: 0회.
```

---

## Safety section ("그래서 안전한가요?" — IT/보안 책임자용)

```
다 자동화해도 안전합니다. 핵심 가드 3개:

[1] HMAC consent token gate
  - 모든 destructive 명령 (deploy/update/login) 은 vibe coder 본인 OK 없이 실행 불가
  - LLM 이 자율적으로 prod 변경 시도 → PreToolUse hook 가 deny
  - HMAC 키는 per-user, mode 0600, 다른 노트북에 복사 X

[2] cosign 서명 검증 (sigstore OIDC)
  - helper 바이너리 매 릴리즈마다 키리스 서명
  - AXHUB_REQUIRE_COSIGN=1 회사 정책에서 unsigned 시 경고
  - update 시 검증 실패하면 절대 진행 안 함 (IT 호출 안내)

[3] 한국어 4-part 에러 안내 (vibe coder 가 자력 회복)
  - 감정 ("당신 앱은 안전합니다") + 원인 + 해결 + 버튼
  - exit 65 (token 만료) → 한국어 안내 + 재로그인 자동
  - exit 67 (앱 없음) → did-you-mean 후보 + 선택
  - 90% 에러를 vibe coder 가 IT 안 부르고 처리

회사 보안 정책 우선:
  - AXHUB_TELEMETRY default OFF (사용량 데이터 미수집)
  - 토큰 redact 자동 (transcript 에 axhub_pat_* 노출 X)
  - cross-team API 카탈로그 default 격리 + 명시 동의 시 audit log
```

---

## How it works section (architecture diagram)

```
사용자 발화 ("paydrop 배포해")
        │
        ▼
Claude Code  ──→  axhub plugin
        │              ├── skills/* (NL 자동 트리거)
        │              ├── commands/* (슬래시)
        │              ├── hooks/* (PreToolUse HMAC consent)
        │              └── bin/axhub-helpers (TS/Bun)
        │                       │
        ▼                       │
   Bash tool ────────────────────┘
        │
        ▼
   ax-hub-cli binary (cosign 서명, v0.1.0+)
        │
        ▼
   https://hub-api.jocodingax.ai
```

플러그인 = **얇은 routing/recovery layer**. 비즈니스 로직은 모두 ax-hub-cli (회사 SaaS) 에. 플러그인이 하는 일:
1. 자연어 인텐트 → CLI 명령어 매핑
2. HMAC consent token 으로 destructive op 보호
3. Exit code 기반 한국어 자동 복구

---

## Tech specs ("개발자가 보고 싶은 숫자")

| 항목 | 값 |
|---|---|
| Plugin version | 0.1.0 (initial release) |
| ax-hub-cli compatibility | `>=0.1.0,<0.2.0` |
| Test coverage | 295 passing, 2136 expect() assertions |
| Cross-arch binaries | macOS arm64/Intel, Linux x86_64/arm64, Windows x86_64 |
| Hook gate latency | <50ms (PreToolUse HMAC verify) |
| First deploy time (vibe coder) | <30분 (median, pilot 측정) |
| Korean trigger lexicon | 11 skills, 200+ trigger phrases |
| License | MIT |

---

## Get started section (3-step CTA)

```
지금 바로 시작:

[1] 본인이 직접 try (5분)
    ┌────────────────────────────────────────────────┐
    │ /plugin marketplace add jocoding-ax-partners/  │
    │   axhub-plugin-cc                              │
    │ /plugin install axhub@axhub                    │
    │ bash ${CLAUDE_PLUGIN_ROOT}/bin/install.sh      │
    │ /axhub:login                                   │
    └────────────────────────────────────────────────┘
    → "안녕" 이라고 말해보세요. 첫 배포까지 도와드려요.

[2] 회사 5명 무료 파일럿 신청
    → docs/pilot/README.md 보고 IT/admin 한테 보여주세요
    → 또는 우리한테 직접: hello@jocodingax.ai

[3] 더 알아보기
    → README: https://github.com/jocoding-ax-partners/axhub
    → 6 phases of audit + 65 design decisions: PLAN.md
    → Open source MIT license — 코드 다 보세요
```

---

## FAQ ("물어보고 싶을 만한 거 미리 답함")

**Q: vibe coder가 LLM으로 prod에 잘못된 코드 push 하면 어떻게 막나요?**
A: 그건 axhub 플러그인이 아니라 axhub SaaS (서비스 자체) 의 정책 영역이에요. 플러그인은 "vibe coder의 의도를 안전하게 명령으로 변환" 까지가 책임. 코드 review/CI 정책은 회사 GitHub Actions / 다른 도구로 따로 운영하시면 돼요. 둘은 layer 가 다릅니다.

**Q: telemetry 절대 안 보내요?**
A: 환경변수 `AXHUB_TELEMETRY=1` 명시 enable 안 하면 단 한 줄도 안 보냅니다. ON 으로 했을 때 기록되는 항목은 `docs/pilot/admin-rollout.ko.md` 의 telemetry section 에 모두 명시 (이벤트 타입 + exit code + version metadata만, command 인자 절대 X).

**Q: 회사 IT 가 모든 vibe coder 의 토큰을 admin 에서 관리해야 하나요?**
A: 네, axhub team 안에서 IT 가 issue/revoke 하시면 됩니다. 토큰 자체는 per-user 노트북에서 mode 0600 으로 보관 — IT 가 vibe coder 노트북에 SSH 접근하지 않아도 OK.

**Q: ax-hub-cli 새 버전 (v0.2.x) 나오면 어떻게 되나요?**
A: 플러그인이 semver gate 갖고 있어요. v0.2.x 가 나오면 `MAX_AXHUB_CLI_VERSION` 늘리는 새 플러그인 minor (0.2.0) 가 같은 시점에 나올 예정. 그동안은 기존 v0.1.x 그대로 작동.

**Q: Claude Code 가 아니면 못 쓰나요?**
A: 현재는 Claude Code 전용입니다. Cursor / Copilot CLI / 다른 IDE 지원은 Phase 4+ 후 ROI 보고 결정.

---

## Footer

```
axhub Claude Code 플러그인 v0.1.0
MIT License — github.com/jocoding-ax-partners/axhub
Made by jocoding-ax-partners (https://jocodingax.ai)
한국 vibe coder 들이 자력으로 자기 앱을 안전하게 배포하는 환경을 만들고자 합니다.
```

---

## A/B test 후보 (Phase 4 진행 시)

- **Hero CTA copy**: "5분 안에 try" vs "5명 무료 파일럿" — vibe coder 본인 vs 회사 IT 누가 먼저 클릭
- **Safety section 위치**: 위 (현재) vs 아래 — 회사 IT 타겟이면 위, vibe coder 타겟이면 아래
- **Tech specs**: 표 형태 vs 인포그래픽 — 개발자 컨버전 차이 측정

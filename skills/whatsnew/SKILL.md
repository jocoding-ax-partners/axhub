---
name: whatsnew
description: '이 스킬은 사용자가 axhub 의 새 기능, 변경점, 릴리즈 노트, changelog 를 알고 싶어할 때 사용해요. 다음 표현에서 활성화: "뭐 새로", "뭐 새로 나왔어", "새 기능 뭐야", "신규 기능", "changelog", "release notes", "what''s new", "whatsnew", 또는 axhub 변경점 확인 의도. axhub whatsnew 를 read-only 로 호출해요.'
multi-step: false
needs-preflight: false
allows-dependency-execution: false
---

# Whatsnew

axhub CLI 가 제공하는 변경점 요약을 read-only 로 보여줘요.

## Workflow

To show what is new:

1. **CLI 명령을 호출해요.**

   ```bash
   axhub whatsnew --json
   ```

   JSON 이 지원되지 않는 CLI 라면 `axhub whatsnew` 로 fallback 하고, 출력은 원문 그대로 길게 붙이지 말고 요약해요.

2. **사용자에게 필요한 변화만 묶어요.** breaking change, migration, security note, new command 를 구분해요.

3. **plugin release 와 CLI release 를 구분해요.** 플러그인 업그레이드 의도면 upgrade skill 로 넘기고, CLI binary 업데이트 의도면 update skill 로 넘겨요.

## NEVER

- NEVER 인터넷 release note 를 임의로 source of truth 로 삼지 않아요.
- NEVER update/upgrade 를 자동 실행하지 않아요.
- NEVER changelog 원문을 과도하게 길게 붙이지 않아요.

## v0.3.2 — 라우팅 단순화

### 무엇이 바뀌었어요

이전 (v0.3.x):
- Rust prompt-route hook 가 키워드 체인 ~600줄로 의도 분류했어요.
- 메타 질문 ("왜 ~ 키워드 매칭이야?") → 잘못된 skill 라우팅 위험이 있었어요.
- 자연어 변형 ("어제 만든 결제 페이지 라이브로 띄워봐") robust 부족이었어요.

이후 (v0.3.2):
- Claude 가 SKILL.md description 을 보고 직접 매칭해요.
- 메타 질문 자동 처리 (Claude native 인식)예요.
- 자연어 composition robust ("어제", "결제 페이지", "라이브로", "띄워봐" 모두 cohesive 해석)이에요.
- 200MB 모델 도입 X (binary 변경 거의 0)이에요.
- 600줄 Rust 폐기 → 유지보수 부담 감소예요.

### Before / After 데모

예시 1 — 메타 질문:
- Before: "왜 배포가 키워드 매칭이지?" → /axhub:deploy 실행 (잘못)
- After: 같은 prompt → LLM 자유 응답 (정상, deploy 실행 X)

예시 2 — 모호한 prompt:
- Before: "올려" → /axhub:deploy (단순 substring 매칭, 의도 추측)
- After: "올려" → "deploy 인지 logs 인지 모호해요. 어느 거예요?" 모호 처리

예시 3 — 자연어 변형:
- Before: "어제 만든 결제 페이지 라이브로 띄워줘" → keyword "라이브" 안 잡혀 fail
- After: 같은 prompt → embedding-free description 매칭 → /axhub:deploy

### 라우팅 통계 보기

```bash
axhub-helpers routing-stats --since 7d
```

출력 예시:

```
[지난 prompt 통계]
==========================================
총 prompt:           234
axhub 관련 가능성:    89 (38.0%)
auth 실패:           3
prompt 길이 p50/p95: 42 / 180 bytes

CLI 버전:
  0.11.0: 234

상위 axhub 관련 prompt (hash):
  sha256:abc...:  12
```

### 환경 변수

- `AXHUB_NO_AUDIT=1` — audit log 비활성

### Privacy

- prompt content 저장 X (sha256 hash + length + cli_version + auth_ok 만 저장해요)
- 외부 전송 X (모두 로컬)
- 짧은 prompt (예: "deploy") 의 hash 는 익명화 보장 X
- 7일 자동 회전 (오래된 파일 삭제)
- 전체 삭제: `axhub-helpers cleanup-audit --all`

상세 architecture 는 [docs/routing.md](../../docs/routing.md) 를 참고해요.

## Non-interactive AskUserQuestion guard

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 은 현재 structured AskUserQuestion 을 쓰지 않지만, 질문을 추가할 때는 `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서 안전 기본값을 사용해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 등록해요.

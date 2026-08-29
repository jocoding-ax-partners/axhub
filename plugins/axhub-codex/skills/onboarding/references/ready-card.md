# Ready Cards

Load this after core gaps are resolved. 처리 순서는 AI 활용 기록 옵트인 → final card 예요.

마무리 진입 시 사용자에게 먼저 한 줄로 예고해요: "마지막 단계예요 — AI 활용 기록(선택)만 정하면 끝나요." 이 단계의 원칙은 **카드 1장 · 질문은 옵트인 1개**예요.

## AI 활용 기록 옵트인 (선택)

AI 활용 기록은 내 Codex 프롬프트·응답·툴콜 내용을 팀 워크스페이스로 보내는 수집 기능이에요 (`axhub axrouter`). 켜는 것은 항상 사용자 선택이에요 — 동의 없이 켜지 않아요. headless 면 이 섹션을 통째로 건너뛰어요 (묻지도 실행하지도 않아요).

1. `axhub axrouter status --json` 을 실행해요 (read-only). 명령 실패(구 CLI 포함)거나 `data.workspaces[]` 에 `available: true` 인 워크스페이스가 없으면 조용히 건너뛰어요. 이미 수집 중인지는 `data.active_workspace` 로만 판단해요 — 값이 있으면 묻지 않고 최종 카드에 상태만 반영해요. `data.local_monitoring` 이 true 라도 `active_workspace` 가 null 이면 axhub 수집이 아니라 다른 도구의 텔레메트리 설정일 수 있으니 건너뛰지 말고 정상적으로 물어봐요.
2. 명시 텍스트 승인 한 번으로 물어요. 표준 질문은 아래 모양이에요 — 무엇이 수집되는지·어디로 가는지·선택 사항이라는 점·나중에 켜는 방법이 전부 질문 안에 담겨야 하고, 기본 선택지는 "이번엔 건너뛰기"(첫 번째 옵션)예요.

```json
{
  "questions": [{
    "question": "AI 활용 기록을 켤까요? 이 컴퓨터의 Codex 프롬프트·응답·툴콜 내용이 <workspace-slug> 워크스페이스로 전송돼요. 선택 사항이에요 — 지금 건너뛰어도 나중에 'AI 활용 기록 켜줘' 한마디로 켤 수 있어요.",
    "header": "AI 활용 기록",
    "multiSelect": false,
    "options": [
      {"label": "이번엔 건너뛰기", "description": "수집 없이 온보딩을 마무리해요. 나중에 'AI 활용 기록 켜줘' 라고 말하면 켜져요"},
      {"label": "켜기", "description": "<workspace-slug> 워크스페이스로 수집을 시작해요 (Codex 재시작 후 적용)"}
    ]
  }]
}
```

   available 워크스페이스가 여러 개면 같은 질문에서 대상 워크스페이스도 골라요 — 옵션을 워크스페이스별로 나누되 질문은 여전히 1개예요. `local_monitoring` 이 true 인데 `active_workspace` 가 null 이면 다른 텔레메트리 설정이 이미 있는 상태라, "켜면 기존 수집 설정이 axhub 설정으로 교체돼요" 한 문장을 question 에 덧붙여요 (CLI 가 켤 때 외부 OTEL 키를 제거하고 제거 목록을 알려줘요).
3. 건너뛰기 → "나중에 켜고 싶으면 'AI 활용 기록 켜줘' 라고 말하면 돼요." 한 줄만 남기고 같은 온보딩에서 다시 묻지 않아요.
4. 켜기 → `axhub axrouter monitor --tenant <slug> --json`.
   - 성공 → 적용은 Codex 재시작 후예요. 별도 카드나 안내 문단을 만들지 않아요 — 최종 카드 한 곳에 "AI 활용 기록 켜짐 — 재시작 후 적용" 한 줄로만 반영해요.
   - `error.subcode` 가 `consent_required` → 본문 수집 동의 기록이 아직이에요. 위 옵트인 질문에 수집 내용 고지가 이미 담겨 있고 사용자가 "켜기" 를 골랐으므로, `axhub axrouter consent --agree --execute --tenant <slug> --json` 으로 동의를 기록한 뒤 monitor 를 1회만 재시도해요 (consent·재시도 각 1회 상한). 이 lane 이 성립하려면 옵트인 질문에 프롬프트·응답·툴콜 내용 고지가 반드시 남아 있어야 해요.
   - consent 명령까지 실패하면(구 CLI·권한 등) 콘솔 fallback 이에요 — `error.doc_url` 의 동의 페이지 주소를 보여주고 "콘솔 동의 후 'AI 활용 기록 켜줘' 라고 말해 주세요" 로 남겨요. green check 는 달지 않아요.
5. 끄기·해제는 온보딩 범위 밖이에요 — 물으면 `axhub axrouter monitor --off`(이 컴퓨터만 끔) / `axhub axrouter revoke`(등록 해제) / `axhub axrouter revoke-consent --execute`(본문 수집 동의 철회 — 활성 등록도 함께 해제돼요)를 알려줘요.

수집 전용 토큰은 CLI 가 settings.json 에만 기록해요 — 토큰 값이나 raw JSON 을 chat 에 출력하지 않아요.

## VIBE_READY Card

Use `VIBE_READY` only when checked items are actually green.

```text
axhub 온보딩 완료예요. [VIBE_READY]
  ✓ CLI v<CLI_VERSION>
  ✓ 로그인 <masked-email>
  ✓ git v<GIT_VERSION>
  ✓ node v<NODE_VERSION> (pm: <bun|pnpm|npm|yarn>)
  ✓ <저장소 준비 상태 — backend별 한 줄만>
  ✓ 앱 <app-slug> 연결됨
  ✓ 첫 배포 live: <deployment-url>
  ✓ 점검 통과
  ✓ AI 활용 기록 켜짐 — <workspace-slug> (Codex 재시작 후 적용) — 이번에 켰거나 status 의 `active_workspace` 로 확인될 때만, 아니면 줄 생략

이제 바로 코딩하면 돼요.
다음에 말할 수 있는 것: "첫 앱 만들어줘", "배포해", "로그 봐줘", "환경변수 추가해줘", "테이블 추천해줘"
```

Choose exactly one repository line after `axhub apps get <app> --json` or fresh `axhub apps git-backend --tenant <tenant> --json`. For selfhosted, render only `✓ axhub 저장소 준비됨` and never include provider login/install copy or `github.install_url`. For GitHub, preserve the existing App line and include the install URL when detect provided it.

## Degraded Cards

아래 상태 이름은 내부 분기용이에요. 사용자에게는 enum 이름이나 대괄호 표식을 출력하지 말고, 현재 상태·남은 행동·다시 말할 문장만 자연스러운 한국어로 보여줘요.

`READY_WITH_USER_ACTION`: an applicable external approval or local user action remains. Provider login/App install examples are GitHub-only; selfhosted cards never list them. Include exactly what to do and what to say next.

`SAFE_STOP_NONINTERACTIVE`: CI/headless/subprocess mode avoided mutation. Include manual commands or natural next phrase; do not suggest that the agent already completed setup.

`BLOCKED_UNSUPPORTED`: no safe OS, package manager, permissions, or install path exists. Explain the unsupported condition and the safest next human-owned step.

Never mix a degraded card with green check marks for unverified items.

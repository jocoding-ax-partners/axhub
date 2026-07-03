# 온보딩 MCP 재시작 안내 + 세션 이어가기 설계

- 날짜: 2026-07-03
- 범위: `skills/onboarding` (+ `hooks/hooks.json`, `CLAUDE.md` 동기화)
- 관련 이슈: "온보딩 시 MCP 설치 제대로 안됨 → 설치 후 재시작 안내 + 세션 이어가기 필요"

## 문제

새로 등록한 MCP 서버는 Claude Code 를 재시작해야 세션에 로드돼요 (development skill 의 `references/mcp-setup.md` 가 이미 "재시작 제약 (핵심)"으로 문서화한 사실이에요). 그런데 현재 onboarding 의 `references/mcp-ready-card.md` 는 `claude mcp add` 직후 **같은 세션에서** `/mcp` OAuth 를 하라고 안내해요. 새로 add 한 서버는 현재 세션 `/mcp` 목록에 보이지 않으니 사용자는 OAuth 를 완료할 수 없고, 온보딩은 `READY_WITH_USER_ACTION` 에서 사실상 끊겨요. 재시작하고 나면 새 세션은 온보딩 맥락이 없어서 사용자가 뭘 말해야 이어지는지도 몰라요.

## 결정 요약

| 결정 | 선택 | 근거 |
| --- | --- | --- |
| 적용 범위 | onboarding 만 | development 는 이미 CLI fallback + 재시작 한 줄 안내로 처리돼요. 최소 diff. |
| resume 방식 | marker 파일 + SessionStart hook (proactive) + 카드 안내 fallback | 사용자가 재시작 후 뭘 말할지 몰라도 새 세션이 먼저 제안해요. 기존 auto-update hook 패턴 재사용이에요. |
| marker 소유 | plugin bash marker (CLI 변경 없음) | 온보딩은 CLI 설치 전·구버전에서도 돌아야 하는 스킬이라 CLI 의존 breadcrumb(init-resume 패턴)을 쓸 수 없어요. 빈 파일 + mtime 이면 충분해요. |

## 설계

### 1. MCP step 재정렬 (`skills/onboarding/references/mcp-ready-card.md`)

- `claude mcp get axhub` 가 이미 `Status: Connected` 면 (이전에 등록·연동됨) 기존처럼 바로 VIBE_READY 로 가요. marker 없음.
- 이 세션에서 **새로 `claude mcp add` 를 실행한 경우**: 현재 세션 `/mcp` OAuth 안내를 제거하고, marker 를 쓴 뒤 **restart handoff card** 로 종료해요.

```text
axhub MCP 등록했어요. 도구 활성화에는 Claude Code 재시작이 필요해요. [READY_WITH_USER_ACTION]
  1. 이 세션 종료 후 claude 다시 실행해 주세요
  2. 새 세션이 온보딩 마무리를 먼저 제안해요 — 안 뜨면 "온보딩"이라고 말해 주세요
```

- Claude Desktop / `claude` CLI 없음 경로는 기존 수동 커넥터 안내 그대로예요. marker 안 써요.
- headless(`SAFE_STOP_NONINTERACTIVE`) 경로도 기존대로 add 자체를 안 하니 marker 없음.

### 2. Marker 파일

- 경로: `~/.axhub/cache/.onboarding-mcp-restart` (기존 `.plugin-update-check` 와 같은 디렉토리).
- 내용: 의미 없음 (`date` 출력 한 줄). 판정은 mtime 만 사용해요.
- 쓰기 (skill, add 직후): `mkdir -p ~/.axhub/cache && date > ~/.axhub/cache/.onboarding-mcp-restart`
- 삭제 (skill): resume 절차가 최종 카드를 출력한 뒤, 그리고 onboarding 이 어떤 경로로든 VIBE_READY 를 출력할 때 `rm -f` 해요. hook 은 삭제하지 않아요 (읽기 전용 유지).
- 진행상태 JSON 은 저장하지 않아요 — onboarding 은 detect-first 라 재감지로 전부 복원돼요.

### 3. SessionStart hook (`hooks/hooks.json` 두 번째 entry)

기존 auto-update 훅과 같은 계약이에요: cheap bash, `"shell": "bash"` (Git Bash 호환 — `$HOME`/`find` 만 사용), best-effort·비차단, kill switch.

```bash
[ -n "$AXHUB_NO_ONBOARDING_RESUME" ] && exit 0
M="$HOME/.axhub/cache/.onboarding-mcp-restart"
[ -f "$M" ] || exit 0
[ -n "$(find "$M" -mmin -10080 2>/dev/null)" ] || exit 0
echo "[axhub] 온보딩 MCP 마무리가 남았어요. 사용자에게 이어서 확인할지 물어보고, 동의하면 skills/onboarding 의 resume 절차(claude mcp get axhub 확인 → 필요시 /mcp OAuth 안내 → 최종 카드 → marker 삭제)를 진행하세요. best-effort — 실패하면 조용히 건너뛰고 사용자의 작업을 막지 마세요."
```

- TTL 7일(`-mmin -10080`): 초과하면 침묵해요. stale 파일 잔존은 무해하고, 다음 onboarding/VIBE_READY 경로가 지워요.
- hook 은 `axhub` 바이너리도 네트워크도 안 건드려요 — 파일 존재 + mtime 만 봐요.

### 4. Resume 절차 (`mcp-ready-card.md` 에 신설 섹션)

새 세션에서 hook nudge 를 받았거나 사용자가 "온보딩"이라고 하면:

1. 사용자에게 이어서 확인할지 물어요 (hook 경로일 때. 사용자가 직접 "온보딩"이라 했으면 바로 진행).
2. `claude mcp get axhub 2>&1 | grep -i status` 확인.
   - `Status: Connected` → VIBE_READY 카드 (MCP 줄 green) + marker 삭제.
   - `Needs authentication` → `/mcp` 에서 `axhub` 선택 → 브라우저 OAuth 안내 (재시작 후라 서버가 목록에 보여요). 완료 신호 받으면 재확인 → Connected 면 카드 + 삭제, 여전히 실패면 `READY_WITH_USER_ACTION` (marker 는 남겨서 다음 세션에 다시 nudge).
   - 명령 실패/서버 미등록 → 등록 명령부터 다시 안내 (기존 manual command 경로).
3. 절차 전체가 read-only(`claude mcp get`) + 사용자 action 안내뿐이라 headless 에서도 안전해요. headless 면 질문 생략하고 `SAFE_STOP_NONINTERACTIVE` 카드로 수동 명령만 남겨요.

resume 이 detect 를 다시 돌리는 것도 허용돼요 (read-only) — 온보딩 도중 환경이 바뀐 경우 first_gap 이 다시 잡아줘요.

### 5. 문서·게이트 동기화

- `skills/onboarding/SKILL.md`: step 4 문구를 restart handoff 흐름으로 갱신, NEVER 두 개 추가:
  - NEVER `claude mcp add` 를 실행한 그 세션에서 `mcp__axhub__*` 도구가 활성이라고 안내하지 말아요 — 재시작 handoff card 로 종료해요.
  - NEVER 최종 카드(VIBE_READY) 이후 marker 를 남기지 말아요.
- `CLAUDE.md` "자동 업데이트 hook" 섹션: SessionStart hook 이 2개(auto-update + onboarding-resume)임을 반영하고 `AXHUB_NO_ONBOARDING_RESUME` kill switch 를 문서화해요.
- 기존 quality gate 통과: `bun run lint:tone --strict` (해요체), frontmatter validity, `bun run plugin:bundle`.
- `tests/smooth-behavior.test.ts` 계열에 계약 문자열 테스트 추가: restart handoff card 마커, hook entry 존재, marker 경로 일치, NEVER 규칙 존재.

## Edge cases

| 상황 | 동작 |
| --- | --- |
| MCP 이미 Connected (사전 연동) | marker 없이 기존 VIBE_READY. hook 무관. |
| 사용자가 재시작 없이 같은 세션에서 "온보딩" 재호출 | detect 재실행 → MCP step 에서 get 이 여전히 미연결이면 handoff card 재출력 (marker mtime 갱신). |
| 재시작 후 hook nudge 무시하고 다른 작업 | 비차단 — 한 줄 제안 후 사용자 작업 우선. 다음 세션에도 TTL 내면 다시 nudge. |
| marker 있는데 사용자가 이미 스스로 OAuth 완료 | resume 확인이 Connected 로 즉시 통과 → 카드 + 삭제. |
| TTL(7일) 초과 | hook 침묵. 파일은 다음 onboarding 실행이 정리. |
| `AXHUB_NO_ONBOARDING_RESUME=1` | hook 즉시 skip. 카드 안내 fallback 만 남음. |
| Claude Desktop (claude CLI 없음) | 기존 수동 커넥터 안내. marker/hook 경로 미사용. |
| headless / CI | add 자체를 안 하므로 marker 없음. resume nudge 가 떠도 read-only 확인만. |

## Non-goals

- development skill 변경 (기존 CLI fallback 유지).
- ax-hub-cli 변경 (onboarding-detect 의 MCP 인식, breadcrumb 명령 등은 follow-up).
- 진행상태 JSON/step 트래킹 — detect-first 가 대체해요.
- hook 의 stale marker 자동 삭제 — 읽기 전용 유지.

## 테스트 계획 개요

1. `hooks/hooks.json` 파싱 + SessionStart entry 2개 + 두 번째 entry 가 marker 경로·TTL·kill switch 를 포함하는지.
2. `mcp-ready-card.md` 에 restart handoff card, resume 절차, marker 쓰기/삭제 명령이 존재하는지 (계약 문자열).
3. `SKILL.md` NEVER 규칙 2개 존재.
4. tone lint 0 err.

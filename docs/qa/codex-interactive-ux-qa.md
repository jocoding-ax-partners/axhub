# Codex 대화형 UX QA — 재현 절차와 관측 기록

> 대상: codex-cli **0.148.0** · 플랜: `docs/plans/2026-08-20-1146-feat-codex-interactive-ux-parity-plan.md` (U1)
> 원칙: 사용자 `~/.codex` 는 읽지도 쓰지도 않아요. 전 과정 격리 `CODEX_HOME` 이에요.

## 왜 격리 프로파일이어야 하나

메인테이너 머신의 `~/.codex/config.toml` 은 `approval_policy = "never"` + `sandbox_mode = "danger-full-access"` 예요. 그 상태로 QA 하면 승인 팝업도 elicitation 폼도 **원래 안 떠요** — 카드가 안 뜬다는 관측이 우리 번들 탓인지 정책 탓인지 구분되지 않아요. 그래서 QA 는 기본 정책(`on-request`) 프로파일에서만 유효해요.

## 재현 절차

```bash
export CODEX_HOME="$(mktemp -d)/codex-home"
mkdir -p "$CODEX_HOME/skills"
cp -R plugins/axhub-codex/skills/. "$CODEX_HOME/skills/"
printf 'approval_policy = "on-request"\nsandbox_mode = "workspace-write"\n' > "$CODEX_HOME/config.toml"

codex features list                 # 기본 flag 상태
codex -c features.default_mode_request_user_input=true features list   # opt-in 경로
codex debug prompt-input            # 스킬 카탈로그 주입 확인
```

## 관측 기록 (2026-08-20)

### 헤드리스로 확인된 것 `[confirmed]`

| # | 관측 | 결과 |
|---|---|---|
| 1 | `default_mode_request_user_input` 기본값 | `under development` / **false** — 선행 실측과 일치 |
| 2 | opt-in 경로가 실제로 열리나 | `codex -c features.default_mode_request_user_input=true` 로 **true** 로 바뀜 — 컴파일 타임 게이팅이 아님을 실행으로 확인 |
| 3 | elicitation 관련 feature | `auth_elicitation` stable/**true**, `tool_call_mcp_elicitation` stable/**true** |
| 4 | 격리 프로파일 스킬 발견 | 9개 스킬이 `r0` 루트로 카탈로그에 주입됨 |
| 5 | 게이트 byte offset | 실행 5스킬 전부 첫 8,000B 안 — `tests/codex-bundle.test.ts` 의 offset assert 가 계약으로 고정 |
| 6 | 사용자 config 불변 | `~/.codex/config.toml` 무수정 — 전 과정 `CODEX_HOME` 격리 |

### 라이브 TUI 관측 `[confirmed]` — 2026-08-20 실측

격리 프로파일은 인증이 없어 **401 Unauthorized** 로 모델 턴이 시작되지 않아요 (`codex exec` 실행으로 확인). 그래서 이 항목만 사용자 승인을 받아 **인증 프로파일에서 호출 단위 override** 로 관측했어요 — `-c approval_policy=on-request -c sandbox_mode=read-only`, config 파일 무수정, 임시 git 디렉토리에서 실행. TUI 는 pty 로 구동해 렌더 프레임을 캡처했어요.

| # | 관측 | 실측 결과 |
|---|---|---|
| A | 카드 렌더 | flag on 에서 네이티브 오버레이가 떠요 — `Question 1/1 (1 unanswered)` + 선택지 3개 + `1. 한식 (Recommended)` 접미 + `4. None of the above` 자동 추가 + `tab to add notes / enter to submit answer / esc to interrupt` |
| B | 빈 답변 auto-resolve | 카드에 `auto-resolves in 1m 00s` 카운트다운이 뜨고 0 에서 해제돼요. 모델에게는 `Questions 0/1 answered · (unanswered)` 가 돌아가고, 모델의 최종 응답은 **`답변이 입력되지 않았습니다.`** — 추측으로 진행하지 않았어요 |
| C | Esc 취소 | 카드에서 Esc 는 카드만 닫는 게 아니라 **턴 전체를 중단**해요 (`Conversation interrupted - tell the model what to do differently`). 승인으로 새는 경로가 아니에요 |
| D | 제어권 반환 시점 | 카드가 열려 있는 동안 모델은 `Working` 에 머물고 다른 도구를 부르지 않아요. 제어권은 답변 또는 auto-resolve 시점에만 돌아와요 |
| E | flag off 텍스트 lane | 번호 메뉴 없이 한 문장으로 묻고 턴을 끝내요 — 실측 발화: `한식, 일식, 양식 중 오늘 가장 끌리는 건 뭐야?` |

**판정:** 승인 게이트가 사용자 응답 없이 통과한 경로는 **0건**이에요 (Verification Contract 의 QA 기준). B·C·D 가 fail-closed 전제를 각각 다른 각도에서 확인해줘요 — 플랜 KTD6 의 3채널 중복은 이 동작을 강화하는 방향이고 뒤집지 않아요.

### 관측 중 확인된 환경 노이즈

캡처에 `SessionStart hook (failed) error: hook exited with code 127` 이 보이는데 **우리 번들과 무관**해요 — 사용자 `~/.codex/hooks.json` 의 Otty·jcode·superset 훅이고, axhub-codex 는 이 codex 환경에 설치돼 있지 않아요 (`~/.codex/plugins/cache/` 에 axhub 없음).

### 남은 것

전체 대표 여정(첫 셋업 → 앱 생성 → 배포)을 codex 에서 끝까지 도는 QA 는 실제 클라우드 리소스를 만들어요. 위 A~E 가 승인 게이트의 안전 기준을 직접 측정하므로 릴리즈 게이트로는 충분하고, 전체 여정 QA 는 릴리즈 후 첫 실사용에서 확인해요.

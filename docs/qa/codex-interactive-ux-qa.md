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

### 사람이 TUI 앞에 앉아야 확인되는 것 `[pending]`

대화형 TUI 렌더·타이밍이라 헤드리스로 대체할 수 없어요. flag on/off 두 번 돌려요.

| # | 관측 | 확인 방법 | 왜 중요한가 |
|---|---|---|---|
| A | 카드 렌더 | flag on 세션에서 배포 승인 지점에 방향키 선택 오버레이가 뜨는가 | KTD1 의 T1 전제 |
| B | 빈 답변 auto-resolve | 2분 방치 후 모델이 받는 값과 그 뒤 행동 | **KTD6 fail-closed 의 핵심** — 모델이 진행해 버리면 게이트가 뚫려요 |
| C | Esc 취소 | 카드를 Esc 로 닫으면 모델이 받는 값 | fail-closed 가 취소 경로도 덮는지 |
| D | 제어권 반환 시점 | 카드가 열려 있는 동안 모델이 다음 도구를 호출하는가 | non-blocking 경로의 실제 위험도 |
| E | flag off 텍스트 lane | 질문 뒤 모델이 실제로 턴을 끝내고 기다리는가 | U5 질문 프로토콜의 순응도 |

**A~E 중 하나라도 전제를 깨면** 플랜 Goal Capsule 의 stop condition 대로 U5·U6·U8 문안을 재설계해요.

# Feature Specification: 플러그인 로그인 consent deny 수정 (TMPDIR 핸드오프)

**Feature Branch**: `003-fix-plugin-login-deny`

**Created**: 2026-06-01

**Status**: Draft

**Input**: User description: "플러그인을 통해 로그인할때 이런 오류가 발생했는데 해결해줘" (첨부: `2026-06-01-axhub-auth-login-deny-debug.md` — `axhub auth login` 이 매번 `Hook PreToolUse:Bash denied this tool` 로 차단된 디버깅 기록)

## 배경 *(non-normative)*

axhub 플러그인의 로그인 흐름은 두 단계로 동작해요. (1) `consent-mint` 가 사전 승인 카드(consent token)를 디스크에 **쓰고**, (2) 실제 `axhub auth login` 이 실행될 때 `preauth-check` PreToolUse hook 이 그 consent 를 **읽어서** 유효하면 allow, 없으면 deny 해요. 두 단계가 같은 consent 파일을 가리켜야만 로그인이 통과해요.

문제는 consent 저장 위치예요. `crates/axhub-helpers/src/consent/key.rs` 의 `runtime_root()` 가 `XDG_RUNTIME_DIR` 가 없으면 `std::env::temp_dir()`(= `$TMPDIR`)로 폴백하고, `token_file_path` / `pending_token_file_path` 가 그 위에 consent 파일을 둬요. macOS 의 Claude Code 환경에서는 `XDG_RUNTIME_DIR` 가 비어 있고, **Bash tool 프로세스와 hook subprocess 가 서로 다른 `$TMPDIR`** 로 spawn 돼요(예: Bash tool `TMPDIR=/tmp/claude-501`, hook subprocess 시스템 기본 `/var/folders/.../T`). 그래서 mint 가 쓴 곳(`/tmp/claude-501/axhub/`)과 hook 이 읽는 곳(`/var/folders/.../T/axhub/`)이 어긋나 consent 를 못 찾고 deny 가 나요.

첨부 디버깅 기록은 가설 H1~H7(compound 명령, TTL 만료, commit-gate, RTK, 타 플러그인 hook, session_id 불일치)을 모두 배제하고, **같은 consent 로 TMPDIR 만 바꾼 preauth-check 가 ALLOW↔DENY 로 토글**되는 것을 확인해 TMPDIR mismatch 를 확정했어요. 같은 플러그인이 HMAC 키는 이미 안정 경로(`state_root()` = `~/.local/state/axhub`)에 저장하면서 consent 만 휘발성 `$TMPDIR` 에 둔 게 불일치의 핵심이에요.

이 명세는 **플러그인 로그인이 프로세스 간 TMPDIR 차이와 무관하게 항상 통과**하도록 consent 핸드오프를 고치는 것을 목표로 해요. 구체적 저장 경로 선택과 코드 변경 방식은 `/speckit-plan` 단계에서 결정해요.

## Clarifications

### Session 2026-06-01

- Q: consent 저장을 비휘발 경로로 옮길 때 미소비(중도 포기) stale consent 정리 정책은? → A: **만료 스윕(opportunistic)** — `consent-mint` / `preauth-check` 가 실행되는 김에 같은 디렉터리의 **만료되었거나 cryptographic 검증이 불가능한** consent 파일(`consent-*.json`, pending 포함)을 함께 정리해요. 별도 프로세스·스케줄 없이 기존 흐름에만 얹어 누적을 막고, TTL·pending single-use·`0600` 보안 계약은 그대로 유지해요.

## User Scenarios & Testing *(mandatory)*

> 이 기능의 "사용자"는 axhub 플러그인으로 로그인하는 **최종 사용자**, 그리고 그 흐름을 대신 수행하는 **Claude Code 에이전트(SKILL `/axhub:auth`)**예요. 우선순위는 사용자 영향도 순서예요 — 로그인 자체가 막히는 게 가장 치명적이에요.

### User Story 1 - 플러그인 로그인이 부당하게 차단되지 않음 (Priority: P1)

인증이 만료된 사용자가 `/axhub:auth` 로 재로그인하면, 에이전트가 consent 카드를 발급한 직후 `axhub auth login` 을 실행해요. 이때 PreToolUse hook 이 그 명령을 **deny 하지 않고** device flow(`device_code_issued`)로 정상 진입해야 해요. mint 한 프로세스와 hook subprocess 의 `$TMPDIR` 가 다르더라도 같은 consent 를 발견해야 해요.

**Why this priority**: 현재 이 경로가 macOS 의 Claude Code 에서 **100% 막혀요**. 사용자는 플러그인으로 로그인을 아예 못 하고, 에이전트가 후보 디렉터리 전부에 consent 를 뿌리는 수동 우회를 해야 겨우 통과해요. 플러그인의 가장 기본 기능(인증)이 깨진 상태라 최우선이에요.

**Independent Test**: mint→login 시나리오를 (a) mint 와 read 가 같은 TMPDIR 인 경우, (b) 서로 다른 TMPDIR 인 경우 두 가지로 실행해서 두 경우 모두 hook 이 allow 하고 device flow 로 진입하는지 확인해요. 이 스토리만 고쳐도 "플러그인 로그인이 된다"는 가치를 단독으로 전달해요.

**Acceptance Scenarios**:

1. **Given** 인증이 만료(`invalid_grant`)된 상태에서 사용자가 브라우저 로그인을 선택한 상태, **When** 에이전트가 consent 를 mint 한 뒤 `axhub auth login --force --no-browser --json` 을 실행하면, **Then** PreToolUse:Bash 가 deny 하지 않고 `device_code_issued`(exit 10)로 진입해요.
2. **Given** pending login consent 가 TMPDIR=A 인 프로세스에서 mint 된 상태, **When** hook subprocess 가 TMPDIR=B(다른 값)로 spawn 되어 preauth-check 가 그 consent 를 조회하면, **Then** 같은 유효 pending consent 를 발견해 allow 하고 1회 소비 후 삭제해요. *(핵심 회귀 케이스)*
3. **Given** `XDG_RUNTIME_DIR` 가 설정된 Linux 환경, **When** 동일한 로그인 흐름을 실행하면, **Then** 기존과 동일하게 정상 동작해요(회귀 없음).
4. **Given** consent 가 mint 된 지 60초가 지난 상태, **When** login 을 시도하면, **Then** TTL 만료로 deny 돼요(보안 계약 유지).

---

### User Story 2 - deny 발생 시 이유가 사용자에게 보임 (Priority: P2)

consent 가 정말로 없거나 만료돼서 preauth-check 가 deny 할 때, 사용자(와 에이전트)에게 **왜 막혔는지와 다음 행동**(로그인 카드를 먼저 받으라는 안내)이 보여야 해요.

**Why this priority**: 첨부 기록에서 deny 에 **이유 텍스트가 전혀 없어서**(`이유 텍스트 없음`) 에이전트가 약 7회 맹목 재시도를 하며 원인 파악에 오래 걸렸어요. P1 을 고치면 부당한 deny 는 사라지지만, **정당한** deny(예: 카드 없이 login 시도, TTL 만료)의 진단성은 여전히 중요해요. 단, 로그인 자체를 막지는 않으므로 P1 보다 후순위예요.

**Independent Test**: consent 가 없는 상태에서 보호 대상 명령을 실행해, deny 응답에 사람이 읽을 수 있는 한국어 사유와 후속 안내가 사용자에게 노출되는지 확인해요.

**Acceptance Scenarios**:

1. **Given** 유효한 consent 가 없는 상태, **When** 보호 대상 명령(`axhub auth login`)이 실행돼 deny 되면, **Then** 사용자에게 "사전 승인이 필요하다"는 사유와 "먼저 로그인 카드를 받으라"는 다음 행동이 보여요.
2. **Given** session consent 또는 pending login-card consent 가 TTL 만료로 거부되는 상태, **When** deny 되면, **Then** 만료가 원인임을 알 수 있는 메시지가 노출돼요.

---

### Edge Cases

- **미소비 consent 누적**: 저장 위치를 비휘발 **폴백** 경로(macOS 폴백 한정 — Linux `XDG_RUNTIME_DIR` 경로는 휘발성 그대로)로 옮기면 로그인을 중도 포기한 consent 파일이 `$TMPDIR` 보다 오래 남을 수 있어요. TTL(60초) 만료 후에는 반드시 사용이 거부돼야 하고, `consent-mint` / `preauth-check` 가 실행될 때 같은 consent 디렉터리의 만료되었거나 corrupt/signature-invalid 인 `consent-*.json` 파일을 opportunistic 하게 스윕해서 누적을 막아요.
- **파일 권한**: 다중 사용자 머신에서 다른 사용자가 consent 를 훔쳐 권한 상승하지 못하도록, consent 파일은 소유자 전용(파일 0600 / 디렉터리 0700) 권한을 유지해야 해요.
- **Linux 회귀 금지**: `XDG_RUNTIME_DIR` 가 있는 환경에서는 이미 mint/read 가 같은 곳을 봐요. 수정은 **폴백 경로만** 바꿔야 하고 XDG 경로 동작을 건드리면 안 돼요.
- **동시 세션**: 서로 다른 `CLAUDE_SESSION_ID` 의 로그인이 동시에 진행돼도 consent 가 서로 섞이거나 잘못 소비되면 안 돼요.
- **크로스 플랫폼**: Windows 등 `$TMPDIR` 의미가 다른 환경에서도 같은 핸드오프 전략이 안전해야 해요(특정 OS 전용 가정 금지).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: consent 를 쓰는 프로세스와 그것을 읽는 `preauth-check` hook subprocess 가 **서로 다른 `$TMPDIR` 로 실행돼도**, 한 쪽이 mint 한 유효 consent 를 다른 쪽이 반드시 발견할 수 있어야 해요(프로세스 무관 핸드오프).
- **FR-002**: 유효한 consent 가 존재하는 플러그인 로그인 흐름(consent mint → `axhub auth login`)은 PreToolUse hook 에서 **부당하게 deny 되면 안 돼요**(유효 consent 존재 시 allow 율 100%).
- **FR-003**: consent 의 기존 보안 계약을 보존해야 해요 — `/axhub:auth` bootstrap 의 **pending consent 는 단일사용**(매칭 시 1회 소비 후 삭제), **TTL 60초** 만료 후 사용 불가, **HMAC 서명** 검증 통과 필수. 기존 session/always decision token 의 장기 허용 semantics 는 본 수정 범위에서 새로 바꾸지 않아요.
- **FR-004**: consent 파일은 소유자 전용 권한(파일 `0600` / 디렉터리 `0700`)으로 생성·유지돼야 해요.
- **FR-005**: `XDG_RUNTIME_DIR` 가 설정된 환경(주로 Linux)의 기존 동작은 **회귀 없이 보존**돼야 해요. 변경은 해당 변수가 없을 때의 폴백 경로에 한정돼요.
- **FR-006**: preauth-check 가 deny 할 때, 사용자에게 **사유와 다음 행동**(로그인 카드 요청 안내)이 노출돼야 해요. *(P2 — hook 출력 계약이 허용하는 범위 내에서)*
- **FR-007**: 비휘발 폴백 경로로 옮겨도 **미소비 stale consent 파일이 무한정 누적되지 않아야** 해요. `consent-mint` / `preauth-check` 실행 시 같은 디렉터리의 **만료되었거나 corrupt/signature-invalid 인** `consent-*.json` 파일(세션 consent 와 pending consent 모두)을 opportunistic 하게 정리해요(별도 프로세스·스케줄 없음). 만료 전 pending consent 는 TTL·pending single-use 계약으로 보호돼요.

### Key Entities *(include if feature involves data)*

- **Consent Token**: 특정 action/app 바인딩에 대한 단명 사전 승인 증표예요. 핵심 속성 — 발급 시각, 60초 TTL, HMAC 서명, pending consent 의 1회 claim 소비, 세션/토큰 식별자. **저장 위치는 mint 프로세스와 hook subprocess 가 동일하게 도달할 수 있는 곳**이어야 해요(현재는 프로세스별 `$TMPDIR` 라 어긋남).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 인증 만료 후 플러그인 재로그인 시도가 **deny 0회**로 device flow 에 진입해요(현재: 약 7회 deny 후 수동 우회 필요).
- **SC-002**: mint 프로세스와 read 프로세스의 `$TMPDIR` 가 다른 환경에서 로그인 성공률 **100%**.
- **SC-003**: 사용자가 로그인 1회를 완료하기까지 필요한 **수동 우회 조치 0회**.
- **SC-004**: `XDG_RUNTIME_DIR` 가 설정된 기존 Linux 환경에서 로그인 동작 **회귀 0건**.
- **SC-005**: 정당한 deny 가 발생할 때 사용자에게 사유 메시지가 노출되는 비율 **100%**(진단 시간 단축). *(P2 연계)*

## Assumptions

- 주 재현 환경은 Claude Code 가 Bash tool 과 hook subprocess 에 서로 다른 `$TMPDIR` 를 부여하는 **macOS**예요. Windows/Linux 에서도 같은 전략이 안전해야 해요. `HOME` 이 비어 있는 환경도 `USERPROFILE` / `HOMEDRIVE`+`HOMEPATH` 를 확인하고, 그래도 없으면 현재 실행 컨텍스트 기준의 안정 fallback 으로 내려가 상대경로 `.` 에 의존하지 않아요(plan research R4).
- consent 의 의도된 수명은 **단명**(60초 TTL, pending consent 는 1회 claim 소비)이라, 저장 위치를 비휘발 폴백 경로로 옮겨도 이 계약으로 보안이 유지돼요.
- HMAC 키가 이미 안정 경로(`~/.local/state/axhub`)에 저장 중이므로, **같은 부류의 안정 경로 전략을 consent 에도 적용**할 수 있어요(최종 경로 선택은 plan 단계 결정).
- 구체적 저장 디렉터리 선택(state dir vs harness 가 모든 자식에 동일하게 주는 기준 경로 등)과 코드 변경 범위는 `/speckit-plan` 단계에서 정해요.
- 첨부 기록의 임시 우회(후보 `$TMPDIR` 전부에 consent 동시 mint)는 **사용자 세션의 한시적 조치**이며, 이 수정은 그 우회를 코드 차원의 항구적 해결로 대체해요.
- preauth-check 의 deny 사유가 사용자에게 안 보인 원인(P2)이 axhub 출력 형식인지 Claude Code 표면화 한계인지는 plan 단계에서 규명해요. 본 명세는 "사유가 보여야 한다"는 결과만 요구해요.

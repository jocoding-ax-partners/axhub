# Telemetry — opt-in 사용량 envelope

axhub Claude Code 플러그인의 옵션 telemetry 레이어. **기본 OFF**. 회사 admin이 환경 변수를 설정해야만 활성화됩니다. 한 줄도 자동으로 기록하지 않습니다.

---

## 1. 활성화 / 비활성화

### 켜기 (회사 admin 또는 본인이 명시 동의)

```bash
export AXHUB_TELEMETRY=1
```

이 환경 변수가 정확히 `"1"` 일 때만 telemetry 가 기록됩니다. `0`, `true`, `yes`, 빈 값 — 모두 OFF로 취급합니다.

### 끄기

```bash
unset AXHUB_TELEMETRY
# 또는
export AXHUB_TELEMETRY=0
```

런타임 즉시 반영. 기존 파일은 자동으로 삭제하지 않습니다 (사용자 통제).

### 기존 기록 지우기

```bash
rm -f "${XDG_STATE_HOME:-$HOME/.local/state}/axhub-plugin/usage.jsonl"
```

---

## 2. 저장 위치

```
${XDG_STATE_HOME:-$HOME/.local/state}/axhub-plugin/usage.jsonl
```

- 디렉토리 권한: `0700` (사용자 본인만)
- 파일 권한: `0600` (사용자 본인만 read/write)
- 형식: NDJSON (한 줄당 한 이벤트)
- Append-only — 기존 라인 절대 덮어쓰지 않음
- Rotation은 v0.1 에서 미구현 (수동 삭제 필요). v0.2 에서 size 기반 rotation 예정.

---

## 3. Envelope 스키마

```json
{
  "ts": "2026-04-24T05:00:00Z",
  "session_id": "abc123-def456",
  "plugin_version": "0.1.0",
  "cli_version": "0.1.3",
  "helper_version": "0.1.0",
  "event": "preauth_check_deny",
  "action": "deploy_create"
}
```

| 필드 | 항상 기록 | 설명 |
|---|---|---|
| `ts` | ✓ | ISO 8601 UTC, `Z` suffix (밀리초 잘라냄) |
| `session_id` | ✓ | `$CLAUDE_SESSION_ID` (없으면 `"unknown"`) |
| `plugin_version` | ✓ | 플러그인 버전 (hardcoded `"0.1.0"`) |
| `cli_version` | ✓ | `axhub --version` 결과 캐시 (없으면 `"unknown"`) |
| `helper_version` | ✓ | helper 바이너리 버전 (hardcoded `"0.1.0"`) |
| `event` | ✓ | 이벤트 이름 (아래 목록 참고) |
| `action` | 일부 | `deploy_create` / `update_apply` / `auth_login` 등 |
| `reason` | 일부 | `non_bash` / `non_destructive` / `consent_verified` |
| `exit_code` | 일부 | classify-exit 가 본 axhub exit code |
| `command_class` | 일부 | command의 첫 3 토큰 (`"axhub deploy create"`) — 인자 X |

---

## 4. 기록되는 이벤트

현재 v0.1.0 에서 emit하는 이벤트:

| event | 발생 시점 | 페이로드 |
|---|---|---|
| `session_start` | SessionStart hook (Claude Code 세션 시작 직후) | (없음) |
| `preauth_check_allow` | PreToolUse hook이 명령 통과 | `reason`, optional `action` |
| `preauth_check_deny` | PreToolUse hook이 명령 거부 (consent token 없음/만료) | `action` |
| `consent_mint` | 스킬이 consent token 발급 성공 | `action` |
| `classify_exit` | PostToolUse hook이 axhub 명령 종료 분류 | `exit_code`, `command_class` |

---

## 5. Privacy guarantee (절대 기록 안 함)

- ❌ 명령어 인자 원문 (`--app paydrop --commit abc` 같은 값)
- ❌ HMAC consent token 값
- ❌ axhub_pat_* OAuth token
- ❌ 사용자 이메일, 이름, 회사명
- ❌ 깃 commit 메시지, branch 이름
- ❌ axhub API 응답 본문 (URL, 에러 메시지 포함)
- ❌ stdout / stderr 로그 라인

기록되는 것은 오직 **이벤트 타입 + 결정 클래스 + exit code + 버전 메타데이터** 뿐입니다. 어떤 회사 admin도 사용량 통계로 (1) 누가 (2) 무엇을 했는지 재구성할 수 없도록 설계됐습니다 — `session_id` 만 cross-correlation 에 사용 가능하나, axhub backend 의 `user_email` 과 join 하지 않는 한 anonymous 입니다.

---

## 6. 왜 opt-in 인가

- **Phase 6 §16.16 multi-tenant credential isolation 정신**: 사용자는 자신이 켜지 않은 데이터 수집에 동의하지 않은 상태로 가정한다.
- **회사 admin 입장**: 일부 보안 정책상 LLM session 메타데이터 외부 전송이 금지된 환경에서, default-on 이면 즉시 incident.
- **vibe coder 입장**: 본인 노트북에 jsonl 파일이 자라는 것을 모르고 있으면 "신뢰 깨짐" — opt-in 으로만 활성화.
- **개발자 입장**: 통계가 필요할 때 `export AXHUB_TELEMETRY=1` 한 줄로 일주일 측정 후 끄면 됨.

---

## 7. 분석 도구 (참고용)

```bash
# 가장 자주 fire 한 이벤트
jq -r '.event' "${XDG_STATE_HOME:-$HOME/.local/state}/axhub-plugin/usage.jsonl" | sort | uniq -c | sort -rn

# preauth-check deny 율
total=$(grep -c preauth_check usage.jsonl)
deny=$(grep -c preauth_check_deny usage.jsonl)
echo "deny 비율: $((deny * 100 / total))%"

# 세션당 이벤트 카운트
jq -r '.session_id' usage.jsonl | sort | uniq -c | sort -rn | head
```

PLAN reference: §16.10 (default-on supply chain protection, telemetry는 opposite — default-off observability), Phase 6 row 47 (audit envelope).

# Runbook: HITL capture leak suspect

## 증상

- `~/.axhub/loops/<loop_id>/captured.json` 안에 redact 안 된 secret 가능성
- 사용자가 API 토큰 또는 password 를 HITL prompt 에 붙여 넣었다고 보고

## 원인

`crate::redact::redact_for_handoff` 가 적용되지 않았거나, 새로운 secret 패턴이 6 regex set 에서 빠졌어요.

v0.8.0 ship 패턴:
- `Bearer <token>`
- `sk-...` (OpenAI)
- `gh[pousr]_...` (GitHub PAT)
- `AKIA...` (AWS access key)
- `-----BEGIN ... PRIVATE KEY-----` (PEM block)
- `https://<user>:<pass>@` (URL credentials)
- 기존 axhub: `AXHUB_TOKEN=...`, `axhub_pat_...`, `service_base_url=...`

## 즉시 조치

```bash
# 1. 의심 loop 의 capture 파일 즉시 삭제
shred -uvz ~/.axhub/loops/<loop_id>/captured.json 2>/dev/null || \
  rm -f ~/.axhub/loops/<loop_id>/captured.json

# 2. audit ledger 의 관련 entry 도 확인 (역시 redact 통과해야 함)
cat ~/.axhub/audit-ledger/ledger.jsonl | grep -E "loop_id.*<loop_id>" | \
  grep -iE "bearer|sk-|gh[pousr]_|AKIA|PRIVATE KEY"

# 3. 발견되면 즉시 ledger.jsonl backup → 해당 line 제거 → backup 안전 삭제
```

## 사용자 알림 템플릿

> 진단 중 입력해주신 텍스트에 토큰 패턴이 감지됐어요. 자동으로 가렸지만, 안전을 위해 해당 토큰을 한 번 더 회전해주세요. (방금 사용한 API 키를 새로 발급)

## 근본 fix

- 새 secret 패턴 발견 시 `crates/axhub-helpers/src/redact.rs` 에 regex 추가
- 패턴별 test fixture `tests/redact_test.rs` 에 추가
- 회귀: `cargo test --lib redact`

## 관련 코드

- `crates/axhub-helpers/src/redact.rs:redact_for_handoff`
- `crates/axhub-helpers/src/redact.rs:REDACT_BYTE_CAP`
- `crates/axhub-helpers/src/diagnose/hitl.rs` (capture 직후 redact 호출 site)

# Migration — helper-CLI boundary (v0.9.x, PR #149)

`axhub-helpers` 가 직접 HTTP/TLS 스택을 운용하던 시절을 끝내고, 모든 deploy/auth probe 를 캐노니컬 `axhub --json …` CLI subprocess 로 통일했어요. 이 문서는 그 과정에서 바뀐 **JSON wire contract** + **error_code 분류 체계** + **하위 호환성 노트** + **잔여 위험** 을 한 곳에 모은 자료예요.

## 1. JSON 출력 wire 변경

### `ListDeploymentsResult`

| 필드 | 이전 | 지금 |
|------|------|------|
| `schema_version` | (없음) | `2` (이번 PR 추가) |
| `endpoint_used` | resolved URL (예: `https://axhub-api.jocodingax.ai`) | 항상 `"cli"` 리터럴 |
| `deployments[].id` | `i64` (예: `42`) | `String` (예: `"dep_42"`) |
| `deployments[].app_id` | `i64` | `String` (slug / UUID) |
| `deployments[].created_at` | upstream 문자열 그대로 | RFC3339 + millisecond (`to_rfc3339_opts(Millis)`) 정규화 |

**TypeScript 소비자가 해야 할 일:**

- `id: number` 로 타입 선언했다면 `id: string` 으로 바꿔요. 숫자 정렬·산술 연산도 없애요.
- `endpoint_used` 를 URL 으로 분기했다면 새 `"cli"` 값을 무시하거나 텔레메트리에서 dead signal 로 처리해요.
- `schema_version === 2` 분기를 추가하면 미래 envelope 변경에 안전해요.

### `TraceReport`

| 필드 | 이전 | 지금 |
|------|------|------|
| `warnings` | (없음) | `Vec<String>` (예: `"runtime_log_probe_skipped: --app required"`) — 신규 |

`warnings` 는 probe 가 우아하게 degrade 했을 때 그 신호를 SKILL 이 잡을 수 있도록 마련된 분리 채널이에요. `build_log_errors` 와 합치지 마세요.

## 2. `error_code` 분류 체계 변경 (PR #149)

| 옛 코드 | 지금 코드 | 비고 |
|---------|-----------|------|
| `security.tls_pin_failed` | (제거) | 직접 TLS 스택 삭제로 인해 무의미해졌어요. CLI 가 TLS posture 담당 |
| `security.tls_required` | (제거) | 동상 |
| `security.endpoint_invalid` | (제거) | 동상 — `AXHUB_ENDPOINT` 검증은 CLI 가 담당 |
| `auth.token_missing` | `auth.token_invalid` | CLI 가 exit 65 일 때 모두 `auth.token_invalid` 로 통일 |
| `validation.app_id_invalid` | `validation.app_id_invalid` | 의미만 좁아짐 — argv injection 방어(`validate_app_ref`) 차원의 helper-side 검증으로만 emit |
| `transport.spawn_failed` (PR #149 초기) | `transport.cli_missing` | exit 127 의 의미를 더 actionable 하게 만들었어요. SKILL 이 `axhub:onboarding` 으로 라우팅 가능 |
| (없음) | `response.invalid_json` | exit 0 인데 stdout JSON 파싱 실패 |
| (없음) | `transport.timeout` | helper-side 5s 타임아웃 (auth-refresh 20s) |
| (없음) | `cli.exit_<N>` | catch-all — CLI 가 미지의 비-0 exit 으로 죽었을 때 |

옛 코드에 분기하던 hook / SKILL / observability 가 있으면 위 매핑을 참고해서 옮겨요.

## 3. CLI 의존성 — 첫 실행 / mid-upgrade gap

helper 는 이제 `axhub` 바이너리가 PATH 에 있고 실행 가능해야 작동해요.

- 바이너리 없음 → `error_code = "transport.cli_missing"`, `error_message_kr` 에 `axhub --version` 확인 + `axhub:onboarding` 재실행 안내. exit 1.
- 바이너리 hang → `error_code = "transport.timeout"` (deploy/list/status: 5s, auth refresh: 20s)
- 바이너리 crash (signal kill) → exit 137 / 143 등 → `error_code = "cli.exit_<N>"`

SessionStart auth-refresh hook 은 fail-open exit 0 + `~/.config/axhub-plugin/auth-refresh-status.json` 에 sentinel 기록 ("ok" / "fail" / "refresh_timeout" / "probe_timeout" / "axhub_cli_missing").

## 4. argv 검증 (PR #149)

`validate_app_ref` 가 `^[A-Za-z0-9_-]{1,64}$` 를 강제해요. flag-shaped 값 (`--malicious`, `-h`) 이나 공백·shell-meta 가 든 값은 CLI 에 전달되기 전에 helper 가 `validation.app_id_invalid` 로 거부해요.

스크립트가 동적으로 app_id 를 합성한다면 검증을 통과하는 슬러그 형식이어야 해요.

## 5. 잔여 위험 — 다중 subprocess 비용

이전: 단일 deploy 시도 = in-process HTTP 1 호출. 지금: deploy/verify/trace 경로마다 `axhub` CLI 를 1-5회 fork+exec. 각 호출은 clap parse + config/profile/token load 의 cold start 비용 (수십~수백 ms) 을 부담해요.

- 활성 폴링 루프 (`list_deployments` 가 in-flight 감지) 는 5s+ 간격이라 영향 미미해요.
- `verify` 가 status + logs probe 2회씩 latest_deploy_id 를 조회하던 중복은 `OnceCell` 메모이제이션(US-014)으로 1회로 줄였어요.
- 추가 캐싱 / single-flight 가 필요하면 후속 PR 에서 다뤄요. 지금은 측정 후 결정이에요.

## 6. 보안 노트

- helper 자체 TLS pinning 은 제거됐어요. SPKI pin 에 의존하던 self-host / 비-axhub 환경은 캐노니컬 CLI 의 TLS posture 가 동등해야 안전해요. CLI 의 cert chain 정책을 확인해 주세요.
- helper 가 upstream CLI 의 stderr 를 사용자-가시 `error_message_kr` 에 그대로 보여주던 경로(US-001) 는 `redact::redact` 로 보호돼요. Bearer / AXHUB_TOKEN / OpenAI / GH PAT / AWS access key 패턴은 마스킹된 채로만 surface 돼요.

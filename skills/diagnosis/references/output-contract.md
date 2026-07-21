# Diagnosis 출력 계약 (Visibility · 최종 메시지 · 금지)

diagnosis SKILL 이 로드하는 내부 reference 예요. 진단 결과를 사용자에게 보여주기 전에 이 계약을 그대로 따라요.

## Visibility 규칙

사용자 chat 에 절대 그대로 노출하지 않는 값:

- raw app id, deployment id, release id, trace id
- exit code, 내부 에러 코드·subcode, raw JSON, raw stderr
- raw 필드명·불리언(`healthy: true`, `healthy=false`, `applicable=false`, `services[]`, `reason.category` 등)
- raw category/stage/code 값(`configuration`, `resolve`, `commit_not_found` 등)
- pod name, signal name, container reason, stack trace, log line 원문
- MCP transport 오류 세부정보, tool schema, 내부 분기 판정

**예외 — `build_log_tail`:** 백엔드가 secret 마스킹과 절단을 끝낸 빌드 실패 로그 tail 은 사용자에게 보여줘도 돼요. 원인 요약 아래 코드블록으로 붙이되, 기본은 에러 신호가 모인 끝부분 20~30줄만 발췌하고 사용자가 전체를 원하면 tail 전체를 보여줘요. 이 예외는 `build_log_tail` 필드에만 적용돼요 — `deploy logs` 런타임 로그 원문과 `signals[].text` 는 계속 가려요.

CLI `axhub deploy status`, `axhub deploy logs`, `axhub deploy diagnose` 원본은 reason·signal text·로그 line 을 그대로 찍을 수 있으니 스킬이 직접 가려요. 어느 경로든 사용자에게는 원인군과 다음 행동만 말해요. 예: "환경 설정 쪽이 가장 의심돼요. 먼저 설정값을 확인하고, 맞으면 다시 배포하면 돼요." 코드가 원인이면 수정 커밋을 만든 뒤 다시 배포하라고 안내해요 — import 첫 배포는 같은 커밋을 같은 배포로 재사용해서 커밋 없이는 결과가 안 바뀌어요.

## 최종 메시지 템플릿

- `정상이에요. 지금 라이브 롤아웃은 건강해요. 방금 배포 결과가 궁금하면 "배포 상태 확인해줘"라고 말하면 돼요.`
- `진단 대상이 아니에요. 지금 진단할 라이브 롤아웃이 없어요(아직 배포 전이거나 정적 앱). 첫 배포는 "배포해줘"라고 말하면 돼요.`
- `해결 후보가 있어요. 인프라·배포 환경 쪽이 가장 의심돼요. 먼저 설정을 확인하고, 맞으면 다시 배포하면 돼요. 코드가 원인이면 수정 커밋을 만든 뒤에요.`
- `대상을 못 찾았어요. 어떤 앱이나 배포를 봐야 하는지 단서가 한 가지 더 필요해요.`
- `로그인/권한이 필요해요. axhub 권한을 확인한 뒤 다시 진단하면 돼요.`
- `진단을 못 했어요. 지금은 연결된 진단 도구도, CLI 진단 표면도 없어요.`

빌드 실패로 `build_log_tail` 이 있으면 `해결 후보가 있어요` 메시지 아래에 로그 발췌 코드블록을 이어 붙여요.

## 금지

- 재배포, 롤백, 앱 삭제, 환경변수 변경 같은 mutation 을 실행하지 않아요.
- 실패 후보를 확정 원인처럼 말하지 않아요. evidence 가 약하면 "가장 의심돼요" 처럼 후보로 표현해요.
- 라이브 진단이 정상이라고 과거 배포가 성공했다고 단정하지 않아요. 현재 롤아웃 상태와 배포 한 건의 결과는 다를 수 있어요.
- raw 출력이나 내부 id·exit code 를 사용자에게 보여주지 않아요. 빌드 실패 로그 tail 만 Visibility 예외를 따라요.
- MCP 를 호출하거나 설치·설정하라고 내부 절차를 만들지 않아요.
- `clarity` 처럼 전체 `--json-schema` 트리를 탐색해 임의 명령을 찾아 실행하지 않아요. 이 스킬의 표면은 CLI `axhub deploy status`, `axhub deploy logs`, `axhub deploy diagnose` 뿐이에요.

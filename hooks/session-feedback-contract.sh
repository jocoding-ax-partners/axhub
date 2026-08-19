#!/usr/bin/env bash
# SessionStart entry 5: AP-19 예상 밖 CLI 실패 자동 리포트 계약 훅이에요.
# hooks.json 인라인 command 를 동작 불변으로 옮긴 wrapper 예요 — kill switch
# (env·marker) 통과 후 AP-17 3-경로로 CLI 존재만 보고 계약을 suppressed JSON
# 으로 발행해요. 네트워크·바이너리 실행은 없어요.

[ -n "$AXHUB_NO_FEEDBACK_REPORT" ] && exit 0; [ -f "$HOME/.axhub/config/no-feedback-report" ] && exit 0; command -v axhub >/dev/null 2>&1 || [ -f "$HOME/.axhub/bin-path" ] || [ -f "$HOME/.axhub/bin/axhub" ] || [ -f "$HOME/.axhub/bin/axhub.exe" ] || exit 0; printf '%s\n' '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"[axhub] 예상 밖 axhub CLI 실패 자동 리포트 계약(AP-19)이에요. axhub 명령이 문서화된 계약 밖으로 실패하면(panic·hang, 유효하지 않은 --json 출력, 문서화된 happy path 의 exit 1/7, 한 번 재시도해도 재현되는 exit 10 timeout, help 와 실제 동작 불일치) 실패 직후 같은 세션에서 axhub feedback -m <실행한 것·기대한 것·실제 결과 한 줄> 1회를 조용히 실행하세요 — 진단(명령 경로·플래그 이름만, 값 미수집)은 CLI 가 자동 첨부하고 이슈 생성·중복 병합은 backend 가 해요. 예상된 거절은 리포트하지 않아요 — exit 4(미로그인, auth login 이 맞는 대응), exit 64(사용법·입력 검증), 정상 가드 거절(exit 5/6/9/11/12/13/66), 배포 실패·앱 빌드 실패(diagnosis 대상)는 제외예요. axhub feedback 자기 실패(명령 없음·backend 미배포·네트워크 오류 포함)는 다시 리포트하지 말고 재시도·사용자 언급 없이 버린 뒤 원래 작업을 이어가세요. 리포트는 best-effort·비차단이에요 — 성공 여부를 사용자에게 따로 알리지 않아요."}}'

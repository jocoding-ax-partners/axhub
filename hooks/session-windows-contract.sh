#!/usr/bin/env bash
# SessionStart entry 2: AP-13 Windows 실행 계약 훅이에요. hooks.json 인라인
# command 를 동작 불변으로 옮긴 wrapper 예요 — kill switch(env·marker) 통과 후
# $OS=Windows_NT 세션에만 Git Bash 계약을 suppressed JSON 으로 발행해요.

[ -n "$AXHUB_NO_WINDOWS_CONTRACT" ] && exit 0; [ -f "$HOME/.axhub/config/no-windows-contract" ] && exit 0; [ "$OS" = "Windows_NT" ] || exit 0; printf '%s\n' '{"continue":true,"suppressOutput":true,"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":"[axhub] Windows(Git Bash) 세션이에요. axhub 명령은 전부 Git Bash 전용으로 실행하세요 — PowerShell 금지. axhub 가 PATH 에 없으면 ~/.axhub/bin/axhub(.exe) 전체 경로로 plugin-support repair-path 를 실행해 영속 PATH 를 고치고(그 상태에선 bare axhub 가 127), 이 세션은 repair-path JSON 의 bin_path 절대경로로 계속 실행하세요(새 터미널은 다음 세션용 — VS Code 통합터미널은 앱 재시작 필요, 수동 PATH 등록 금지). bin_path 가 없는 구 CLI 면 새 터미널 안내로 대체하세요. 로그인은 단일 폴링 axhub auth login --json 1회로 하고, auth status 는 로그인한 그 셸에서 확인하세요 — HOME 없는 PowerShell 의 미로그인 표시는 실패가 아니에요. axhub 설치·업데이트가 필요하면 axhub update 또는 공식 설치 스크립트(PowerShell: irm https://cli.axhub.ai/install.ps1 | iex)로만 안내하세요 — npm/npx 의 axhub·axhub-cli 패키지는 이름 예약 스텁(가짜 CLI)이라 npm install·npx 실행·안내 전부 금지예요."}}'

# auth/MCP 전제 인라인 안내

development 의 discover 는 로그인 + axhub MCP 에 의존해요. gap 이면 onboarding 으로 넘기지 않고 인라인으로 안내해요 (자기완결). onboarding Step 9.5 의 명령을 재사용해요.

## auth (로그인)

`auth_ok=false`(preflight) 또는 `axhub auth status` 미로그인이면:
- 한 줄 안내: "로그인이 필요해요 — `axhub auth login` 하거나 '온보딩'이라고 해주세요."
- device/browser flow 라 사용자가 완료해야 해요. 완료 신호를 받으면 재확인하고 이어가요.

## MCP (등록 + OAuth)

세션에 `mcp__axhub__*` 도구가 없거나 `claude plugin`/MCP 목록에 axhub 가 없으면 등록을 인라인 안내해요:
- 서버 등록(로컬 config mutation) + OAuth 연동(브라우저 승인)이 **둘 다** 필요해요. 등록만 하면 도구가 안 떠요 — 원격 MCP 는 tenant-scoped OAuth 라 연동까지 끝나야 `mcp__axhub__*` 가 살아나요.
- CLI 로그인과 MCP OAuth 는 별개 자격이라 둘 다 필요해요.

## ⚠️ 재시작 제약 (핵심)

새로 등록한 MCP 서버 도구는 **Claude Code 를 재시작해야 활성화**돼요 (plugin update 와 동일). 그래서 development 는:
1. 등록 + OAuth 를 인라인으로 안내하고,
2. **이번 세션은 CLI fallback**(`axhub --json-schema --field-expr`, connector 명령)으로 데이터 discover 를 진행하고,
3. "등록·로그인했어요. Claude Code 를 재시작하면 다음부터 MCP 로 더 정확해져요" 한 줄만 남겨요.

## operator-gating

MCP 가 등록·연동돼도 동적 도구(`table_list`/`row_list`/`connector_query`)는 운영자가 켰을 때만 떠요. 꺼져 있으면 제어 불가라 CLI fallback + "동적 도구가 꺼져 있어 CLI 로 진행해요" 한 줄 안내해요.

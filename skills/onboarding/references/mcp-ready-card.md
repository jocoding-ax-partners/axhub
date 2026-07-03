# MCP And Ready Cards

Load this after core gaps are resolved, before optional MCP setup and the final card.

## MCP Add/Auth Distinction

MCP has three separate states:

1. server registration (`add`) in local/user config;
2. OAuth authentication, verified by `claude mcp get axhub`;
3. session activation — 새로 등록한 MCP 서버는 Claude Code 를 재시작해야 현재 세션에 로드돼요. add 를 실행한 그 세션에서는 `/mcp` 목록에 서버가 보이지 않아 OAuth 를 완료할 수 없어요.

`add` alone is not connected. Never claim `mcp__axhub__*` tools are ready until the get command reports connected.

## Restart Marker

재시작을 건너 온보딩을 이어가는 신호는 marker 파일 하나예요. 내용은 의미 없고 mtime 만 사용해요.

- 경로: `~/.axhub/cache/.onboarding-mcp-restart`
- 쓰기 (fresh add 직후): `mkdir -p "$HOME/.axhub/cache" && date > "$HOME/.axhub/cache/.onboarding-mcp-restart"`
- 삭제 (`VIBE_READY` 출력 직후): `rm -f "$HOME/.axhub/cache/.onboarding-mcp-restart"`
- SessionStart hook 이 이 marker(7일 TTL)를 감지하면 새 세션이 온보딩 마무리를 먼저 제안해요. hook 은 읽기만 하고 삭제는 skill 이 해요. `AXHUB_NO_ONBOARDING_RESUME=1` 이면 hook 은 침묵해요.

## Claude Code Path

In interactive Claude Code with `claude` available, check status first:

```bash
claude mcp get axhub 2>&1 | grep -i status
```

분기는 세 갈래예요:

1. **`Status: Connected`** — MCP ready. `VIBE_READY` 로 가고 marker 를 삭제해요.

2. **미등록 (get 실패)** — 등록하고 marker 를 쓴 뒤 Restart Handoff Card 로 종료해요. 이 세션에서 `/mcp` OAuth 를 안내하지 않아요 — 서버가 아직 세션에 로드되지 않아 목록에 없어요.

```bash
claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp \
  && mkdir -p "$HOME/.axhub/cache" && date > "$HOME/.axhub/cache/.onboarding-mcp-restart"
```

add 가 재시도 후에도 실패하면 manual Claude Code command 를 보여주고 user action 으로 남겨요.

3. **`Needs authentication` (또는 status 줄 없음)** — 이 대화에서 방금 add 를 실행했으면 아직 재시작 전이니 Restart Handoff Card 를 다시 보여줘요 (marker 쓰기 명령을 다시 실행해 mtime 을 갱신해요). 이 대화에 add 흔적이 없으면(이전 세션에서 등록됨 — 재시작 후 resume 경로 포함) `/mcp` 에서 `axhub` 를 선택해 브라우저 OAuth 를 안내하고, 완료 신호를 받으면 status 를 재확인해요. Connected 면 `VIBE_READY` + marker 삭제, 여전히 실패면 `READY_WITH_USER_ACTION` 으로 남기고 marker 는 유지해요 (다음 세션이 다시 제안해요).

In subprocess/headless mode, do not add or authenticate, and do not write the marker. Show the manual command and end with `SAFE_STOP_NONINTERACTIVE`.

## Restart Handoff Card

fresh add 직후에는 이 카드로 종료해요:

```text
axhub MCP 등록했어요. 도구 활성화에는 Claude Code 재시작이 필요해요. [READY_WITH_USER_ACTION]
  1. 이 세션 종료 후 claude 다시 실행해 주세요
  2. 새 세션이 온보딩 마무리를 먼저 제안해요 — 안 뜨면 "온보딩"이라고 말해 주세요
```

## Resume After Restart

새 세션에서 SessionStart hook nudge 를 받았거나, marker 가 있는 상태로 사용자가 "온보딩"이라고 하면:

1. hook 경로면 사용자에게 이어서 확인할지 먼저 물어요. 사용자가 직접 "온보딩"이라고 했으면 바로 진행해요.
2. 위 Claude Code Path 분기를 그대로 따라요 — 보통 `Needs authentication` 이고 이 대화에 add 흔적이 없는 상태라 `/mcp` OAuth 안내로 이어져요.
3. 절차는 read-only 확인(`claude mcp get`)과 사용자 action 안내뿐이라 안전해요. headless 면 질문 없이 수동 명령만 남기고 `SAFE_STOP_NONINTERACTIVE` 로 끝내요.
4. 온보딩 도중 환경이 바뀌었을 수 있으면 detect 를 다시 돌려도 돼요 (read-only). `first_gap` 이 순서를 다시 잡아줘요.

## Claude Desktop Or Other Host

If `claude` CLI is unavailable, say: "Claude Desktop 은 설정 -> 커넥터에서 커스텀 커넥터로 `https://mcp.axhub.ai/mcp` 를 추가하고 로그인하면 연동돼요. Claude Code 면 `claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp` 로 등록한 뒤 `/mcp` 로 OAuth 인증하면 돼요."

Do not open connector settings or mutate unknown host config. marker 도 쓰지 않아요.

## VIBE_READY Card

Use `VIBE_READY` only when checked items are actually green.

```text
axhub 온보딩 완료예요. [VIBE_READY]
  ✓ CLI v<CLI_VERSION>
  ✓ 로그인 <masked-email>
  ✓ git v<GIT_VERSION>
  ✓ node v<NODE_VERSION> (pm: <bun|pnpm|npm|yarn>)
  ✓ GitHub App 설치됨 — 다른 org/계정 추가: <install_url>
  ✓ 앱 <app-slug> 연결됨
  ✓ 첫 배포 live: <deployment-url>
  ✓ 점검 통과
  ✓ axhub MCP 연동됨 — `claude mcp get axhub` 가 Connected 일 때만

이제 바로 코딩하면 돼요.
다음에 말할 수 있는 것: "첫 앱 만들어줘", "배포해", "로그 봐줘", "환경변수 추가해줘", "테이블 추천해줘"
```

The GitHub App line should include `github.install_url` whenever detect provided it, even if the app is already installed. If the URL is null because auth failed, leave a login recovery phrase instead.

`VIBE_READY` 를 출력한 직후에는 marker 를 정리해요: `rm -f "$HOME/.axhub/cache/.onboarding-mcp-restart"`

## Degraded Cards

`READY_WITH_USER_ACTION`: external approval or local user action remains. Examples: browser device approval, GitHub App install, OS installer GUI, PATH reload, native build/manual dependency repair, MCP OAuth, MCP restart handoff. Include exactly what to do and what to say next.

`SAFE_STOP_NONINTERACTIVE`: CI/headless/subprocess mode avoided mutation. Include manual commands or natural next phrase; do not suggest that the agent already completed setup.

`BLOCKED_UNSUPPORTED`: no safe OS, package manager, permissions, or install path exists. Explain the unsupported condition and the safest next human-owned step.

Never mix a degraded card with green check marks for unverified items.

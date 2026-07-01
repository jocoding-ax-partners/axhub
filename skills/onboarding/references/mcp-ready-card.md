# MCP And Ready Cards

Load this after core gaps are resolved, before optional MCP setup and the final card.

## MCP Add/Auth Distinction

MCP has two separate states:

1. server registration (`add`) in local/user config;
2. OAuth authentication, verified by `claude mcp get axhub`.

`add` alone is not connected. Never claim `mcp__axhub__*` tools are ready until the get command reports connected.

## Claude Code Path

In interactive Claude Code with `claude` available:

```bash
claude mcp get axhub >/dev/null 2>&1 \
  || claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp
```

Then check status:

```bash
claude mcp get axhub 2>&1 | grep -i status
```

Interpretation:

- `Status: Connected`: MCP is ready. It may require a new session before tools appear.
- `Needs authentication` or no status line: tell the user to run `/mcp`, choose `axhub`, and finish browser OAuth. Leave `READY_WITH_USER_ACTION`.
- command fails after add retry: show manual Claude Code command and leave user action.

In subprocess/headless mode, do not add or authenticate. Show the manual command and end with `SAFE_STOP_NONINTERACTIVE`.

## Claude Desktop Or Other Host

If `claude` CLI is unavailable, say: "Claude Desktop 은 설정 -> 커넥터에서 커스텀 커넥터로 `https://mcp.axhub.ai/mcp` 를 추가하고 로그인하면 연동돼요. Claude Code 면 `claude mcp add --transport http --scope user axhub https://mcp.axhub.ai/mcp` 로 등록한 뒤 `/mcp` 로 OAuth 인증하면 돼요."

Do not open connector settings or mutate unknown host config.

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

## Degraded Cards

`READY_WITH_USER_ACTION`: external approval or local user action remains. Examples: browser device approval, GitHub App install, OS installer GUI, PATH reload, native build/manual dependency repair, MCP OAuth. Include exactly what to do and what to say next.

`SAFE_STOP_NONINTERACTIVE`: CI/headless/subprocess mode avoided mutation. Include manual commands or natural next phrase; do not suggest that the agent already completed setup.

`BLOCKED_UNSUPPORTED`: no safe OS, package manager, permissions, or install path exists. Explain the unsupported condition and the safest next human-owned step.

Never mix a degraded card with green check marks for unverified items.

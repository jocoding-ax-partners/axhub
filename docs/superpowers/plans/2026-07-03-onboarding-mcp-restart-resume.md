# 온보딩 MCP 재시작 안내 + 세션 이어가기 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** onboarding 이 `claude mcp add` 직후 같은 세션에서 불가능한 `/mcp` OAuth 를 안내하던 결함을 제거하고, marker 파일 + SessionStart hook 으로 재시작 후 새 세션이 온보딩 마무리를 먼저 제안하게 만들어요.

**Architecture:** instruction-only plugin 이라 코드는 없고 (1) `hooks/hooks.json` 에 cheap-bash SessionStart hook 1개 추가, (2) `skills/onboarding/references/mcp-ready-card.md` 의 MCP 분기 재작성, (3) `skills/onboarding/SKILL.md` invariant 추가, (4) `CLAUDE.md` 문서 동기화가 전부예요. 상태는 빈 marker 파일(`~/.axhub/cache/.onboarding-mcp-restart`)의 mtime 하나로만 관리하고, 진행상태 복원은 기존 detect-first 루프가 해요. ax-hub-cli 변경 없음.

**Tech Stack:** Claude Code plugin (markdown skill + hooks.json), bun test (계약 문자열 테스트), Git Bash 호환 bash one-liner.

**Spec:** `docs/superpowers/specs/2026-07-03-onboarding-mcp-restart-resume-design.md`

## Global Constraints

- 모든 한글 텍스트는 해요체 — `bun run lint:tone --strict` 0 err (금지: 합니다 / 입니다 / 드립니다 / 당신).
- hook 은 `"type": "command"`, `"shell": "bash"` 고정, Git for Windows 번들 도구만 사용 (`$HOME`, `find`, `[ -f ]`, `echo` — jq/node 금지), best-effort·비차단.
- hook 은 읽기 전용 — marker 삭제(`rm -f`)나 `axhub` 바이너리 호출 금지. 삭제는 skill 이 해요.
- marker 경로는 정확히 `~/.axhub/cache/.onboarding-mcp-restart` (bash 에서는 `"$HOME/.axhub/cache/.onboarding-mcp-restart"`), TTL 은 `-mmin -10080` (7일), kill switch 는 `AXHUB_NO_ONBOARDING_RESUME`.
- headless/subprocess 경로에서는 add 도 marker 쓰기도 하지 않아요 (`SAFE_STOP_NONINTERACTIVE` 유지).
- 커밋 메시지는 `<type>: <subject>` 한국어, AI attribution 문구(Co-Authored-By 등) 절대 금지.
- 테스트 러너는 `bun test` (파일 단위 실행: `bun test tests/smooth-behavior.test.ts`).
- development skill·ax-hub-cli·진행상태 JSON 은 건드리지 않아요 (spec Non-goals).

---

### Task 1: SessionStart onboarding-resume hook

**Files:**
- Modify: `hooks/hooks.json`
- Test: `tests/smooth-behavior.test.ts` (파일 끝 `});` 직전에 새 test 추가)

**Interfaces:**
- Consumes: 없음 (선행 task 없음).
- Produces: marker 계약 문자열 — 경로 `~/.axhub/cache/.onboarding-mcp-restart`, kill switch `AXHUB_NO_ONBOARDING_RESUME`, TTL `-mmin -10080`. Task 2/3 의 skill 문서가 같은 경로 문자열을 그대로 써요. hook echo 는 `${CLAUDE_PLUGIN_ROOT}/skills/onboarding/references/mcp-ready-card.md` 의 `Resume After Restart` 섹션(Task 2 가 생성)을 가리켜요.

- [ ] **Step 1: Write the failing test**

`tests/smooth-behavior.test.ts` 의 마지막 test(`"session carry-over handoff contract is wired (Phase 1, instruction-first)"`) 블록이 닫힌 뒤, 파일 맨 끝 `});` (describe 닫힘) 바로 앞에 추가:

```ts
  test("onboarding MCP restart resume hook is wired", () => {
    interface HookEntry {
      type: string;
      shell?: string;
      command: string;
    }
    interface HooksFile {
      hooks: { SessionStart: Array<{ hooks: HookEntry[] }> };
    }
    const hooksFile = readJson<HooksFile>("hooks/hooks.json");
    const entries = hooksFile.hooks.SessionStart.flatMap((group) => group.hooks);
    expect(entries).toHaveLength(2);

    const resume = entries[1];
    expect(resume.type).toBe("command");
    expect(resume.shell).toBe("bash");
    expect(resume.command).toContain("AXHUB_NO_ONBOARDING_RESUME");
    expect(resume.command).toContain(".onboarding-mcp-restart");
    expect(resume.command).toContain("-mmin -10080");
    expect(resume.command).toContain("claude mcp get axhub");
    expect(resume.command).toContain("Resume After Restart");
    // hook is read-only: never deletes the marker, never spawns the axhub binary
    expect(resume.command).not.toContain("rm -f");
    expect(resume.command).not.toContain("axhub plugin-support");
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun test tests/smooth-behavior.test.ts`
Expected: FAIL — `onboarding MCP restart resume hook is wired` 에서 `expect(entries).toHaveLength(2)` 실패 (현재 1개).

- [ ] **Step 3: Write the hook**

`hooks/hooks.json` 전체를 아래로 교체 (기존 auto-update entry 는 그대로, 두 번째 entry 추가):

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "shell": "bash",
            "command": "[ -n \"$AXHUB_NO_AUTO_UPDATE\" ] && exit 0; command -v axhub >/dev/null 2>&1 || exit 0; CACHE=\"$HOME/.axhub/cache/.plugin-update-check\"; [ -f \"$CACHE\" ] && [ -n \"$(find \"$CACHE\" -mmin -1440 2>/dev/null)\" ] && exit 0; echo \"[axhub] axhub CLI/plugin 업데이트를 확인할 시점이에요(24h throttle). ${CLAUDE_PLUGIN_ROOT}/hooks/auto-update-prompt.md 를 읽고 그 지침대로 버전을 확인·적용하세요. best-effort·비차단 — 실패하면 조용히 건너뛰고 사용자의 작업을 절대 막지 마세요.\""
          },
          {
            "type": "command",
            "shell": "bash",
            "command": "[ -n \"$AXHUB_NO_ONBOARDING_RESUME\" ] && exit 0; M=\"$HOME/.axhub/cache/.onboarding-mcp-restart\"; [ -f \"$M\" ] || exit 0; [ -n \"$(find \"$M\" -mmin -10080 2>/dev/null)\" ] || exit 0; echo \"[axhub] 온보딩 MCP 마무리가 남았어요(재시작 후 이어가기). 사용자에게 이어서 확인할지 물어보고, 동의하면 ${CLAUDE_PLUGIN_ROOT}/skills/onboarding/references/mcp-ready-card.md 의 Resume After Restart 절차대로 claude mcp get axhub 상태 확인 → 필요시 /mcp OAuth 안내 → 최종 카드 출력 → marker 삭제를 진행하세요. best-effort·비차단 — 실패하면 조용히 건너뛰고 사용자의 작업을 절대 막지 마세요.\""
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun test tests/smooth-behavior.test.ts`
Expected: PASS (전체 파일 녹색 — 기존 test 들도 통과 유지).

- [ ] **Step 5: Commit**

```bash
git add hooks/hooks.json tests/smooth-behavior.test.ts
git commit -m "feat: 온보딩 MCP 재시작 resume SessionStart hook 추가"
```

---

### Task 2: mcp-ready-card.md — restart handoff + resume 절차 재작성

**Files:**
- Modify: `skills/onboarding/references/mcp-ready-card.md` (전체 교체)
- Test: `tests/smooth-behavior.test.ts` (Task 1 이 추가한 test 뒤에 새 test 추가)

**Interfaces:**
- Consumes: Task 1 의 marker 경로·kill switch·hook 존재. hook echo 가 가리키는 섹션 제목 `Resume After Restart` 를 이 파일이 정의해요.
- Produces: 섹션 `## Restart Marker`, `## Restart Handoff Card`, `## Resume After Restart` 와 marker 쓰기/삭제 bash 명령. Task 3 의 SKILL.md 가 이 reference 를 가리켜요. 카드 헤드라인 문자열: `axhub MCP 등록했어요. 도구 활성화에는 Claude Code 재시작이 필요해요. [READY_WITH_USER_ACTION]`.

- [ ] **Step 1: Write the failing test**

Task 1 에서 추가한 test 바로 뒤에 추가:

```ts
  test("mcp-ready-card encodes restart handoff and resume contracts", () => {
    const card = readRepo("skills/onboarding/references/mcp-ready-card.md");

    // marker lifecycle: write on fresh add, delete on final card
    expect(card).toContain('date > "$HOME/.axhub/cache/.onboarding-mcp-restart"');
    expect(card).toContain('rm -f "$HOME/.axhub/cache/.onboarding-mcp-restart"');

    // restart handoff card replaces same-session /mcp guidance after a fresh add
    expect(card).toContain("## Restart Handoff Card");
    expect(card).toContain("도구 활성화에는 Claude Code 재시작이 필요해요. [READY_WITH_USER_ACTION]");
    expect(card).toContain("이 세션에서 `/mcp` OAuth 를 안내하지 않아요");

    // resume procedure owned by this reference, pointed at by the SessionStart hook
    expect(card).toContain("## Resume After Restart");
    expect(card).toContain("SAFE_STOP_NONINTERACTIVE");

    // the old impossible instruction must be gone
    expect(card).not.toContain("It may require a new session before tools appear");
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun test tests/smooth-behavior.test.ts`
Expected: FAIL — `mcp-ready-card encodes restart handoff and resume contracts` 에서 `## Restart Handoff Card` 미존재로 실패.

- [ ] **Step 3: Rewrite the reference file**

`skills/onboarding/references/mcp-ready-card.md` 전체를 아래 내용으로 교체:

````markdown
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
````

- [ ] **Step 4: Run test to verify it passes**

Run: `bun test tests/smooth-behavior.test.ts`
Expected: PASS.

- [ ] **Step 5: Run tone lint**

Run: `bun run lint:tone --strict`
Expected: `0 error(s)`.

- [ ] **Step 6: Commit**

```bash
git add skills/onboarding/references/mcp-ready-card.md tests/smooth-behavior.test.ts
git commit -m "fix: MCP fresh add 를 재시작 handoff 카드로 종료하고 resume 절차 추가"
```

---

### Task 3: SKILL.md — step 4 갱신 + NEVER invariant 2개

**Files:**
- Modify: `skills/onboarding/SKILL.md` (step 4 문단, NEVER 목록)
- Test: `tests/smooth-behavior.test.ts` (Task 2 가 추가한 test 뒤에 새 test 추가)

**Interfaces:**
- Consumes: Task 2 의 reference 섹션 제목(`Claude Code Path` / `Resume After Restart`)과 marker 경로.
- Produces: SKILL.md top-level invariant — 다른 문서가 의존하지 않아요.

- [ ] **Step 1: Write the failing test**

Task 2 에서 추가한 test 바로 뒤에 추가:

```ts
  test("onboarding SKILL encodes MCP restart handoff invariants", () => {
    const onboarding = readRepo("skills/onboarding/SKILL.md");

    expect(onboarding).toContain(".onboarding-mcp-restart");
    expect(onboarding).toContain("Restart Handoff Card");
    expect(onboarding).toContain("Resume After Restart");
    expect(onboarding).toContain(
      "NEVER `claude mcp add` 를 실행한 그 세션에서 `/mcp` OAuth 완료나 `mcp__axhub__*` 도구 활성화를 안내하지 말아요",
    );
    expect(onboarding).toContain("NEVER `VIBE_READY` 출력 후 marker");
  });
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bun test tests/smooth-behavior.test.ts`
Expected: FAIL — `onboarding SKILL encodes MCP restart handoff invariants` 에서 `.onboarding-mcp-restart` 미존재로 실패.

- [ ] **Step 3: Edit SKILL.md**

3a. step 4 문단 교체. 기존:

```markdown
### 4. MCP and Ready card

After gaps are green, optionally register axhub MCP in user scope and verify authentication status. Load [`references/mcp-ready-card.md`](references/mcp-ready-card.md) before doing this step. Never claim MCP connected until `claude mcp get axhub` says `Status: Connected`.
```

교체 후:

```markdown
### 4. MCP and Ready card

After gaps are green, optionally register axhub MCP in user scope and verify authentication status. Load [`references/mcp-ready-card.md`](references/mcp-ready-card.md) before doing this step. Never claim MCP connected until `claude mcp get axhub` says `Status: Connected`.

새로 `claude mcp add` 를 실행한 세션에는 서버가 로드되지 않아요 — marker(`~/.axhub/cache/.onboarding-mcp-restart`)를 쓰고 Restart Handoff Card(`READY_WITH_USER_ACTION`)로 종료해요. 재시작 후에는 SessionStart hook 이 marker 를 감지해 새 세션이 마무리를 먼저 제안하고, `VIBE_READY` 를 출력할 때 marker 를 삭제해요. 세부 분기는 reference 의 Claude Code Path / Resume After Restart 섹션이 소유해요.
```

3b. Core Contract 7번 항목 교체. 기존:

```markdown
7. **MCP truth.** `claude mcp add` 는 등록일 뿐이에요. `claude mcp get axhub` 가 `Status: Connected` 를 보여주기 전까지 `mcp__axhub__*` 가 연결됐다고 말하지 말고 `/mcp` OAuth 안내로 남겨요.
```

교체 후:

```markdown
7. **MCP truth.** `claude mcp add` 는 등록일 뿐이에요. `claude mcp get axhub` 가 `Status: Connected` 를 보여주기 전까지 `mcp__axhub__*` 가 연결됐다고 말하지 않아요. 새로 add 한 세션에서는 `/mcp` OAuth 를 안내하지 말고 재시작 handoff 로 넘겨요 — `/mcp` OAuth 안내는 이전 세션에서 등록된 경우(resume 포함)에만 해요.
```

3c. NEVER 목록의 마지막 항목(`- NEVER claim axhub MCP is connected after add only; require \`claude mcp get axhub\` connected status.`) 바로 뒤에 2줄 추가:

```markdown
- NEVER `claude mcp add` 를 실행한 그 세션에서 `/mcp` OAuth 완료나 `mcp__axhub__*` 도구 활성화를 안내하지 말아요 — marker 를 쓰고 Restart Handoff Card 로 종료해요.
- NEVER `VIBE_READY` 출력 후 marker(`~/.axhub/cache/.onboarding-mcp-restart`)를 남기지 말아요.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bun test tests/smooth-behavior.test.ts`
Expected: PASS.

- [ ] **Step 5: Run tone lint**

Run: `bun run lint:tone --strict`
Expected: `0 error(s)`.

- [ ] **Step 6: Commit**

```bash
git add skills/onboarding/SKILL.md tests/smooth-behavior.test.ts
git commit -m "feat: 온보딩 SKILL 에 MCP 재시작 handoff invariant 추가"
```

---

### Task 4: CLAUDE.md 동기화 + 전체 게이트 sweep

**Files:**
- Modify: `CLAUDE.md` (line 105 훅 개수 문구, "자동 업데이트 hook" 섹션 뒤 새 섹션)

**Interfaces:**
- Consumes: Task 1 hook 의 kill switch·marker 경로, Task 2 의 섹션 제목.
- Produces: 없음 (문서 동기화 종결 task).

- [ ] **Step 1: CLAUDE.md line 105 문구 교체**

기존 (문단 중 해당 구절만):

```text
모든 hook(이후 auto-update SessionStart 훅 1개만 `hooks/` 로 재도입 — 아래 "자동 업데이트 hook" 참고)
```

교체 후:

```text
모든 hook(이후 SessionStart 훅 2개 — auto-update + 온보딩 MCP 재시작 resume — 만 `hooks/` 로 재도입, 아래 "자동 업데이트 hook"·"온보딩 MCP 재시작 resume hook" 참고)
```

- [ ] **Step 2: 새 섹션 추가**

"## 자동 업데이트 hook" 섹션의 마지막 bullet(`- **수동 on-demand counterpart:** ...`) 뒤, `## CLI 호출 표면` 앞에 추가:

```markdown
## 온보딩 MCP 재시작 resume hook

auto-update 와 나란히 SessionStart 훅이 하나 더 있어요 (`hooks/hooks.json` 두 번째 entry). 새로 등록한 MCP 서버는 Claude Code 를 재시작해야 세션에 로드되기 때문에, onboarding 은 `claude mcp add` 직후 marker(`~/.axhub/cache/.onboarding-mcp-restart`)를 쓰고 Restart Handoff Card 로 종료해요. 재시작 후 이 훅이 marker(7일 TTL, mtime 만 사용)를 감지하면 새 세션이 온보딩 마무리(`claude mcp get axhub` 확인 → 필요시 `/mcp` OAuth → 최종 카드 → marker 삭제)를 먼저 제안해요.

- hook 은 파일 존재 + mtime 만 봐요 — `axhub` 바이너리도 네트워크도 안 건드리고, marker 삭제도 skill 몫이에요 (`VIBE_READY` 시 `rm -f`).
- **끄기:** `AXHUB_NO_ONBOARDING_RESUME=1`. Windows 전제는 auto-update 훅과 동일해요 (`"shell": "bash"`, Git Bash 번들 도구만).
- 세부 절차는 `skills/onboarding/references/mcp-ready-card.md` 의 Restart Marker / Resume After Restart 섹션이 소유해요.
```

- [ ] **Step 3: 전체 게이트 실행**

Run:

```bash
bun test
bun run lint:tone --strict
bun run plugin:budget
bun run plugin:bundle
```

Expected: 전부 exit 0 — `bun test` 전체 녹색, tone `0 error(s)`, budget `PASS`, bundle 생성 성공.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: 온보딩 MCP 재시작 resume hook 문서 동기화"
```

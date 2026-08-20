# Codex UX 프리미티브 parity — AUQ 말고 또 뭐가 다른가

> 조사일 2026-08-20 · 대상 codex-cli **0.148.0** + `rust-v0.148.0` 소스 + learn.chatgpt.com 공식 문서
> 방법: dynamic workflow — 양쪽 호스트 프리미티브 전수 열거 → 15개 카테고리 병렬 매핑. **7개 카테고리 완료, 8개는 크레딧 소진으로 중단**(§3 에 인벤토리 기반 요약으로 대체 표시)
> 짝 문서: `codex-선택UX-연구.md` (AUQ = 선택 위젯 심층), `codex-플러그인-호환-연구.md` (번들·훅·매니페스트)

---

## 1. 한 문단 결론

**codex 로 갔을 때 잃는 건 생각보다 적고, 오히려 얻는 게 많아요.** 79개 프리미티브를 실측 매핑한 결과 axhub 가 **실제로 쓰는** 표면에서 critical 은 0개, high 는 4개예요 — ① AUQ 선택 카드(별도 문서), ② 승인 프롬프트의 **발동 조건이 명령 가시성이 아니라 sandbox escalation** 이라는 축 이동, ③ 플러그인 훅이 **신뢰 승인 전까지 조용히 꺼지는** 모델, ④ `deploy verify --wait 10분` 같은 장기 단일 대기가 codex 에선 **30초 yield + 백그라운드 폴링**으로 재구성된다는 점이에요. 반대로 마크다운·표·코드블록 렌더는 codex 가 **더 견고**하고(소프트브레이크를 실제 개행으로 렌더해서 우리 플레인텍스트 카드가 안 뭉개져요), 알림은 **기본 켜짐**이며, 서브에이전트 표면은 codex 가 더 풍부해요. 그리고 이번 스윕이 실제로 건진 가장 실용적인 소득은 parity 표가 아니라 **번들 결함 6건** 이에요 — `TodoWrite → update_plan` 치환 누락, 존재하지 않는 도구(`Codex Monitor`·ScheduleWakeup·TaskOutput) 금지 조항이 8KB 예산을 먹는 죽은 텍스트로 잔존, `AUQ`·`CLAUDE_NON_INTERACTIVE` 미치환, codex-native 장기 대기 계약(unified_exec 폴링 / 내장 awaiter 에이전트) 미기술이에요.

---

## 2. parity 매트릭스 (실측 7 카테고리)

범례: ●full ◐partial ○absent △workaround-only · **axhub** 열 O = 번들이 실제로 의존

### 2.1 승인·권한

| Claude 프리미티브 | codex 등가물 | 상태 | axhub | 영향 |
|---|---|---|---|---|
| 도구 실행 승인 프롬프트 (명령·diff 다이얼로그) | `item/commandExecution·fileChange/requestApproval` | ◐ | O | **high** |
| 권한 모드 순환 (default/acceptEdits/plan/bypass) | `approval_policy` × `sandbox_mode` | ◐ | - | medium |
| PreToolUse `permissionDecision` (allow/deny/**ask**) | allow/deny 만 (ask 없음) | ◐ | - | low |
| PermissionRequest 훅 | 동명 이벤트, 스키마 상이 | ◐ | - | low |
| PermissionDenied 훅 (retry) | 없음 | ○ | - | low |
| `updatedPermissions` 훅 출력 | 없음 (대신 팝업에서 사용자가 영속 규칙 작성) | ○ | - | low |
| settings `permissions` 규칙 (`Bash(...:*)` 패턴) | `/permissions` + `permissions.<name>` 프로파일 | ◐ | - | low |
| skill frontmatter `allowed-tools` | 없음 (frontmatter 미파싱) | ○ | - | low |
| `--dangerously-skip-permissions` | `--dangerously-bypass-approvals-and-sandbox` | ● | - | low |
| 플러그인 훅 무조건 실행 | **훅 신뢰 hash 모델** (미신뢰 = 조용한 제외) | ◐ | O | **high** |
| 워크스페이스 신뢰 다이얼로그 | 동일 | ● | - | low |
| plan 모드 편집 차단 + ExitPlanMode 승인 | `/plan` 모드 | ◐ | - | low |
| Notification 훅 `permission_prompt` matcher | 없음 | ○ | - | low |
| Bash 샌드박스 | `sandbox_mode` (네트워크 기본 차단) | ● | O | **high** |
| **AskUserQuestion** | `request_user_input` (Plan 전용) | △ | O | **high** |

### 2.2 작업 진행 표시

| Claude 프리미티브 | codex 등가물 | 상태 | axhub | 영향 |
|---|---|---|---|---|
| TodoWrite 체크리스트 | **`update_plan`** (거의 동일 스키마, 기본 노출) | ● | O | low |
| TaskCreate/Get/List/Update | `update_plan` 하나 | ◐ | - | low |
| Ctrl+T todo 패널 · `/todos` | 없음 (인라인 셀 + `/ps`) | ◐ | - | low |
| 진행 스피너 | `tui.animations` + shimmer | ● | - | low |
| 훅 `statusMessage` | **동일 필드 지원** | ● | - | low |
| `statusLine` (임의 스크립트 출력) | `/statusline` + `tui.status_line` (큐레이션 항목만) | ◐ | - | low |
| 백그라운드 셸 + Ctrl+B | `unified_exec` PTY 세션 | ● | - | low |
| Monitor | 없음 (write_stdin 폴링 근사) | △ | O | low |
| 장기 명령 대기 UX (`verify --wait` 10분) | 30초 yield → 백그라운드 터미널 | ◐ | O | medium |
| 자연어 한 줄 진행 알림 (스킬 규약) | 동일 | ● | O | low |

### 2.3 리치 출력

| Claude 프리미티브 | codex 등가물 | 상태 | axhub | 영향 |
|---|---|---|---|---|
| GFM 표 렌더 | pulldown-cmark ENABLE_TABLES + 좁은 터미널 전치 | ● | O | low |
| bold/italic/취소선/인라인코드 | 동일 | ● | O | low |
| 중첩 목록 | 동일 | ● | O | low |
| 코드블록 + 신택스 하이라이팅 | `/theme` 포함 동일 | ● | O | low |
| **플레인텍스트 카드 줄바꿈 보존** | soft break → 실제 개행 (표준보다 관대) | ● | O | low |
| 마크다운 링크 + OSC 8 | label 뒤 URL 병기, 로컬 링크는 라벨 무시 | ◐ | O | low |
| Edit/Write diff 프리뷰 · `/diff` | 승인 diff + 정적 `/diff` | ● | O | low |
| GFM 체크박스 `- [ ]` | 없음 (리터럴로 남음) | ○ | - | low |
| 마크다운 이미지 `![]()` | 조용히 사라짐 | ◐ | - | low |
| Artifact (호스팅 페이지·공유·댓글) | 없음 (로컬 HTML viewer) | ○ | - | low |
| mermaid 네이티브 렌더 | 없음 | ○ | - | low |
| SendUserFile `display:render` | 없음 | ○ | - | low |
| 테마 | `/theme` (신택스 한정) | ◐ | - | low |
| 헤딩·블록쿼트·수평선 | `#` 문자 유지 스타일 | ● | O | low |

### 2.4 파일·미디어

| Claude 프리미티브 | codex 등가물 | 상태 | axhub | 영향 |
|---|---|---|---|---|
| SendUserFile (파일 카드) | 없음 (경로 링크 안내) | △ | - | low |
| 이미지 붙여넣기 입력 | composer attachment + `-i/--image` | ● | - | low |
| 이미지 시각적 읽기 | `view_image` | ● | - | low |
| PDF·노트북 읽기 | 없음 | △ | - | low |
| Artifact | inline visualization (로컬 HTML) | ◐ | - | low |
| 평문 URL·코드블록 산출물 전달 | 동일 + WebHyperlink 셀 | ● | O | low |
| 스크린샷 (MCP 경유) | browser_use / computer_use | ◐ | - | low |
| MCP 리소스 읽기 | `list_mcp_resources`/`read_mcp_resource` | ● | - | low |
| `/export` | `/export` | ● | - | low |

### 2.5 알림

| Claude 프리미티브 | codex 등가물 | 상태 | axhub | 영향 |
|---|---|---|---|---|
| PushNotification 도구 | 없음 | ○ | - | low |
| Notification 훅 이벤트 | 없음 (훅 11종에 미포함) | ○ | - | low |
| 터미널 알림 채널 설정 | `tui.notifications` + `notification_method`(osc9/bel) | ● | - | low |
| 입력 필요·승인 대기 알림 | TUI 알림 3종 (**기본 켜짐**) | ● | O | medium |
| 커스텀 알림 명령 (훅) | `notify` config (턴 종료 1종, 사용자 config 전용) | ◐ | - | low |
| `terminalSequence` 훅 출력 | 없음 | ○ | - | low |
| 폰 푸시 (Remote Control) | 없음 (CLI 기준) | ○ | - | low |
| `awaySummaryEnabled` recap | 없음 (TUI 기준) | ○ | - | low |
| 백그라운드 작업 완료 알림 | 셸 백그라운드는 알림 없음 | ◐ | - | low |

### 2.6 백그라운드·장기 실행

| Claude 프리미티브 | codex 등가물 | 상태 | axhub | 영향 |
|---|---|---|---|---|
| `run_in_background` + 종료 시 자동 재호출 | unified_exec 백그라운드 (**자동 재호출 없음**) | ◐ | - | low |
| BashOutput / KillShell | write_stdin 폴링 + `/stop` | ● | - | low |
| Bash timeout 120s~600s 단일 블로킹 | **30초 yield 강제** (legacy shell 은 10초) | ◐ | O | **high** |
| Monitor | 없음 | ○ | - | low |
| ScheduleWakeup | 없음 (`clock.sleep` 은 기본 off) | △ | O | medium |
| Cron 도구군 | 없음 | ○ | - | low |
| RemoteTrigger · `/schedule` | codex cloud (성격 다름) | ○ | - | low |
| `/loop` | 없음 | ○ | - | low |
| TaskStop/TaskOutput · `/tasks` | `/ps` + `/stop` (전체 중지) | ◐ | - | low |
| 서브에이전트 완료 자동 재호출 | mailbox trigger-turn | ● | - | low |
| plugin `experimental.monitors` | 없음 | ○ | - | low |

### 2.7 서브에이전트·병렬

| Claude 프리미티브 | codex 등가물 | 상태 | axhub | 영향 |
|---|---|---|---|---|
| Agent(Task) 도구 | `spawn_agent`/`wait_agent`/`close_agent` | ● | - | low |
| 병렬 fan-out | 병렬 스폰 (가이던스가 오히려 권장) | ● | - | low |
| `.claude/agents` 커스텀 에이전트 | `~/.codex/agents/*.toml` (+ `/import` 자동 이식) | ● | - | low |
| 플러그인 매니페스트 `agents` | 없음 (설치 스크립트 복사) | △ | - | low |
| Workflow 도구 | 없음 | ○ | - | low |
| TaskStop/TaskOutput · `/tasks` 패널 | `/subagents` + `/ps` 분리 | ● | - | low |
| Task fork 모드 | `spawn_agent fork_turns` | ● | - | low |
| skill frontmatter `context: fork` | 없음 | ○ | - | low |
| SubagentStart/Stop 훅 | 동명 이벤트 | ● | - | low |
| Agent teams | 계층형 트리 + 메시징 | ◐ | - | low |
| ListAgents/SendMessage (크로스세션) | 같은 트리 한정 | ◐ | - | low |

---

## 3. 미측정 8 카테고리 — 인벤토리 기반 요약

아래는 정밀 매핑을 못 돌린 카테고리예요. 다만 양쪽 인벤토리 전수 조사와 선행 문서가 이미 상당 부분을 덮고 있어서, **알려진 결론만** 신뢰도와 함께 적어요. 정밀 매핑은 후속 과제예요.

| 카테고리 | 알려진 결론 | 근거 |
|---|---|---|
| **훅 매트릭스** | codex 훅 이벤트 11종 (SessionStart/End, PreToolUse, PermissionRequest, PostToolUse, PreCompact, **PostCompact**, UserPromptSubmit, SubagentStart/Stop, Stop) — Notification 만 부재. 출력 필드는 `suppressOutput` **미구현**(문서에도 "Parsed but not implemented"), `additionalContext` 는 TUI 에 상시 노출, `statusMessage` 는 **완전 지원** | `[confirmed]` 인벤토리 + 선행 문서 |
| **스킬 로딩·라우팅** | 본문 **8,000B 절단**(frontmatter 포함 파일 선두 기준), description 1,024자 절단, `examples` 미파싱. codex 트리 실측 합계 198,377B / 최대 skill 28,985B | `[confirmed]` 선행 문서 + 본 조사 byte 실측 |
| **MCP 표면** | elicitation ✅(기본 on) · sampling ❌ · roots ❌ · structuredContent ✅ · ResourceLink ✅ · plugin manifest `mcpServers` ✅ · OAuth ✅ | `[confirmed]` — 상세는 `codex-선택UX-연구.md` §3 |
| **세션·컨텍스트** | `/compact` ✅ · resume picker ✅(rename/archive/fork 포함) · PreCompact+**PostCompact** 훅 ✅ · `AGENTS.md`(project_doc_max_bytes 등 config) · `~/.codex/memories` + `/memories` · `get_context_remaining` 도구 | `[confirmed]` 인벤토리 |
| **계획 모드** | `/plan` 항상 활성(CollaborationModes = Stage::Removed = 상시), Shift+Tab 전환, Plan 모드에서만 `request_user_input` blocking | `[confirmed]` |
| **슬래시·@멘션** | 슬래시 명령 50종+ (`/import` 로 Claude Code 설정·에이전트 이식 포함), 파일 `@멘션` popup ✅, skill popup ✅. **커스텀 프롬프트(`~/.codex/prompts`)는 2026-03-28 제거** — 공식 문서에 남은 건 stale | `[confirmed]` / 문서 stale 은 `[likely]` |
| **입력 UX** | 멀티라인·붙여넣기·paste burst 감지·외부 에디터·vim 모드·`/keymap` 리맵·Esc-Esc 백트래킹·`turn/steer` 스티어링·`/side` 사이드 대화 — **Claude 보다 넓어요** | `[confirmed]` 인벤토리 |
| **상태줄·브랜딩** | `/statusline` + `tui.status_line` 은 **큐레이션된 item 선택형**이라 임의 스크립트 출력 불가. `/title` 터미널 타이틀 ✅ | `[confirmed]` |

---

## 4. critical/high 격차 상세

### G1 · 승인 프롬프트의 축이 "명령 가시성" → "sandbox escalation" 으로 이동 `high`
Claude 는 미허용 도구 호출마다 명령 원문을 보여주고 물어요. codex 기본(`on-request`)은 **sandbox 를 통과하는 명령은 아예 안 물어요**. 게다가 네트워크 승인에서 `Yes, and allow this host in the future` 를 한 번 고르면 이후 `axhub apps delete --yes` 같은 파괴 명령도 호스트 승인 없이 통과해요 `[confirmed — approval_overlay.rs:850,866]`.
→ **대응**: 호스트 레벨 게이트에 기대지 말고 AP-2/AP-3/AP-12 대화 승인을 유일한 방어선으로 간주해요 (이미 그렇게 설계돼 있어요). README codex 판에 "첫 host 승인의 의미" 를 한 줄 명시하면 좋아요.

### G2 · 플러그인 훅이 신뢰 승인 전까지 조용히 꺼짐 `high`
Claude 는 설치된 플러그인 훅을 그냥 실행해요. codex 는 hash 기반 신뢰가 필요하고, 시작 팝업에서 관성적으로 `Continue without trusting` 를 고르면 auto-update·update-first 라우팅·Windows 계약·재시작 확인·AP-19 실패 리포트가 **전부 침묵으로** 꺼져요 — 꺼진 사실조차 안 보여요 `[confirmed]`.
→ **대응**: 이미 부분 적용돼 있어요 (README/POLICY codex 판이 꺼지는 표면을 열거, update 스킬을 훅-무관 완결 계약으로 설계, wrapper 화로 재신뢰 빈도 축소). 추가로 온보딩 첫 화면에서 `/hooks` 안내 한 줄이 남은 갭이에요.

### G3 · 샌드박스 네트워크 기본 차단 `high`
codex `workspace-write` 기본은 네트워크 비활성이라 **첫 axhub 명령부터** escalation 승인이나 실패-재시도가 나요. 비개발자가 "명령이 왜 실패하지" 에서 멈출 수 있어요 `[confirmed]`.
→ **대응**: onboarding codex 판에 "첫 실행에서 네트워크 허용을 한 번 물어요 — 허용해야 axhub 백엔드에 닿아요" 를 선제 안내.

### G4 · 장기 단일 대기 계약이 성립하지 않음 `high`
`axhub deploy verify --wait` 10분은 Claude 에서 "한 번의 블로킹 호출" 이지만, codex `unified_exec` 는 최대 30초에 yield 하고 백그라운드 터미널로 넘겨요 (legacy `shell` 경로면 `timeout_ms` 기본 **10초**에 그냥 kill) `[confirmed — core/src/exec.rs:61, spec_plan.rs:980-999]`. 모델이 yield 를 실패·완료로 오판해 verify 를 재실행하거나 조기 종료할 위험이 있어요.
→ **대응**: codex 판 deploy 스킬에 **폴링 계약**을 명시해요 — `exec_command` 후 빈 `write_stdin` 폴링(각 ≤300초)으로 완주, yield 는 실패가 아님. 더 나은 선택지로 codex **내장 awaiter 에이전트**(`core/src/agent/builtins/awaiter.toml`, background_terminal_max_timeout 1시간)에 위임하는 경로가 있는데 현재 번들은 이를 전혀 언급하지 않아요.

### G5 · AUQ 선택 카드 (별도 문서)
`codex-선택UX-연구.md` 참조. 요약: 위젯은 존재하되 Plan 모드 게이트, 사용자 config 한 줄로 개방 가능.

---

## 5. 의외의 발견 — codex 가 더 나은 지점

- **표·플레인텍스트 카드 렌더가 더 견고해요.** 좁은 터미널에서 표를 key/value 로 자동 전치하고, 소프트브레이크를 실제 개행으로 렌더해요 — 우리 ✓·①-⑤ 카드가 fence 없이도 안 뭉개지는 이유예요.
- **알림이 기본 켜짐**이에요 (Claude 는 opt-in). 타입 필터 배열·포커스 조건까지 지원하고, `notify` payload 에 마지막 assistant 메시지를 통째로 실어줘요.
- **승인 팝업이 영속 규칙을 직접 써요** — `don't ask again for commands that start with <prefix>` 선택이 execpolicy amendment 로 기록돼요. Claude 보다 강한 durable rule UX 예요.
- **네트워크가 host 단위 승인 대상**이에요 — Claude 권한 시스템에 없는 축이에요.
- **`request_permissions` 도구** — 모델이 스스로 권한 상승을 요청하는 3택 팝업. Claude 에 등가물이 없어요.
- **승인 정책에 문서화가 얕은 4번째 변형 `Granular`** 가 있어요 — 승인 유형별(sandbox/execpolicy/skill/permissions/mcp_elicitation) 개별 on/off `[confirmed — protocol.rs:933-975]`.
- **내장 awaiter 에이전트**(대기 전용 저-effort 에이전트) — 장기 배포 대기의 codex-native 정답 후보예요.
- **`/import`** 가 `~/.claude/agents`·설정·최근 대화를 codex 로 자동 이식해요.
- **터미널 인라인 이미지**(Kitty/Sixel), **내장 이미지 생성**, **오디오 입력·realtime 음성** 표면이 있어요.
- **입력 UX 가 전반적으로 넓어요** — vim 모드, `/keymap` 리맵, Esc-Esc 백트래킹, 실행 중 스티어링(`turn/steer`), `/side` 사이드 대화.

---

## 6. 이번 스윕이 찾아낸 번들 결함 6건 (실제 액션)

| # | 결함 | 위치 | 영향 |
|---|---|---|---|
| D1 | `TodoWrite` → `update_plan` 치환 누락 (AUQ 는 치환하면서 체크리스트 도구는 미치환) | `scripts/build-plugin-bundle.ts:72-98` | 모델이 존재하지 않는 도구명을 봄 |
| D2 | codex update 스킬만 TodoWrite 섹션을 통째로 드롭 — 스킬 간 처리 비일관 | `codex-overrides/skills/update/SKILL.md` | 일관성 |
| D3 | 존재하지 않는 도구 금지 조항 잔존 (`Codex Monitor`, `ScheduleWakeup`, `TaskOutput`) | codex 번들 다수 | 죽은 텍스트가 **8KB 절단 예산**을 먹음 |
| D4 | `AUQ` 약어 미치환 | `deploy/SKILL.md:34`, `import/SKILL.md:65`, `development/references/write-gate.md:19,21` | 호스트 문자열 누출 |
| D5 | `CLAUDE_NON_INTERACTIVE` 잔존 (FORBIDDEN 목록에 없음) | `onboarding/SKILL.md:63` | headless 판정이 codex 에서 무의미 |
| D6 | codex-native 장기 대기 계약(unified_exec 폴링 / awaiter 에이전트) 미기술 | codex 판 deploy 스킬 | G4 의 실제 원인 |

---

## 7. 우선순위 액션

**P0 (반나절, 기계적)**
1. `CODEX_SUBSTITUTIONS` 에 `["TodoWrite", "update_plan"]`, `["AUQ", "명시 텍스트 승인"]` 추가 (longest-first 정렬 유지) — D1·D4
2. `FORBIDDEN_STRINGS` 에 `"AUQ"`, `"TodoWrite"`, `"CLAUDE_NON_INTERACTIVE"` 추가 — D4·D5
3. `Monitor`/`ScheduleWakeup`/`TaskOutput` 금지 조항을 codex 트리에서 제거 (override 또는 host-scoped 문단) — D3, 절단 예산 회수
- 검증: `bun run plugin:bundle:all` 후 codex 트리 grep 0건 + `bun run plugin:budget:codex` green

**P1 (1~2일, 계약 추가)**
4. codex 판 deploy 스킬에 **장기 대기 폴링 계약** 추가 (yield 는 실패 아님 / 빈 write_stdin 폴링 / 중복 verify 금지) — D6·G4
5. onboarding codex 판에 **네트워크 host 승인 1회** 선제 안내 — G3
6. onboarding codex 판에 **`/hooks` 신뢰 안내** 한 줄 — G2
- 검증: 기본 정책(`on-request`) 프로파일 TUI 에서 배포 1회 완주 QA

**P2 (후속)**
7. 미측정 8 카테고리 정밀 매핑 (특히 훅 출력 필드 × 이벤트 전수, 세션·컨텍스트)
8. awaiter 에이전트 위임 경로 실험 — 장기 배포 대기의 codex-native 해법
9. `Granular` 승인 정책이 우리 게이트에 주는 영향 조사

---

## 8. 미해결

- **8 카테고리 정밀 매핑 미완** — 크레딧 소진(subagent 19개 중 10개 실패). 완료 7개 + 인벤토리 요약으로 대체했어요. 재개하려면 워크플로 resume 으로 캐시 재사용이 가능해요.
- **검증 단계 미실행** — "codex 에 없다" 판정 중 high 3건의 적대적 재검증이 크레딧으로 못 돌았어요. `absent` 판정은 `[likely]` 로 읽는 게 안전해요.
- **라이브 TUI QA 0회** — 전부 소스·바이너리·문서 실측이에요.
- **로컬 config 교란** — 이 머신은 `approval_policy="never"` + `danger-full-access` 라 승인·elicitation 이 전부 자동 처리돼요. QA 는 별도 프로파일에서 해야 해요.

---

## 부록 — 재현 명령

```bash
BIN=~/.nvm/versions/node/v24.14.0/lib/node_modules/@openai/codex/node_modules/@openai/codex-darwin-arm64/vendor/aarch64-apple-darwin/bin/codex

codex features list                       # feature stage/기본값 전수
codex app-server generate-json-schema --out /tmp/cs   # ClientRequest 95 / ServerNotification 72 / ServerRequest 10
codex debug prompt-input                  # 실제 주입되는 컨텍스트(skills_instructions 등)
strings "$BIN" | rg -i "update_plan|exec_command|write_stdin|statusMessage"
rg "approval_policy|sandbox_mode|notify|tui\." ~/.codex/config.toml

# 번들 결함 확인
rg -n "AUQ|TodoWrite|CLAUDE_NON_INTERACTIVE|ScheduleWakeup|Monitor|TaskOutput" plugins/axhub-codex/
```

> 원자료: 워크플로 저널 `~/.claude/projects/.../workflows/wf_bf7165d1-dae/journal.jsonl` (매핑 7건 + 인벤토리 2건), `wf_aaea6db0-4e7/journal.jsonl` (AUQ 6 lane + 검증 18건)

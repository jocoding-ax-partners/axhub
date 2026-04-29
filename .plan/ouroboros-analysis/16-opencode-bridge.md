# 16. OpenCode TS Bridge Plugin

## 위치

`src/ouroboros/opencode/plugin/`:
- `ouroboros-bridge.ts` (560 LOC, 22.7 KB) — 본체
- `ouroboros-bridge.test.ts` (22.8 KB) — Bun 테스트
- `opencode-plugin.d.ts` (732 B) — 타입 정의
- `package.json` (349 B)
- `tsconfig.json` (293 B)
- `__init__.py` (Python 패키지 마커)

## 책임

Ouroboros MCP 의 `_subagent` (단일) / `_subagents` (병렬) 페이로드를 OpenCode 의 `session.promptAsync` 로 fire-and-forget 디스패치.

## 왜 필요한가

`session.prompt` (블로킹) → 200s+ 응답 지연.
`session.promptAsync` (비동기) → ~10 ms return + 백그라운드 실행.

→ MCP hook 이 200s 안 막힘 → main session 계속 진행 가능 → silent "dispatched but never ran" 막힘.

## package.json

```json
{
  "name": "ouroboros-bridge",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "description": "OpenCode subagent bridge plugin — runtime hook that PATCHes parallel child sessions as inline Task panes.",
  "scripts": {"test": "bun test"},
  "devDependencies": {
    "@types/bun": "latest",
    "@types/node": "latest"
  }
}
```

## 핵심 상수

```typescript
export const MAX_BYTES = 100_000           // 100 KB prompt cap
export const DEDUPE_MS = 5_000              // 5초 FNV-1a 해시 dedupe
export const MAX_FANOUT = 10                 // 병렬 디스패치 최대
export const MAX_SEEN = 256                  // dedupe table size
export const ID_LEN = 26
export const CHILD_TIMEOUT_MS = num(process.env.OUROBOROS_CHILD_TIMEOUT_MS, 20 * 60 * 1000)
const PATCH_RETRIES = 3
const RESOLVE_RETRIES = 5
const BACKOFF_MS = 100
```

## 플랫폼-aware Config 디렉토리

```typescript
export function cfg(): string {
  const home = process.env.HOME ?? process.env.USERPROFILE ?? "/tmp"
  if (process.platform === "win32")
    return join(process.env.APPDATA ?? join(home, "AppData", "Roaming"), "OpenCode")
  if (process.platform === "darwin")
    return join(home, "Library", "Application Support", "OpenCode")
  return join(process.env.XDG_CONFIG_HOME ?? join(home, ".config"), "opencode")
}

const DIR = join(cfg(), "plugins", "ouroboros-bridge")
const LOG = join(DIR, "bridge.log")
```

## 모노토닉 ID 생성

OpenCode `src/id/id.ts` 의 ascending 포맷 매치:

```typescript
const B62 = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"

let lastTs = 0
let ctr = 0

export function rand62(n: number): string {
  const b = randomBytes(n)
  let s = ""
  for (let i = 0; i < n; i++) s += B62[b[i] % 62]
  return s
}

export function id(prefix: "prt" | "tool"): string { ... }
```

## FNV-1a Hashing (Dedupe)

5 초 윈도우 + 256 entry table. 같은 prompt+session 재요청 → drop.

## v22 Hardening (위 CHANGELOG `[Unreleased]`)

"No uncaught errors under any input":

1. **모든 reject path 로깅** — Promise.reject 누락 추적
2. **Frozen-content guard** — Object.isFrozen 체크
3. **Empty-sessionID guard** — null sessionID 거부
4. **Client init-order guard** — 초기화 안 된 client 사용 거부
5. **5초 FNV-1a prompt dedupe** — 중복 디스패치 방지
6. **100 KB prompt byte cap** + truncation marker — 거대 페이로드 방어
7. **`fail()` + `notify()` 사용자 가시 에러** — silent dispatch failure 방지 (CHANGELOG.md `[Unreleased]` 블록 의 `surfaceErr` 명칭은 코드와 불일치 — 본 section 의 "Section 27 정정 — 1차 라운드 추정 함수 정리" 표 참조)
8. **절대 outer try/catch** — opencode runLoop 으로 throw 안 됨

## v23 Multi-subagent Fan-out

`_subagents` (plural) 배열 → 병렬 dispatch:

1. 페이로드별 검증 (`build(p, idx)`)
2. truncation (UTF-8 safe `truncateUtf8(s, maxBytes)`)
3. dedupe (개별 FNV-1a 해시)
4. `MAX_FANOUT=10` 만큼 병렬 `promptAsync`
5. 한 dispatch 실패해도 나머지 계속 (Promise.allSettled 패턴)
6. 응답에 `ouroboros_subagents`, `ouroboros_dispatch_errors` 메타 추가
7. v22 단일 `_subagent` 호환 유지

## 주요 함수 — Section 28 deep-dive (440 LOC 정독 후)

### `id(prefix: "prt" | "tool"): string` (line 62–70)

OpenCode `src/id/id.ts` ascending 형식 mirror:

```typescript
let lastTs = 0
let ctr = 0

export function id(prefix: "prt" | "tool"): string {
  const now = Date.now()
  if (now !== lastTs) { lastTs = now; ctr = 0 }
  ctr++
  let v = BigInt(now) * BigInt(0x1000) + BigInt(ctr)
  const buf = Buffer.alloc(6)
  for (let i = 0; i < 6; i++) buf[i] = Number((v >> BigInt(40 - 8 * i)) & BigInt(0xff))
  return prefix + "_" + buf.toString("hex") + rand62(ID_LEN - 12)
}
```

→ `(timestamp << 12) | counter` 형식 6 byte hex + base62 random suffix. 시간 단조 증가 + 같은 ms 안에서 counter 보장.

### `fnv(s: string): string` (line 72–79)

FNV-1a 32-bit hash:

```typescript
let h = 0x811c9dc5
for (let i = 0; i < s.length; i++) {
  h ^= s.charCodeAt(i)
  h = Math.imul(h, 0x01000193)
}
return (h >>> 0).toString(16)
```

→ FNV-prime 0x01000193, FNV-offset-basis 0x811c9dc5. `Math.imul` 가 32-bit 정수 곱 보장.

### `truncateUtf8(s: string, maxBytes: number): string` (line 106–113)

UTF-8 safe truncation. Continuation byte (0x80–0xBF) 면 한 칸씩 backward:

```typescript
const buf = Buffer.from(s, "utf8")
if (buf.length <= maxBytes) return s
let end = maxBytes
while (end > 0 && (buf[end] & 0xC0) === 0x80) end--   // 10xxxxxx continuation
return buf.subarray(0, end).toString("utf8")
```

→ multi-byte UTF-8 character 가 잘리지 않게 valid boundary 찾을 때까지 backward.

### `build(p: unknown, idx: number): Sub | null` (line 115–133)

Raw payload validation + truncation + hash:

```typescript
if (!p || typeof p !== "object") { log("REJECT reason=payload_not_object"); return null }
if (typeof r.tool_name !== "string" || !r.tool_name) { log("REJECT reason=missing_tool_name"); return null }
if (typeof r.prompt !== "string" || !r.prompt) { log("REJECT reason=missing_prompt"); return null }

const truncated = Buffer.byteLength(r.prompt, "utf8") > MAX_BYTES
const prompt = truncated
  ? truncateUtf8(r.prompt, MAX_BYTES) + `\n\n[...truncated at ${Math.round(MAX_BYTES / 1024)}KB]`
  : r.prompt

return {
  tool: r.tool_name,
  title: r.title || r.tool_name,           // title 없으면 tool 이름
  agent: r.agent || "general",              // agent 없으면 "general"
  prompt,
  truncated,
  hash: fnv(prompt),
}
```

→ `tool_name` + `prompt` 필수. truncate 시 마커 `[...truncated at 100KB]` 첨부. dedupe 용 fnv hash.

### `parse(raw: string): { subs, responseShape }` (line 138–168)

```typescript
const empty = { subs: [], responseShape: {} }
if (!raw || raw.length < 2) return empty
let obj: unknown
try { obj = JSON.parse(raw) } catch { return empty }
if (!obj || typeof obj !== "object") return empty

const record = obj as Record<string, unknown>

// response_shape: _subagent/_subagents 외 모든 top-level key
const responseShape: Record<string, unknown> = {}
for (const [k, v] of Object.entries(record)) {
  if (k !== "_subagent" && k !== "_subagents") responseShape[k] = v
}

const multi = record._subagents
if (Array.isArray(multi)) {
  if (multi.length === 0) return empty
  if (multi.length > MAX_FANOUT) log("WARN fanout_capped requested=N cap=10")
  const subs = multi.slice(0, MAX_FANOUT).flatMap((p, i) => {
    const s = build(p, i)
    return s ? [s] : []
  })
  return { subs, responseShape }
}

const single = record._subagent
if (single && typeof single === "object") {
  const s = build(single, 0)
  return s ? { subs: [s], responseShape } : empty
}
return empty
```

→ `_subagents` (array) 우선, 없으면 `_subagent` (single). MAX_FANOUT=10 cap. **`responseShape` 추출**: `session_id`, `job_id`, `status` 같은 contract 필드 보존 → `stamp()` 가 LLM 가시 텍스트에 포함시켜 LLM 이 contract 정보 잃지 않도록.

### `dupe(pid: string, callID: string): boolean` (line 264–282)

```typescript
const seen = new Map<string, number>()  // 모듈 레벨 state

export function dupe(pid: string, callID: string): boolean {
  const key = `${pid}::${callID}`         // 정확히 (parent session, MCP callID)
  const now = Date.now()
  const prev = seen.get(key)
  if (prev !== undefined && now - prev < DEDUPE_MS) return true   // 5초 내 dup
  seen.set(key, now)

  // LRU eviction (절반 삭제)
  if (seen.size > MAX_SEEN) {
    let i = 0
    for (const k of seen.keys()) {
      if (i++ >= Math.floor(MAX_SEEN / 2)) break
      seen.delete(k)
    }
  }
  return false
}
```

→ **Identity = `sessionID::callID`** (1 MCP call = 1 dispatch). hook 이 같은 callID 로 두 번 발사되면 (opencode edge case) 두 번째는 dedupe. 다른 MCP 호출은 다른 callID → 절대 dedupe 안 됨.

→ **LRU eviction 은 절반 삭제** (Map.keys() iteration order = insertion order). 약 128 개 oldest 항목 제거.

### `resolveMid(cli, pid, callID)` (line 342–356)

```typescript
async function resolveMid(cli: Cli, pid: string, callID: string): Promise<string | null> {
  for (let i = 0; i < RESOLVE_RETRIES; i++) {
    const res = await cli.session.messages({ path: { id: pid } }).catch(() => null)
    const msgs = res?.data
    if (Array.isArray(msgs)) {
      // 뒤에서부터 (최신 message 우선)
      for (let j = msgs.length - 1; j >= 0; j--) {
        const m = msgs[j]
        if (m.info.role !== "assistant") continue
        if (m.parts.some((p) => p.type === "tool" && p.callID === callID)) return m.info.id
      }
    }
    if (i < RESOLVE_RETRIES - 1) await sleep(BACKOFF_MS)   // 100ms 대기
  }
  return null
}
```

→ **Fail closed**: `callID` 정확 매치 못 찾으면 `null` 반환. 절대 임의의 message 로 fallback 안 함 → 같은 session 의 다른 dispatch 와 cross-talk 방지. RESOLVE_RETRIES=5, BACKOFF_MS=100 → 최대 ~500ms 대기.

### `patch(b, pid, mid, partID, body, tag)` (line 323–337)

```typescript
async function patch(b: Base, pid: string, mid: string, partID: string, body: unknown, tag: string) {
  let last: unknown
  for (let i = 0; i < PATCH_RETRIES; i++) {
    const r = await b.patch({
      url: "/session/{sessionID}/message/{messageID}/part/{partID}",
      path: { sessionID: pid, messageID: mid, partID },
      body,
    }).catch((e) => ({ error: e }))
    if (!r.error) return
    last = r.error
    log(`PATCH_RETRY tag=${tag} attempt=${i + 1} err=${errMsg(last)}`)
    await sleep(BACKOFF_MS * (i + 1))   // linear backoff: 100, 200, 300ms
  }
  throw new Error(`PATCH failed after ${PATCH_RETRIES} attempts: ${errMsg(last)}`)
}
```

→ Linear backoff (100ms, 200ms, 300ms). 3 회 재시도 후 throw.

### `dispatch(cli, b, pid, mid, s)` (line 375–460) — Fire-and-forget 핵심

```typescript
async function dispatch(cli, b, pid, mid, s: Sub): Promise<{ childID: string }> {
  const partID = id("prt")
  const callID = id("tool")
  const start = Date.now()
  const input = { description: s.title, prompt: s.prompt, subagent_type: s.agent }

  // === Awaited phase (~10–100ms) ===
  const created = await cli.session.create({ body: { parentID: pid, title: s.title } })
  const childID = created?.data?.id
  if (!childID) throw new Error("child session create returned no id")

  await patch(b, pid, mid, partID, {
    id: partID, messageID: mid, sessionID: pid,
    type: "tool", tool: "task", callID,
    state: {
      status: "running",
      input,
      title: s.title,
      metadata: { sessionId: childID },
      time: { start },
    },
  }, `running:${partID}`)

  // === Fire-and-forget phase (NOT awaited) ===
  const ctrl = new AbortController()
  const timer = setTimeout(() => ctrl.abort(), CHILD_TIMEOUT_MS)   // 20분 default

  cli.session.prompt({
    path: { id: childID },
    body: { agent: s.agent, parts: [{ type: "text", text: s.prompt }] },
    signal: ctrl.signal,
  }).then(async (res) => {
    clearTimeout(timer)
    const out = childOutput(childID, res.data)
    // PATCH widget → completed
    await patch(b, pid, mid, partID, {
      ...,
      state: { status: "completed", input, output: out, ..., time: { start, end: Date.now() } },
    }, `done:${partID}`).catch((e) => log(`PATCH_DONE_FAIL`))
  }).catch(async (e) => {
    clearTimeout(timer)
    const msg = ctrl.signal.aborted ? `child timed out after ${CHILD_TIMEOUT_MS}ms` : err.message
    await cli.session.abort({ path: { id: childID } }).catch(...)  // child 정리
    // PATCH widget → error
    await patch(b, pid, mid, partID, {
      ...,
      state: { status: "error", input, error: `${msg} (child=${childID})`, ..., time: { start, end: Date.now() } },
    }, `error:${partID}`).catch(...)
  })

  return { childID }   // ← prompt 완료 안 기다리고 즉시 return
}
```

→ **Hook 은 ~100 ms 안에 return**. session.create + patch-running 만 await. session.prompt 는 background. CHILD_TIMEOUT_MS (default 20분) 후 abort + child 정리 + widget error.

→ Trade-off: **prompt 실패 시 retry 안 됨**. 사용자가 retry 원하면 새 dispatch 호출 필요 (= 새 MCP call).

### `childOutput(childID, data)` (line 308–320)

OpenCode `src/tool/task.ts:158` mirror:

```typescript
const parts = (data as { parts?: Array<{ type: string; text?: string }> })?.parts
const text = Array.isArray(parts)
  ? [...parts].reverse().find((p) => p?.type === "text" && typeof p?.text === "string")?.text ?? ""
  : ""
return [
  `task_id: ${childID}`,
  "",
  "<task_result>",
  text,
  "</task_result>",
].join("\n")
```

→ parts 끝에서부터 (reverse) 마지막 text part 추출. XML-like wrapping (`<task_result>...</task_result>`).

### `notify(ok, failed, skipped)` (line 200–224)

사람-가시 banner. 다국어 처리 안 함, 영어 only:

```
[Ouroboros] Dispatched 3 subagents. Task widgets will update as they complete.
  • Title1 → agent='general' [child=ses_xxx]
  • Title2 → agent='hacker' (truncated to 100KB) [child=ses_yyy]
  • Title3 → agent='architect' [child=ses_zzz]
Failed 1 subagent before dispatch:
  • Title4
Skipped 1 duplicate (within 5s window):
  • Title5
```

### `buildEnvelope(ok, failed, skipped)` (line 237–256) — 구조화

```typescript
interface DispatchEnvelope {
  status: "dispatched" | "dispatch_failed" | "skipped" | "nothing"
  mode: "plugin_subagent"
  dispatched_at: string  // ISO timestamp
  children: Array<{ title; childID; agent; tool; truncated }>
  failed: Array<{ title; tool; reason? }>
  skipped: Array<{ title; tool }>
}

// status 결정 우선순위:
// ok > 0    → "dispatched"
// failed > 0 → "dispatch_failed"
// skipped > 0 → "skipped"
// else      → "nothing"
```

→ `out.metadata.ouroboros_dispatch` 에 첨부. MCP 호출자가 구조화 정보로 후처리 가능.

### `OuroborosBridge` Plugin (line 462–548) — `tool.execute.after` hook

```typescript
export const OuroborosBridge: Plugin = async (ctx) => {
  log(`INIT dir=${ctx.directory ?? "?"} timeout=${CHILD_TIMEOUT_MS}ms`)
  return {
    "tool.execute.after": async (input, output) => {
      try {
        // 1. tool 이름이 ouroboros_ prefix 가 아니면 skip
        if (typeof input.tool !== "string" || !input.tool.startsWith("ouroboros_")) return

        // 2. parse output → subs + responseShape
        const out = output as Output
        const { subs, responseShape } = parse(readText(out))
        if (subs.length === 0) return   // _subagent/_subagents 없음 → skip

        // 3. sessionID + callID 추출
        const pid = input.sessionID || ""
        const callID = input.callID || ""
        if (!pid || !callID) {
          fail(out, subs[0].tool, new Error("empty sessionID/callID"))
          return
        }

        // 4. client 초기화 검증
        const cli = ctx.client as Cli
        const b = base(ctx.client)
        if (!cli?.session?.create || !cli.session.prompt || !cli.session.abort
            || !cli.session.messages || !b) {
          fail(out, subs[0].tool, new Error("client not ready"))
          return
        }

        // 5. dedupe 검사
        if (dupe(pid, callID)) {
          // dedupe 시도 알림 + responseShape 보존
          stamp(out, notify([], [], subs) + dedupeShapeSuffix)
          out.metadata.ouroboros_dispatch = buildEnvelope([], [], subs)
          if (responseShape) out.metadata.ouroboros_response_shape = responseShape
          return
        }

        // 6. messageID resolve (callID 정확 매치)
        const mid = await resolveMid(cli, pid, callID)
        if (!mid) {
          fail(out, subs[0].tool, new Error("could not resolve messageID"))
          return
        }

        // 7. 병렬 dispatch (Promise.allSettled)
        const results = await Promise.allSettled(subs.map((s) => dispatch(cli, b, pid, mid, s)))
        const ok = results.flatMap((r, i) => r.status === "fulfilled"
          ? [{ sub: subs[i], childID: r.value.childID }]
          : [])
        const failed = results.flatMap((r, i) => r.status === "rejected"
          ? [{ sub: subs[i], reason: errMsg(r.reason) }]
          : [])

        // 8. banner + responseShape 보존
        const banner = notify(ok, failed.map((f) => f.sub), [])
        const shapeSuffix = Object.keys(responseShape).length > 0
          ? "\n\n```json\n" + JSON.stringify(responseShape, null, 2) + "\n```"
          : ""
        stamp(out, banner + shapeSuffix)

        // 9. 모든 metadata 첨부
        const envelope = buildEnvelope(ok, failed, [])
        out.metadata.ouroboros_dispatch = envelope
        out.metadata.ouroboros_subagents = subs.map(...)
        out.metadata.ouroboros_children = ok.map(...)
        if (failed.length > 0) out.metadata.ouroboros_dispatch_failed = ...
        if (responseShape) out.metadata.ouroboros_response_shape = responseShape

      } catch (e) {
        log(`HOOK_CRASH err=${...}`)   // 절대 outer try/catch — opencode runLoop 으로 throw 안 됨
      }
    },
  }
}
```

### Default export (V1 호환)

```typescript
export default {
  id: "ouroboros-bridge",
  server: OuroborosBridge,
}
```

→ V1 plugin loader 가 `Object.values(mod)` iteration 하면 `MAX_BYTES` 같은 non-function 에서 throw → V1 path 는 default 만 읽고 scan skip.

### Test-only exports

```typescript
export {
  resolveMid as _resolveMid,
  dispatch as _dispatch,
  patch as _patch,
  sleep as _sleep,
  PATCH_RETRIES as _PATCH_RETRIES,
  RESOLVE_RETRIES as _RESOLVE_RETRIES,
}
```

→ test 에서 mock client 로 fully cover 가능.

### Section 27 정정 — 1차 라운드 추정 함수 정리

| 1차 라운드 추정 | 실제 |
|---|---|
| `surfaceErr(...)` | **존재 안 함** — `fail()` (line 258) 와 `notify()` 가 사용자 가시 에러 처리 |
| `notify(ok, failed)` 2 인자 | `notify(ok, failed, skipped)` 3 인자 |
| `dupe()` 가 prompt hash 기반 | 실제는 `${sessionID}::${callID}` 기반. prompt hash 는 `Sub.hash` 에 별도 저장 (사용 안 됨) |
| 5 페르소나 fan-out 전용 | 일반 `_subagents` array — 페르소나 제한 없음. MAX_FANOUT=10 |

## 데이터 흐름

```
MCP server (Ouroboros)
   ↓ tool result with _subagent or _subagents
OpenCode runtime hook (this plugin)
   ↓ parse + validate + dedupe
Promise.allSettled([
  client.session.promptAsync(child1),
  client.session.promptAsync(child2),
  ...
])  → ~10ms 리턴
   ↓ stamp result with metadata
   ↓ fail() + notify() on failure
   ↓ patch parent message PATCH
OpenCode 화면에 inline Task 패널 표시
```

## 의존

- `@opencode-ai/plugin` — Plugin Protocol
- Node `fs`, `path`, `crypto`
- Bun (테스트만)

## 테스트 (`ouroboros-bridge.test.ts` 22.8 KB)

소스 파일과 같은 크기 — 테스트 커버리지 강력. Bun runtime: `bun test`.

## Hardening 검증

테스트:
- `tests/unit/cli/test_bridge_plugin_hardening.py`
- `tests/unit/cli/test_bridge_plugin_lifecycle.py`

→ Python 측에서 plugin 설치/생애주기 검증.

## 자동 설치

`cli/opencode_config.py` + `setup` 스킬이 OpenCode 검출 후 자동 install:
1. config 디렉토리 검출 (위 `cfg()`)
2. `~/.local/share/opencode/plugins/ouroboros-bridge/` 생성
3. `ouroboros-bridge.ts` + `package.json` + `tsconfig.json` 복사
4. OpenCode config.toml 에 plugin 등록

## docs/guides/opencode-subagent-bridge.md

브리지 플러그인 가이드. (확인 못 함, 파일 존재만 검증.)

## CHANGELOG 출처

```
[Unreleased]
### Added
- opencode: Subagent bridge plugin (src/ouroboros/opencode/plugin/ouroboros-bridge.ts)
  — routes MCP ouroboros_* tool calls with a _subagent parameter into OpenCode's
  native Task subagent panes via session.promptAsync. Fire-and-forget dispatch
  returns from the hook in ~10ms, eliminating the blocking 200s+ latency of the
  previous session.prompt approach. Installed automatically by ouroboros setup.
  See OpenCode Subagent Bridge.
```

## 한계 / 위험

- 빌드 안 함 (`.ts` 직접 실행 — Bun 또는 OpenCode 내장 TS runtime)
- OpenCode plugin API 변경 시 호환성 깨질 가능
- `bridge.log` 가 무한 append (rotation 안 함)
- v22/v23 hardening 도 완벽 안 함 — 여전히 silent failure 가능 영역 존재

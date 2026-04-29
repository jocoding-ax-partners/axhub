# 90 — Risks, Gaps, Tooling Failures

> **Status:** Phase 7 finalized. Worker 발견 + lead Phase 5/6 게이트 결과를 통합.
> 모든 entry 는 file:line 인용 또는 도메인 doc cross-ref 를 필수로 해요.

## §Tooling Gaps (Phase 0/2)

- **gitnexus analyze**: 성공. `/private/tmp/llm_wiki_inspect/.gitnexus/` 에 isolated index 생성 (1,525 nodes / 4,299 edges / 154 communities / 119 processes / 214 graph-files). axhub repo 의 .gitnexus 와 충돌 없음.
- **Cluster derivation method**: gitnexus cluster top-20 → 03..09 best-guess 도메인 슬롯 매핑 (`00-overview.md §Domain Mapping`). 10..18 reserved 슬롯은 사용 안 함 — top cluster 가 best-guess 와 깔끔히 정렬.
- **Long-tail clusters**: 154 중 134 (≤8 symbol) 는 best-fit 도메인 doc 또는 `[leaf-utility]` tag 로 흡수.
- **Denominator drift**: 처음 `find` 가 255 → gitnexus 부산물 (`.gitnexus/lbug`, `.gitnexus/meta.json`, `AGENTS.md`, `CLAUDE.md`, `.claude/skills/gitnexus/*`) 10 개 포함. canonical = `git ls-files | wc -l = 245` 로 lock.

## §Code Smells & Risk Surfaces (도메인별)

### Rust (04-backend-rust.md, 08-pdf-ocr-pipeline.md)

- **`unsafe` 키워드 0개** — 표면적으로 깨끗. 하지만 `pdfium-render` 의 `bind_to_library` (`fs.rs:142, 282`) + `bind_to_system_library` 가 dlopen 으로 C++ PDFium 을 로드하는 순간 메모리 안전성 보증은 깨져요. 대응: `panic_guard::run_guarded[_async]` 가 모든 Tauri command 본문 wrap. Cargo `panic = "unwind"` 의존.
- **`.unwrap()` / `.expect()` 매치 ~80개 (production 75 + test 65)**: lib.rs:106 expect 1 + clip_server.rs 9 + extract_images.rs:405 (`mime_type.unwrap()`, 직전 line if-none guard 로 false positive — 04 doc 인용). riser/refactor 시 `let Some(mt) = mime_type else { continue; }` 로 elision 권장.
- **`panic!` 매크로 3개** — 모두 `#[cfg(test)]` 가드 안. production code 에서 panic! 호출 0.
- **Mutex acquisition 5 사이트**: `PDFIUM_LOCK` (전역 `std::sync::Mutex<()>`, 비재진입) + clip_server statics 3 + claude_cli `tokio::sync::Mutex<HashMap<String, Child>>` 1. PDFium lock 호출 4 회 (extract_images.rs:112, 256, 649 + fs.rs 의 wrapper) 모두 `let _guard = lock_pdfium();` 패턴 — `_guard` 명명으로 drop 시점 명시. 모든 호출이 `tauri::async_runtime::spawn_blocking` 안 — `.await` 가로지르는 hold 없음. **invariant**: `extract_images.rs:103` 주석 "Holds the global pdfium lock for its full duration. Callers MUST NOT acquire the lock themselves before calling this (would deadlock — `std::sync::Mutex` is non-reentrant)."
- **claude_cli 의 lock-drop discipline (claude_cli.rs:289-298)**: 주석 자체가 "Don't hold the map lock across .wait() — kill could race." child remove 후 lock drop, 그 다음에 wait. 이거 깨면 tokio Mutex await-while-locked 회귀.
- **abort silent dispatch** — `claude_cli_kill` 호출 실패가 catch swallow 되고 즉시 `onDone` 호출. kill 실패 시 stale child 가 다음 stream listener 에 garbage emit 가능. test 0건.
- **Result swallow Rust equivalent (`let _ = ...`) 8 사이트**: lib.rs window ops + clip_server respond + claude_cli emit/kill — 04 Internal Risk 에 인용.

### Frontend TS (03-frontend.md, 09-ui-components.md)

- **`as unknown as` casts 3 사이트**: `App.tsx:47, 48` (devtools window 객체 mutate, prod 인입 안 됨) + `welcome-screen.tsx:78` (keyboard event → mouse event coercion).
- **빈 `catch {}` 9 사이트** 03-frontend (App.tsx 의 last-project / init / update-check / 등) + 3 사이트 09-ui (graph-view 의 readFile fallbacks). 모두 사용자에게 표면화 안 됨 — startup-time race 디버깅 어려움.
- **console-only error 6 라인**: 사용자 UI 신호 없음. handleNodeClick (graph-view.tsx:362), AppLayout.loadFileTree (app-layout.tsx:37) 등.
- **Sigma WebGL crash workaround (graph-view.tsx:435-451)**: `data-panel-resizing` body attribute → MutationObserver → 50/100ms timeout 후 `sigmaKey++` 강제 remount. 크래시 원인 ("could not find suitable program for node type circle") 을 fix 안 하고 race window 늘려 우회. 패널 resize 가 50ms 안에 끝나면 여전히 crash 가능.
- **GraphView 의 module-scoped globals (`positionCache`, `lastLayoutDataKey` graph-view.tsx:90-91)**: 동시 두 GraphView 인스턴스 가정 안 됨. 패널 toggle 로 unmount/remount 시 캐시 잔존 — 의도된 동작이지만 invariant 코드에 명시 X.

### Extension (05-extension.md)

- **innerHTML 보간 1 사이트 (popup.js:53)**: `projectSelect.innerHTML = \`<option value="${data.path}">${name}</option>\`` — 19827 응답이 `path: '"></option><script>...</script>'` 같은 페이로드 보내면 popup XSS. extension popup default CSP (manifest 명시 X) 에만 의존.
- **빈 catch 3곳 + console-only 부재**: /status / /projects / /project 호출 실패 디버깅 불가.
- **MV3 trust assumption**: `host_permissions` 가 `http://127.0.0.1:19827/*` plain-HTTP + 19827 응답에 인증 토큰 없음. 같은 머신의 다른 프로세스가 19827 점유하면 extension 이 그 응답 신뢰 → `data.path` 가 `<select>` value → 다음 POST `/clip` 의 `projectPath` 에 user-controlled 경로 흘러감.

### Data layer (06-data-layer.md)

- **TS↔Rust drift `WikiProject.id`**: TS 는 필수 (`{id, name, path}`), Rust 는 없음 (`{name, path}`). `create_project`/`open_project` 가 id 없는 객체 반환, TS 가 직후 `ensureProjectId()` 로 채움. 정적 검출 안 됨.
- **project-mutex cleanup branch dead code (project-mutex.ts)**: `locks.get(projectPath) === next` 가드가 항상 false (맵엔 `prev.then(() => next)` 가 들어감). 1024 size guard 만 실효. 1024+ distinct projectPath cycle 시 메모리 누수.
- **auto-save silent swallow**: `saveReviewItems(...).catch(() => {})`, `saveChatHistory(...).catch(() => {})` — 디스크 가득 / 권한 실패 시 사용자 / 콘솔 어디에도 신호 없음. 테스트 0건.
- **persist non-atomic write (saveChatHistory)**: conversations.json + N 개 chats/<id>.json 순차 덮어씀. 도중 crash 시 phantom (메타 있고 메시지 없는) 상태.
- **project-identity write 실패 zombie**: identity write fail → caller 한테 새 UUID 반환되지만 디스크엔 없음. 재부팅마다 새 UUID = 같은 프로젝트 다른 ID 로 인식.

### LLM integration (07-llm-integration.md)

- **`tauri-fetch.ts:48` 의 `as unknown as` cast** — plugin-http 의 fetch 가 native fetch 와 정확 호환된다고 단언. 모든 LLM/embedding/web-search 호출 통과. wide-reach.
- **`claude-cli-transport.ts:55-77` stream chunk handler 캐스팅**: `obj`, `event`, `delta`, `message`, `content` 모두 `Record<string, unknown>` narrow. Anthropic stream-json schema 변경 시 silent token loss 가능.
- **project-mutex + ingest deadlock surface**: `autoIngest` 가 `withProjectLock` 으로 6 단계 await 직렬화. 한 ingest 의 catastrophic hang (LLM 무한 대기, claude CLI deadlock) 이 같은 프로젝트의 모든 후속 ingest/save-to-wiki/auto-research-ingest 블록. project-mutex 자체가 "no timeouts" 라 인정.
- **ingest-queue 5 군데 stale-context guard** (`processNext`): 매 await 후 `currentProjectId` mismatch 검사. orphan promise silent return — 호출자는 task 결과 모름.
- **sweep-reviews race surface**: race.test 픽스 됐지만 rule-stage 안에서 부분 적용 가능 — store.resolveItem 동기 호출 + guard 동기 check 사이의 race.
- **ingest-queue crash recovery 부분만**: restoreQueue 가 `processing` → `pending` 되돌리고 retry. mid-ingest crash 시 disk partial wiki page 가 cleanupWrittenFiles 안 거치고 stale.
- **abort silent dispatch (claude-cli)**: kill 호출 실패 catch swallow 후 즉시 onDone. stale child garbage emit 가능. test 0건.

## §Untested Paths

- worker 보고에서 명시적으로 "test 0건" 으로 지목된 사이트:
  - `auto-save.ts` 의 디스크 가득 / 권한 실패 (06)
  - `project-identity.ts` 의 write fail zombie UUID (06)
  - `claude_cli_kill` 실패 후 stale child (07 + 04)
  - persist `saveChatHistory` 의 mid-write crash partial state (06)
  - extension popup `data.path` injection (05)
- 50-source-mapping 중 `.test.ts` 사이블링 없는 production lib (.ts):
  - `src/lib/auto-save.ts` — 테스트 없음
  - `src/lib/clip-watcher.ts` — 테스트 없음
  - `src/lib/deep-research.ts` — 테스트 없음
  - `src/lib/extract-source-images.ts` — 테스트 없음
  - `src/lib/file-types.ts` — 테스트 없음
  - `src/lib/graph-insights.ts` — 테스트 없음
  - `src/lib/graph-relevance.ts` — 테스트 없음
  - `src/lib/latex-to-unicode.ts` — 테스트 없음
  - `src/lib/persist.ts` (integration test 만 — unit 없음)
  - `src/lib/project-identity.ts` — 테스트 없음
  - `src/lib/project-store.ts` — 테스트 없음
  - `src/lib/raw-source-resolver.ts` — 테스트 없음
  - `src/lib/templates.ts` — 테스트 없음 (654 라인 frontmatter 정의)
  - `src/lib/utils.ts` — 테스트 없음 (cn helper)
  - `src/lib/wiki-graph.ts` — 테스트 없음
- Rust 측: production .rs 파일에 nextor 별도 unit test 없음 (cargo test 가 0). 모든 검증이 Tauri command 통합 시 frontend 시험에 의존.

## §Ambiguous Ownership

- `src/components/error-boundary.tsx`: 03-frontend (App.tsx wrap) + 09-ui-components (재사용 컴포넌트 contract) 양쪽에서 cross-ref. 50-source-mapping 의 backlink 는 03 으로 단일.
- `src/lib/templates.ts`: 03-frontend 가 cite (frontmatter UI 측면) 하지만 ingest pipeline (07) 도 사용. 단일 backlink 03 으로 정해 — risk 분석 시 양쪽 전부 봐야 함.
- `src-tauri/src/commands/extract_images.rs`: 04-backend-rust 가 1차 risk doc, 08-pdf-ocr-pipeline 이 pipeline 측면. 50-source-mapping 의 backlink = 08. 두 doc 모두 quote 동일 site (FFI loader, lock_pdfium 호출).
- `src/lib/clip-watcher.ts`: 07-llm-integration 가 cite (ingest pipeline 측면) — 실제로는 일종의 stand-alone polling. 별도 mini-domain 일 수도 있지만 cluster size 작아 통합.

## §Suspicious "None observed"

- 03-frontend 의 Rust risk 5개 카테고리 (unsafe / unwrap / panic / Mutex / FFI) 모두 "None observed (TS only)" — 정상. TS 도메인.
- 05-extension 의 Rust risk 5개도 동일 — 정상. JS 도메인.
- 06-data-layer 의 unsafe = "None observed" — 정상 (ts/rs production code).
- 06-data-layer 의 panic! = "None observed" — production 0건 ✓ verified by grep.
- 09-ui-components 의 Rust risk 5개 = "None observed (TS only)" — 정상.
- 04-backend-rust 의 unsafe = "None observed" — 표면적 사실이지만 PDFium dynamic load 시점부터 메모리 안전성 보증 깨짐. 04 doc Internal Risk 에 caveat 명시. False sense of safety 위험.

## §Architectural Trade-offs (의도된 위험)

- **Cargo `panic = "unwind"`** (`Cargo.toml:70`): "Slightly larger binary, but prevents single-file corruption from killing the app." 의도된 architecture choice. abort 였다면 panic_guard 동작 안 함.
- **`tauri-plugin-http` `unsafe-headers` feature** (`Cargo.toml:30`): 임의 헤더 허용 — LLM provider auth 헤더 + 임의 endpoint 지원하기 위함. trust boundary 의도적 약화.
- **Tauri capabilities http allowlist 모든 http/https** (`capabilities/default.json`): 사실상 전 인터넷. LLM provider URL 임의 입력 지원. trust = 사용자 입력에서 시작.
- **Tauri assetProtocol `scope: ["**"]`** (`tauri.conf.json:24-27`): 전체 파일시스템 접근. wiki 페이지가 임의 경로 이미지 표시하기 위함. file traversal 가능성 (markdown-image-resolver 가 일부 검증).
- **clip_server tiny_http 인증 없음**: 같은 머신 안 다른 프로세스가 19827 listen 가능 (port hijack). 의도적 — 로컬 trust 가정.

## §`/team ralph` Audit Pass 결과 (iteration 1)

### Found + fixed
- **50-source-mapping.md anchor 부재** (245 row): `[path](50-source-mapping.md#srcapptsx)` 형식 cross-ref 71개 가 anchor 미존재로 broken. → 각 row 위에 `<a id="<path>"></a>` HTML anchor 추가 (Option A) — 245 anchor 추가, 도메인 doc cross-ref 형식과 정확 일치. 50-source-mapping.md 가 271 → 516 lines.
- **07-llm-integration.md:645 typo**: `srcliblm-clientts` (`lm` 2글자) → `srclibllm-clientts` (`llm` 3글자) 수정.

### Verified (변경 없음)
- 91 risk site count 정확 ✓
- production `unsafe` = 0 ✓
- production `panic!` = 0 (모든 panic! 은 panic_guard 의 `#[cfg(test)]` 안) ✓
- 도메인 doc 의 "None observed" 5 카테고리 모두 정당화 ✓
- 50-source-mapping 의 sample 10 row 모두 substantive purpose ✓
- 모든 production lib `.ts` 가 1+ 도메인 doc 에서 cited (uncited 0) ✓
- vendored / generated / asset / leaf-utility / config-only tag 분류 모두 적절 ✓

### Phase 6 Gate 8 강화 (이번 audit 의 결과)
- 기존: "각 도메인 doc 가 ≥3 mapping anchor reference + ≥1 sibling link"
- 강화: 추가로 "anchor MUST resolve to actual `<a id>` in 50-source-mapping.md"
- 자세히는 `.plan/llm-wiki-analysis/_audit/AUDIT-SUMMARY.md` 참고

## §50-source-mapping Phase 6 게이트 결과 (lead)

전 8 게이트 PASS:

1. Row count == 245 ✓
2. Purpose ≥80 chars (0 위반) ✓
3. Backlink valid (0 위반) ✓
4. Path coverage (0 missing on disk) ✓
5. Reverse coverage (`comm -23` against `git ls-files` = empty) ✓
6. Template instantiation (5/5 sections × 7 domain docs) ✓
7. `\`\`\`rust` quotes in `04-backend-rust.md` = 103 (≥10 게이트) ✓
8. Cross-ref density (≥3 mapping anchors + ≥1 sibling × 7 docs) ✓ — actual: 03=7/4, 04=10/5, 05=5/3, 06=18/8, 07=27/6, 08=7/5, 09=7/5

## §관찰자가 추가 검증 권장

이 분석은 read-only static. dynamic 검증은 안 했어요. 신뢰 전 권장:

- claude-cli 자식 프로세스 lifecycle 실측 (kill 후 process 잔존 여부)
- pdfium FFI 가 corrupt PDF 입력에서 panic_guard 실제 catch 하는지 fuzz
- ingest-queue crash recovery 시 부분 작성된 wiki 페이지 잔존 여부
- LanceDB v1 → v2 마이그레이션 시 vector 정합성 (vector_drop_legacy 후 재인덱스)
- project-mutex 의 1024+ projectPath cycle 메모리 누수 실측
- extension popup CSP 가 inline script 차단하는지 MV3 default 검증

## Cross-refs

- 도메인 doc 전체: [00-overview.md](00-overview.md) 의 reading order
- mapping 검증 자료: [50-source-mapping.md](50-source-mapping.md) (245 행)
- 빌드/CI/번들 trade-off: [80-build-and-tooling.md](80-build-and-tooling.md)
- Top 5 risks 요약: [99-summary.md](99-summary.md)

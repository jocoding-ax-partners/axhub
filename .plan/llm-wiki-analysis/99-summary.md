# 99 — Executive Summary

> **Status:** Phase 7 finalized. 245 source files / 91 Rust risk sites / 22 markdown deliverables / 8 Phase 6 게이트 모두 PASS.

## System Purpose

`llm_wiki` 는 Karpathy의 LLM Wiki 패턴을 데스크톱 앱으로 구현한 **개인 지식베이스 자동 빌더** 예요. 전통적인 RAG (매 쿼리마다 retrieve-and-answer) 대신 LLM이 사용자 문서를 한 번 분석해 영속 wiki 를 만들고 점증적으로 갱신해요. 핵심 특징은 (1) 두 단계 CoT ingest (analyze → generate, source 추적성 + 캐시), (2) 멀티모달 — PDF embedded 이미지 추출 + vision LLM caption, (3) 4-신호 knowledge graph + Louvain 커뮤니티 + Sigma 시각화, (4) RRF 검색 (vector ⊕ keyword), (5) Deep Research (multi-step LLM + 자동 ingest), (6) 비동기 review 시스템, (7) Chrome MV3 webclipper.

기술 스택: **Tauri 2 + React 19 + TypeScript + Rust + LanceDB + pdfium-render**. 빌드 산물 — macOS .dmg / Linux .deb / Windows .msi/.nsis + Chrome extension zip.

## Architecture in One Page

```
Renderer (Vite-served React 19 SPA, port 1420)
  ├─ App.tsx — 7-view 토글 (wiki/sources/search/graph/lint/review/settings)
  ├─ 6 Zustand stores (chat / wiki / review / research / activity / update)
  ├─ Milkdown 7.20 마크다운 에디터 + KaTeX 수식
  ├─ Sigma 3 + graphology + Louvain + forceatlas2 그래프 뷰
  └─ i18next (en / zh)
        │
        │  Tauri IPC (invoke + event, JSON only)
        │  asset protocol scope ** (전체 fs 접근)
        ▼
Tauri Core (Rust, panic = "unwind", LTO, opt-s)
  ├─ commands/{fs, project, vectorstore, claude_cli, extract_images, mod}
  ├─ panic_guard::run_guarded[_async] — third-party 파서 panic catch
  ├─ clip_server (tiny_http :19827) — extension webclipper endpoint, CORS *
  ├─ FFI: pdfium-render → libpdfium.{dylib,so,dll} (vendored)
  └─ tokio::process — claude CLI 자식 spawn + stdout 라인 → emit `claude-cli:{stream_id}`
        │
        │  HTTP outbound (tauri-plugin-http with unsafe-headers)
        │  capability allowlist = 모든 http/https
        ▼
External: LLM provider (OpenAI-compatible) / Vision LLM / Web Search

(병렬 채널)
Chrome MV3 Extension (popup-only, no background SW)
  ├─ manifest: permissions [activeTab, scripting], host_permissions [http://127.0.0.1:19827/*]
  ├─ popup.js → Readability (vendored) + Turndown (vendored) → POST /clip
  └─ web_accessible_resources: Readability.js, Turndown.js for <all_urls>
```

데이터 파이프라인 3 핵심:
- **Ingest**: file/clip → ingest-queue (직렬, crash recovery) → ingest-cache (SHA-256 hit) → ingest-parse (PDF/Office/이미지 분기) → text-chunker → embedding → LanceDB v2 chunk upsert → wiki-graph 재빌드
- **Search**: query → embedding parallel ⊕ keyword index → search-rrf (K=60 fusion) → image-aware results
- **Deep Research**: topic → optimize-research-topic (LLM 다중 query) → web-search (각 query) → ingest-queue 자동 enqueue

## Top 5 Risks (cite path:line)

> 자세히는 [90-risks-gaps.md](90-risks-gaps.md) 참고.

### 1. Trust boundary 의도적 약화 — 모든 외부 communication 사용자 입력에 의존
- `tauri-plugin-http` `unsafe-headers` feature (`src-tauri/Cargo.toml:30`) — 임의 헤더 허용
- `capabilities/default.json:13-25` — 모든 http/https 패턴 allowlist (사실상 전 인터넷)
- `tauri.conf.json:23` CSP `connect-src 'self' https: http:` + `assetProtocol scope: ["**"]` (전체 fs 접근)
- `clip_server.rs` tiny_http 인증 없는 localhost listen — 같은 머신 다른 프로세스 port hijack 가능
- **영향**: LLM provider 임의 endpoint 입력 지원하기 위한 의도적 choice. trust = 사용자 입력에서 시작. 사용자가 신뢰 못 할 endpoint 입력하면 임의 외부 송신 가능.

### 2. 동시성 deadlock surface — `withProjectLock` no-timeouts
- `src/lib/project-mutex.ts:32-73` — projectPath 별 promise-chain mutex, timeout/fairness/re-entrancy 없음
- `src/lib/ingest.ts` autoIngest 가 6 단계 await 모두 lock 안에 직렬화
- **영향**: 한 ingest 의 catastrophic hang (LLM 무한 대기, claude CLI deadlock) 이 같은 프로젝트의 모든 후속 ingest / save-to-wiki / auto-research-ingest 블록. timeout 없어 정지 후 사용자가 재시작 외엔 회복 어려움.
- `project-mutex.ts` cleanup branch dead code: 1024+ distinct projectPath cycle 시 메모리 누수 (worker-3 발견)

### 3. PDFium FFI 안전 boundary — `unsafe` 0개의 false sense of safety
- `pdfium-render = "0.9"` (`src-tauri/Cargo.toml:28`) — safe Rust wrapper
- 호출 사이트: `src-tauri/src/commands/fs.rs:142, 282` (`Pdfium::bind_to_library`, `bind_to_system_library`)
- production code 의 `unsafe` 키워드 = 0
- **영향**: dlopen 으로 C++ PDFium 을 로드하는 순간 메모리 안전 보증 깨짐. 대응: `panic_guard::run_guarded[_async]` (`src-tauri/src/panic_guard.rs`) 가 catch_unwind 로 panic Result 변환 + Cargo `panic = "unwind"` 의존. 한 가정만 깨지면 (panic_guard 우회 / abort profile / pdfium 가 SIGSEGV) 앱 전체 죽음.

### 4. 정적 검출 안 되는 silent error swallow 다수
- TS swallow: 03/05/06/07/09 도메인 doc Internal Risk 에 26 사이트 인용
- Rust swallow (`let _ = expr`): 04 도메인 doc 8 사이트
- 핵심: `auto-save.ts` 의 `.catch(() => {})` (06), `claude-cli` abort silent dispatch (07), extension popup 빈 catch 3곳 (05), graph-view readFile fallbacks (09), App.tsx init catch-all (03)
- **영향**: 디스크 가득 / 권한 실패 / 네트워크 끊김 / LLM 503 등 모든 일시적 실패가 사용자 / 콘솔 어디에도 표면화 안 됨. 디버깅 불가능. 테스트 0건.

### 5. 데이터 일관성 boundary — non-atomic write + zombie state
- `src/lib/persist.ts` `saveChatHistory`: conversations.json + N 개 chats/<id>.json 순차 덮어씀 — 중간 crash 시 phantom 메타 (worker-3 발견)
- `src/lib/project-identity.ts` write 실패 zombie: identity write fail → caller 한테 새 UUID 반환되지만 디스크엔 없음 → 재부팅마다 같은 프로젝트 다른 ID
- `src/lib/ingest-queue.ts` crash recovery 부분만 — restoreQueue 가 processing → pending 되돌리지만 mid-ingest crash 시 disk partial wiki page 잔존, cleanupWrittenFiles 안 거침
- TS↔Rust drift: `WikiProject.id` (TS 필수, Rust 없음) — `ensureProjectId()` 가 매번 채움
- **영향**: 부팅마다 invariant 가 다를 수 있음. 같은 프로젝트가 다른 UUID 로 인식 → recentProjects 중복 / projectRegistry 충돌. recovery 가 best-effort 라 사용자가 직접 disk 정리 필요할 수도.

## What's Solid

- **Test infrastructure**: vitest mocks vs real-llm 분리 (`package.json:11-13`) + scenarios + property (fast-check 4) + race + integration suffix 분류로 견고. 70+ test 파일.
- **Cross-platform CI**: `.github/workflows/ci.yml` 3 OS 매트릭스 (macos / ubuntu-22.04 / windows) — protoc 분기 + libwebkit2gtk + rust-cache. 각 platform 빌드 가시성.
- **Release pipeline**: `build.yml` Apple 코드 서명 (6 secrets) + tauri-action + extension zip 자동 동기화 (manifest version → package.json version).
- **panic_guard architecture**: third-party 파서 (pdfium / calamine / docx-rs) panic 의도된 design. abort 였다면 단일 파일이 앱 죽임.
- **RRF search**: `search-rrf.ts` rank-only fusion (K=60). vector 와 keyword 점수 incommensurable 문제 해결. Tie-break = path 알파벳.
- **claude-cli lifecycle discipline**: `claude_cli.rs:289-298` 주석 자체가 "Don't hold the map lock across .wait() — kill could race." 명시 + 패턴 강제 (child remove → lock drop → wait).
- **Sigma WebGL force-remount workaround**: 크래시 ("could not find suitable program for node type circle") 회피용 `data-panel-resizing` body attribute MutationObserver — fix 가 아니지만 race window 늘려 우회 (graph-view.tsx:435-451).
- **Chrome extension scope tightening**: `host_permissions` 가 `http://127.0.0.1:19827/*` 단일 호스트 — broad permission 회피.
- **i18n parity test**: `i18n-parity.test.ts` 가 en/zh 키 set 차집합 0 강제. 로케일 drift 자동 catch.

## What I'd Verify Before Trusting

> static read-only 분석. 다음은 dynamic / fuzz 가 필요한 항목 — 신뢰 전 검증 권장.

1. **panic_guard 실측 catch**: corrupt PDF / docx / xlsx 입력 fuzz. `panic = "unwind"` profile 이 디버그 빌드에서도 동작하는지.
2. **claude-cli 자식 프로세스 정리**: `claude_cli_kill` 호출 후 process tree 잔존 여부 (특히 abort silent dispatch 후). `kill_on_drop` 동작 검증.
3. **ingest-queue crash recovery**: mid-ingest force kill 후 부팅. partial wiki page 잔존 + recovery 동작 검증. `cleanupWrittenFiles` 누락 케이스.
4. **LanceDB v1 → v2 마이그레이션**: `vector_drop_legacy` 후 재인덱스 시 vector 정합성 + 마이그레이션 도중 crash 시 recovery.
5. **project-mutex 의 1024+ projectPath cycle**: 메모리 누수 실측 (worker-3 가 dead code branch 식별).
6. **Extension popup CSP**: MV3 default CSP 가 inline-script 차단하는지 — popup.js:53 의 innerHTML 보간 XSS 가능성.
7. **clip_server port hijack**: 같은 머신 다른 프로세스가 19827 listen 시 extension 동작.
8. **persist.ts mid-write crash**: `saveChatHistory` 의 conversations.json + chats/<id>.json 다중 파일 mid-write force kill 시 partial state 자동 복구 가능 여부.
9. **Tauri assetProtocol scope ["**"] file traversal**: `markdown-image-resolver.ts` 의 검증이 임의 path 입력 차단하는지.
10. **CI 시험 커버**: `ci.yml` 이 `vite build` + `cargo build` 만 — `npm test` 호출 0 건. PR 마다 mock test 실행 여부 / Tauri 번들 검증 미포함.

## ADR Reference

[PLAN.md §6 ADR](PLAN.md#6-adr) — Decision: template-driven domain docs + cluster-derived cuts + content-verified Phase 6. Drivers: 빠짐없이 + auditability + drift control. Why: only option satisfying floor (245 mapped) + ceiling (5-section template).

## 분석 Coverage 통계

- **Source files mapped**: 245 / 245 (100%)
- **Domain docs with 5-section template**: 7 / 7 (100%)
- **Rust risk sites quoted**: 91 / 91 (100%)
- **Phase 6 mechanical gates**: 8 / 8 (100% PASS)
- **gitnexus clusters absorbed**: top 20 of 154 (covers ~21% of graph node mass; long-tail 134 small clusters absorbed via best-fit or `[leaf-utility]` tag)
- **Output files**: 17 markdown (PLAN, _template, 00 / 01 / 02 / 03 / 04 / 05 / 06 / 07 / 08 / 09 / 50 / 80 / 90 / 99) — 22 file plan 의 10..18 reserved 슬롯 미사용 (top cluster 가 03..09 best-guess 와 깔끔히 정렬해서 별도 슬롯 불필요).

## Cross-refs

- 청사진 + ADR: [PLAN.md](PLAN.md)
- TOC + reading order: [00-overview.md](00-overview.md)
- 245-row index: [50-source-mapping.md](50-source-mapping.md)
- Risk 자세히: [90-risks-gaps.md](90-risks-gaps.md)

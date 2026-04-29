# 03 — Frontend (React 19 + TypeScript)

> 5-section template instantiation (Phase 4). 모든 인용은 `/tmp/llm_wiki_inspect/` HEAD `1434e08` 기준 raw line 번호예요.

## Purpose

`llm_wiki` 의 React 19 렌더러 레이어예요. Vite 가 번들한 SPA 가 Tauri 2 webview 안에서 단일 `App` 컴포넌트로 부팅되고, 그 위에 7 개 view (`wiki | sources | search | graph | lint | review | settings`) 를 한 화면에서 토글하는 "view-mode" 라우팅 모델을 따라요. 라우터 라이브러리 없이 `useWikiStore.activeView` enum 한 개로 모든 화면 전환을 처리해요 — URL 이 의미를 갖지 않는 데스크톱 앱이라 의도적으로 가벼운 선택이에요. 전역 상태는 Zustand 스토어 6 개 (`wiki / chat / review / research / activity / update`) 로 잘게 쪼개 도메인별 cross-talk 을 줄였고, i18next + 한국어/중국어/영어 번역 번들 (현재 en.json + zh.json 만 출하) 가 prompt 외 UI 카피를 담당해요. 프런트엔드 경계 안에서 끝나지 않고 `@tauri-apps/api/core::invoke` 로 Rust 커맨드를 부르거나 `fetch("http://127.0.0.1:19827")` 로 로컬 clip-server 와 직접 IPC 하기 때문에, 트러스트 바운더리는 webview ↔ Rust core ↔ Chrome extension 세 점이 맞물려요. `App.tsx` 가 마운트 시 set up 하는 부수효과 (auto-save, clip-watcher, update-check, last-project restore) 가 사실상의 lifecycle hook 컬렉션이고, view 라우팅은 `AppLayout` 에 위임해 `App` 자체는 가벼운 어드미션 컴포넌트예요.

## Public Interface

- `default App — () => JSX.Element — src/App.tsx:17-389 — 부팅·라이프사이클·라우팅 디스패처. project null 이면 WelcomeScreen, 아니면 AppLayout`
- `ReactDOM.createRoot(...).render(<App/>) — src/main.tsx:7-11 — 단일 root mount, StrictMode + i18n side-effect import`
- `__APP_VERSION__ — declare const string — src/vite-env.d.ts:4 — Vite define 으로 package.json version 주입, update-check 비교에 사용`
- `useWikiStore — Zustand store — src/stores/wiki-store.ts:185-252 — project / fileTree / selectedFile / fileContent / activeView / llmConfig / providerConfigs / activePresetId / searchApiConfig / embeddingConfig / multimodalConfig / outputLanguage / dataVersion + setters + bumpDataVersion`
- `useChatStore — Zustand store — src/stores/chat-store.ts:69-228 — conversations / messages / isStreaming / streamingContent / mode / ingestSource + createConversation / addMessage / appendStreamToken / finalizeStream / removeLastAssistantMessage`
- `useReviewStore — Zustand store — src/stores/review-store.ts:35-117 — items + addItem / addItems (de-dup by normalizeReviewTitle) / resolveItem / dismissItem / clearResolved`
- `useResearchStore — Zustand store — src/stores/research-store.ts:31-80 — tasks queue (queued|searching|synthesizing|saving|done|error) + addTask / updateTask / getRunningCount / getNextQueued + maxConcurrent=3`
- `useActivityStore — Zustand store — src/stores/activity-store.ts:23-55 — running activity feed (ingest|lint|query) + addItem returning id / appendDetail / clearDone`
- `useUpdateStore — Zustand store — src/stores/update-store.ts:34-47 — checking / lastResult / lastCheckedAt / dismissedVersion / enabled + setChecking / setResult / setDismissed / hydrate`
- `hasAvailableUpdate — (state: UpdateStoreState) => boolean — src/stores/update-store.ts:59-62 — passive 빨간 점 dot 결정`
- `shouldShowUpdateBanner — (state: UpdateStoreState) => boolean — src/stores/update-store.ts:76-87 — banner gate (dismissedVersion ≠ remote 일 때만)`
- `chatMessagesToLLM — (messages: DisplayMessage[]) => ChatMessage[] — src/stores/chat-store.ts:230-235 — UI 메시지 → LLM payload 매핑`
- `i18n — i18next instance — src/i18n/index.ts:6-16 — initReactI18next + en/zh resources, lng="en", fallbackLng="en"`
- `flattenKeys — (obj: unknown, prefix?: string) => string[] — src/i18n/i18n-parity.test.ts:18-30 — 도트 경로 평탄화, en↔zh 키 parity 가드`
- `readFile — async (path: string) => Promise<string> — src/commands/fs.ts:11-13 — Tauri "read_file" wrapper`
- `writeFile — async (path: string, contents: string) => Promise<void> — src/commands/fs.ts:15-17 — Tauri "write_file" wrapper`
- `listDirectory — async (path: string) => Promise<FileNode[]> — src/commands/fs.ts:19-21 — 디렉터리 트리 invoke`
- `copyFile / preprocessFile / deleteFile / createDirectory / fileExists — async — src/commands/fs.ts:23-51 — 파일 시스템 mutate Tauri wrappers`
- `findRelatedWikiPages — async (projectPath, sourceName) => Promise<string[]> — src/commands/fs.ts:38-43 — wiki 역참조 lookup invoke`
- `readFileAsBase64 — async (path: string) => Promise<FileBase64> — src/commands/fs.ts:65-67 — vision-caption 용 base64 + mime invoke`
- `createProject — async (name, path) => Promise<WikiProject> — src/commands/fs.ts:69-77 — project create + ensureProjectId + upsertProjectInfo wrap`
- `openProject — async (path) => Promise<WikiProject> — src/commands/fs.ts:79-84 — open + identity reconcile`
- `clipServerStatus — async () => Promise<string> — src/commands/fs.ts:86-88 — clip-server health invoke`
- `ErrorBoundary — class Component<Props, State> — src/components/error-boundary.tsx:13-45 — getDerivedStateFromError → fallback 카드 + Retry 버튼; componentDidCatch 는 console.error 만 함`
- `getOutputLanguage — (fallbackText?: string) => string — src/lib/output-language.ts:10-16 — auto 면 detectLanguage, 아니면 configured`
- `buildLanguageDirective — (fallbackText?: string) => string — src/lib/output-language.ts:21-32 — 시스템 프롬프트 강제 언어 블록`
- `buildLanguageReminder — (fallbackText?: string) => string — src/lib/output-language.ts:37-40 — 짧은 언어 리마인더`
- `OUTPUT_LANGUAGE_OPTIONS — readonly array — src/lib/output-language-options.ts:17-39 — 21 개 언어 + auto, label/value 쌍`
- `detectLanguage — (text: string) => string — src/lib/detect-language.ts:5-47 — Unicode script + Latin diacritics + 공통 단어 두 단계 분류`
- `isGreeting — (text: string) => boolean — src/lib/greeting-detector.ts:45-60 — 화이트리스트 정규식 13 종, 길이 cap 20`
- `convertLatexToUnicode — (text: string) => string — src/lib/latex-to-unicode.ts:58-75 — \$\\cmd\$ / \$\$..\$\$ / \$..\$ 를 Unicode 글리프로 치환`
- `normalizePath / joinPath / getFileName / getFileStem / getRelativePath / isAbsolutePath — src/lib/path-utils.ts:5-64 — 크로스플랫폼 경로 헬퍼 (Windows + POSIX 동시 인식)`
- `cn — (...inputs: ClassValue[]) => string — src/lib/utils.ts:4-6 — twMerge(clsx(...)) Tailwind 클래스 머지`
- `WikiTemplate / researchTemplate / ... — interface + objects — src/lib/templates.ts:1-654 — wiki 스키마 템플릿 (research / general / project) — Phase 4 ingest 가 사용자에게 보여주는 frontmatter 규약`
- `runStructuralLint — async (projectPath: string) => Promise<LintResult[]> — src/lib/lint.ts:69-… — wikilink/orphan/broken-link 린팅 진입점`

## Internal Risk

### unsafe blocks (Rust)

None observed in this domain. (TS only.)

### `.unwrap()` / `.expect()` chains (Rust)

None observed in this domain. (TS only.)

### `panic!` / `unreachable!` / `todo!` (Rust)

None observed in this domain. (TS only.)

### `Mutex::lock` / `RwLock::write` acquisition + drop discipline (Rust)

None observed in this domain. (TS only.)

### FFI loads, `extern "C"`, dlopen-style (Rust → pdfium et al.)

None observed in this domain. (TS only.)

### Result swallow (TypeScript)

`as unknown as` window cast (devtools 스텁이라 타입 검증 없이 전역 객체 mutate). 두 번 — 첫 번째는 store 노출, 두 번째는 banner 트리거 함수 노출:

```typescript src/App.tsx:47
;(window as unknown as { __llmwiki_updateStore?: typeof useUpdateStore }).__llmwiki_updateStore = useUpdateStore
```

```typescript src/App.tsx:48
;(window as unknown as { __llmwiki_testUpdateBanner?: (clear?: boolean) => void }).__llmwiki_testUpdateBanner = (clear = false) => {
```

빈 `catch {}` — last-project 복구 실패를 통째로 삼킴 (사용자에게 표시되지 않고 startup-time race 만 디버깅 어렵게 함):

```typescript src/App.tsx:234-236
            await handleProjectOpened(proj)
          } catch {
            // Last project no longer valid
          }
```

`init()` 의 catch-all — savedConfig / providerConfigs / preset resolve / search / embedding / multimodal / outputLanguage / language / lastProject 어느 단계에서 throw 해도 무음 fallthrough:

```typescript src/App.tsx:238-240
      } catch {
        // ignore init errors
      } finally {
```

update-check 의 silent-on-failure (1.5 s 지연 IIFE 안의 모든 throw 를 흡수):

```typescript src/App.tsx:165-167
      } catch {
        // Silent — Settings → About lets the user retry manually.
      }
```

`handleProjectOpened` 의 review/chat 영속화 복구 — disk 손상 시 빈 스토어로 fallthrough (silent recovery):

```typescript src/App.tsx:298-300
      }
    } catch {
      // ignore, start fresh
    }
```

```typescript src/App.tsx:313-315
      }
    } catch {
      // ignore, start fresh
    }
```

`handleSelectRecent` / `handleOpenProject` 는 throw 를 `window.alert(\`Failed to open project: ${err}\`)` 로 흘려요 — `err` 의 raw 스트링이 사용자에게 노출되는데 서버 사이드 stack-trace 가 그대로 alert 박스로 새는 위험:

```typescript src/App.tsx:322-324
    } catch (err) {
      window.alert(`Failed to open project: ${err}`)
    }
```

```typescript src/App.tsx:337-339
    } catch (err) {
      window.alert(`Failed to open project: ${err}`)
    }
```

console-only error (rethrow 없음) — 큐 복원 실패가 사용자 액션 차단 없이 콘솔에만 남고 앱은 계속 진행:

```typescript src/App.tsx:266-268
      restoreQueue(proj.id, proj.path).catch((err) =>
        console.error("Failed to restore ingest queue:", err)
      )
```

```typescript src/App.tsx:289-291
    } catch (err) {
      console.error("Failed to load file tree:", err)
    }
```

clip-server `fetch` 호출의 promise rejection 무음 처리 — 19827 포트가 죽어있어도 항상 `.catch(() => {})` 로 묻힘:

```typescript src/App.tsx:271-275
    fetch("http://127.0.0.1:19827/project", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: proj.path }),
    }).catch(() => {})
```

```typescript src/App.tsx:280-285
      fetch("http://127.0.0.1:19827/projects", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ projects }),
      }).catch(() => {})
    }).catch(() => {})
```

`ErrorBoundary` 도 `componentDidCatch` 에서 console.error 만 — 텔레메트리 없이 fallback 카드만 보여줌:

```typescript src/components/error-boundary.tsx:23-25
  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info.componentStack)
  }
```

`runStructuralLint` 의 wiki 디렉터리 누락 / 개별 파일 읽기 실패도 빈 catch 로 흘려보내요 — orphan 검사 결과의 신뢰도가 디스크 상태에 따라 바뀌는데 사용자에게는 표면화되지 않음:

```typescript src/lib/lint.ts:73-76
  let tree: FileNode[]
  try {
    tree = await listDirectory(wikiRoot)
  } catch {
    return []
  }
```

dev-only `console.log` 묶음이 production 빌드에서도 살아남아요 — `import.meta.env.DEV` 가드는 banner 주입 IIFE 만 감싸고 update-check 라인 (`App.tsx:131 / 140 / 150 / 154 / 158`) 은 게이트되지 않아요. 사용자 콘솔에 `[update-check]` 메시지가 항상 출력돼요:

```typescript src/App.tsx:130-159
          console.log(
            `[update-check] skipped: cache hit (last check ${ageMin} min ago, ` +
              `cache window ${UPDATE_CHECK_CACHE_MS / 60_000} min). ` +
              `Last result: kind=${state.lastResult?.kind ?? "none"}`,
          )
```

## Cross-refs

- Tauri invoke surface 전체는 [04-backend-rust.md#evidence](04-backend-rust.md#evidence) — `read_file`, `write_file`, `list_directory`, `read_file_as_base64`, `create_project`, `open_project`, `clip_server_status` 명령 정의가 거기 있어요.
- 영속 store + 외부 IPC (`fetch http://127.0.0.1:19827`) 는 [06-data-layer.md](06-data-layer.md) 와 동일 트러스트 바운더리를 공유해요 — clip-watcher / project-store / persist 모듈이 그쪽에 묶여요.
- `llmConfig` / `providerConfigs` / `multimodalConfig` 는 [07-llm-integration.md](07-llm-integration.md) 의 provider preset 시스템에서 hydrate 돼요.
- React 19 컴포넌트 트리 (AppLayout / WelcomeScreen / CreateProjectDialog 외) 는 [09-ui-components.md](09-ui-components.md) 에서 다뤄요.
- 소스 매핑 행: [src/App.tsx](50-source-mapping.md#srcapptsx), [src/main.tsx](50-source-mapping.md#srcmaintsx), [src/stores/wiki-store.ts](50-source-mapping.md#srcstoreswiki-storets), [src/stores/chat-store.ts](50-source-mapping.md#srcstoreschat-storets), [src/i18n/index.ts](50-source-mapping.md#srci18nindexts), [src/commands/fs.ts](50-source-mapping.md#srccommandsfsts), [src/components/error-boundary.tsx](50-source-mapping.md#srccomponentserror-boundarytsx).

## Evidence

- `src/App.tsx:1-15` — 모든 import 가 `@/` alias 경유. 라우터 없이 store + Tauri invoke 로 부팅.
- `src/App.tsx:27-30` — `setupAutoSave()` + `startClipWatcher()` 마운트 시 1 회 실행.
- `src/App.tsx:40-83` — DEV-only banner-UX 테스트 헬퍼 IIFE, `import.meta.env.DEV` 가드 안에서 window 객체 mutate (`as unknown as` 두 번).
- `src/App.tsx:92-173` — 백그라운드 update-check setTimeout(1500) IIFE; cache key 는 `lastCheckedAt` AND `lastResult` 동시 존재 요구; cancelled 플래그로 cleanup.
- `src/App.tsx:114-128` — fresh 캐시 판정. 한 줄 주석에 사용자 보고 버그 ("kind=none, no banner") 의 fix 의도 명시.
- `src/App.tsx:165-167` — update-check try/catch 가 모든 실패를 무음 흡수.
- `src/App.tsx:176-245` — `init()` 마운트 IIFE — config preset 9 종 hydrate (`loadLlmConfig` → `loadProviderConfigs` → `loadActivePresetId` → preset re-resolve → `loadSearchApiConfig` → `loadEmbeddingConfig` → `loadMultimodalConfig` → `loadOutputLanguage` → `loadLanguage` → `getLastProject`).
- `src/App.tsx:189-208` — preset resolve 가 동적 import (`@/components/settings/llm-presets`, `preset-resolver`) — 부팅 critical path 에서 lazy chunking.
- `src/App.tsx:247-316` — `handleProjectOpened` 가 reset → setProject → bumpDataVersion → saveLastProject → restoreQueue → fetch 19827 (project + projects) → listDirectory → loadReviewItems → loadChatHistory → setActiveConversation 까지 sequential coordinator.
- `src/App.tsx:271-285` — clip-server 19827 IPC 두 번 (POST /project + POST /projects) — `fetch` 의 `.catch(() => {})` 두 곳.
- `src/App.tsx:344-350` — `handleSwitchProject` 가 `resetProjectState` 후 store 클리어. 비동기 race 에서 stale 데이터 새지 않게 await 부터.
- `src/main.tsx:1-11` — 7-라인 entry. `<App/>` 을 `StrictMode` 안에서 렌더, `import "@/i18n"` 의 side-effect (`i18n.use(initReactI18next).init`) 로 부팅.
- `src/index.css:1-130` — Tailwind v4 `@import "tailwindcss"` + `tw-animate-css` + `shadcn/tailwind.css` + Geist Variable. `@custom-variant dark` 로 dark-mode 토글, oklch 컬러 팔레트.
- `src/vite-env.d.ts:1-5` — `<reference types="vite/client">` + `__APP_VERSION__: string` declare (Vite `define` 으로 package.json 에서 주입).
- `src/stores/wiki-store.ts:10-11` — `CustomApiMode = "chat_completions" | "anthropic_messages"`.
- `src/stores/wiki-store.ts:12-20` — `LlmConfig` provider 7 종 (`openai|anthropic|google|ollama|custom|minimax|claude-code`) + `apiKey/model/ollamaUrl/customEndpoint/maxContextSize/apiMode`.
- `src/stores/wiki-store.ts:97-118` — `OutputLanguage` 21 가지 (`auto` 포함).
- `src/stores/wiki-store.ts:135-183` — `WikiState` 인터페이스 — 14 개 필드 + 14 setters + `bumpDataVersion`.
- `src/stores/wiki-store.ts:156` — `activeView` enum — `wiki|sources|search|graph|lint|review|settings` (라우터 대신).
- `src/stores/wiki-store.ts:185-251` — store 인스턴스화. `multimodalConfig.enabled = false` 가 기본 (token-spend opt-in).
- `src/stores/chat-store.ts:4-23` — `Conversation` + `MessageReference` + `DisplayMessage` 타입. `references?: MessageReference[]` 가 인용 페이지 보존.
- `src/stores/chat-store.ts:58-67` — `messageCounter` 단순 increment + `generateConversationId` 는 `Date.now() + Math.random().toString(36)`.
- `src/stores/chat-store.ts:118-152` — `addMessage` 가 first-user-message 50 자를 conversation title 로 자동 사용.
- `src/stores/chat-store.ts:165-194` — `finalizeStream` 가 isStreaming 플립 + 메시지 push 를 atomic 한 set 안에서.
- `src/stores/chat-store.ts:209-221` — `removeLastAssistantMessage` 가 regenerate 시 마지막 응답만 pop.
- `src/stores/chat-store.ts:230-235` — `chatMessagesToLLM` 가 UI → LLM payload 매핑.
- `src/stores/review-store.ts:9-21` — `ReviewItem` 5 종 type (`contradiction|duplicate|missing-page|confirm|suggestion`).
- `src/stores/review-store.ts:51-97` — `addItems` 가 `normalizeReviewTitle` 키로 pending dedup + `affectedPages` / `searchQueries` 머지.
- `src/stores/research-store.ts:6-14` — `ResearchTask` 6 단계 status (queued|searching|synthesizing|saving|done|error) + `webResults` / `synthesis` / `savedPath`.
- `src/stores/research-store.ts:35` — `maxConcurrent: 3` 기본.
- `src/stores/research-store.ts:69-79` — `getRunningCount` (searching+synthesizing+saving) + `getNextQueued`.
- `src/stores/activity-store.ts:3-12` — `ActivityItem` 3 종 (`ingest|lint|query`) + `filesWritten: string[]`.
- `src/stores/activity-store.ts:51-54` — `clearDone` 가 running 만 남김.
- `src/stores/update-store.ts:11-32` — `UpdateStoreState` 5 필드 + 5 setters.
- `src/stores/update-store.ts:34-47` — store 본체. `enabled: true` 기본.
- `src/stores/update-store.ts:59-87` — passive `hasAvailableUpdate` vs active `shouldShowUpdateBanner` 분리, banner = active interruption / dots = passive presence 주석.
- `src/i18n/index.ts:1-16` — i18next + initReactI18next, `lng:"en"`, `fallbackLng:"en"`, `escapeValue:false`.
- `src/i18n/i18n-parity.test.ts:18-30` — `flattenKeys` 재귀, dot-path. 4 개 it: en→zh missing / zh→en orphan / non-empty leaf / `_plural` ↔ singular pair.
- `src/i18n/en.json` 240 라인, `src/i18n/zh.json` 240 라인 — 라인 수가 같지만 parity test 가 키 단위 결성 보장.
- `src/commands/fs.ts:1-2` — `invoke` from `@tauri-apps/api/core` + `WikiProject / FileNode` types.
- `src/commands/fs.ts:6-9` — `RawProject` shape — Rust 가 `id` 미반환, 클라이언트가 `ensureProjectId` 로 부착.
- `src/commands/fs.ts:11-89` — 12 개 Tauri invoke wrapper.
- `src/commands/fs.ts:54-58` — `FileBase64 { base64, mimeType }` Rust mirror — vision-caption 파이프라인 진입.
- `src/commands/fs.ts:69-84` — `createProject` / `openProject` 가 `RawProject → ensureProjectId → upsertProjectInfo → WikiProject` 3 단계 reconcile.
- `src/components/error-boundary.tsx:13-45` — class Component, `getDerivedStateFromError` 동기 + `componentDidCatch` 비동기 로깅. fallback 미지정 시 destructive 색상 카드 + Retry 버튼.
- `src/lib/output-language.ts:10-40` — `auto` → `detectLanguage` fallback. `buildLanguageDirective` 가 system-prompt 7-라인 강제 블록 (대문자 MANDATORY + 다른 모든 instructions override).
- `src/lib/output-language-options.ts:17-39` — 21 개 항목 readonly tuple. value 가 `OutputLanguage` 와 byte-동기.
- `src/lib/detect-language.ts:5-47` — 두 단계: Unicode script 카운트 (Japanese vs Chinese 경합 시 Japanese 우선) → Latin diacritics + 공통 단어. 22 개 script range.
- `src/lib/detect-language.ts:166-169` — Vietnamese 는 VN 전용 톤마크만 매치하도록 좁혀진 history (이전 false-positive 버그 fix).
- `src/lib/detect-language.ts:173-176` — Turkish 는 `ğış` + 공통 단어 둘 다 요구 (`ç/ö/ü` 단독 매치 제거).
- `src/lib/detect-language.ts:208-213` — Portuguese 가 Spanish 보다 먼저 (PT char 가 더 좁음). 주석에 "running ES first steals legitimate PT text" 명시.
- `src/lib/greeting-detector.ts:13` — `MAX_GREETING_LEN = 20`.
- `src/lib/greeting-detector.ts:18` — `TRAILING_PUNCT` 가 trailing 만 strip — 시작 점 (`.`) 은 인사말 아님.
- `src/lib/greeting-detector.ts:20-43` — 13 종 정규식: 영어 4 / 중국어 3 / 일본어 1 / 한국어 1 / 유럽 1 (스페인+프랑스+독일+스칸디 등 묶음).
- `src/lib/latex-to-unicode.ts:2-52` — 약 200 개 LaTeX command → Unicode 매핑.
- `src/lib/latex-to-unicode.ts:62-72` — `$\cmd$` → glyph, `$$..$$` → newline-padded, `$..$` → 인라인 LaTeX command 만 변환 (모르는 cmd 는 backslash 보존).
- `src/lib/path-utils.ts:5-7` — Windows backslash → forward slash normalize.
- `src/lib/path-utils.ts:58-64` — `isAbsolutePath` 가 `/`, `C:/`, `C:\\`, UNC `\\\\server`, `//server` 모두 인식 — 이중-join 버그 fix history 주석.
- `src/lib/utils.ts:1-6` — `cn = twMerge(clsx(...))` 만 export (shadcn 표준).
- `src/lib/templates.ts:1-67` — wiki schema base 상수 (BASE_SCHEMA_TYPES, BASE_NAMING, BASE_FRONTMATTER, BASE_INDEX_FORMAT, BASE_LOG_FORMAT, BASE_CROSSREF, BASE_CONTRADICTION).
- `src/lib/lint.ts:31-39` — `extractWikilinks` regex `\[\[([^\]|]+?)(?:\|[^\]]+?)?\]\]/g` — pipe-aliased 위키링크 지원.
- `src/lib/lint.ts:51-65` — `buildSlugMap` 키 lowercase + relative path + basename 둘 다 인덱싱 (case-insensitive 매칭).
- `src/lib/lint.ts:73-76` — wiki 디렉터리 못 읽으면 빈 배열 (silent fallthrough).
- 위험 grep 결과 요약: `src/App.tsx` 에 `as unknown as` 2 회 (47, 48), 빈 `catch {}` 4 회 (165, 234, 238, 298, 313), `console.error` w/o rethrow 2 회 (267, 290), `console.log` 6 회 (55, 78, 110, 131, 140, 150, 154, 158) — 일부는 DEV 가드 밖. `src/lib/lint.ts` 에 `catch {}` 4 회 (74, 96, 182, 199). 다른 store/i18n/utility 파일에서는 risk 패턴 미관측.

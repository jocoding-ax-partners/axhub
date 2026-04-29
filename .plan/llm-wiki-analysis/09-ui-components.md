# 09 — UI Components (Sigma Graph + Milkdown Editor + shadcn / base-ui)

> 컴포넌트 contract 와 visualization wiring 에 집중해요. 도메인 데이터 흐름은 03/04/06/07/11 에서 다뤄요.

## Purpose

`src/components/` 는 React 19 함수형 컴포넌트 42 개 (.tsx) 를 7 개 폴더 (`chat / editor / graph / layout / lint / project / review / search / settings / sources / ui`) 로 묶은 UI 레이어예요. 디자인 시스템 베이스는 shadcn `components.json` (style: `base-nova`, baseColor: `neutral`, iconLibrary: `lucide`, alias `@/components`) 기반이고, primitive 는 `@base-ui/react` (Button / Dialog / Tooltip / ScrollArea) + `react-resizable-panels` 로 깔려 있어요. 응용 시각화는 두 가지 — (1) `@milkdown/react` 기반 WYSIWYG 마크다운 에디터 (commonmark + gfm + math + history + listener 5 plugin), (2) `@react-sigma/core` + `graphology` + `graphology-layout-forceatlas2` 기반 지식 그래프 캔버스. 레이아웃은 `AppLayout` 이 manual mouse-drag resizer (한 컴포넌트 안 mousemove/mouseup listener 직접 등록) 로 좌우 패널 폭 (150-400 / 250-50% clamp) 을 유지하고, `data-panel-resizing` body attribute 를 모든 시각화 컴포넌트가 MutationObserver 로 받아 sigma WebGL canvas 를 강제 unmount/remount 하는 신호로 써요 — Sigma 가 외부 layout 변경 중 "could not find suitable program for node type circle" 크래시를 일으키는 알려진 버그 회피 코드예요. 디자인 토큰은 `index.css` 의 oklch 컬러 + Geist Variable 폰트 + Tailwind v4 `@theme inline` 으로 묶이고 light/dark 두 모드 모두 정의돼 있어요. 컴포넌트 트러스트 surface 는 (a) Milkdown listener 의 first-emit race (initial parse 이벤트를 onSave 로 흘리지 않게 ref guard), (b) Sigma 의 layout-key 기반 force remount, (c) `ErrorBoundary` 클래스 컴포넌트가 SigmaContainer / 우측 패널 양쪽을 감싸는 fault isolation 세 점이에요.

## Public Interface

- `WikiEditor — ({content: string, onSave: (md:string)=>void}) => JSX — src/components/editor/wiki-editor.tsx:65-75 — MilkdownProvider 래퍼, wrapBareMathBlocks 전처리 후 WikiEditorInner 렌더`
- `WikiEditorInner — internal — src/components/editor/wiki-editor.tsx:18-51 — Editor.make() 체인 (nord 테마 + commonmark + gfm + math + history + listener), initialEmitConsumedRef 로 첫 emit drop`
- `wrapBareMathBlocks — (text: string) => string — src/components/editor/wiki-editor.tsx:58-63 — \begin{}..\end{} 가 \$\$..\$\$ 로 감싸이지 않은 경우 자동 wrap`
- `GraphView — () => JSX — src/components/graph/graph-view.tsx:298-877 — 메인 그래프 페이지. dataVersion gating + buildWikiGraph + insights 패널 + Deep Research dialog`
- `GraphLoader — internal — src/components/graph/graph-view.tsx:93-166 — graphology Graph 빌드 + ForceAtlas2 layout 150 iter (positionCache map 으로 re-layout skip)`
- `HighlightManager — internal — src/components/graph/graph-view.tsx:168-206 — highlightedNodes Set 동기화로 node/edge attribute mutate + sigma.refresh`
- `EventHandler — internal — src/components/graph/graph-view.tsx:208-252 — clickNode/enterNode/leaveNode 등록, neighbor highlight + dim`
- `ZoomControls — internal — src/components/graph/graph-view.tsx:254-294 — animatedZoom/animatedUnzoom/animatedReset 3 버튼`
- `ErrorBoundary — class Component — src/components/error-boundary.tsx:13-45 — getDerivedStateFromError + componentDidCatch (console.error 만)`
- `AppLayout — ({onSwitchProject: ()=>void}) => JSX — src/components/layout/app-layout.tsx:19-162 — 좌 220 / 우 400 초기, mousedown→mousemove→mouseup 으로 manual resize, isSettings 분기로 좌측 패널 hide`
- `IconSidebar / SidebarPanel / ContentArea / PreviewPanel / ResearchPanel / ActivityPanel / UpdateBanner / ChatBar / FileTree / KnowledgeTree — src/components/layout/*.tsx — 레이아웃 노드들; AppLayout 가 wire`
- `ChatPanel / ChatInput / ChatMessage — src/components/chat/*.tsx — 대화 UI 트리`
- `FilePreview — internal preview component — src/components/editor/file-preview.tsx — 마크다운 read-only 렌더`
- `LintView / ReviewView / SearchView / SourcesView — src/components/<view>/<view>.tsx — activeView 별 main pane`
- `CreateProjectDialog / TemplatePicker / WelcomeScreen — src/components/project/*.tsx — 프로젝트 생성 / 템플릿 / 환영 화면`
- `SettingsView + 7 sections (about / embedding / interface / llm-provider / multimodal / output / web-search) — src/components/settings/**.tsx — 설정 UI 트리; ContextSizeSelector 헬퍼 포함`
- `Button — base-ui Button + cva variants — src/components/ui/button.tsx:1-58 — 6 variant (default/outline/secondary/ghost/destructive/link) × 9 size (default/xs/sm/lg/icon/icon-xs/icon-sm/icon-lg) Tailwind 매트릭스`
- `Dialog / DialogTrigger / DialogPortal / DialogClose / DialogOverlay / DialogContent / DialogHeader / DialogFooter / DialogTitle / DialogDescription — src/components/ui/dialog.tsx:1-160 — base-ui Dialog 래핑 + lucide XIcon close button`
- `Input / Label / Separator — src/components/ui/{input,label,separator}.tsx — 기본 폼 primitive`
- `ResizablePanelGroup / ResizablePanel / ResizableHandle — src/components/ui/resizable.tsx:1-57 — react-resizable-panels Group/Panel/Separator wrap (실 사용은 GraphView 가 아니라 AppLayout manual drag — 이 컴포넌트는 다른 화면 용 spare)`
- `ScrollArea / ScrollBar — src/components/ui/scroll-area.tsx:1-54 — base-ui scroll-area Root/Viewport/Scrollbar/Thumb`
- `Tooltip / TooltipTrigger / TooltipContent / TooltipProvider — src/components/ui/tooltip.tsx:1-66 — base-ui tooltip + Portal/Positioner/Popup/Arrow`

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

`as unknown as` event-type coercion — keyboard event 를 mouse event 로 바꿔치기. handler signature 가 `React.MouseEvent` 만 받게 좁아져 있는데 키보드 트리거에서 호출하기 위해 type-check bypass:

```typescript src/components/project/welcome-screen.tsx:78
                      if (e.key === "Enter") handleRemoveRecent(e as unknown as React.MouseEvent, proj.path)
```

`GraphView` 의 `optimizeResearchTopic` 호출이 throw 하면 → fallback 로 `gapTitle` 사용. 사용자에게는 LLM 실패가 표면화 안 됨. catch 빈 처리 + 두 번의 빈 catch (overview / purpose readFile):

```typescript src/components/graph/graph-view.tsx:380-381
      try { overview = await readFile(`${pp}/wiki/overview.md`) } catch {}
      try { purpose = await readFile(`${pp}/purpose.md`) } catch {}
```

```typescript src/components/graph/graph-view.tsx:392-395
    } catch {
      // Fallback: use raw title
      setResearchDialog({ loading: false, topic: gapTitle, queries: [gapTitle] })
    }
```

`handleNodeClick` 의 readFile 실패 시 console-only — node click 이 무반응 보일 수 있음:

```typescript src/components/graph/graph-view.tsx:361-363
      } catch (err) {
        console.error("Failed to open wiki page:", err)
      }
```

`AppLayout.loadFileTree` 의 파일 트리 로드 실패도 console-only — 사용자에게 빈 트리만 보임:

```typescript src/components/layout/app-layout.tsx:36-38
    } catch (err) {
      console.error("Failed to load file tree:", err)
    }
```

`positionCache` 와 `lastLayoutDataKey` 가 모듈 스코프 globals — 동시에 두 GraphView 인스턴스가 존재할 수 없다는 가정에 의존. 패널 toggle 로 컴포넌트가 unmount/remount 될 때 캐시는 살아있어 stale layout 잔존 가능 (의도된 동작이지만 invariant 가 코드에 명시되지 않음):

```typescript src/components/graph/graph-view.tsx:90-91
const positionCache = new Map<string, { x: number; y: number }>()
let lastLayoutDataKey = ""
```

Sigma WebGL crash 회피용 force-remount 휴리스틱 — `data-panel-resizing` body attribute 를 MutationObserver 로 추적하고 50ms / 100ms timeout 후 sigmaKey++ 로 강제 remount. 크래시 원인을 fix 하지 않고 race window 를 늘려 우회 — 패널 resize 가 50ms 안에 끝나면 여전히 크래시 가능:

```typescript src/components/graph/graph-view.tsx:435-451
  useEffect(() => {
    const observer = new MutationObserver(() => {
      const dragging = document.body.dataset.panelResizing === "true"
      if (dragging && !isResizing) {
        setIsResizing(true)
      }
      if (!dragging && isResizing) {
        // Drag ended — remount sigma after a tick
        setTimeout(() => {
          setSigmaKey((k) => k + 1)
          setIsResizing(false)
        }, 50)
      }
    })
    observer.observe(document.body, { attributes: true, attributeFilter: ["data-panel-resizing"] })
    return () => observer.disconnect()
  }, [isResizing])
```

Milkdown 의 `markdownUpdated` listener 가 initial parse 1 회에서 normalized markdown 을 흘려보내는 race — `initialEmitConsumedRef` 로 fix 했지만 `useEditor` 의존성이 `[content]` 라서 content prop 이 바뀔 때마다 ref 가 false 로 reset 되고 또 다음 first emit 을 drop 해야 하는 invariant. ref guard 가 깨지면 (예: hot-reload 가 ref 를 재초기화 못 하면) 사용자 파일이 normalized 버전으로 덮어쓰기 위험:

```typescript src/components/editor/wiki-editor.tsx:24-40
  const initialEmitConsumedRef = useRef(false)

  useEditor(
    (root) =>
      Editor.make()
        .config(nord)
        .config((ctx) => {
          ctx.set(rootCtx, root)
          ctx.set(defaultValueCtx, content)
          initialEmitConsumedRef.current = false
          ctx.get(listenerCtx).markdownUpdated((_ctx, markdown) => {
            if (!initialEmitConsumedRef.current) {
              initialEmitConsumedRef.current = true
              return
            }
            onSave(markdown)
          })
        })
```

`AppLayout` 의 manual mouse-drag resize — `document.addEventListener("mousemove" + "mouseup")` 가 cleanup 을 mouseup handler 안에서 직접 `removeEventListener` 로 함. mouseup 이 발생하지 않는 edge case (예: dragging 중 윈도우 focus 잃음, popup 열림) 에서 listener 누수 + body cursor 영구 col-resize 가능:

```typescript src/components/layout/app-layout.tsx:70-81
      const handleMouseUp = () => {
        isDraggingLeft.current = false
        isDraggingRight.current = false
        document.body.style.cursor = ""
        document.body.style.userSelect = ""
        delete document.body.dataset.panelResizing
        document.removeEventListener("mousemove", handleMouseMove)
        document.removeEventListener("mouseup", handleMouseUp)
      }

      document.addEventListener("mousemove", handleMouseMove)
      document.addEventListener("mouseup", handleMouseUp)
```

## Cross-refs

- `WikiEditor.onSave` 콜백이 결국 `writeFile` Tauri command 로 흘러감 → [04-backend-rust.md#evidence](04-backend-rust.md#evidence) 의 `write_file` + [03-frontend.md](03-frontend.md) 의 `commands/fs.ts` 인터페이스.
- `GraphView` 가 호출하는 `buildWikiGraph` / `findSurprisingConnections` / `detectKnowledgeGaps` / `queueResearch` / `optimizeResearchTopic` 은 [07-llm-integration.md](07-llm-integration.md) (deep-research / optimize-research-topic) + [06-data-layer.md](06-data-layer.md) (wiki-graph 빌드) 와 묶임.
- `dataVersion` flag 를 흘려주는 `useWikiStore.bumpDataVersion` 은 [03-frontend.md](03-frontend.md) 에서 정의 (App.tsx 가 프로젝트 전환 시 호출).
- `data-panel-resizing` body attribute coordination 은 같은 페이지 안의 `AppLayout`(layout/app-layout.tsx) 와 `GraphView` 둘만 사용 — cross-cutting hidden contract.
- 소스 매핑 행: [src/components/editor/wiki-editor.tsx](50-source-mapping.md#srccomponentseditorwiki-editortsx), [src/components/graph/graph-view.tsx](50-source-mapping.md#srccomponentsgraphgraph-viewtsx), [src/components/layout/app-layout.tsx](50-source-mapping.md#srccomponentslayoutapp-layouttsx), [src/components/error-boundary.tsx](50-source-mapping.md#srccomponentserror-boundarytsx), [src/components/ui/button.tsx](50-source-mapping.md#srccomponentsuibuttontsx), [src/components/ui/dialog.tsx](50-source-mapping.md#srccomponentsuidialogtsx), [components.json](50-source-mapping.md#componentsjson).

## Evidence

- `components.json:1-25` — shadcn 설정. `style:"base-nova"`, `tsx:true`, `tailwind.css:"src/index.css"`, `iconLibrary:"lucide"`, alias `@/components` / `@/lib/utils` / `@/components/ui` / `@/lib` / `@/hooks`.
- `src/components/editor/wiki-editor.tsx:1-11` — Milkdown imports: `@milkdown/kit/core` (Editor, rootCtx, defaultValueCtx) + `kit/preset/{commonmark,gfm}` + `kit/plugin/{history,listener}` + `@milkdown/plugin-math` + `@milkdown/theme-nord` + `@milkdown/react` (Milkdown, MilkdownProvider, useEditor) + katex CSS.
- `src/components/editor/wiki-editor.tsx:24` — `initialEmitConsumedRef = useRef(false)` race-guard.
- `src/components/editor/wiki-editor.tsx:34-40` — `markdownUpdated` listener — first emit drop, second 부터 onSave.
- `src/components/editor/wiki-editor.tsx:42-46` — plugin chain: commonmark → gfm → math → history → listener.
- `src/components/editor/wiki-editor.tsx:58-63` — `wrapBareMathBlocks` lookbehind `(?<!\$\$\s*)` + lookahead `(?!\s*\$\$)` 로 이미 wrapping 된 블록 skip.
- `src/components/editor/wiki-editor.tsx:69-74` — `MilkdownProvider` 래퍼 + `prose prose-invert min-w-0 max-w-none overflow-hidden p-6` Tailwind 클래스.
- `src/components/graph/graph-view.tsx:1-16` — Sigma + graphology imports: `Graph from "graphology"`, `SigmaContainer / useLoadGraph / useRegisterEvents / useSigma from "@react-sigma/core"`, `forceAtlas2 from "graphology-layout-forceatlas2"`, lucide 14 아이콘 + ErrorBoundary + 5 store/lib import.
- `src/components/graph/graph-view.tsx:18-27` — `NODE_TYPE_COLORS` 8 종 type→hex (entity/concept/source/query/synthesis/overview/comparison/other).
- `src/components/graph/graph-view.tsx:40-53` — `COMMUNITY_COLORS` 12 색 cycling.
- `src/components/graph/graph-view.tsx:57-58` — `BASE_NODE_SIZE = 8`, `MAX_NODE_SIZE = 28`.
- `src/components/graph/graph-view.tsx:64-79` — `hexToRgba` + `mixColor` 헬퍼.
- `src/components/graph/graph-view.tsx:81-85` — `nodeSize` = base + sqrt(linkCount/maxLinks) × (max-base).
- `src/components/graph/graph-view.tsx:90-91` — module-scope mutable globals (`positionCache`, `lastLayoutDataKey`).
- `src/components/graph/graph-view.tsx:97-98` — `dataKey = nodes.id sort + edges.length` — layout invalidation.
- `src/components/graph/graph-view.tsx:142-153` — `forceAtlas2.assign` 150 iter, `gravity:1, scalingRatio:2, strongGravityMode:true, barnesHutOptimize: nodes.length>50`.
- `src/components/graph/graph-view.tsx:157-159` — layout 후 `positionCache.set(nodeId, {x,y})`.
- `src/components/graph/graph-view.tsx:208-249` — `EventHandler` 가 `clickNode` / `enterNode` / `leaveNode` 등록 — neighbor 강조 + dimmed.
- `src/components/graph/graph-view.tsx:298-325` — `GraphView` 16 useState/useRef.
- `src/components/graph/graph-view.tsx:327-345` — `loadGraph` callback — `buildWikiGraph(normalizePath(project.path))` → 4 setter + `findSurprisingConnections` + `detectKnowledgeGaps` + `lastLoadedVersion.current = dataVersion`.
- `src/components/graph/graph-view.tsx:347-351` — `useEffect` 가 dataVersion 변화에만 re-load.
- `src/components/graph/graph-view.tsx:412-432` — `layoutKey = ${!!selectedFile}-${researchPanelOpen}-${showInsights}` 변화 시 100ms 후 sigmaKey++ remount.
- `src/components/graph/graph-view.tsx:435-451` — `MutationObserver` 가 `data-panel-resizing` attribute 추적.
- `src/components/graph/graph-view.tsx:564-619` — `<ErrorBoundary>` 안에서 `<SigmaContainer key={sigmaKey}>` — nodeReducer + edgeReducer (insightHighlight / hovering / dimmed / highlighted 분기).
- `src/components/graph/graph-view.tsx:684-792` — Insights side panel — Surprising Connections + Knowledge Gaps 두 카드 그룹, dismissedInsights Set 으로 사라짐 영구화.
- `src/components/graph/graph-view.tsx:796-873` — Deep Research 다이얼로그 — loading state + topic input + searchQueries 동적 input list + Cancel/Start.
- `src/components/error-boundary.tsx:13-45` — class Component, props.fallback 우선, 기본 fallback 카드 (destructive 색상 + Retry).
- `src/components/layout/app-layout.tsx:1-13` — sub-component imports (IconSidebar, UpdateBanner, SidebarPanel, ContentArea, PreviewPanel, ResearchPanel, ActivityPanel) + ErrorBoundary.
- `src/components/layout/app-layout.tsx:25-26` — `leftWidth = useState(220)`, `rightWidth = useState(400)` 초기값.
- `src/components/layout/app-layout.tsx:31-39` — `loadFileTree` callback — listDirectory(normalizePath) → setFileTree, console.error catch.
- `src/components/layout/app-layout.tsx:45-84` — `startDrag` factory — body cursor / userSelect / `data-panel-resizing="true"` 설정 후 mousemove/mouseup listener 등록. Width clamp: left 150-400, right 250-50%.
- `src/components/layout/app-layout.tsx:90-91` — `isSettings = activeView === "settings"`, `hasRightPanel = !isSettings && !!(selectedFile || researchPanelOpen)`.
- `src/components/layout/app-layout.tsx:99-160` — JSX: `flex h-screen flex-col` 외곽, `<UpdateBanner/>` 위, IconSidebar + center + 우측 패널 구조. center 와 right panel 모두 `<ErrorBoundary>` 으로 감쌈.
- `src/components/ui/button.tsx:1-58` — base-ui `Button` + cva variants. 6 variant × 9 size, defaultVariants `default + default`. `[a]:hover` selector 로 anchor 하위 케이스 처리.
- `src/components/ui/dialog.tsx:1-160` — base-ui Dialog Root/Trigger/Portal/Close/Backdrop/Popup/Title/Description wrapping. Default `showCloseButton=true`, lucide XIcon. DialogFooter 가 `-mx-4 -mb-4` 음수 margin 으로 muted bg 까지 확장.
- `src/components/ui/resizable.tsx:1-57` — `react-resizable-panels` Group/Panel/Separator wrap. Default direction `horizontal`. (AppLayout 은 이걸 안 쓰고 manual drag 사용 — 컴포넌트 spare.)
- `src/components/ui/scroll-area.tsx:1-54` — base-ui ScrollArea Root + Viewport + Scrollbar + Thumb + Corner.
- `src/components/ui/tooltip.tsx:1-66` — base-ui Tooltip Provider/Root/Trigger + Portal/Positioner/Popup + Arrow. Default delay=0, side=top, sideOffset=4.
- 컴포넌트 inventory: chat (3) + editor (2) + graph (1) + layout (12) + lint (1) + project (3) + review (1) + search (1) + settings (8) + sources (1) + ui (8) + error-boundary (1) = **42 .tsx 파일**.
- Risk 패턴 grep 결과 (components 트리 전체): `as unknown as` 1 건 (welcome-screen.tsx:78). 빈 catch 다수 (graph-view.tsx:380, 381, 392, 그 외 chat/sources/settings 트리에 산재). console-only error 다수 (graph-view.tsx:362, app-layout.tsx:37). Milkdown listener race-guard ref 1 건. Sigma WebGL force-remount 회피 휴리스틱 2 useEffect.

# 50 — Source Mapping (Exhaustive Floor)

> **Status:** Phase 5 lead-built. 245 행 / 245 source files. 빠짐없이 mapping 됐어요.
> **Source:** `nashsu/llm_wiki@1434e08` — `git ls-files | wc -l` == 245 (canonical, gitnexus 부산물 제외).

## Header (locked)

- **Source root:** `/tmp/llm_wiki_inspect/`
- **HEAD:** `1434e08`
- **Denominator (locked):** **245 files** (git-tracked only — `.gitnexus/`, untracked `AGENTS.md`, `CLAUDE.md` 등 도구 부산물 제외)
- **Inventory method:** `cd /tmp/llm_wiki_inspect && git ls-files | wc -l` → 245
- **By extension:** 135 ts, 44 tsx, 14 json, 13 rs, 11 jpg, 7 png, 4 md, 3 js, 2 yml, 2 html, 1 toml, 1 so, 1 lock, 1 ico, 1 icns, 1 gitignore, 1 dylib, 1 dll, 1 css, 1 LICENSE
- **Backlink legend:** `[domain](file.md#section)` | `[leaf-utility]` (도메인 없는 단순 헬퍼) | `[generated]` (lockfile/빌드 산물) | `[vendored]` (외부 라이브러리 사본) | `[asset]` (이미지/아이콘/문서) | `[config-only]` (설정 파일)

## Phase 6 gates this file

1. Row count == 245 (`awk -F'|' 'NR>2 && NF>=4' 50-source-mapping.md | wc -l`)
2. Every row's purpose column ≥ 80 chars
3. Every row's backlink column matches `#|leaf-utility|generated|vendored|asset|config-only`
4. Every path resolves on disk (`test -f /tmp/llm_wiki_inspect/$path`)
5. Reverse coverage (`comm -23` against `git ls-files | sort` ⇒ 빈 집합)

## Mapping Table

| Path | Purpose | Backlink |
|------|---------|----------|
<a id="githubworkflowsbuildyml"></a>
| `.github/workflows/build.yml` | Tag-triggered + workflow_dispatch 매트릭스 빌드. 3 OS (macOS aarch64 / ubuntu-22.04 / windows) Tauri 번들 + Apple 코드 서명 + extension zip release upload. | [80-build-and-tooling.md#ci-release-pipelines](80-build-and-tooling.md) |
<a id="githubworkflowsciyml"></a>
| `.github/workflows/ci.yml` | push/PR to main 시 3-OS 매트릭스에서 protoc 설치, rust cache, Node 20 setup, vite build, cargo build (full Tauri 번들 검증은 안 함 — drift 위험). | [80-build-and-tooling.md#ci-release-pipelines](80-build-and-tooling.md) |
<a id="gitignore"></a>
| `.gitignore` | git 추적 제외 패턴. node_modules / dist / src-tauri/target 등 표준 + Tauri 빌드 산물 제외. gitnexus analyze 가 추가 항목 잡아넣었을 수 있음. | [config-only] |
<a id="license"></a>
| `LICENSE` | 프로젝트 오픈소스 라이선스 텍스트 — MIT 또는 유사. 배포 시 번들에 포함되며 법적 의무 만족. | [asset] |
<a id="readmemd"></a>
| `README.md` | 영문 readme. 제품 소개, 기능 목록 (CoT ingest, 4-신호 KG, Louvain, RRF, Deep research), tech stack, install 가이드, credits. | [asset] |
<a id="readme_cnmd"></a>
| `README_CN.md` | 중문 readme — 영문 README 와 parity 유지. 영어/중국어 두 로케일 출하 정책의 일부예요. | [asset] |
<a id="assets1-deepresearchjpg"></a>
| `assets/1-deepresearch.jpg` | README 첨부 스크린샷. Deep Research UI 보여주는 마케팅용 이미지. 빌드/런타임에 영향 없음. | [asset] |
<a id="assets2-ai_chatjpg"></a>
| `assets/2-ai_chat.jpg` | README 첨부 — AI Chat 기능 스크린샷. 마케팅/문서 자산이라 빌드/런타임에 영향 없음. | [asset] |
<a id="assets3-knowledge_graphjpg"></a>
| `assets/3-knowledge_graph.jpg` | README 첨부 — Knowledge Graph (Sigma + Louvain) 시각화 스크린샷. 문서용 이미지. | [asset] |
<a id="assets4-chrome_extension_webclipperjpg"></a>
| `assets/4-chrome_extension_webclipper.jpg` | README 첨부 — Chrome MV3 webclipper 동작 스크린샷. 문서용 이미지로 런타임 의존 없음. | [asset] |
<a id="assets5-obsidian_compatibilityjpg"></a>
| `assets/5-obsidian_compatibility.jpg` | README 첨부 — Obsidian 폴더 구조 호환성 스크린샷. 문서용 이미지로 런타임 의존 없음. | [asset] |
<a id="assetskg_communityjpg"></a>
| `assets/kg_community.jpg` | README 첨부 — Louvain 커뮤니티 색상 클러스터 스크린샷. 문서용 이미지. | [asset] |
<a id="assetskg_insightsjpg"></a>
| `assets/kg_insights.jpg` | README 첨부 — Graph Insights (놀라운 연결, 지식 갭) 패널 스크린샷. 문서용 이미지. | [asset] |
<a id="assetsllm_wiki_archjpg"></a>
| `assets/llm_wiki_arch.jpg` | README 첨부 — 시스템 아키텍처 다이어그램 (renderer / Tauri / extension). 문서용 이미지. | [asset] |
<a id="assetsoverviewjpg"></a>
| `assets/overview.jpg` | README hero 이미지 — 앱 메인 화면 상위 뷰. 문서용 이미지로 런타임 의존 없음. | [asset] |
<a id="componentsjson"></a>
| `components.json` | shadcn 설정. style `base-nova`, baseColor `neutral`, cssVariables on, lucide icon library, alias 매핑 (@/components 등). | [01-tech-stack.md#shadcn-config](01-tech-stack.md) |
<a id="extensionreadabilityjs"></a>
| `extension/Readability.js` | Mozilla Readability 라이브러리 vendored 사본. 웹페이지 메인 콘텐츠 추출 (clutter 제거 후 글 본문만 남김). | [vendored] |
<a id="extensionturndownjs"></a>
| `extension/Turndown.js` | Turndown 라이브러리 vendored 사본. HTML → Markdown 변환 — Readability 출력을 wiki ingest 가능 형태로 만듦. | [vendored] |
<a id="extensionicon128png"></a>
| `extension/icon128.png` | Chrome MV3 toolbar/install 아이콘 128×128 픽셀. manifest.json 의 action.default_icon + icons 에 등록. | [asset] |
<a id="extensionicon16png"></a>
| `extension/icon16.png` | Chrome MV3 toolbar 아이콘 16×16 픽셀 — 작은 사이즈 favicon 용도. manifest.json 등록. | [asset] |
<a id="extensionicon48png"></a>
| `extension/icon48.png` | Chrome MV3 extensions 페이지 아이콘 48×48 픽셀. manifest.json 의 action.default_icon + icons 에 등록. | [asset] |
<a id="extensionmanifestjson"></a>
| `extension/manifest.json` | MV3 manifest. permissions [activeTab, scripting], host_permissions [http://127.0.0.1:19827/*], action popup.html, web_accessible_resources Readability/Turndown. | [05-extension.md#public-interface](05-extension.md) |
<a id="extensionpopuphtml"></a>
| `extension/popup.html` | Chrome extension popup 화면 — 사용자가 extension 아이콘 클릭 시 보이는 UI. clip 트리거 + 상태 표시. | [05-extension.md#public-interface](05-extension.md) |
<a id="extensionpopupjs"></a>
| `extension/popup.js` | popup.html 동작 스크립트. chrome.scripting.executeScript 로 Readability/Turndown 주입 → HTML→MD → fetch POST localhost:19827/clip. | [05-extension.md#internal-risk](05-extension.md) |
<a id="indexhtml"></a>
| `index.html` | Vite SPA 진입 HTML. <div id="root"> + main.tsx 모듈 로드. webview 가 처음 fetch 하는 페이지. | [03-frontend.md#purpose](03-frontend.md) |
<a id="llm-wikimd"></a>
| `llm-wiki.md` | Karpathy 의 LLM Wiki 패턴 원문 인용 + 프로젝트 해석. 디자인 의도 + 기능 매핑 설명한 디자인 doc. | [asset] |
<a id="logojpg"></a>
| `logo.jpg` | 프로젝트 로고 이미지 — README + Tauri bundle icon source. 마케팅 + 앱 ID 양쪽에 사용. | [asset] |
<a id="package-lockjson"></a>
| `package-lock.json` | npm install 결정성 잠금 파일 — 모든 의존성 정확한 버전 트리. 자동 생성, 직접 편집 금지. | [generated] |
<a id="packagejson"></a>
| `package.json` | 프론트엔드 매니페스트. v0.4.3, scripts (dev/build/typecheck/test mocks-vs-llm/tauri), 30+ 의존성 (React 19, Tauri 2 plugins, Milkdown, Sigma, Louvain). | [01-tech-stack.md#frontend-dependencies-package-json](01-tech-stack.md) |
<a id="plansmultimodal-imagesmd"></a>
| `plans/multimodal-images.md` | Phase 1-3 멀티모달 이미지 ingest 디자인 + 구현 단계 문서. pdfium → image PNG → vision LLM caption → 이미지 인식 검색 결과. | [asset] |
<a id="src-tauricargolock"></a>
| `src-tauri/Cargo.lock` | Rust 의존성 결정성 잠금 — 모든 transitive crate 버전. cargo build 가 자동 갱신, 커밋 필수. | [generated] |
<a id="src-tauricargotoml"></a>
| `src-tauri/Cargo.toml` | Rust crate 매니페스트. tauri 2 (protocol-asset), pdfium-render 0.9, lancedb 0.27, tokio process, image/base64/sha2. release profile panic=unwind + LTO + opt-level s. | [01-tech-stack.md#rust-dependencies-src-tauricargotoml](01-tech-stack.md) |
<a id="src-tauribuildrs"></a>
| `src-tauri/build.rs` | tauri-build 호출만 들어 있는 최소 빌드 스크립트 — `tauri_build::build()` 한 줄. 빌드 시점에 Tauri 메타 생성. | [01-tech-stack.md#rust-build-chain](01-tech-stack.md) |
<a id="src-tauricapabilitiesdefaultjson"></a>
| `src-tauri/capabilities/default.json` | Tauri 2 capability allowlist. core/opener/dialog/store default + http allowlist 가 모든 http/https 패턴 (LLM endpoint 임의 입력 의도). | [01-tech-stack.md#tauri-capabilities-src-tauricapabilitiesdefaultjson](01-tech-stack.md) |
<a id="src-tauriicons128x128png"></a>
| `src-tauri/icons/128x128.png` | Tauri bundle 아이콘 128×128 — 모든 desktop 플랫폼에서 사용. tauri.conf.json bundle.icon 등록. | [asset] |
<a id="src-tauriicons128x128@2xpng"></a>
| `src-tauri/icons/128x128@2x.png` | HiDPI 환경용 256×256 (2× scale 의 128×128) 번들 아이콘. macOS/Windows 고해상도 대응. | [asset] |
<a id="src-tauriicons32x32png"></a>
| `src-tauri/icons/32x32.png` | Tauri bundle 아이콘 32×32 — 작은 사이즈 (taskbar/title bar) 대응. tauri.conf.json bundle.icon 등록. | [asset] |
<a id="src-tauriiconsiconicns"></a>
| `src-tauri/icons/icon.icns` | macOS Apple Icon Image format 번들 아이콘 — DMG 패키징 시 .app 의 Resources/ 에 들어감. | [asset] |
<a id="src-tauriiconsiconico"></a>
| `src-tauri/icons/icon.ico` | Windows ICO format 번들 아이콘 — MSI/NSIS installer + .exe 메타에 사용. | [asset] |
<a id="src-tauriiconsiconpng"></a>
| `src-tauri/icons/icon.png` | 기본 PNG 번들 아이콘 — Linux .desktop 파일 + AppImage/.deb 패키징 시 사용. | [asset] |
<a id="src-tauripdfiumlibpdfiumdylib"></a>
| `src-tauri/pdfium/libpdfium.dylib` | macOS PDFium 동적 라이브러리 (vendored 바이너리). pdfium-render crate 가 dlopen-style 로 로드. tauri.macos.conf.json frameworks 등록. | [vendored] |
<a id="src-tauripdfiumlibpdfiumso"></a>
| `src-tauri/pdfium/libpdfium.so` | Linux PDFium 공유 객체 (vendored 바이너리). tauri.linux.conf.json resources 로 번들 포함. pdfium-render 가 dlopen. | [vendored] |
<a id="src-tauripdfiumpdfiumdll"></a>
| `src-tauri/pdfium/pdfium.dll` | Windows PDFium DLL (vendored 바이너리). tauri.windows.conf.json resources 등록. pdfium-render 가 LoadLibrary. | [vendored] |
<a id="src-taurisrcclip_serverrs"></a>
| `src-tauri/src/clip_server.rs` | Chrome extension webclipper endpoint. tiny_http 가 127.0.0.1:19827 listen, /clip POST 받아 ingest 큐로 forward. CORS * 로 모든 origin 허용. | [04-backend-rust.md#public-interface](04-backend-rust.md) |
<a id="src-taurisrccommandsclaude_clirs"></a>
| `src-tauri/src/commands/claude_cli.rs` | Tauri command — claude CLI 자식 프로세스 lifecycle. tokio::process::Command spawn, stdout 라인 → app.emit `claude-cli:{stream_id}`, kill_on_drop. | [04-backend-rust.md#public-interface](04-backend-rust.md) |
<a id="src-taurisrccommandsextract_imagesrs"></a>
| `src-tauri/src/commands/extract_images.rs` | Tauri command — pdfium FFI 로 PDF embedded 이미지 추출 + image crate 로 PNG 재인코딩 + base64 직렬화. Office (zip 내 media/) 추출도 같은 명령군. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="src-taurisrccommandsfsrs"></a>
| `src-tauri/src/commands/fs.rs` | 파일시스템 명령 모음. read/write/list/copy/preprocess/delete/find_related_wiki_pages 등. PDF/Office/이미지/미디어 분기 추출 + .cache/<name>.txt 저장. | [04-backend-rust.md#public-interface](04-backend-rust.md) |
<a id="src-taurisrccommandsmodrs"></a>
| `src-tauri/src/commands/mod.rs` | commands 모듈 re-export 진입점. claude_cli/extract_images/fs/project/vectorstore 하위 모듈 노출. | [04-backend-rust.md#public-interface](04-backend-rust.md) |
<a id="src-taurisrccommandsprojectrs"></a>
| `src-tauri/src/commands/project.rs` | 프로젝트 폴더 스캐폴딩 명령. create_project (schema.md/purpose.md/.obsidian 보일러), open_project (검증 후 디렉터리명에서 name 도출). | [04-backend-rust.md#public-interface](04-backend-rust.md) |
<a id="src-taurisrccommandsvectorstorers"></a>
| `src-tauri/src/commands/vectorstore.rs` | LanceDB 래핑 명령군. v1 (페이지 단위) + v2 (chunk 단위) 두 테이블 — upsert/search/delete/count/legacy_drop. KNN distance → score 변환. | [06-data-layer.md#tauri-command-surface-data-layer-슬라이스](06-data-layer.md) |
<a id="src-taurisrclibrs"></a>
| `src-tauri/src/lib.rs` | Tauri app 진입 라이브러리. tauri::Builder + plugin 4종 + clip_server 스폰 + 31 command generate_handler 등록. State<ClaudeCliState> 공유. | [04-backend-rust.md#public-interface](04-backend-rust.md) |
<a id="src-taurisrcmainrs"></a>
| `src-tauri/src/main.rs` | 바이너리 진입 — `llm_wiki_lib::run()` 한 줄 호출. windows GUI subsystem (no console). | [04-backend-rust.md#purpose](04-backend-rust.md) |
<a id="src-taurisrcpanic_guardrs"></a>
| `src-tauri/src/panic_guard.rs` | panic_guard::run_guarded[_async] — third-party 파서 panic 을 catch_unwind 로 잡아 Result<_, String> 으로 변환. unwind panic profile 의존. | [04-backend-rust.md#internal-risk](04-backend-rust.md) |
<a id="src-taurisrctypesmodrs"></a>
| `src-tauri/src/types/mod.rs` | types 서브모듈 re-export. wiki 타입 노출. 다른 명령군이 의존하는 공유 타입 진입점. | [06-data-layer.md#tsrust-type-mapping-ipc-경계](06-data-layer.md) |
<a id="src-taurisrctypeswikirs"></a>
| `src-tauri/src/types/wiki.rs` | Rust 측 IPC 타입. WikiProject {name, path}, FileNode {name, path, is_dir, children?}. TS 의 wiki.ts 와 매핑되며 `id` 필드는 TS 만 가짐 (drift). | [06-data-layer.md#tsrust-type-mapping-ipc-경계](06-data-layer.md) |
<a id="src-tauritauriconfjson"></a>
| `src-tauri/tauri.conf.json` | Tauri 코어 설정. productName, version 0.4.3, identifier com.llmwiki.app, devUrl 1420, CSP, assetProtocol scope **, bundle targets all, icon 리스트. | [01-tech-stack.md#tauri-config-tauriconfjson--플랫폼별](01-tech-stack.md) |
<a id="src-tauritaurilinuxconfjson"></a>
| `src-tauri/tauri.linux.conf.json` | Linux 전용 번들 — pdfium/libpdfium.so 를 resources 로 번들 포함. AppImage/.deb 빌드에서 적용. | [01-tech-stack.md#플랫폼별-번들](01-tech-stack.md) |
<a id="src-tauritaurimacosconfjson"></a>
| `src-tauri/tauri.macos.conf.json` | macOS 전용 번들 — pdfium/libpdfium.dylib 를 frameworks 로 등록. .app 번들의 Frameworks/ 디렉터리에 포함. | [01-tech-stack.md#플랫폼별-번들](01-tech-stack.md) |
<a id="src-tauritauriwindowsconfjson"></a>
| `src-tauri/tauri.windows.conf.json` | Windows 전용 번들 — pdfium/pdfium.dll 을 resources 로 번들 포함. MSI/NSIS installer 에서 .exe 옆에 배치. | [01-tech-stack.md#플랫폼별-번들](01-tech-stack.md) |
<a id="srcapptsx"></a>
| `src/App.tsx` | React 19 root 컴포넌트. 7-view 토글 (wiki/sources/search/graph/lint/review/settings) + auto-save/clip-watcher/update-check/last-project lifecycle hooks. ErrorBoundary wrap. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcassetslogojpg"></a>
| `src/assets/logo.jpg` | webview 안에서 import 되는 로고 자산. main process logo.jpg 와 별개로 SPA 번들에 포함되는 사본. | [asset] |
<a id="srccommandsfsts"></a>
| `src/commands/fs.ts` | Tauri invoke 래퍼 모음. readFile/writeFile/listDirectory/copyFile/preprocessFile/createProject/openProject 등 — Rust 측 fs 명령 1:1 wrapping. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srccomponentschatchat-inputtsx"></a>
| `src/components/chat/chat-input.tsx` | 채팅 입력 컴포넌트. 사용자 메시지 입력 + send 트리거 + IME composition 처리. useChatStore.addMessage 호출. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentschatchat-messagetsx"></a>
| `src/components/chat/chat-message.tsx` | 채팅 메시지 렌더링 컴포넌트. user/assistant 분기 + react-markdown + KaTeX 수식 + streaming 부분 표시. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentschatchat-paneltsx"></a>
| `src/components/chat/chat-panel.tsx` | 채팅 패널 컨테이너. chat-message 리스트 + chat-input + 스트리밍 상태 표시. useChatStore.streamingContent 구독. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentseditorfile-previewtsx"></a>
| `src/components/editor/file-preview.tsx` | 파일 프리뷰 컴포넌트 — 이미지/PDF/마크다운 렌더 분기. wiki-editor 와 별도로 read-only 표시. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentseditorwiki-editortsx"></a>
| `src/components/editor/wiki-editor.tsx` | Milkdown 7.20 wiki 마크다운 에디터 wrapper. plugin-math + theme-nord + react 통합. auto-save 구독 + frontmatter 보존. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentserror-boundarytsx"></a>
| `src/components/error-boundary.tsx` | React ErrorBoundary 클래스 컴포넌트. getDerivedStateFromError → fallback 카드 + Retry. componentDidCatch 는 console.error 만 (swallow). | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srccomponentsgraphgraph-viewtsx"></a>
| `src/components/graph/graph-view.tsx` | Sigma 3 + graphology 0.26 + Louvain communities + forceatlas2 layout 통합 그래프 뷰. wiki-graph 데이터 받아 렌더. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutactivity-paneltsx"></a>
| `src/components/layout/activity-panel.tsx` | activity-store 구독 — 진행중 ingest/lint/query 활동 피드 표시. 진행률 바 + 로그 라인. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutapp-layouttsx"></a>
| `src/components/layout/app-layout.tsx` | 메인 레이아웃 셸. resizable 패널 (sidebar + content + chat-bar) 분할 + activeView 분기 렌더. App.tsx 가 마운트. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutchat-bartsx"></a>
| `src/components/layout/chat-bar.tsx` | 우측 채팅 사이드바 컨테이너. chat-panel + 모드 토글 (chat / research) + ingestSource 표시. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutcontent-areatsx"></a>
| `src/components/layout/content-area.tsx` | 메인 콘텐츠 영역 라우터. activeView 따라 wiki-editor / sources-view / search-view / graph-view / lint-view / review-view / settings-view 분기. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutfile-treetsx"></a>
| `src/components/layout/file-tree.tsx` | 좌측 파일 트리. wiki/ 디렉터리 재귀 표시 + 클릭 시 fileContent 로드. listDirectory invoke 결과 사용. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayouticon-sidebartsx"></a>
| `src/components/layout/icon-sidebar.tsx` | 좌측 좁은 아이콘 사이드바. 7-view 전환 아이콘 (lucide-react) + 활성 view 하이라이트. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutknowledge-treetsx"></a>
| `src/components/layout/knowledge-tree.tsx` | wiki 페이지 + cluster 계층 트리. Louvain 커뮤니티별 그룹 + cohesion 점수 표시. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutpreview-paneltsx"></a>
| `src/components/layout/preview-panel.tsx` | 사이드 프리뷰 패널 — 검색 결과 / 그래프 노드 클릭 시 file-preview 띄움. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutresearch-paneltsx"></a>
| `src/components/layout/research-panel.tsx` | research-store 구독 — deep-research 작업 큐 (queued/searching/synthesizing/saving/done/error) 진행 표시. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutsidebar-paneltsx"></a>
| `src/components/layout/sidebar-panel.tsx` | 좌측 사이드바 컨테이너. file-tree + knowledge-tree + activity-panel 통합 + resizable. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslayoutupdate-bannertsx"></a>
| `src/components/layout/update-banner.tsx` | 업데이트 배너 — useUpdateStore.lastResult 의 새 버전 알림 + dismiss 액션. shouldShowUpdateBanner 게이트. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentslintlint-viewtsx"></a>
| `src/components/lint/lint-view.tsx` | wiki 구조 lint 뷰 — wikilink/orphan/broken-link 결과 표시 + 자동 수정 액션. runStructuralLint 호출. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsprojectcreate-project-dialogtsx"></a>
| `src/components/project/create-project-dialog.tsx` | 새 프로젝트 생성 다이얼로그. name + path 입력 + template-picker + Tauri dialog plugin 으로 디렉터리 선택. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsprojecttemplate-pickertsx"></a>
| `src/components/project/template-picker.tsx` | 프로젝트 템플릿 선택 컴포넌트. WikiTemplate (research/general/project) 미리보기 + 선택. templates.ts 의 정의 사용. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsprojectwelcome-screentsx"></a>
| `src/components/project/welcome-screen.tsx` | 프로젝트 미선택 상태의 WelcomeScreen. 최근 프로젝트 리스트 + 새로 생성 + 기존 열기 진입점. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsreviewreview-viewtsx"></a>
| `src/components/review/review-view.tsx` | 비동기 리뷰 시스템 뷰 — review-store 구독, predefined actions + 검색 query 표시. resolve/dismiss 액션. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssearchsearch-viewtsx"></a>
| `src/components/search/search-view.tsx` | 검색 뷰 — RRF (vector ⊕ keyword) 결과 + 이미지 thumbnail + lightbox + jump-to-source. search.ts 호출. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingscontext-size-selectortsx"></a>
| `src/components/settings/context-size-selector.tsx` | LLM context window 크기 선택 컴포넌트. context-budget.ts 의 토큰 견적과 연결. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingsllm-presetsts"></a>
| `src/components/settings/llm-presets.ts` | LLM provider preset 정의 (OpenAI / Claude / Gemini / Ollama / 기타 OpenAI-compatible). 기본 endpoint + 모델 후보. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingspreset-resolverts"></a>
| `src/components/settings/preset-resolver.ts` | activePresetId → 실제 LlmConfig 매핑. providerConfigs 와 결합해 final config 도출. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssectionsabout-sectiontsx"></a>
| `src/components/settings/sections/about-section.tsx` | Settings → About 섹션. __APP_VERSION__ 표시 + update-check 수동 트리거 + GitHub 링크. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssectionsembedding-sectiontsx"></a>
| `src/components/settings/sections/embedding-section.tsx` | 임베딩 provider 설정 섹션. endpoint/model/api key + 차원 자동 감지 + 기존 인덱스 호환성 경고. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssectionsinterface-sectiontsx"></a>
| `src/components/settings/sections/interface-section.tsx` | UI 언어 (i18next 로케일) + 테마 + 폰트 등 인터페이스 설정. saveLanguage 트리거. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssectionsllm-provider-sectiontsx"></a>
| `src/components/settings/sections/llm-provider-section.tsx` | LLM provider 설정. preset 선택 + endpoint/model/api key + claude-cli detect 결과 표시. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssectionsmultimodal-sectiontsx"></a>
| `src/components/settings/sections/multimodal-section.tsx` | Vision LLM 설정 섹션 컴포넌트. caption 모델 + endpoint + dedup 캐시 토글 + 차원 정보. multimodalConfig 영속에 의존. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssectionsoutput-sectiontsx"></a>
| `src/components/settings/sections/output-section.tsx` | 출력 언어 설정 섹션 컴포넌트. OUTPUT_LANGUAGE_OPTIONS 21 개 + auto detect 토글 + LLM 시스템 프롬프트 강제 directive 미리보기. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssectionsweb-search-sectiontsx"></a>
| `src/components/settings/sections/web-search-section.tsx` | 웹 검색 API 설정 섹션 컴포넌트. provider + key + endpoint + deep-research 의존 표시 + 키 검증 액션. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssettings-typests"></a>
| `src/components/settings/settings-types.ts` | Settings 컴포넌트들이 공유하는 TS 타입 정의. SettingsSectionProps + section 별 props. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssettingssettings-viewtsx"></a>
| `src/components/settings/settings-view.tsx` | Settings 메인 뷰 — 7개 섹션 (llm/embedding/multimodal/web-search/output/interface/about) 탭 라우팅. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentssourcessources-viewtsx"></a>
| `src/components/sources/sources-view.tsx` | 소스 트리 뷰 — sources/ 폴더 계층 + ingest 상태 + cascade-delete 다이얼로그. ingest-queue 진행률 구독. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuibuttontsx"></a>
| `src/components/ui/button.tsx` | shadcn Button — class-variance-authority 변형 (primary/secondary/destructive/outline/ghost/link) + size + asChild. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuidialogtsx"></a>
| `src/components/ui/dialog.tsx` | shadcn Dialog — @base-ui/react primitives 기반. portal + overlay + content + header/footer 슬롯. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuiinputtsx"></a>
| `src/components/ui/input.tsx` | shadcn Input — controlled text input. focus ring + invalid state. cn() 으로 className merge. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuilabeltsx"></a>
| `src/components/ui/label.tsx` | shadcn Label — form input 라벨. peer-disabled / peer-invalid 스타일 분기. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuiresizabletsx"></a>
| `src/components/ui/resizable.tsx` | react-resizable-panels 4.9 wrapper — Panel/PanelGroup/PanelResizeHandle. app-layout 가 사용. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuiscroll-areatsx"></a>
| `src/components/ui/scroll-area.tsx` | shadcn ScrollArea — @base-ui scroll primitives. overflow 처리 + 커스텀 스크롤바. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuiseparatortsx"></a>
| `src/components/ui/separator.tsx` | shadcn Separator — 가로/세로 구분선. orientation prop + decorative=true 기본값. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srccomponentsuitooltiptsx"></a>
| `src/components/ui/tooltip.tsx` | shadcn Tooltip — @base-ui tooltip primitives. provider/root/trigger/content + delayDuration. | [09-ui-components.md#public-interface](09-ui-components.md) |
<a id="srci18nenjson"></a>
| `src/i18n/en.json` | 영어 번역 번들. 모든 UI 카피 키-값 (settings/dialogs/buttons/messages). i18n-parity.test 가 zh.json 과 키 비교. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srci18ni18n-paritytestts"></a>
| `src/i18n/i18n-parity.test.ts` | i18n 키 parity 테스트. flattenKeys 로 en/zh 도트 경로 평탄화 후 set 차집합 0 검증. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srci18nindexts"></a>
| `src/i18n/index.ts` | i18next 인스턴스 초기화. initReactI18next + en/zh resources, lng="en" fallbackLng="en". main.tsx 에서 side-effect import. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srci18nzhjson"></a>
| `src/i18n/zh.json` | 중국어 번역 번들. en.json 과 키 parity 유지 (i18n-parity.test 강제). | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcindexcss"></a>
| `src/index.css` | 전역 CSS. Tailwind 4 + tw-animate-css + @fontsource-variable/geist + base color (neutral) 변수 + dark mode 지원. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclib__tests__claude-cli-transporttestts"></a>
| `src/lib/__tests__/claude-cli-transport.test.ts` | claude-cli-transport unit 테스트. event listener 등록/해제, stream-json 파서, kill 시그널 전파 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclib__tests__llm-providerstestts"></a>
| `src/lib/__tests__/llm-providers.test.ts` | llm-providers unit 테스트 (별도 디렉터리). provider 매트릭스 + 기본 모델 + endpoint 정규화 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibauto-savets"></a>
| `src/lib/auto-save.ts` | wiki-store 구독 + debounce 시간 후 file write 트리거. setTimeout 기반 + projectPath 별 격리. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibclaude-cli-transportts"></a>
| `src/lib/claude-cli-transport.ts` | claude CLI 자식 프로세스 transport. spawn invoke + listen `claude-cli:{stream_id}` 이벤트 + stream-json 파싱. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibclip-watcherts"></a>
| `src/lib/clip-watcher.ts` | clip-server 큐 폴링. /clips/pending GET → ingest-queue enqueue. App 마운트 시 setInterval 시작. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibcontext-budgettestts"></a>
| `src/lib/context-budget.test.ts` | context-budget unit 테스트. token 견적 + chat history 절단 + system prompt overhead 계산 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibcontext-budgetts"></a>
| `src/lib/context-budget.ts` | LLM context window 토큰 budget 관리. 메시지 토큰 견적 + 윈도우 초과 시 가장 오래된 메시지 절단. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibdeep-researchts"></a>
| `src/lib/deep-research.ts` | deep research 오케스트레이션. optimize-research-topic 으로 query 다중 생성 → web-search → ingest-queue auto-enqueue. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibdetect-languagepropertytestts"></a>
| `src/lib/detect-language.property.test.ts` | detect-language fast-check property 테스트. 임의 입력 → 결정론적 출력 + 한글 텍스트 → "ko" 등 invariant. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibdetect-languagetestts"></a>
| `src/lib/detect-language.test.ts` | detect-language unit 테스트. Unicode script 분기 + Latin diacritics + common-word fallback 케이스 검증. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibdetect-languagets"></a>
| `src/lib/detect-language.ts` | 텍스트 → 언어 코드 (ko/zh/ja/en/...). Unicode script 우선 + Latin diacritics + 공통 단어 매칭 두 단계. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibembeddingreal-llmtestts"></a>
| `src/lib/embedding.real-llm.test.ts` | embedding 실제 LLM 호출 테스트. .env.test.local 의 endpoint/model 사용. 차원 검증 + 결과 numerical stability. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibembeddingtestts"></a>
| `src/lib/embedding.test.ts` | embedding unit 테스트 (mock). 입력 정규화 + 배치 분할 + 에러 회복 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibembeddingts"></a>
| `src/lib/embedding.ts` | 임베딩 호출 추상화. OpenAI-compatible /embeddings endpoint POST + 배치 분할 + 차원 캐싱. vector_upsert 의존. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibendpoint-normalizertestts"></a>
| `src/lib/endpoint-normalizer.test.ts` | endpoint-normalizer unit 테스트. 사용자 입력 endpoint URL 정규화 (trailing slash, /v1 자동 추가) 케이스. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibendpoint-normalizerts"></a>
| `src/lib/endpoint-normalizer.ts` | LLM provider endpoint URL 정규화. trailing slash 제거 + /v1 자동 추가 + Anthropic vs OpenAI 분기. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibenrich-wikilinksreal-llmtestts"></a>
| `src/lib/enrich-wikilinks.real-llm.test.ts` | enrich-wikilinks 실제 LLM 테스트. wiki 본문에서 [[wikilink]] 후보 자동 감지 + 기존 페이지와 매칭 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibenrich-wikilinksscenariostestts"></a>
| `src/lib/enrich-wikilinks.scenarios.test.ts` | enrich-wikilinks 시나리오 테스트. 사전 정의 wiki 입력 매트릭스 → 예상 wikilink 목록. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibenrich-wikilinkstestts"></a>
| `src/lib/enrich-wikilinks.test.ts` | enrich-wikilinks unit 테스트 (mock LLM). 결정론 + 빈 입력 + 중복 wikilink 처리 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibenrich-wikilinksts"></a>
| `src/lib/enrich-wikilinks.ts` | wiki 본문에 LLM 으로 [[wikilink]] 자동 추가. 기존 페이지 인덱스와 후보 비교 + 자동 머지. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibextract-source-imagests"></a>
| `src/lib/extract-source-images.ts` | wiki 페이지 본문에서 ![alt](path) 이미지 참조 추출. raw-source-resolver 와 함께 dedup cache 입력 생성. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibfile-typests"></a>
| `src/lib/file-types.ts` | 파일 확장자 → 카테고리 분류 (text/pdf/office/image/audio/video/binary). ingest-parse 분기 + UI 아이콘 결정. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibgraph-insightsts"></a>
| `src/lib/graph-insights.ts` | 그래프 surprise + knowledge-gap 산출. "놀라운 연결 = 거리 큰데 강한 연결" + "갭 = 클러스터 안에서 약한 노드". | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibgraph-relevancets"></a>
| `src/lib/graph-relevance.ts` | 4-신호 relevance 점수. 직접 link + source overlap + Adamic-Adar + type affinity. wiki-graph 가 호출. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibgreeting-detectortestts"></a>
| `src/lib/greeting-detector.test.ts` | isGreeting unit 테스트. 한국어 "안녕" / 영어 "hi" / 길이 cap 20 / 화이트리스트 정규식 13 종 케이스. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibgreeting-detectorts"></a>
| `src/lib/greeting-detector.ts` | 사용자 입력이 인사말인지 판정. 화이트리스트 정규식 13 종 + 길이 cap 20. chat-input 이 사용. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibimage-caption-pipelinetestts"></a>
| `src/lib/image-caption-pipeline.test.ts` | image-caption-pipeline unit 테스트. dedup cache hit/miss + vision 모델 호출 횟수 + 실패 회복 검증. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibimage-caption-pipelinets"></a>
| `src/lib/image-caption-pipeline.ts` | 이미지 → vision-caption 파이프라인. extract-source-images 결과 → dedup cache 조회 → vision-caption 호출 → 결과 저장. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibingest-cachetestts"></a>
| `src/lib/ingest-cache.test.ts` | ingest-cache unit 테스트. file hash → ingest 결과 캐싱 hit/miss + invalidate 케이스. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingest-cachets"></a>
| `src/lib/ingest-cache.ts` | ingest 결과 캐싱. SHA-256 file hash → wiki 페이지 + 청크 + 임베딩 결과. 같은 입력 재 ingest 시 LLM 호출 우회. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingest-parsetestts"></a>
| `src/lib/ingest-parse.test.ts` | ingest-parse unit 테스트. PDF/Office/이미지 추출 분기 + raw bytes → text + metadata + frontmatter 케이스. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingest-queueintegrationtestts"></a>
| `src/lib/ingest-queue.integration.test.ts` | ingest-queue integration 테스트. 다중 모듈 결합 (cache + parse + chunker + embed + lancedb upsert) 시나리오. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingest-queuetestts"></a>
| `src/lib/ingest-queue.test.ts` | ingest-queue unit 테스트. 직렬화 보장 + crash recovery + cancel 시그널 전파 + 진행률 이벤트 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingest-queuets"></a>
| `src/lib/ingest-queue.ts` | 직렬 ingest 큐 + crash recovery. enqueue 시 .llm-wiki/ingest-queue.json 영속 + 부팅 시 미완료 작업 재개. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingestprompttestts"></a>
| `src/lib/ingest.prompt.test.ts` | ingest LLM prompt 테스트. 시스템 프롬프트 안정성 + frontmatter 강제 + wiki 페이지 형식 강제 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingestreal-llmtestts"></a>
| `src/lib/ingest.real-llm.test.ts` | ingest 실제 LLM 테스트. CoT 두 단계 (analyze → generate) end-to-end + 실제 wiki 페이지 + source traceability. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingestscenariostestts"></a>
| `src/lib/ingest.scenarios.test.ts` | ingest 시나리오 테스트. 사전 정의 입력 (PDF / docx / 이미지 / 텍스트) 매트릭스 → 예상 wiki 페이지. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibingestts"></a>
| `src/lib/ingest.ts` | CoT 두 단계 ingest 진입점. analyze (LLM 으로 구조 분석) → generate (wiki 페이지 + source link) → cache. ingest-queue 가 호출. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srcliblatex-to-unicodets"></a>
| `src/lib/latex-to-unicode.ts` | $\\cmd$ / $$..$$ / $..$ → Unicode 글리프 치환. KaTeX 에 의존하지 않는 가벼운 inline 변환 (chat 메시지에서 사용). | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcliblintreal-llmtestts"></a>
| `src/lib/lint.real-llm.test.ts` | lint 실제 LLM 테스트. wiki 본문 + 페이지 인덱스 → LLM 으로 깨진 wikilink 후보 자동 감지 검증. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcliblintscenariostestts"></a>
| `src/lib/lint.scenarios.test.ts` | lint 시나리오 테스트. 사전 정의 wiki 디렉터리 매트릭스 → 예상 lint 결과 (orphan/broken-link). | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcliblinttestts"></a>
| `src/lib/lint.test.ts` | lint unit 테스트 (mock). wikilink 정규식 + orphan 그래프 + broken link 검증. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcliblintts"></a>
| `src/lib/lint.ts` | wiki 구조 lint. wikilink/orphan/broken-link/missing-frontmatter 4종 검증 + LLM-aided suggested fix. lint-view 가 사용. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibllm-clientreal-llmtestts"></a>
| `src/lib/llm-client.real-llm.test.ts` | llm-client 실제 LLM 테스트. chat completion + streaming + 에러 회복 + 다양한 provider 매트릭스 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibllm-clienttestts"></a>
| `src/lib/llm-client.test.ts` | llm-client unit 테스트 (mock). request 페이로드 + streaming chunk 디코딩 + cancel 시그널 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibllm-clientts"></a>
| `src/lib/llm-client.ts` | chat completion + streaming 추상화. claude-cli-transport / OpenAI-compatible HTTP 분기. abort signal 지원. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibllm-providerstestts"></a>
| `src/lib/llm-providers.test.ts` | llm-providers unit 테스트. preset 별 endpoint/model 정규화 + 알 수 없는 provider 분기 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibllm-providersts"></a>
| `src/lib/llm-providers.ts` | LLM provider 정의 매트릭스. OpenAI/Claude/Gemini/Ollama/기타 OpenAI-compatible — 기본 endpoint/모델/auth 헤더. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibmarkdown-image-resolvertestts"></a>
| `src/lib/markdown-image-resolver.test.ts` | markdown-image-resolver unit 테스트. 상대 경로 / asset:// scope / data URI / 외부 URL 분기 검증. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibmarkdown-image-resolverts"></a>
| `src/lib/markdown-image-resolver.ts` | wiki 본문 ![alt](path) → asset:// 또는 data URI 변환. Tauri asset protocol scope ** 의존. file traversal 가능성 주의. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srcliboptimize-research-topictestts"></a>
| `src/lib/optimize-research-topic.test.ts` | optimize-research-topic unit 테스트. 사용자 토픽 → 다중 검색 query 생성 + 결정론 + 빈 응답 케이스. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srcliboptimize-research-topicts"></a>
| `src/lib/optimize-research-topic.ts` | 사용자 research topic 을 LLM 으로 다중 검색 query 로 분해. deep-research 가 query 매트릭스 입력으로 사용. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srcliboutput-language-optionsts"></a>
| `src/lib/output-language-options.ts` | OUTPUT_LANGUAGE_OPTIONS — 21 개 언어 + auto. label/value 쌍. settings UI 가 렌더 + LLM prompt 에 강제 언어. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcliboutput-languagetestts"></a>
| `src/lib/output-language.test.ts` | output-language unit 테스트. auto 분기 (detectLanguage 호출) + configured 직접 사용 + buildLanguageDirective 형식 검증. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcliboutput-languagets"></a>
| `src/lib/output-language.ts` | 출력 언어 결정 + LLM 시스템 프롬프트 강제 언어 directive 생성. auto 면 detectLanguage 호출 fallback. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibpath-utilspropertytestts"></a>
| `src/lib/path-utils.property.test.ts` | path-utils fast-check property 테스트. normalizePath idempotent + joinPath associative + Windows/POSIX 등가 invariant. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibpath-utilstestts"></a>
| `src/lib/path-utils.test.ts` | path-utils unit 테스트. Windows ↔ POSIX 정규화, joinPath/getFileName/getFileStem/getRelativePath 케이스. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibpath-utilsts"></a>
| `src/lib/path-utils.ts` | 크로스플랫폼 경로 헬퍼. normalizePath/joinPath/getFileName/getFileStem/getRelativePath/isAbsolutePath. Tauri / Win 동시 인식. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibpersistintegrationtestts"></a>
| `src/lib/persist.integration.test.ts` | persist integration 테스트. ensureDir + saveReviewItems + saveChatHistory + 디스크 round-trip 시나리오. | [06-data-layer.md#per-project-on-disk-schema-projectpathllm-wiki](06-data-layer.md) |
<a id="srclibpersistts"></a>
| `src/lib/persist.ts` | per-project .llm-wiki/ 영속 IO. saveReviewItems / saveChatHistory / loadChatHistory + ensureDir already-exists 삼킴. | [06-data-layer.md#per-project-on-disk-schema-projectpathllm-wiki](06-data-layer.md) |
<a id="srclibproject-identityts"></a>
| `src/lib/project-identity.ts` | 프로젝트 UUID 관리. .llm-wiki/project.json 의 ProjectIdentity {id, createdAt} + projectRegistry 갱신 (path ↔ id). | [06-data-layer.md#per-project-on-disk-schema-projectpathllm-wiki](06-data-layer.md) |
<a id="srclibproject-mutextestts"></a>
| `src/lib/project-mutex.test.ts` | project-mutex unit 테스트. withProjectLock 직렬화 + 다른 path 병렬 + 예외 시 lock 해제 검증. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibproject-mutexts"></a>
| `src/lib/project-mutex.ts` | withProjectLock — projectPath 별 promise-chain mutex. 같은 path 직렬, 다른 path 병렬. timeout/fairness/re-entrancy 없음. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibproject-storets"></a>
| `src/lib/project-store.ts` | Tauri plugin-store 래퍼. recentProjects / lastProject / llmConfig / providerConfigs / activePresetId / 검색·임베딩·멀티모달·언어·업데이트 상태 영속. | [06-data-layer.md#tauri-plugin-store-app-statejson-키-스키마](06-data-layer.md) |
<a id="srclibraw-source-resolverts"></a>
| `src/lib/raw-source-resolver.ts` | wiki 페이지가 참조하는 원본 source 파일 lookup. 같은 stem 매칭 + 확장자 후보 우선순위. extract-source-images 가 사용. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibreset-project-statetestts"></a>
| `src/lib/reset-project-state.test.ts` | reset-project-state unit 테스트. 모든 store 초기화 + 디스크 영속 클리어 + 진행 중 ingest 취소 검증. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibreset-project-statets"></a>
| `src/lib/reset-project-state.ts` | 프로젝트 전환 시 모든 store + 디스크 영속을 동기 클리어. wiki/chat/review/research/activity/update store 전부 reset. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibreview-utilspropertytestts"></a>
| `src/lib/review-utils.property.test.ts` | review-utils fast-check property 테스트. normalizeReviewTitle idempotent + 중복 입력 → 단일 항목 invariant. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibreview-utilstestts"></a>
| `src/lib/review-utils.test.ts` | review-utils unit 테스트. normalizeReviewTitle 케이스 + addItems 중복 제거 + resolveItem 상태 전이 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibreview-utilsts"></a>
| `src/lib/review-utils.ts` | review 유틸 모음. normalizeReviewTitle (대소문자 + 공백 통일) + 카테고리 매핑 + 액션 후보 생성. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsearch-rrftestts"></a>
| `src/lib/search-rrf.test.ts` | search-rrf unit 테스트. RRF 공식 score(d) = Σ 1/(k + rank_i(d)) 검증 + 두 랭킹 fusion 케이스. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsearchscenariostestts"></a>
| `src/lib/search.scenarios.test.ts` | search 시나리오 테스트. 사전 정의 wiki 인덱스 + 쿼리 매트릭스 → 예상 RRF 결과. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsearchts"></a>
| `src/lib/search.ts` | 메인 검색 진입. embedding query → vector_search_chunks + keyword index 동시 실행 → RRF fusion → 이미지-aware 결과. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsource-delete-decisiontestts"></a>
| `src/lib/source-delete-decision.test.ts` | source-delete-decision unit 테스트. cascade 옵션 분기 + dependent wiki 페이지 발견 + 사용자 확인 흐름 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsource-delete-decisionts"></a>
| `src/lib/source-delete-decision.ts` | 소스 파일 삭제 시 cascade 의사결정. 의존하는 wiki 페이지 자동 감지 + delete/keep/orphan 액션 후보 생성. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsources-mergetestts"></a>
| `src/lib/sources-merge.test.ts` | sources-merge unit 테스트. wiki 본문에 source YAML frontmatter 병합 + 중복 source 제거 + 정렬 안정성 검증. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibsources-mergets"></a>
| `src/lib/sources-merge.ts` | wiki 본문에 source YAML frontmatter 병합. parseSources + mergeSourcesIntoContent — 같은 source path 중복 방지 + 안정 정렬. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibsources-tree-deletetestts"></a>
| `src/lib/sources-tree-delete.test.ts` | sources-tree-delete unit 테스트. cascade 모드 + dependent wiki 페이지 cascade-delete + 빈 디렉터리 정리 케이스. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibsources-tree-deletets"></a>
| `src/lib/sources-tree-delete.ts` | sources/ 트리에서 파일/디렉터리 cascade 삭제. dependent wiki 페이지 자동 감지 + ingest-cache invalidate. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibsweep-chainedreal-llmtestts"></a>
| `src/lib/sweep-chained.real-llm.test.ts` | sweep-chained 실제 LLM 테스트. 여러 review 항목 chained 처리 + 의존성 + 부분 실패 회복 시나리오. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsweep-reviewspropertytestts"></a>
| `src/lib/sweep-reviews.property.test.ts` | sweep-reviews fast-check property 테스트. 임의 review 큐 → idempotent 처리 + 같은 input 같은 output invariant. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsweep-reviewsracetestts"></a>
| `src/lib/sweep-reviews.race.test.ts` | sweep-reviews race 테스트. 동시 sweep 호출 + 같은 review 두 번 처리 방지 + lock 해제 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsweep-reviewsreal-llmtestts"></a>
| `src/lib/sweep-reviews.real-llm.test.ts` | sweep-reviews 실제 LLM 테스트. predefined actions + LLM 으로 검색 query 생성 end-to-end. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsweep-reviewsscenariostestts"></a>
| `src/lib/sweep-reviews.scenarios.test.ts` | sweep-reviews 시나리오 테스트. 사전 정의 review 매트릭스 → 예상 액션 분기. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsweep-reviewstestts"></a>
| `src/lib/sweep-reviews.test.ts` | sweep-reviews unit 테스트 (mock). 큐 처리 + 액션 dispatch + 재시도 + cancel 시그널 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibsweep-reviewsts"></a>
| `src/lib/sweep-reviews.ts` | 비동기 review 일괄 처리. review-store 큐 → predefined actions 또는 LLM-aided 분기 → resolve/dismiss 액션 자동 dispatch. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibtauri-fetchtestts"></a>
| `src/lib/tauri-fetch.test.ts` | tauri-fetch unit 테스트. plugin-http invoke 매핑 + 헤더 처리 + 에러 회복 + abort signal 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibtauri-fetchts"></a>
| `src/lib/tauri-fetch.ts` | tauri-plugin-http 래퍼. fetch-like API 로 unsafe-headers feature 활용 + LLM provider 임의 endpoint 지원. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibtemplatests"></a>
| `src/lib/templates.ts` | wiki 스키마 템플릿 정의. WikiTemplate (research / general / project) 654 라인 — frontmatter 규약 + 초기 페이지 본문. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibtext-chunkertestts"></a>
| `src/lib/text-chunker.test.ts` | text-chunker unit 테스트. heading-aware 분할 + chunk size 한도 + heading_path 메타데이터 보존 케이스. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibtext-chunkerts"></a>
| `src/lib/text-chunker.ts` | wiki 본문 → 청크 분할. heading-aware (## section 경계 우선) + chunk size 한도 + heading_path 메타 추출. ingest 가 호출. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibupdate-checktestts"></a>
| `src/lib/update-check.test.ts` | update-check unit 테스트. GitHub Releases API 호출 + 버전 비교 + dismissedVersion 게이트 검증. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibupdate-checkts"></a>
| `src/lib/update-check.ts` | GitHub Releases API 호출로 새 버전 감지. __APP_VERSION__ 비교 + dismissedVersion 비교 + update-store 갱신. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibutilsts"></a>
| `src/lib/utils.ts` | cn(...inputs) — twMerge(clsx(...)) Tailwind 클래스 머지 헬퍼. 모든 shadcn 컴포넌트가 className prop 머지에 사용. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srclibvision-captionreal-llmtestts"></a>
| `src/lib/vision-caption.real-llm.test.ts` | vision-caption 실제 vision LLM 테스트. PNG/JPG 입력 → factual caption + 다양한 모델 매트릭스 + 결정론. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibvision-captiontestts"></a>
| `src/lib/vision-caption.test.ts` | vision-caption unit 테스트. base64 페이로드 + provider 분기 + 빈 응답 회복 + dedup cache hit 검증. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibvision-captionts"></a>
| `src/lib/vision-caption.ts` | vision LLM caption 생성 추상화. base64 이미지 → factual caption. multimodalConfig 의 endpoint/model 사용. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibvisionreal-llmtestts"></a>
| `src/lib/vision.real-llm.test.ts` | vision LLM 통합 실제 테스트 (caption 외 다른 vision 작업). 다양한 입력/모델 매트릭스 검증. | [08-pdf-ocr-pipeline.md#public-interface](08-pdf-ocr-pipeline.md) |
<a id="srclibweb-searchts"></a>
| `src/lib/web-search.ts` | 웹 검색 추상화. searchApiConfig provider/key/endpoint → 검색 API 호출 → 결과 정규화. deep-research 가 호출. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibwiki-cleanuptestts"></a>
| `src/lib/wiki-cleanup.test.ts` | wiki-cleanup unit 테스트. extractFrontmatterTitle + cleanIndexListing + orphan link 제거 케이스 검증. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibwiki-cleanupts"></a>
| `src/lib/wiki-cleanup.ts` | wiki 본문 정리. extractFrontmatterTitle + cleanIndexListing + orphan wikilink 제거 + frontmatter 정규화. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibwiki-filenametestts"></a>
| `src/lib/wiki-filename.test.ts` | wiki-filename unit 테스트. makeQueryFileName 결정론 + 중복 충돌 회피 + Windows/POSIX 안전 문자 케이스. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibwiki-filenamets"></a>
| `src/lib/wiki-filename.ts` | wiki 파일명 생성. makeQueryFileName — 사용자 query 텍스트를 파일시스템 안전 문자로 + 충돌 시 (n) 접미사. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibwiki-graphts"></a>
| `src/lib/wiki-graph.ts` | wiki 페이지 그래프 빌드. graphology 노드/엣지 + graph-relevance 점수 + Louvain 커뮤니티 + forceatlas2 초기 layout. | [07-llm-integration.md#public-interface](07-llm-integration.md) |
<a id="srclibwiki-page-deletetestts"></a>
| `src/lib/wiki-page-delete.test.ts` | wiki-page-delete unit 테스트. cascade 옵션 + 의존하는 source 파일 처리 + lancedb 인덱스 클린업 케이스. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srclibwiki-page-deletets"></a>
| `src/lib/wiki-page-delete.ts` | wiki 페이지 삭제 + cascade. cascadeDeleteWikiPage — vector_delete_page + ingest-cache invalidate + sources/ orphan 처리. | [06-data-layer.md#purpose](06-data-layer.md) |
<a id="srcmaintsx"></a>
| `src/main.tsx` | React 18 createRoot mount. StrictMode + i18n side-effect import + <App/> 한 번 렌더. webview 진입 모듈. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoresactivity-storets"></a>
| `src/stores/activity-store.ts` | Zustand activity 스토어. 진행중 ingest/lint/query 활동 피드. addItem (id 반환) / appendDetail / clearDone. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoreschat-storets"></a>
| `src/stores/chat-store.ts` | Zustand chat 스토어. conversations / messages / isStreaming / streamingContent / mode (chat or research) / ingestSource + finalizeStream. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoresresearch-storets"></a>
| `src/stores/research-store.ts` | Zustand research 스토어. tasks 큐 + 6 상태 (queued / searching / synthesizing / saving / done / error) + maxConcurrent=3 + addTask / updateTask / getNextQueued. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoresreview-storepropertytestts"></a>
| `src/stores/review-store.property.test.ts` | review-store fast-check property 테스트. addItems 중복 제거 invariant + resolveItem idempotent. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoresreview-storetestts"></a>
| `src/stores/review-store.test.ts` | review-store unit 테스트. addItem / addItems (de-dup by normalizeReviewTitle) / resolveItem / dismissItem 케이스. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoresreview-storets"></a>
| `src/stores/review-store.ts` | Zustand review 스토어. items + addItem / addItems / resolveItem / dismissItem / clearResolved. normalizeReviewTitle 로 dedup. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoresupdate-storets"></a>
| `src/stores/update-store.ts` | Zustand update 스토어. checking / lastResult / lastCheckedAt / dismissedVersion / enabled + hasAvailableUpdate / shouldShowUpdateBanner 셀렉터. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srcstoreswiki-storets"></a>
| `src/stores/wiki-store.ts` | Zustand wiki 스토어 (가장 큰 스토어). project / fileTree / selectedFile / fileContent / activeView / llm/provider/preset/search/embedding/multimodal/output 설정. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="srctest-helpersdeferredts"></a>
| `src/test-helpers/deferred.ts` | promise resolve/reject 핸들 노출 헬퍼. 테스트에서 외부에서 promise 진행을 컨트롤할 때 사용. | [leaf-utility] |
<a id="srctest-helpersfs-tempts"></a>
| `src/test-helpers/fs-temp.ts` | 테스트용 임시 디렉터리 생성/정리 헬퍼. 각 테스트가 격리된 temp 폴더에서 실제 파일시스템 작업 가능. | [leaf-utility] |
<a id="srctest-helpersload-test-envts"></a>
| `src/test-helpers/load-test-env.ts` | .env.test.local 파일을 process.env 로 로드. real-llm 테스트가 endpoint/key 를 가져올 때 사용. vitest setupFiles. | [leaf-utility] |
<a id="srctest-helpersmock-stream-chatts"></a>
| `src/test-helpers/mock-stream-chat.ts` | streaming chat completion mock. SSE-style chunk 시뮬레이션 + abort + 에러 분기. mock LLM 테스트의 핵심. | [leaf-utility] |
<a id="srctest-helpersreal-contentts"></a>
| `src/test-helpers/real-content.ts` | 테스트용 실제 콘텐츠 fixture. 마크다운 / PDF / 이미지 샘플 → real-llm 테스트가 안정적인 입력 사용. | [leaf-utility] |
<a id="srctest-helpersscenariosenrich-scenariosts"></a>
| `src/test-helpers/scenarios/enrich-scenarios.ts` | enrich-wikilinks 시나리오 정의. wiki 입력 매트릭스 + 예상 wikilink 출력 — scenarios.test.ts 가 import. | [leaf-utility] |
<a id="srctest-helpersscenariosingest-scenariosts"></a>
| `src/test-helpers/scenarios/ingest-scenarios.ts` | ingest 시나리오 정의. 입력 (PDF/docx/이미지/텍스트) 매트릭스 + 예상 wiki 페이지 — scenarios.test.ts 가 import. | [leaf-utility] |
<a id="srctest-helpersscenarioslint-scenariosts"></a>
| `src/test-helpers/scenarios/lint-scenarios.ts` | lint 시나리오 정의. wiki 디렉터리 매트릭스 + 예상 lint 결과 — scenarios.test.ts 가 import. | [leaf-utility] |
<a id="srctest-helpersscenariosmaterializets"></a>
| `src/test-helpers/scenarios/materialize.ts` | 시나리오 fixture 를 실제 임시 디렉터리에 materialize. fs-temp 와 결합해 테스트 격리. | [leaf-utility] |
<a id="srctest-helpersscenariossearch-scenariosts"></a>
| `src/test-helpers/scenarios/search-scenarios.ts` | search 시나리오 정의. 인덱스 + 쿼리 매트릭스 + 예상 RRF 결과 — scenarios.test.ts 가 import. | [leaf-utility] |
<a id="srctest-helpersscenariossweep-scenariosts"></a>
| `src/test-helpers/scenarios/sweep-scenarios.ts` | sweep-reviews 시나리오 정의. 사전 정의 review 매트릭스 + 예상 액션 — scenarios.test.ts 가 import. | [leaf-utility] |
<a id="srctest-helpersscenariostypests"></a>
| `src/test-helpers/scenarios/types.ts` | 시나리오 fixture 공유 타입 정의. Scenario / Fixture / ExpectedResult — scenarios 디렉터리 다른 파일이 사용. | [leaf-utility] |
<a id="srctypeswikits"></a>
| `src/types/wiki.ts` | TS 측 IPC 타입. WikiProject {id, name, path}, FileNode {name, path, is_dir, children?}, WikiPage {path, content, frontmatter}. | [06-data-layer.md#tsrust-type-mapping-ipc-경계](06-data-layer.md) |
<a id="srcvite-envdts"></a>
| `src/vite-env.d.ts` | Vite 환경 타입 선언. ImportMetaEnv 확장 + __APP_VERSION__ declare const string. webview 모듈에서 사용. | [03-frontend.md#public-interface](03-frontend.md) |
<a id="tsconfigappjson"></a>
| `tsconfig.app.json` | TS app 컴파일 설정. target ES2020, lib ES2020+DOM, jsx react-jsx, strict, paths @/* → ./src/*. include ["src"]. | [01-tech-stack.md#typescript-config](01-tech-stack.md) |
<a id="tsconfigjson"></a>
| `tsconfig.json` | TS root project references 진입점. files []. references → tsconfig.app.json + tsconfig.node.json. | [01-tech-stack.md#typescript-config](01-tech-stack.md) |
<a id="tsconfignodejson"></a>
| `tsconfig.node.json` | TS node-side 설정. target ES2022, lib ES2023, include ["vite.config.ts"]. vite config 자체 type check. | [01-tech-stack.md#typescript-config](01-tech-stack.md) |
<a id="viteconfigts"></a>
| `vite.config.ts` | Vite 번들러 설정. plugin-react + tailwindcss. alias @ → ./src. __APP_VERSION__ define. dev port 1420 strict. vitest setupFiles. | [01-tech-stack.md#vite-config-viteconfigts](01-tech-stack.md) |

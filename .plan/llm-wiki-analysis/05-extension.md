# 05 — Chrome Extension (MV3 Web Clipper)

> Mozilla Readability + Turndown 은 third-party vendored asset — `[vendored]` 마크 처리하고 코드 audit 하지 않아요.

## Purpose

`extension/` 는 Chrome Manifest V3 액션 익스텐션 한 개로, 사용자가 임의의 웹 페이지에서 툴바 아이콘을 누르면 480 × 500 popup 이 떠서 (1) 활성 탭에 Readability + Turndown 을 inject 해 본문 마크다운을 추출하고, (2) 로컬에서 돌아가는 `llm_wiki` Tauri 앱의 clip-server (`http://127.0.0.1:19827`) 에 POST 해 자동으로 ingest 큐에 넣는 한 방향 web-to-wiki 파이프라인이에요. 모든 통신은 사용자 머신 안 loopback 으로만 일어나도록 host_permissions 가 19827 단일 호스트로 제한돼 있고, content script 는 popup 클릭 시점에만 `chrome.scripting.executeScript` 로 동적 인젝션돼서 권한이 `activeTab + scripting` 두 개로 쪼개져요. background service worker, persistent content script, options page 어느 것도 없어 trust surface 는 popup.html ↔ inject 함수 ↔ Tauri clip-server 세 점이에요. 보안 경계 관점에서는 (a) extension → page DOM (Readability inject), (b) extension popup → 로컬 HTTP API (인증 없이 19827 신뢰), (c) Tauri 앱 → extension (via /projects 응답 페이로드) 세 단계가 모두 같은 머신 안에서 끝난다는 가정에 깔려 있어요. CSP 는 popup.html 에 명시되지 않고 MV3 기본값에 의존하고, Readability/Turndown 본체는 vendored 라 매니페스트가 web_accessible_resources 로 노출시키는 점이 별도 risk vector 예요.

## Public Interface

- `manifest.json — manifest_version 3 — extension/manifest.json:1-27 — name=LLM Wiki Clipper version=0.1.0; permissions=[activeTab, scripting]; host_permissions=[http://127.0.0.1:19827/*]; action.default_popup=popup.html; web_accessible_resources=[Readability.js, Turndown.js] matches=<all_urls>`
- `popup.html — UI document — extension/popup.html:1-155 — 480×500 fixed-size; 4 fields (Project select, Title input, URL preview, Content preview); 1 button (#clipBtn); status bar at top; loads popup.js as last <script>`
- `API_URL — const string — extension/popup.js:1 — "http://127.0.0.1:19827"`
- `checkConnection — async () => Promise<boolean> — extension/popup.js:13-29 — GET /status; on success → loadProjects()`
- `loadProjects — async () => Promise<void> — extension/popup.js:31-58 — GET /projects (multi); fallback GET /project (current single); fills <select id="projectSelect">`
- `extractContent — async () => Promise<void> — extension/popup.js:60-173 — query active tab → inject Readability+Turndown → executeScript(func) → markdown 추출 → fallbackExtract on failure`
- `fallbackExtract — async (tabId: number) => Promise<void> — extension/popup.js:176-200 — DOM cloneNode + script/style/nav/header/footer/.sidebar/.ad/.comments removal → innerText 50000-char cap`
- `sendClip — async () => Promise<void> — extension/popup.js:202-243 — POST /clip with {title, url, content, projectPath}; status bar feedback + button text mutation`
- `resizePreview — () => void — extension/popup.js:248-258 — bottom-space heuristic, clamp 100..300px on #contentPreview.maxHeight`
- `Top-level IIFE — extension/popup.js:260-269 — checkConnection → extractContent → enable/disable clipBtn → setTimeout(resizePreview, 100)`
- `Readability.js — vendored MIT (Mozilla) — extension/Readability.js — [vendored] reader-mode article extractor; injected into page via chrome.scripting.executeScript`
- `Turndown.js — vendored MIT — extension/Turndown.js — [vendored] HTML → Markdown converter; popup.js extends with custom rules (tableCell / tableRow / table / removeSmallImages)`
- `icon{16,48,128}.png — [asset] — extension/icon16.png + icon48.png + icon128.png — toolbar + manifest icons; PNG bytes only, not audited`

## Internal Risk

### unsafe blocks (Rust)

None observed in this domain. (Vanilla JS only.)

### `.unwrap()` / `.expect()` chains (Rust)

None observed in this domain. (Vanilla JS only.)

### `panic!` / `unreachable!` / `todo!` (Rust)

None observed in this domain. (Vanilla JS only.)

### `Mutex::lock` / `RwLock::write` acquisition + drop discipline (Rust)

None observed in this domain. (Vanilla JS only.)

### FFI loads, `extern "C"`, dlopen-style (Rust → pdfium et al.)

None observed in this domain. (Vanilla JS only.)

### Result swallow (TypeScript)

`extension/popup.js` 는 plain JS (TypeScript 가 아니라서 `as any` 패턴은 부재). 하지만 동등 risk 패턴 — 빈 `catch {}` + console-only 로깅 부재 + cross-origin trust assumption — 가 다음과 같이 누적돼요:

빈 `catch {}` — `/status` 호출 실패를 통째로 묻고 disconnected 분기로 fallthrough. 19827 가 죽었는지 / 다른 프로세스가 그 포트를 점유했는지 / 인증서 문제인지 사용자도 개발자도 구분 불가:

```javascript extension/popup.js:13-29
async function checkConnection() {
  try {
    const res = await fetch(`${API_URL}/status`, { method: "GET" });
    const data = await res.json();
    if (data.ok) {
      statusBar.className = "status connected";
      statusBar.textContent = "✓ Connected to LLM Wiki";
      await loadProjects();
      return true;
    }
  } catch {}
  statusBar.className = "status disconnected";
  statusBar.textContent = "✗ LLM Wiki app is not running";
```

`/projects` 와 fallback `/project` 둘 다 빈 catch — 둘 다 throw 하면 사용자에게는 "No projects" 만 보이고 원인 파악 어려움:

```javascript extension/popup.js:31-58
async function loadProjects() {
  try {
    const res = await fetch(`${API_URL}/projects`, { method: "GET" });
    ...
  } catch {}
  try {
    const res = await fetch(`${API_URL}/project`, { method: "GET" });
    ...
  } catch {
    projectSelect.innerHTML = '<option value="">No projects</option>';
  }
}
```

`extractContent` 안에서 inject 된 Readability + Turndown 이 throw 하면 `err.message` 를 그대로 contentPreview 에 텍스트로 흘려요 — page-controlled DOM 의 error string 이 popup UI 에 escape 없이 입력되는 텍스트노드라 XSS 자체는 blocked 이지만, 페이지가 의도적으로 가짜 에러를 throw 해 사용자 추출 흐름을 가로챌 수 있어요:

```javascript extension/popup.js:139-141
        } catch (err) {
          return { error: err.message };
        }
```

`fallbackExtract` 의 selector list (`.sidebar`, `.ad`, `.comments`) 는 string 기반 — 페이지가 의도적으로 본문을 `class="ad"` 안에 숨기면 추출 결과가 비어요. 입력 검증 부재 + silent fallthrough:

```javascript extension/popup.js:181-189
      ["script", "style", "nav", "header", "footer", ".sidebar", ".ad", ".comments"]
        .forEach((sel) => clone.querySelectorAll(sel).forEach((el) => el.remove()));

      return clone.innerText
        .split("\n")
        .map((l) => l.trim())
        .filter((l) => l.length > 0)
        .join("\n\n")
        .slice(0, 50000);
```

`sendClip` 의 응답 파싱이 `data.error` 를 그대로 statusBar 텍스트에 넣어요 — 서버 응답이 textContent 로 흘러서 DOM injection 은 아니지만, server-controlled 문자열이 사용자 UI 에 그대로 표면화돼요. 길이 검증 / sanitize 없음:

```javascript extension/popup.js:226-237
    const data = await res.json();
    if (data.ok) {
      const projectName = projectSelect.options[projectSelect.selectedIndex]?.textContent || "project";
      statusBar.className = "status success";
      statusBar.textContent = `✓ Saved to ${projectName}`;
      clipBtn.textContent = "✓ Clipped!";
    } else {
      statusBar.className = "status error";
      statusBar.textContent = `✗ Error: ${data.error}`;
      clipBtn.disabled = false;
    }
```

MV3 trust boundary risk — `host_permissions` 가 `http://127.0.0.1:19827/*` 라 plain-HTTP 이고 19827 의 응답에 인증 토큰이 없어요. 같은 머신 안에서 다른 프로그램이 19827 을 listen 하기만 하면 extension 이 그 응답을 신뢰해 `data.path` 를 `<select>` value 로 넣어버려요 (popup.js:51-53). user-controlled `data.path` 가 그대로 다음 POST `/clip` 의 `projectPath` 에 들어가는 chain 이라 로컬 권한 격리가 부족할 때 문제 vector:

```javascript extension/popup.js:48-54
    const res = await fetch(`${API_URL}/project`, { method: "GET" });
    const data = await res.json();
    if (data.ok && data.path) {
      const name = data.path.replace(/\\/g, "/").split("/").pop() || data.path;
      projectSelect.innerHTML = `<option value="${data.path}">${name}</option>`;
    }
```

`projectSelect.innerHTML = \`<option value="${data.path}">${name}</option>\`` 라인은 `data.path` 와 derived `name` 을 escape 없이 innerHTML 에 보간 — 19827 응답이 `path: '"></option><script>...</script>'` 같은 페이로드를 보내면 popup 안에서 스크립트가 실행돼요. extension popup CSP 가 inline-script 를 차단하는 게 일반적이지만 manifest 에 명시 CSP 가 없어서 MV3 기본값에만 의존:

```javascript extension/popup.js:53
      projectSelect.innerHTML = `<option value="${data.path}">${name}</option>`;
```

content-script injection ordering — Readability 와 Turndown 이 `await chrome.scripting.executeScript({files: [...]})` 한 번에 inject 되는데 두 스크립트가 페이지의 기존 `window.Readability` / `window.TurndownService` 를 덮어써요. MV3 ISOLATED world 가 기본이지만 popup.js 에 `world` 옵션이 명시 안 돼서 injected 함수 안에서 `window.Readability` 를 참조하는 fallback path 가 page-world 의 동명 객체와 충돌 가능:

```javascript extension/popup.js:69-73
    await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      files: ["Readability.js", "Turndown.js"],
    });
```

extension manifest 가 `web_accessible_resources` 로 Readability.js / Turndown.js 를 `<all_urls>` 에 노출 — 임의의 웹 페이지가 `chrome-extension://<id>/Readability.js` 로 fetch 가능. 두 스크립트의 버전 fingerprint 가 extension presence 추적 vector 가 됩니다:

```json extension/manifest.json:21-26
  "web_accessible_resources": [
    {
      "resources": ["Readability.js", "Turndown.js"],
      "matches": ["<all_urls>"]
    }
  ]
```

## Cross-refs

- 19827 포트의 Tauri 측 endpoint 정의 (clip_server `/status`, `/projects`, `/project`, `/clip`) 는 [04-backend-rust.md#evidence](04-backend-rust.md#evidence) 의 `clip_server` 섹션 참고 — extension 은 그 prefix 의 클라이언트일 뿐.
- 같은 19827 호출을 frontend `App.tsx:271-285` 가 POST 하는 부분은 [03-frontend.md#evidence](03-frontend.md#evidence) 와 동일 IPC 채널.
- `data.path` → `projectPath` 흐름은 결국 `wiki/sources/...` 로 ingest 되는데 그 영속화 / 큐 흐름은 [06-data-layer.md](06-data-layer.md) 에서 다뤄요.
- 소스 매핑 행: [extension/manifest.json](50-source-mapping.md#extensionmanifestjson), [extension/popup.html](50-source-mapping.md#extensionpopuphtml), [extension/popup.js](50-source-mapping.md#extensionpopupjs), [extension/Readability.js](50-source-mapping.md#extensionreadabilityjs), [extension/Turndown.js](50-source-mapping.md#extensionturndownjs).

## Evidence

- `extension/manifest.json:2` — `manifest_version: 3`. MV2 deprecated 이슈 회피.
- `extension/manifest.json:6` — `permissions: ["activeTab", "scripting"]`. host_permissions 가 따로라 사용자 install-time prompt 가 최소화돼요. activeTab 은 click 시점에만 권한 부여.
- `extension/manifest.json:7` — `host_permissions: ["http://127.0.0.1:19827/*"]`. plain HTTP, loopback 한정. 인증서 / TLS 무관.
- `extension/manifest.json:8-15` — `action.default_popup: "popup.html"` + 3 size icon. background service worker / content_scripts 등록 없음.
- `extension/manifest.json:21-26` — Readability + Turndown 을 `<all_urls>` web_accessible_resources 로 노출. extension fingerprint vector.
- `extension/popup.html:7-16` — 480px 너비 / 500px 높이 고정. `overflow: hidden !important` 로 popup-level 스크롤 차단.
- `extension/popup.html:17-26` — 다크 헤더 (#1a1a2e), 📚 emoji 아이콘.
- `extension/popup.html:120` — `<div id="statusBar" class="status disconnected">Checking connection...</div>` 가 초기 상태.
- `extension/popup.html:124-127` — `<select id="projectSelect">` 첫 옵션 "Loading projects..." → loadProjects 가 채움.
- `extension/popup.html:144-146` — `<button id="clipBtn" disabled>📎 Clip to Wiki</button>` 기본 비활성. 추출 성공 후 enable.
- `extension/popup.html:153` — `<script src="popup.js"></script>` 가 마지막 줄. inline script 부재 — MV3 CSP 친화적.
- `extension/popup.js:1` — `const API_URL = "http://127.0.0.1:19827"`.
- `extension/popup.js:3-8` — DOM lookups 한 번에 캡처 (statusBar, titleInput, urlPreview, contentPreview, clipBtn, projectSelect).
- `extension/popup.js:10-11` — `extractedContent` + `pageUrl` 모듈 스코프 변수 (popup 인스턴스 라이프사이클 동안 유지).
- `extension/popup.js:13-29` — `checkConnection` GET /status; `data.ok` true 만 connected 분기. catch 무음.
- `extension/popup.js:31-46` — `loadProjects` 가 multi-project (`/projects`) 우선, 빈 catch 후 single (`/project`) fallback.
- `extension/popup.js:51-53` — server 응답 `data.path` 를 innerHTML 보간. (Risk 섹션 참조.)
- `extension/popup.js:60-67` — `chrome.tabs.query({active:true, currentWindow:true})` 로 활성 탭 1 개. 그 후 popup.js 가 tab.url / tab.title 을 채움.
- `extension/popup.js:69-73` — files inject (Readability.js → Turndown.js 순서).
- `extension/popup.js:76-143` — 두 번째 executeScript: anonymous function 이 page-context 에서 실행. `documentClone = document.cloneNode(true)` → `new window.Readability(documentClone).parse()` → `new window.TurndownService(...)` → 4 개 custom rule 등록 (tableCell / tableRow / table / removeSmallImages) → `turndown.turndown(article.content)`.
- `extension/popup.js:91-93` — Turndown 옵션: `headingStyle:"atx"`, `codeBlockStyle:"fenced"`, `bulletListMarker:"-"`.
- `extension/popup.js:97-117` — table rule trio. `table` rule 이 헤더 separator 를 line 1 에 splice insert 해 GFM 표 형성.
- `extension/popup.js:120-128` — `removeSmallImages` 가 width/height < 10 인 이미지 제거 — tracking pixel 컷.
- `extension/popup.js:139-141` — page-context try/catch — error message 만 popup 으로 반환.
- `extension/popup.js:145-152` — popup-context 결과 처리: `result.error` 면 `fallbackExtract(tab.id)`.
- `extension/popup.js:154-157` — `result.title.length > 5` 일 때만 readability title 채택 — Untitled / 1글자 잡음 방지.
- `extension/popup.js:162-164` — excerpt 가 있으면 `📝 ${excerpt}\n\n---\n\n${markdown}` prefix.
- `extension/popup.js:170-172` — outer try/catch 가 인젝션 자체 실패 시 `Error: ${err.message}` 를 contentPreview 에 표시.
- `extension/popup.js:176-200` — `fallbackExtract` — Readability 실패 시 DOM cloneNode 후 selector blacklist 로 boilerplate 제거 + innerText 50000-char 자름.
- `extension/popup.js:202-208` — `sendClip` 이 selectedProject 비면 status error, 즉시 return.
- `extension/popup.js:215-224` — POST /clip 페이로드 4 필드 (`title`, `url`, `content`, `projectPath`). Content-Type JSON.
- `extension/popup.js:228-237` — 응답 `data.ok` 분기 (success/error). 실패 시 `data.error` 를 그대로 보여줌 + 버튼 재활성화.
- `extension/popup.js:238-242` — fetch 자체 throw (네트워크 끊김) 시 `Connection failed: ${err.message}`.
- `extension/popup.js:245` — `clipBtn.addEventListener("click", sendClip)` — 단일 click handler.
- `extension/popup.js:248-258` — `resizePreview` 가 `previewRect.top` 기준 bottom-space 계산, 100..300px clamp.
- `extension/popup.js:260-269` — top-level IIFE — checkConnection → extractContent (병렬 아님, await 순차) → connected 아니면 button disable + label 변경 → setTimeout(resizePreview, 100).
- `extension/Readability.js` — 87.9 KB Mozilla MIT vendored. `[vendored]` 마크.
- `extension/Turndown.js` — 26.3 KB MIT vendored. `[vendored]` 마크.
- `extension/icon{16,48,128}.png` — 433 B / 2.2 KB / 5.9 KB. `[asset]` 마크.
- popup.js 의 빈 `catch {}` 3 곳 (popup.js:23, 46, 55). DOM mutation via innerHTML 1 곳 (popup.js:53). textContent 로 server-controlled string 5 곳 (popup.js:25, 149, 198, 235, 240).

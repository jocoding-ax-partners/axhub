# 06 — Data Layer (Storage, Persistence, IPC Payloads)

## Purpose

이 도메인은 `llm_wiki` 가 디스크와 IPC 경계를 가로지르며 가지고 있는 모든 영속 상태를 다루는 슬라이스예요. 구체적으로 (a) Tauri `plugin-store` (`app-state.json`) 에 들어 있는 앱 전역 설정 — 최근 프로젝트 / 마지막 프로젝트 / LLM 설정 / 임베딩 / 멀티모달 / 검색 API / 출력 언어 / 업데이트 체크 / 프로젝트 레지스트리 — 와, (b) 프로젝트 폴더 안 `.llm-wiki/` 디렉터리에 들어 있는 per-project 영속 상태 (project identity UUID, 리뷰 항목, 채팅 히스토리, ingest queue, LanceDB v1/v2 테이블), 그리고 (c) 이 두 저장소를 실제로 채우는 동시성 메커니즘 — `withProjectLock` mutex, `auto-save` debounce 구독, `resetProjectState` 의 동기 클리어 — 를 묶어요. 그 위에 (d) TS 타입 (`src/types/wiki.ts`) ↔ Rust 타입 (`src-tauri/src/types/wiki.rs`) 의 IPC 경계 매핑과, (e) wiki 파일 / source 트리 / sources YAML 프론트매터 / wiki 파일명에 작용하는 순수한 마크다운/문자열 헬퍼들 — `mergeSourcesIntoContent`, `cascadeDeleteWikiPage`, `cleanIndexListing`, `makeQueryFileName`, `findRawSourceForImage`, `getFileCategory` — 이 같이 들어와요. 슬라이스가 이렇게 묶이는 이유는, 이들 모두가 "한 ingest 가 다른 ingest 와 같은 파일을 동시에 만지면 어떻게 되는가" 라는 동일한 race 면을 공유하고, 또한 디스크에 어떤 모양의 바이트가 떨어져야 다음 부팅에서 정확히 복원되는지를 함께 정의하기 때문이에요.

## Public Interface

### TS↔Rust type mapping (IPC 경계)

| TS (`src/types/wiki.ts`) | Rust (`src-tauri/src/types/wiki.rs`) | drift 메모 |
|---|---|---|
| `WikiProject { id: string; name: string; path: string }` (`src/types/wiki.ts:1-7`) | `WikiProject { name: String, path: String }` (`src-tauri/src/types/wiki.rs:3-7`) | **drift**: TS 는 `id` (project UUID) 를 가지고 있지만 Rust 쪽 struct 에는 없어요. Rust 는 `create_project` / `open_project` 에서 `name + path` 만 채워서 돌려주고, `id` 는 TS 가 직후에 `ensureProjectId()` 로 채워 넣어요 (`src/lib/project-identity.ts:47`). |
| `FileNode { name: string; path: string; is_dir: boolean; children?: FileNode[] }` (`src/types/wiki.ts:9-14`) | `FileNode { name: String, path: String, is_dir: bool, #[serde(skip_serializing_if = "Option::is_none")] children: Option<Vec<FileNode>> }` (`src-tauri/src/types/wiki.rs:9-16`) | 정렬 일치, optional 직렬화 정렬도 일치 (`Option::None` → JSON 미출현 ↔ TS `children?`). |
| `WikiPage { path: string; content: string; frontmatter: Record<string, unknown> }` (`src/types/wiki.ts:16-20`) | (Rust 측 정의 없음 — `read_file` / `write_file` 가 raw `String` 만 주고받음) | TS-only 도메인 모델이에요. Rust 는 마크다운 본문을 이해하지 않고 단순 byte stream 으로 다뤄요. 프론트매터 파싱은 TS 에서 `extractFrontmatterTitle` (`src/lib/wiki-cleanup.ts:74-77`), `parseSources` (`src/lib/sources-merge.ts:27-51`) 가 담당. |

### Tauri `plugin-store` (`app-state.json`) 키 스키마

`load(STORE_NAME, { autoSave: true, defaults: {} })` 로 한 store 를 공유 — 모든 키가 이 한 파일에 들어가요. 호출자: `src/lib/project-store.ts:9-11`, `src/lib/project-identity.ts:72-74`.

| 키 | 타입 | 호출 지점 |
|---|---|---|
| `recentProjects` | `WikiProject[]` (length-cap 10) | `getRecentProjects` (`src/lib/project-store.ts:13-17`), `addToRecentProjects` (31-39), `removeFromRecentProjects` (111-128) |
| `lastProject` | `WikiProject \| null` | `getLastProject` (19-23), `saveLastProject` (25-29), `removeFromRecentProjects` (124-127, 동반 삭제) |
| `llmConfig` | `LlmConfig` | `saveLlmConfig` / `loadLlmConfig` (45-53) |
| `providerConfigs` | `ProviderConfigs` | `saveProviderConfigs` / `loadProviderConfigs` (55-63) |
| `activePresetId` | `string \| null` | `saveActivePresetId` / `loadActivePresetId` (65-73) |
| `searchApiConfig` | `SearchApiConfig` | `saveSearchApiConfig` / `loadSearchApiConfig` (77-85) |
| `embeddingConfig` | `EmbeddingConfig` | `saveEmbeddingConfig` / `loadEmbeddingConfig` (89-97) |
| `multimodalConfig` | `MultimodalConfig` | `saveMultimodalConfig` / `loadMultimodalConfig` (101-109) |
| `language` | `string` | `saveLanguage` / `loadLanguage` (132-140) |
| `outputLanguage` | `OutputLanguage` | `saveOutputLanguage` / `loadOutputLanguage` (144-152) |
| `updateCheckState` | `PersistedUpdateCheckState { enabled, lastCheckedAt, dismissedVersion }` | `saveUpdateCheckState` / `loadUpdateCheckState` (161-181) |
| `projectRegistry` | `Record<string, ProjectRegistryEntry>` (UUID → `{ id, path, name, lastOpened }`) | `loadRegistry` / `upsertProjectInfo` / `getProjectPathById` / `getProjectIdByPath` (`src/lib/project-identity.ts:76-131`) |

### Per-project on-disk schema (`<projectPath>/.llm-wiki/`)

| 경로 | 내용 | writer / reader |
|---|---|---|
| `.llm-wiki/project.json` | `ProjectIdentity { id: <uuid>, createdAt: <ms> }` | `ensureProjectId` 가 없으면 생성 + `crypto.randomUUID()` (`src/lib/project-identity.ts:47-68`) |
| `.llm-wiki/review.json` | `ReviewItem[]` (전체 덤프) | `saveReviewItems` / `loadReviewItems` (`src/lib/persist.ts:11-25`) |
| `.llm-wiki/conversations.json` | `Conversation[]` | `saveChatHistory` (`src/lib/persist.ts:32-62`), `loadChatHistory` (64-112) |
| `.llm-wiki/chats/<convId>.json` | `DisplayMessage[]` (대화별 마지막 100 개) | 위 동일, conversation 마다 한 파일 |
| `.llm-wiki/chat-history.json` | (legacy) flat array OR combined `{ conversations, messages }` | `loadChatHistory` 의 fallback path (84-110) — write 경로는 이미 제거 |
| `.llm-wiki/lancedb/` | LanceDB 디렉터리 — `wiki_vectors` (v1, page 단위) + `wiki_chunks_v2` (v2, chunk 단위) | `db_path` (`src-tauri/src/commands/vectorstore.rs:45-47`) |

`ensureDir(projectPath)` (`src/lib/persist.ts:6-9`) 는 첫 save 마다 `.llm-wiki/` + `.llm-wiki/chats/` 를 만들면서 `.catch(() => {})` 로 already-exists 만 삼켜요.

### Tauri command surface (data-layer 슬라이스)

#### `commands/project.rs`

- `create_project — fn(name: String, path: String) -> Result<WikiProject, String> — src-tauri/src/commands/project.rs:9-12 — 새 프로젝트 폴더 생성 + 9 개 표준 sub-dir + schema.md/purpose.md/wiki/index.md/wiki/log.md/wiki/overview.md/.obsidian/* 보일러플레이트 작성 후 forward-slash 정규화된 path 반환`
- `open_project — fn(path: String) -> Result<WikiProject, String> — src-tauri/src/commands/project.rs:239-278 — 폴더 존재 + dir 여부 + schema.md + wiki/ dir 검증 후 디렉터리 이름에서 project name 도출, forward-slash 정규화된 path 반환`

#### `commands/vectorstore.rs` (LanceDB 래핑)

- `vector_upsert — async fn(project_path: String, page_id: String, embedding: Vec<f32>) -> Result<(), String> — :97-146 — v1 단일-임베딩 upsert. v1 테이블이 있으면 delete-then-add, 없으면 create_table.`
- `vector_search — async fn(project_path: String, query_embedding: Vec<f32>, top_k: usize) -> Result<Vec<VectorSearchResult>, String> — :149-213 — v1 KNN, score = 1.0 / (1.0 + distance).`
- `vector_delete — async fn(project_path: String, page_id: String) -> Result<(), String> — :216-250 — v1 페이지 삭제.`
- `vector_count — async fn(project_path: String) -> Result<usize, String> — :253-284 — v1 row 개수.`
- `vector_upsert_chunks — async fn(project_path: String, page_id: String, chunks: Vec<ChunkUpsertInput>) -> Result<(), String> — :413-478 — v2 chunk-단위 batch upsert. delete-then-add by page_id. 빈 chunks 는 no-op (transient ingest 실패 시 기존 인덱스 보존).`
- `vector_search_chunks — async fn(project_path: String, query_embedding: Vec<f32>, top_k: usize) -> Result<Vec<ChunkSearchResult>, String> — :483-568 — v2 chunk KNN, chunk_text + heading_path 메타데이터 포함 반환.`
- `vector_delete_page — async fn(project_path: String, page_id: String) -> Result<(), String> — :573-609 — v2 page_id 의 모든 chunk 삭제. idempotent.`
- `vector_count_chunks — async fn(project_path: String) -> Result<usize, String> — :613-645 — v2 chunk 개수.`
- `vector_legacy_row_count — async fn(project_path: String) -> Result<usize, String> — :652-683 — v1 row 개수 (settings 의 "re-index to v2" 프롬프트 게이트).`
- `vector_drop_legacy — async fn(project_path: String) -> Result<(), String> — :689-715 — v1 테이블 drop.`

#### Rust 타입 — IPC 페이로드

- `VectorSearchResult { page_id: String, score: f32 }` (`vectorstore.rs:13-18`) — v1 응답
- `ChunkSearchResult { chunk_id, page_id, chunk_index: u32, chunk_text, heading_path, score: f32 }` (`vectorstore.rs:23-31`) — v2 응답
- `ChunkUpsertInput { chunk_index: u32, chunk_text, heading_path, embedding: Vec<f32> }` (`vectorstore.rs:37-43`) — v2 요청. `chunk_id` 는 클라이언트가 못 정함 — 서버가 `${page_id}#${chunk_index}` 로 강제 (:372).

### TS 헬퍼 (data-layer 도메인)

- `withProjectLock<T>(projectPath: string, fn: () => Promise<T>): Promise<T> — src/lib/project-mutex.ts:32-73 — projectPath 별 promise-chain mutex. 같은 path 면 직렬, 다른 path 면 병렬. timeout/fairness/re-entrancy 없음.`
- `__resetProjectLocksForTesting(): void — src/lib/project-mutex.ts:78-80 — 테스트 hook.`
- `ensureProjectId(projectPath: string): Promise<string> — src/lib/project-identity.ts:47-68 — UUID 보장. 없으면 `crypto.randomUUID()` 로 생성 + 디스크 write.`
- `loadRegistry / upsertProjectInfo / getProjectPathById / getProjectIdByPath — src/lib/project-identity.ts:76-131 — UUID ↔ path 양방향 룩업.`
- `setupAutoSave(): void — src/lib/auto-save.ts:9-32 — review 1s + chat 2s debounce 의 zustand 구독 시작 (streaming 중엔 chat skip).`
- `resetProjectState(): Promise<void> — src/lib/reset-project-state.ts:16-71 — 4 개 zustand store 동기 클리어 + ingest-queue.pauseQueue() + graph-relevance.clearGraphCache() 모듈 캐시 await.`
- `parseSources(content: string): string[] — src/lib/sources-merge.ts:27-51 — YAML inline `sources: [...]` 또는 multi-line list 파싱.`
- `writeSources(content: string, sources: string[]): string — src/lib/sources-merge.ts:62-92 — frontmatter sources 필드 in-place rewrite, 없으면 append.`
- `mergeSourcesLists(existing, incoming): string[] — :103-116 — case-insensitive dedup, first-seen casing wins.`
- `mergeSourcesIntoContent(newContent, existingContent): string — :129-148 — 핵심 ingest write hook. 무엇도 추가하지 않으면 newContent reference 그대로 반환.`
- `collectAllFilesIncludingDot(folder: FileNode): FileNode[] — src/lib/sources-tree-delete.ts:23-36 — 트리 leaf 재귀 수집.`
- `decideDeleteClick(currentPending, clicked): DeleteClickAction — :66-76 — 두 단계 delete-confirm state machine ("arm" / "fire-file" / "fire-folder").`
- `findRawSourceForImage(imageUrl, projectPath): Promise<string | null> — src/lib/raw-source-resolver.ts:29-64 — `wiki/media/<slug>/img-N.<ext>` URL → `raw/sources/` 안의 stem 매칭 파일.`
- `imageUrlToAbsolute(imageUrl, projectPath): string — :76-87 — wiki-relative URL → absolute (idempotent).`
- `getFileCategory(filePath): FileCategory — src/lib/file-types.ts:124-127 — 확장자 → 10 가지 카테고리.`
- `isTextReadable(category): boolean — :129-131 — markdown/text/code/data 만 true.`
- `isBinary(category): boolean — :133-135 — image/video/audio/document/unknown 이 true.`
- `getCodeLanguage(filePath): string — :137-164 — 확장자 → highlight.js 언어 키.`
- `makeQuerySlug(title: string): string — src/lib/wiki-filename.ts:32-46 — Unicode-aware slug, NFKC 정규화, `\p{L}\p{N}` 만 보존, 50 자 cap, "query" fallback.`
- `makeQueryFileName(title, now?): { slug, fileName, date, time } — :50-66 — `<slug>-YYYY-MM-DD-HHMMSS.md`. UTC 기반.`
- `cascadeDeleteWikiPage(projectPath, pagePath): Promise<void> — src/lib/wiki-page-delete.ts:56-94 — file delete → embedding cascade → (source page 면) `wiki/media/<slug>/` cascade.`
- `extractFrontmatterTitle(content): string — src/lib/wiki-cleanup.ts:74-77 — `^title:` 라인 캡처.`
- `buildDeletedKeys(infos): Set<string> — :55-62 — slug+title → normalize key set.`
- `cleanIndexListing(text, deletedKeys): string — :92-102 — index 리스트의 `- [[...]]` bullet 정밀 제거.`
- `stripDeletedWikilinks(text, deletedKeys): string — :117-124 — 본문 wikilink 를 plain text 로 강등.`

## Internal Risk

### unsafe blocks (Rust)

None observed in this domain.

### `.unwrap()` / `.expect()` chains (Rust)

production 경로 (`vectorstore.rs` 의 `vector_*` 함수들) 에는 `.unwrap()` 이 0 건이에요 — 모든 LanceDB 호출이 `.map_err(|e| format!(...))` 로 `Result<_, String>` 으로 lift 돼요. `.unwrap()` 은 전부 `#[cfg(test)]` 모듈 안에 있어요.

```rust src-tauri/src/commands/vectorstore.rs:738-748
fn tmp_project() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("llm-wiki-vtest-{}-{}", ts, id));
    std::fs::create_dir_all(&p).unwrap();
    p
}
```

테스트 헬퍼라 panic = 테스트 실패 의도. `commands/project.rs` 는 `.unwrap()` 0 건.

### `panic!` / `unreachable!` / `todo!` (Rust)

None observed in this domain. `vectorstore.rs` 와 `project.rs` 모두 매크로 0 건이에요.

### `Mutex::lock` / `RwLock::write` acquisition + drop discipline (Rust)

Rust 쪽 data-layer 명령들 (`vectorstore.rs`, `project.rs`) 은 자체 `std::sync::Mutex` / `RwLock` 을 들지 않아요. LanceDB 의 내부 lock 은 라이브러리 책임이고, Tauri command 레벨에서 우리 코드가 잠그는 곳이 없어요. 테스트 모듈은 `AtomicU64` (`vectorstore.rs:739`) 만 쓰고, lock 은 안 잡아요. 

```rust src-tauri/src/commands/vectorstore.rs:739
static COUNTER: AtomicU64 = AtomicU64::new(0);
```

대신 동시성 핵심은 TS 쪽 promise-chain mutex (`withProjectLock`) — Internal Risk 의 TS 섹션 참고.

### FFI loads, `extern "C"`, dlopen-style (Rust → pdfium et al.)

None observed in this domain. `vectorstore.rs` / `project.rs` 는 LanceDB / arrow / chrono / std::fs 만 쓰고 외부 동적 라이브러리 로딩이 없어요. (FFI 면은 [04-backend-rust.md](04-backend-rust.md) 와 [08-pdf-ocr-pipeline.md](08-pdf-ocr-pipeline.md) 에서 다뤄요.)

### Result swallow (TypeScript)

#### empty catch — fallback / best-effort

```typescript src/lib/persist.ts:6-9
async function ensureDir(projectPath: string): Promise<void> {
  await createDirectory(`${projectPath}/.llm-wiki`).catch(() => {})
  await createDirectory(`${projectPath}/.llm-wiki/chats`).catch(() => {})
}
```
디렉터리 이미 존재 = 정상. 단, Permission denied 같은 다른 모든 디스크 오류도 같이 삼켜져요. 다음 `writeFile` 에서 다시 실패하므로 silent corruption 은 없지만 — 진짜 에러 원인이 한 번 가려져요.

```typescript src/lib/persist.ts:17-25
export async function loadReviewItems(projectPath: string): Promise<ReviewItem[]> {
  const pp = normalizePath(projectPath)
  try {
    const content = await readFile(`${pp}/.llm-wiki/review.json`)
    return JSON.parse(content) as ReviewItem[]
  } catch {
    return []
  }
}
```
file-missing + corrupt-JSON 이 같은 빈 배열로 묶여요. 통합 테스트 `persist.integration.test.ts:69-73` 가 corrupt JSON → `[]` 를 의도로 못 박아 둠. 사용자가 review 데이터를 잃었는지 vs 처음 여는지 구분이 디스크 inspect 없이는 안 돼요.

```typescript src/lib/persist.ts:64-110
export async function loadChatHistory(projectPath: string): Promise<PersistedChatData> {
  ...
  } catch {
    // Conversation file missing, skip
  }
  ...
  } catch {
    // Fall back to old format
    try {
      ...
    } catch {
      return { conversations: [], messages: [] }
    }
  }
}
```
3 단계 nested catch — new format → legacy combined → legacy flat → 빈 객체. 각 단계 에러는 모두 무음. `persist.integration.test.ts:163-176` 가 missing per-conversation file 을 의도적으로 무시하는 케이스 픽스.

```typescript src/lib/project-identity.ts:49-57
try {
    const raw = await readFile(path)
    const parsed = JSON.parse(raw) as ProjectIdentity
    if (parsed?.id && typeof parsed.id === "string") {
      return parsed.id
    }
  } catch {
    // missing or corrupt — fall through to create
  }
```
**위험 포인트**: corrupt `project.json` 을 fall-through 로 처리하면 새 UUID 가 생성되고 디스크에 덮어 쓰여요 (`src/lib/project-identity.ts:62-66`). 예전 ingest 가 이미 그 UUID 로 등록된 큐 작업을 들고 있다면 — old UUID → 어떤 path 로도 매칭 안 됨 = 작업 고아화. 사용자에게는 silent.

```typescript src/lib/project-identity.ts:76-84
export async function loadRegistry(): Promise<ProjectRegistry> {
  try {
    const store = await getStore()
    const registry = await store.get<ProjectRegistry>(REGISTRY_KEY)
    return registry ?? {}
  } catch {
    return {}
  }
}
```
plugin-store 자체가 corrupt 면 모든 프로젝트가 unregistered 로 보여요 — clip-server 가 path 로 reverse lookup (`getProjectIdByPath`) 하려 할 때 매번 null. 사용자는 "왜 webclip 이 갑자기 어디로도 안 가지?" 만 보임.

```typescript src/lib/raw-source-resolver.ts:41-46
let tree: FileNode[]
  try {
    tree = await listDirectory(`${projectPath}/raw/sources`)
  } catch {
    return null
  }
```
`raw/sources/` 가 없으면 (legacy 프로젝트 구조 / 사용자 수동 삭제) 이미지 → raw 매칭 silent fail. lightbox `null` 처리 있어 UX 는 graceful.

```typescript src/lib/wiki-page-delete.ts:88-93
try {
      await deleteFile(mediaDir)
    } catch {
      // Most common cause: the directory never existed because no
      // images were extracted from this source. Not an error.
    }
```
text-only source 의 미존재 media dir 가 정상 케이스라 의도된 swallow. 하지만 ENOENT 외의 진짜 권한 오류도 같이 묻혀요 — orphaned media dir 로 disk leak 발생 가능. `wiki-page-delete.test.ts:166-178` 가 ENOENT 케이스를 픽스.

#### `console.warn` w/o rethrow — observability-only

```typescript src/lib/project-identity.ts:62-66
try {
    await writeFile(path, JSON.stringify(identity, null, 2))
  } catch (err) {
    console.warn("[project-identity] failed to write identity file:", err)
  }
```
identity write 실패 시 caller 한테는 새로 만든 UUID 가 그대로 반환돼요 (`return identity.id` 가 catch 밖). **즉 메모리상의 ID 는 있지만 디스크엔 없는 상태**. 다음 부팅에 또 새 UUID 가 만들어지고 — 같은 프로젝트인데 ID 가 매번 바뀌는 zombie 상태 가능. 디스크가 read-only 인 흔치 않은 환경에서 등장 가능.

```typescript src/lib/reset-project-state.ts:48-69
if (queueMod.status === "fulfilled") {
    try {
      await queueMod.value.pauseQueue()
    } catch (err) {
      console.warn("[Reset Project State] pauseQueue failed:", err)
    }
  } else {
    console.warn("[Reset Project State] Failed to load ingest-queue:", queueMod.reason)
  }
  ...
  if (graphMod.status === "fulfilled") {
    try {
      graphMod.value.clearGraphCache()
    } catch (err) {
      console.warn("[Reset Project State] clearGraphCache failed:", err)
    }
  } else {
    console.warn("[Reset Project State] Failed to load graph-relevance:", graphMod.reason)
  }
```
프로젝트 전환 시 reset 이 실패하면 — 이전 프로젝트의 ingest queue / graph cache 가 새 프로젝트로 새어요. 테스트 `reset-project-state.test.ts:139-160` 가 일부러 sibling 실패에도 다른 reset 을 진행시키는 정책을 픽스. 의도적이지만 cross-project contamination 표면이 살아 있어요.

```typescript src/lib/auto-save.ts:14-19
reviewTimer = setTimeout(() => {
      const project = useWikiStore.getState().project
      if (project) {
        saveReviewItems(project.path, state.items).catch(() => {})
      }
    }, 1000)
```
**가장 광범위한 swallow**: review 자동 저장이 디스크 write 에 실패해도 사용자 / 콘솔에 어떤 신호도 안 떠요. 같은 패턴이 `auto-save.ts:25-30` 의 chat 저장에도. 디스크 가득 / 권한 실패 시 사용자는 UI 에서는 작업이 저장되는 것처럼 보이지만 실제론 메모리에만 있고, 다음 부팅에 모든 게 사라져요. 테스트 커버 0 건.

```typescript src/lib/persist.ts:8
await createDirectory(`${projectPath}/.llm-wiki`).catch(() => {})
```
`ensureDir` 의 두 호출 모두 — 디렉터리 생성 실패가 silent 라 다음 `writeFile` 의 더 모호한 에러로 surface 돼요.

#### project-mutex 의 race surface

```typescript src/lib/project-mutex.ts:36-47
const prev = locks.get(projectPath) ?? Promise.resolve()
  // We have to install our own promise into the map BEFORE awaiting
  // `prev`, otherwise a third caller can race in and find the map
  // still pointing at `prev`, and chain off the wrong slot.
  let release!: () => void
  const next = new Promise<void>((resolve) => {
    release = resolve
  })
  locks.set(
    projectPath,
    prev.then(() => next),
  )
```
주석이 명시한 race 가 진짜라 — 만약 install (set) 이 await 다음으로 가면, A 가 await 중인 동안 B 가 `locks.get(...) ?? Promise.resolve()` 로 `Promise.resolve()` 를 받고 즉시 lock 없이 진행해요. test `project-mutex.test.ts:88-129` 가 3 번째 caller 가 chain 끝에 정확히 줄 서는지 검증. **하지만**:

```typescript src/lib/project-mutex.ts:53-72
} finally {
    release()
    // Best-effort cleanup: if our promise is still the tail, drop the
    // map entry. Otherwise a later caller has chained on; leave it.
    if (locks.get(projectPath) === next || locks.size > 1024) {
      const tail = locks.get(projectPath)
      if (tail) {
        Promise.resolve().then(() => {
          if (locks.get(projectPath) === tail) {
            locks.delete(projectPath)
          }
        })
      }
    }
  }
```
주석 자체가 "Tail check is approximate (the map stores prev.then(() => next), not next directly)" 라고 인정. 즉 첫 번째 가드 `locks.get(projectPath) === next` 는 **항상 false** 예요 — 맵에는 `prev.then(() => next)` 가 들어 있고 `next` 직접이 아니에요. 그래서 사실상 매번 size > 1024 만이 트리거하는 보호 장치이고, 일반 케이스에서 entry 가 정리되는 시점은 다음 caller 가 와서 `prev` 를 GC 시킬 때예요. 1024 distinct projectPath 를 cycle 하면 메모리 누수. 테스트 `project-mutex.test.ts` 어떤 케이스도 이 cleanup branch 를 직접 안 covers — 라이브 race 의 sharp edge.

또한 `prev` 가 reject 해도 `await prev.catch(() => {})` 로 삼키는 게 의도예요 (`project-mutex.test.ts:69-86` 가 픽스) — 즉 한 caller 의 fail 이 lock chain 을 poison 하지는 않지만, **에러가 어디서도 surface 안 돼요**. 호출자 (예: ingest) 가 본인의 promise reject 만 보고, 다른 race 한 caller 의 fail 은 모름.

#### type drift (TS↔Rust IPC)

위 매핑 표의 `WikiProject` 가 TS 쪽에 `id: string` 을 추가로 가지지만 Rust 쪽엔 없어요 (`src/types/wiki.ts:1-7` vs `src-tauri/src/types/wiki.rs:3-7`). TS 코드는 `as WikiProject` 캐스팅을 어디에도 안 쓰고 — instead `ensureProjectId` (`src/lib/project-identity.ts:47`) 를 통해 별도 path 로 채워요 — 그래서 정적 검출은 안 되지만, Rust 가 `WikiProject` 를 직접 직렬화해서 돌려주는 `create_project` / `open_project` 는 `id` 가 없는 객체를 보내고, TS 쪽 `WikiProject` 인터페이스는 `id: string` 을 필수로 요구해요. 호출자 (예: `App.tsx`, `wiki-store`) 가 직후에 `ensureProjectId` 를 호출하지 않으면 — TS 타입 상으로는 `string` 이지만 runtime 에는 `undefined` 인 시한폭탄. 정적 타입 검사가 못 잡는 silent drift.

`vectorstore.rs` 쪽은 `Vec<f32>` ↔ `number[]` 를 그대로 직렬화해요 — JS `number` 가 64-bit double 이라 32-bit float 으로 다운캐스트할 때 precision loss 가 있을 수 있지만 임베딩 비교가 KNN 거리 기반이라 실용적 영향은 0 에 가까움.

#### persist 의 write atomicity

`persist.ts` 의 `saveReviewItems` 는 `writeFile(...)` 한 번에 전체 JSON 을 덮어 써요 (`src/lib/persist.ts:14`). `saveChatHistory` 는 `conversations.json` + N 개의 `chats/<id>.json` 을 순차로 덮어 써요 (`:42-61`). 둘 다 **atomic 하지 않아요** — 도중 crash / power-cut 시 partial write 가능. fs.rs (참고: [04-backend-rust.md](04-backend-rust.md)) 의 `write_file` 이 tempfile + rename 을 쓰는지 여부에 따라 결과가 갈려요. 만약 직접 `write_all` 이라면 truncate 후 부분 write 시 손상.

복합 위험: `saveChatHistory` 는 conversation 5 개를 쓰던 중 3 번째에서 crash 하면 — `conversations.json` 은 이미 5 개를 알리고 `chats/c4.json`, `chats/c5.json` 은 없음 = 다음 load 가 conversation 메타는 보지만 메시지가 없는 phantom 상태. test `persist.integration.test.ts:163-176` 의 "missing per-conversation file" 케이스가 우회로 (skip) 를 확인해주지만, 정확성 손실은 silent.

## Cross-refs

이 슬라이스가 의존하거나 의존시키는 다른 도메인 문서:

- [04-backend-rust.md](04-backend-rust.md) — `vectorstore.rs` / `project.rs` 의 `run_guarded_async` panic-guard 적용, fs.rs 의 `read_file` / `write_file` / `create_directory` / `delete_file` / `list_directory` 가 이 도메인의 모든 disk I/O 의 실제 실행자에요.
- [07-llm-integration.md](07-llm-integration.md) — `ingest-queue` 가 `withProjectLock` 의 핵심 caller 이고, `embedding.ts` 가 `vector_upsert_chunks` / `vector_search_chunks` / `vector_delete_page` 를 호출, `sweep-reviews` / `wiki-cleanup` 의 cleanup 흐름이 `cascadeDeleteWikiPage` 와 `cleanIndexListing` 을 사용해요.
- [08-pdf-ocr-pipeline.md](08-pdf-ocr-pipeline.md) — `findRawSourceForImage` 의 `wiki/media/<slug>/img-N.<ext>` URL 모양이 PDF 추출기 (`extract_pdf_markdown`) 가 emit 한 경로 규약과 묶여요.
- [03-frontend.md](03-frontend.md) — `useWikiStore` / `useChatStore` / `useReviewStore` 가 `auto-save` / `persist` / `resetProjectState` 의 직접 소비자. App.tsx 의 startup auto-open 흐름이 `getLastProject` → `openProject` → `saveLastProject` 사이클을 밟아요.
- [09-ui-components.md](09-ui-components.md) — `sources-view` 컴포넌트가 `decideDeleteClick` / `collectAllFilesIncludingDot` 의 단독 caller, lint-view 가 `cascadeDeleteWikiPage` 를 호출.

50-source-mapping.md 행 링크:

- [src/lib/project-mutex.ts](50-source-mapping.md#srclibproject-mutexts)
- [src/lib/project-identity.ts](50-source-mapping.md#srclibproject-identityts)
- [src/lib/project-store.ts](50-source-mapping.md#srclibproject-storets)
- [src/lib/persist.ts](50-source-mapping.md#srclibpersistts)
- [src/lib/auto-save.ts](50-source-mapping.md#srclibauto-savets)
- [src/lib/reset-project-state.ts](50-source-mapping.md#srclibreset-project-statets)
- [src/lib/sources-merge.ts](50-source-mapping.md#srclibsources-mergets)
- [src/lib/sources-tree-delete.ts](50-source-mapping.md#srclibsources-tree-deletets)
- [src/lib/raw-source-resolver.ts](50-source-mapping.md#srclibraw-source-resolverts)
- [src/lib/file-types.ts](50-source-mapping.md#srclibfile-typests)
- [src/lib/wiki-filename.ts](50-source-mapping.md#srclibwiki-filenamets)
- [src/lib/wiki-page-delete.ts](50-source-mapping.md#srclibwiki-page-deletets)
- [src/lib/wiki-cleanup.ts](50-source-mapping.md#srclibwiki-cleanupts)
- [src-tauri/src/commands/vectorstore.rs](50-source-mapping.md#src-taurisrccommandsvectorstorers)
- [src-tauri/src/commands/project.rs](50-source-mapping.md#src-taurisrccommandsprojectrs)
- [src/types/wiki.ts](50-source-mapping.md#srctypeswikits)
- [src-tauri/src/types/wiki.rs](50-source-mapping.md#src-taurisrctypeswikirs)

## Evidence

### TS↔Rust 타입 매핑

- `src/types/wiki.ts:1-7` — `WikiProject { id, name, path }`
- `src/types/wiki.ts:9-14` — `FileNode { name, path, is_dir, children? }`
- `src/types/wiki.ts:16-20` — `WikiPage { path, content, frontmatter }`
- `src-tauri/src/types/wiki.rs:3-7` — `WikiProject { name, path }` (no id field — drift)
- `src-tauri/src/types/wiki.rs:9-16` — `FileNode { name, path, is_dir, children: Option<Vec<FileNode>> }`
- `src-tauri/src/types/mod.rs:1` — `pub mod wiki;` (only wiki sub-module exposed)

### Plugin-store 키 + project-store

- `src/lib/project-store.ts:5-7` — `STORE_NAME = "app-state.json"`, `RECENT_PROJECTS_KEY`, `LAST_PROJECT_KEY`
- `src/lib/project-store.ts:9-11` — `load(STORE_NAME, { autoSave: true, defaults: {} })`
- `src/lib/project-store.ts:31-39` — `addToRecentProjects` length-cap 10
- `src/lib/project-store.ts:111-128` — `removeFromRecentProjects` 가 `lastProject` 도 같이 클리어
- `src/lib/project-store.ts:163-167` — `PersistedUpdateCheckState` 인터페이스 형태

### Project identity + registry

- `src/lib/project-identity.ts:20-21` — `STORE_NAME` / `REGISTRY_KEY` 동일 store 공유
- `src/lib/project-identity.ts:39-41` — `identityPath = "${projectPath}/.llm-wiki/project.json"`
- `src/lib/project-identity.ts:47-68` — `ensureProjectId` 의 try/catch + fallthrough 생성
- `src/lib/project-identity.ts:62-66` — `console.warn` only, no rethrow
- `src/lib/project-identity.ts:114-117` — `getProjectPathById` UUID → path
- `src/lib/project-identity.ts:124-131` — `getProjectIdByPath` 역방향 룩업 (clip-server 용)

### Persist + auto-save

- `src/lib/persist.ts:6-9` — `ensureDir` 의 silent `.catch(() => {})`
- `src/lib/persist.ts:14` — `writeFile(reviewJson)` 단일 호출 (atomic 보장은 fs.rs 의 책임)
- `src/lib/persist.ts:42-61` — chat 다중-파일 write 직렬 sequence
- `src/lib/persist.ts:54-56` — "Keep last 100 messages per conversation" cap
- `src/lib/persist.ts:64-110` — 3 단계 nested catch fallback (new format → legacy combined → legacy flat → 빈 객체)
- `src/lib/persist.integration.test.ts:69-73` — corrupt JSON → `[]` 픽스
- `src/lib/persist.integration.test.ts:145-156` — 100 메시지 cap 픽스
- `src/lib/persist.integration.test.ts:163-176` — missing per-conversation file 무시 픽스
- `src/lib/auto-save.ts:9-19` — review 1s debounce
- `src/lib/auto-save.ts:21-31` — chat 2s debounce + `state.isStreaming` skip
- `src/lib/auto-save.ts:16, 28` — 양쪽 모두 `.catch(() => {})` swallow

### Project mutex

- `src/lib/project-mutex.ts:25` — `const locks = new Map<string, Promise<unknown>>()` 모듈 레벨
- `src/lib/project-mutex.ts:36-47` — install-before-await race 회피 주석
- `src/lib/project-mutex.ts:51` — `await prev.catch(() => {})` — 이전 holder 의 reject swallow
- `src/lib/project-mutex.ts:53-72` — finally cleanup의 1024 size guard + approximate tail check
- `src/lib/project-mutex.ts:78-80` — `__resetProjectLocksForTesting`
- `src/lib/project-mutex.test.ts:69-86` — exception propagation + lock release 픽스
- `src/lib/project-mutex.test.ts:88-129` — 3 번째 caller chained 직렬화 픽스
- `src/lib/project-mutex.test.ts:132-165` — cross-project parallelism 픽스

### Reset project state

- `src/lib/reset-project-state.ts:18-39` — 4 zustand store 동기 클리어
- `src/lib/reset-project-state.ts:43-46` — `Promise.allSettled` 모듈 동적 import
- `src/lib/reset-project-state.ts:48-60` — pauseQueue 호출 + `console.warn` swallow
- `src/lib/reset-project-state.ts:62-70` — clearGraphCache 호출 + `console.warn` swallow
- `src/lib/reset-project-state.test.ts:121-137` — pauseQueue + clearGraphCache 둘 다 await 픽스
- `src/lib/reset-project-state.test.ts:139-160` — sibling 실패 후에도 진행 픽스

### Sources merge / sources tree delete

- `src/lib/sources-merge.ts:27-51` — `parseSources` 양쪽 YAML 형태 지원
- `src/lib/sources-merge.ts:62-92` — `writeSources` in-place rewrite, 없으면 append
- `src/lib/sources-merge.ts:103-116` — `mergeSourcesLists` case-insensitive dedup
- `src/lib/sources-merge.ts:129-148` — `mergeSourcesIntoContent` no-op fast path
- `src/lib/sources-merge.test.ts:202-212` — fast-path reference equality 픽스
- `src/lib/sources-merge.test.ts:262-310` — 3-way re-ingest union 시나리오
- `src/lib/sources-tree-delete.ts:23-36` — `collectAllFilesIncludingDot` 재귀
- `src/lib/sources-tree-delete.ts:50-76` — 두 단계 delete-confirm state machine
- `src/lib/sources-tree-delete.test.ts:91-153` — `decideDeleteClick` 모든 분기 픽스

### Raw source resolver / file types / wiki filename / wiki page delete / wiki cleanup

- `src/lib/raw-source-resolver.ts:35-37` — `media/<slug>/` 정규식
- `src/lib/raw-source-resolver.ts:41-46` — `listDirectory` 실패 silent
- `src/lib/raw-source-resolver.ts:48-61` — stem 매칭 재귀
- `src/lib/raw-source-resolver.ts:76-87` — `imageUrlToAbsolute` Windows + Unix 절대-path 감지
- `src/lib/file-types.ts:13-122` — 확장자 → category 매핑 (10 categories)
- `src/lib/file-types.ts:124-127` — `getFileCategory` lookup
- `src/lib/wiki-filename.ts:32-46` — `makeQuerySlug` Unicode-aware NFKC
- `src/lib/wiki-filename.ts:50-66` — `makeQueryFileName` UTC 기반 timestamp
- `src/lib/wiki-filename.test.ts:19-23` — CJK 정상 보존 픽스
- `src/lib/wiki-filename.test.ts:83-95` — 같은 날 같은 title 다른 시각 → 다른 파일명
- `src/lib/wiki-page-delete.ts:35-38` — `isSourcePage` `/wiki/sources/` 매칭
- `src/lib/wiki-page-delete.ts:60-65` — `deleteFile → removePageEmbedding` 순서 강제
- `src/lib/wiki-page-delete.ts:80-93` — `wiki/media/<slug>/` 카스케이드 + dotfile 가드
- `src/lib/wiki-page-delete.test.ts:48-65` — order 픽스
- `src/lib/wiki-page-delete.test.ts:67-76` — file delete 실패 시 embedding cascade 안 돌아감
- `src/lib/wiki-page-delete.test.ts:136-143` — source page 면 media dir 까지 삭제
- `src/lib/wiki-page-delete.test.ts:195-211` — dotfile slug rejection (defensive)
- `src/lib/wiki-cleanup.ts:46-48` — `normalizeKey` 의 case + 공백/하이픈/언더스코어 정규화
- `src/lib/wiki-cleanup.ts:55-62` — `buildDeletedKeys` slug + title 양쪽 add
- `src/lib/wiki-cleanup.ts:74-77` — `extractFrontmatterTitle` regex
- `src/lib/wiki-cleanup.ts:81` — `INDEX_ENTRY_RE` bullet anchor
- `src/lib/wiki-cleanup.ts:104` — `WIKILINK_RE` global
- `src/lib/wiki-cleanup.test.ts:120-141` — Bug B (substring false-positive) 픽스
- `src/lib/wiki-cleanup.test.ts:382-408` — end-to-end 사용자 보고 regression 시나리오

### Vectorstore (LanceDB)

- `src-tauri/src/commands/vectorstore.rs:45-47` — `db_path = "${projectPath}/.llm-wiki/lancedb"`
- `src-tauri/src/commands/vectorstore.rs:50-53` — `TABLE_V1 = "wiki_vectors"`, `TABLE_V2 = "wiki_chunks_v2"`
- `src-tauri/src/commands/vectorstore.rs:56-65` — `validate_page_id` filter-injection 가드
- `src-tauri/src/commands/vectorstore.rs:67-94` — v1 schema + batch builder
- `src-tauri/src/commands/vectorstore.rs:97-146` — `vector_upsert` (v1)
- `src-tauri/src/commands/vectorstore.rs:128-130` — delete-before-add `eprintln!` warning, 명령 실패는 무시 (best-effort)
- `src-tauri/src/commands/vectorstore.rs:149-213` — `vector_search` v1 KNN
- `src-tauri/src/commands/vectorstore.rs:332-348` — v2 schema (`chunk_id, page_id, chunk_index, chunk_text, heading_path, vector`)
- `src-tauri/src/commands/vectorstore.rs:350-406` — `make_batch_v2` dim mismatch 검증
- `src-tauri/src/commands/vectorstore.rs:413-478` — `vector_upsert_chunks`
- `src-tauri/src/commands/vectorstore.rs:422-424` — empty chunks → no-op (transient ingest 보호)
- `src-tauri/src/commands/vectorstore.rs:453-461` — delete-before-add `eprintln!` warning v2
- `src-tauri/src/commands/vectorstore.rs:483-568` — `vector_search_chunks` v2 KNN
- `src-tauri/src/commands/vectorstore.rs:560` — `score = 1.0 / (1.0 + distance)` (v1 / v2 동일 변환)
- `src-tauri/src/commands/vectorstore.rs:573-609` — `vector_delete_page` v2
- `src-tauri/src/commands/vectorstore.rs:652-683` — `vector_legacy_row_count` v1 row 개수
- `src-tauri/src/commands/vectorstore.rs:689-715` — `vector_drop_legacy` v1 drop
- `src-tauri/src/commands/vectorstore.rs:706-710` — drop_table comment: LanceDB 0.27 시그니처 (name, namespace) — 빈 namespace 사용
- `src-tauri/src/commands/vectorstore.rs:743-748` — 테스트 헬퍼 `tmp_project()` 의 `.unwrap()` (test-only)
- `src-tauri/src/commands/vectorstore.rs:914-936` — dim mismatch reject 픽스
- `src-tauri/src/commands/vectorstore.rs:940-952` — `'; DROP` 같은 page_id reject 픽스 (filter injection 회귀 가드)

### Project commands

- `src-tauri/src/commands/project.rs:9-11` — `create_project` Tauri command + `run_guarded`
- `src-tauri/src/commands/project.rs:14-31` — 9 개 기본 sub-dir 생성 (`raw/sources`, `raw/assets`, `wiki/{entities,concepts,sources,queries,comparisons,synthesis}`)
- `src-tauri/src/commands/project.rs:117` — `schema.md` 작성
- `src-tauri/src/commands/project.rs:150` — `purpose.md` 작성
- `src-tauri/src/commands/project.rs:167` — `wiki/index.md` 보일러플레이트
- `src-tauri/src/commands/project.rs:178` — `wiki/log.md` 작성
- `src-tauri/src/commands/project.rs:192` — `wiki/overview.md` 작성
- `src-tauri/src/commands/project.rs:194-230` — `.obsidian/` 호환 설정 작성
- `src-tauri/src/commands/project.rs:232-236` — 반환 시 `path` 의 backslash → forward-slash 정규화
- `src-tauri/src/commands/project.rs:239-278` — `open_project` validation (schema.md + wiki/ dir 존재 확인)
- `src-tauri/src/commands/project.rs:280-287` — `write_file_inner` 의 parent dir 자동 생성

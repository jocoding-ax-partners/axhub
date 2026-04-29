# 07 — LLM Integration (Providers, Prompts, Streaming, Ingest, Search, Graph, Research, Reviews)

## Purpose

이 도메인은 `llm_wiki` 가 LLM 과 외부 search/embedding API 를 호출해서 (1) source document 를 wiki page 로 변환 (ingest pipeline), (2) hybrid retrieval (RRF token + vector), (3) wiki graph 의 community detection + relevance, (4) gap-driven web research, (5) stale review 자동 정리를 수행하는 슬라이스를 묶어요. 슬라이스의 핵심은 6 개 provider (OpenAI / Anthropic / Google Gemini / Ollama / MiniMax / Custom / Claude Code CLI) 를 단일 `streamChat` 인터페이스 뒤로 통합하는 provider abstraction 이고, 그 위에 (a) `claude-cli-transport` 의 subprocess (Tauri 가 spawn 하는 Rust child process) 변환, (b) `endpoint-normalizer` 의 user-paste URL 정리, (c) `context-budget` 의 prompt 사이즈 분할, (d) `text-chunker` 의 markdown-aware 재귀 splitter, (e) `embedding.ts` 의 LanceDB v2 chunk-단위 인덱싱 + RRF 의 vector half, (f) `search.ts` 의 token+vector RRF 융합 retrieval, (g) `wiki-graph` + `graph-insights` + `graph-relevance` 의 graphology Louvain community 분석과 Adamic-Adar 점수, (h) `deep-research` + `optimize-research-topic` + `web-search` 의 Tavily 기반 gap-fill, (i) `sweep-reviews` + `review-utils` + `source-delete-decision` 의 ingest drain 후 stale review 정리, 가 자리잡아요. 각 호출은 `tauri-fetch` 의 Rust-backed HTTP plugin 을 거쳐 CORS-unfriendly endpoint 도 같이 다루고, ingest 는 `withProjectLock` (06 도메인) 으로 직렬화해서 동일 프로젝트의 동시 ingest 가 `wiki/index.md` 를 race 로 덮어쓰지 못하게 막아요. 이 슬라이스가 한 도메인으로 묶이는 이유는, 사실상 모든 LLM-call 모듈이 (provider 선택 → prompt 조립 → streaming token 처리 → JSON 추출 / 파싱) 의 같은 4 단계를 공유하고, 동일한 race 면 (mid-call project switch / mid-stream abort / claude CLI subprocess 종료) 을 가지기 때문이에요.

## Public Interface

### Provider abstraction — `streamChat` & 6 wires

- `streamChat — async (config: LlmConfig, messages: ChatMessage[], callbacks: StreamCallbacks, signal?: AbortSignal, requestOverrides?: RequestOverrides) => Promise<void> — src/lib/llm-client.ts:36-190 — 모든 provider 의 단일 진입. claude-code 는 subprocess transport 로 dispatch, 나머지는 HTTP streaming.`
- `getProviderConfig — (config: LlmConfig) => ProviderConfig — src/lib/llm-providers.ts:428-588 — provider 별 url/headers/buildBody/parseStream 4-tuple 반환. claude-code 진입 시 throw (subprocess transport 가 한 단계 위에서 dispatch).`
- `buildAnthropicUrl — (base: string) => string — src/lib/llm-providers.ts:334-339 — `.../v1/v1/messages` double-append 회귀 가드.`
- `buildOpenAiBody / buildAnthropicBody / buildGoogleBody — src/lib/llm-providers.ts:210-426 — 3 wire native schema 로 `ChatMessage[]` 변환.`
- `toOpenAiContent / toAnthropicContent / toGoogleParts — :194-208, :236-252, :362-373 — multimodal `ContentBlock[]` (text + image) → wire-native 매핑.`
- `parseOpenAiLine / parseAnthropicLine / parseGoogleLine — :115-176 — SSE chunk 파싱. Gemini 는 다중 part + thought-skip 지원.`
- `requiresBearerAuth — :308-319 — MiniMax /anthropic + Alibaba Bailian 처럼 Authorization Bearer 를 요구하는 endpoint 인식.`
- `buildAnthropicHeaders — :341-353 — Bearer vs `x-api-key` + `anthropic-dangerous-direct-browser-access` 헤더 분기.`
- `localLlmOriginHeader — :111-113 — Ollama / LM Studio CORS 우회 (`Origin: http://localhost`).`

#### `LlmConfig.provider` switch surface (`src/lib/llm-providers.ts:428-588`)

| provider | URL 빌딩 | auth header | parseStream | 비고 |
|---|---|---|---|---|
| `openai` | `https://api.openai.com/v1/chat/completions` | `Authorization: Bearer` | OpenAI SSE | 모든 OpenAI-clone (DeepSeek, Groq, Zhipu, Kimi, xAI) 도 buildOpenAiBody 사용 |
| `anthropic` | `buildAnthropicUrl("https://api.anthropic.com")` | `x-api-key` + `anthropic-version` | Anthropic SSE | `anthropic-dangerous-direct-browser-access: true` |
| `google` | `https://generativelanguage.googleapis.com/v1beta/models/${encodedModel}:streamGenerateContent?alt=sse` | `x-goog-api-key` | Google SSE | model segment encodeURIComponent |
| `ollama` | `${ollamaUrl}/v1/chat/completions` (defense: 기존 `/v1/chat/completions` 또는 `/v1` 제거) | none | OpenAI SSE | qwen3 모델은 `chat_template_kwargs.enable_thinking=false` 자동 주입 |
| `minimax` | `buildAnthropicUrl(customEndpoint || "https://api.minimax.io/anthropic")` | Bearer | Anthropic SSE | MiniMax /anthropic 는 Bearer-only CORS |
| `claude-code` | (none — subprocess transport) | (none) | (custom) | `getProviderConfig` 진입 시 즉시 throw |
| `custom` | `apiMode === "anthropic_messages"` 면 `buildAnthropicUrl(customEndpoint)`, 아니면 `${customEndpoint}/chat/completions` | optional Bearer | mode-dependent | apiMode 미설정 = `chat_completions` (legacy compat) |

### Claude Code CLI subprocess transport

- `streamClaudeCodeCli — async (config, messages, callbacks, signal?, overrides?) => Promise<void> — src/lib/claude-cli-transport.ts:117-217 — Tauri invoke 로 `claude_cli_spawn` 호출, `claude-cli:{streamId}` event listen + `claude-cli:{streamId}:done` 으로 종료 대기, abort 시 `claude_cli_kill` 발사.`
- `createClaudeCodeStreamParser — () => (line: string) => string | null — src/lib/claude-cli-transport.ts:30-99 — stream-json 의 `assistant` (full message, prefix-diff) + `stream_event content_block_delta` (incremental) 두 형태를 union 으로 처리. `sawDelta` 클로저로 double-emit 방지.`
- Rust side:
  - `claude_cli_detect — async () -> Result<DetectResult, String> — src-tauri/src/commands/claude_cli.rs:58-124 — `which::which("claude")` + `claude --version` (3s timeout). macOS Gatekeeper quarantine 감지 시 `xattr -d com.apple.quarantine` 가이드 메시지.`
  - `claude_cli_spawn — async (app, state, stream_id, model, messages) -> Result<(), String> — src-tauri/src/commands/claude_cli.rs:132-312 — `claude -p --output-format stream-json --input-format stream-json --verbose --model <model>` spawn, system prompt 를 first user turn 에 prepend, stdin 에 stream-json line 흘리고 close, stdout 을 background tokio task 로 line-drain → emit, stderr 별도 task 로 collect 해서 done event 에 첨부.`
  - `claude_cli_kill — async (state, stream_id) -> Result<(), String> — :317-329 — `state.children` 에서 `start_kill()`, kill_on_drop=true 가 SIGKILL 보냄.`
- Rust state: `ClaudeCliState { children: Arc<Mutex<HashMap<String, Child>>> }` (`:32-35`)

### Endpoint normalization

- `normalizeEndpoint — (raw: string, mode: "chat_completions" | "anthropic_messages") => NormalizedEndpoint — src/lib/endpoint-normalizer.ts:40-135 — 사용자 paste URL 의 trailing `/chat/completions`, `/embeddings`, `/messages` (mode-aware) 제거 + IPv4 octet validation + protocol/version-segment 검사.`
- `EndpointMode` / `NormalizedEndpoint` types — `:20-29`.

### Context budget

- `computeContextBudget — (maxContextSize: number | undefined) => ContextBudget — src/lib/context-budget.ts:67-99 — 5%/50%/15% (index/page/response) char 분할. PER_PAGE_FLOOR 5K, PER_PAGE_FRAC 0.3.`
- `ContextBudget` interface — `:34-52`.

### Prompt assembly + templates

- `buildAnalysisPrompt — (purpose, index, sourceContent?) => string — src/lib/ingest.ts:917-962 — Stage 1 ingest 프롬프트.`
- `buildGenerationPrompt — (schema, purpose, index, sourceFileName, overview?, sourceContent?) => string — src/lib/ingest.ts:967-1083 — Stage 2 ingest 프롬프트. language directive 를 prompt 의 마지막에 한 번 더 반복.`
- `languageRule — (sourceContent?: string) => string — src/lib/ingest.ts:245-247 — 출력 언어 directive (실제 작업은 `buildLanguageDirective` 가 wiki-store 의 outputLanguage + detectLanguage 결합).`
- `templates: WikiTemplate[]` — `src/lib/templates.ts:640-646` — research / reading / personal / business / general 5 종 wiki 보일러플레이트.
- `getTemplate — (id: string) => WikiTemplate — src/lib/templates.ts:648-654` — id 매칭 실패 시 throw.

### Ingest pipeline

- `autoIngest — async (projectPath, sourcePath, llmConfig, signal?, folderContext?) => Promise<string[]> — src/lib/ingest.ts:261-271 — `withProjectLock` 안쪽에서 `autoIngestImpl` 실행 (wiki/index.md race 방지).`
- `autoIngestImpl — :273-721 — 6 단계: cache check → image extract → image caption → analysis → generation → write → review parse → embed.`
- `parseFileBlocks — (text) => { blocks, warnings } — src/lib/ingest.ts:146-239 — line-based parser. CRLF normalize, fence-aware (CommonMark `\`\`\`+|~~~+` 매칭), opener/closer 정규식, path traversal 가드, truncation 감지 → warning.`
- `isSafeIngestPath — (p: string) => boolean — :101-115 — `wiki/` 시작, no `..` 세그먼트, no absolute / drive letter / UNC, no control bytes.`
- `writeFileBlocks — async (projectPath, text) => { writtenPaths, warnings, hardFailures } — :754-846 — language guard (concept-only), log append, index/overview overwrite, content page `mergeSourcesIntoContent` (06 도메인 호출).`
- `parseReviewBlocks — (text, sourcePath) => Omit<ReviewItem, ...>[] — :850-911 — `---REVIEW: type | Title---` regex + OPTIONS / PAGES / SEARCH 라인 파싱.`
- `startIngest — async (projectPath, sourcePath, llmConfig, signal?) => Promise<void> — :1212-1300 — chat-mode interactive ingest 진입.`
- `executeIngestWrites — async (projectPath, llmConfig, userGuidance?, signal?) => Promise<string[]> — :1302-1455 — chat-mode 의 후속 file-write 단계.`
- `injectImagesIntoSourceSummary — async (pp, fileName, savedImages) => Promise<void> — :1111-1174 — `<!-- llm-wiki:embedded-images -->` 마커로 idempotent injection.`
- `reembedSourceSummary — async (pp, fileName) => Promise<void> — :1190-1210 — caption 갱신 후 search index sync.`
- `contentMatchesTargetLanguage — (content, target) => boolean — :730-752 — frontmatter+code+math 제거 후 detectLanguage. CJK / Latin family compatibility table.`

### Ingest queue (background drain + abort)

- `enqueueIngest — async (projectId, sourcePath, folderContext?) => Promise<string> — src/lib/ingest-queue.ts:114-142 — currentProjectId 검증 후 task 추가 + saveQueue + processNext.`
- `enqueueBatch — async (projectId, files[]) => Promise<string[]> — :148-179 — batch enqueue.`
- `retryTask — async (taskId) => Promise<void> — :185-194` / `cancelTask — async (taskId) => Promise<void> — :200-227` / `cancelAllTasks — async () => Promise<number> — :245-264`
- `pauseQueue — async () => Promise<void> — :317-351 — 프로젝트 전환 시 in-flight processing 을 pending 으로 되돌리고 disk flush 후 in-memory clear. AWAITED 필수.`
- `restoreQueue — async (projectId, projectPath) => Promise<void> — :361-407 — 새 프로젝트 진입 시 disk → memory.`
- `clearQueueState — () => void — :291-306 — 테스트용 동기 clear (production 은 pauseQueue 써야 함).`
- `getQueue — () => readonly IngestTask[] — :269-271` / `getQueueSummary — :276-283`
- `cleanupWrittenFiles — async (projectPath, filePaths[]) => Promise<void> — :92-107 — 취소된 ingest 가 쓴 파일을 cascadeDeleteWikiPage (06 도메인) 로 정리.`
- 내부 핵심: `processNext` (`:435-539`) — drain loop, MAX_RETRIES=3, project-switch stale-context guard 매 await 후 (registry await, `saveQueue` await, `autoIngest` await) 검사.
- `onQueueDrained` (`:413-433`) — drain 후 `sweepResolvedReviews` 호출.

### Ingest cache (SHA-256)

- `checkIngestCache — async (projectPath, sourceFileName, sourceContent) => Promise<string[] | null> — src/lib/ingest-cache.ts:61-93 — content hash 일치 + 모든 cached file 이 디스크에 살아있을 때만 hit.`
- `saveIngestCache — async (projectPath, sourceFileName, sourceContent, filesWritten) => Promise<void> — :98-113`
- `removeFromIngestCache — async (projectPath, sourceFileName) => Promise<void> — :118-126`

### Text chunking (embedding-aware)

- `chunkMarkdown — (content, options?) => Chunk[] — src/lib/text-chunker.ts:92-122 — heading-aware 재귀 splitter. fenced code + leading-pipe table 는 indivisible. YAML frontmatter strip.`
- `stripFrontmatter — (content) => { body, bodyOffset } — :131-144`
- `Chunk` interface — `:67-84` (`index, text, headingPath, charStart, charEnd, oversized`).
- `ChunkingOptions` — `:46-57` (default: target 1000 / max 1500 / min 200 / overlap 200).

### Wikilink enrichment

- `enrichWithWikilinks — async (projectPath, filePath, llmConfig) => Promise<void> — src/lib/enrich-wikilinks.ts:25-101 — LLM 에게 page 전체 rewrite 가 아니라 `{term, target}` JSON list 만 받아서 first-occurrence replace. 사용자 content corruption 방지.`
- `parseLinkResponse — (raw: string) => LinkEntry[] — :108-157 — fence/prose tolerant balanced-brace JSON extractor.`
- `applyLinks — (content, links[]) => string — :166-192` / `findUnlinkedOccurrence — :195-210`

### Embedding pipeline

- `fetchEmbedding — async (text, cfg, maxRetries=3) => Promise<number[] | null> — src/lib/embedding.ts:80-161 — POST + auto-halve retry on oversize errors (regex on body text).`
- `looksLikeOversizeError — (status, body) => boolean — :55-68 — HTTP 413 + 8 가지 phrasing fuzzy match.`
- `embedPage — async (projectPath, pageId, title, content, cfg) => Promise<void> — :268-313 — chunk → enrich (title + heading + chunk) → fetchEmbedding → vector_upsert_chunks. 빈 rows 면 no-op.`
- `embedAllPages — async (projectPath, cfg, onProgress?) => Promise<number> — :321-367 — wiki/ 전체 batch 인덱스. structural pages (index/log/overview/purpose/schema) skip.`
- `searchByEmbedding — async (projectPath, query, cfg, topK=10) => Promise<PageSearchResult[]> — :390-447 — top-K × 3 chunks → group by page_id → max-pool primary + 0.3 × tail (capped 1 - max).`
- `removePageEmbedding — async (projectPath, pageId) => Promise<void> — :453-462` / `getEmbeddingCount — :468-474` / `legacyVectorRowCount — :223-231` / `dropLegacyVectorTable — :233-237`
- `getLastEmbeddingError — () => string | null — :41-43 — Settings UI 가 BM25 fallback 이유 표시.`

### Search (RRF hybrid retrieval)

- `searchWiki — async (projectPath, query) => Promise<SearchResult[]> — src/lib/search.ts:218-368 — token search + vector search 의 RRF fusion (K=60). 두 list rank 의 1/(K+rank) 합.`
- `tokenizeQuery — (query: string) => string[] — :88-125 — CJK bigram + char + 원본 token 모두 emit. STOP_WORDS 23 종 (영문 + 중문).`
- 내부: `scoreFile` (`:436-494`) — FILENAME_EXACT_BONUS=200, PHRASE_IN_TITLE_BONUS=50, PHRASE_IN_CONTENT_PER_OCC=20 (cap 10), TITLE_TOKEN_WEIGHT=5, CONTENT_TOKEN_WEIGHT=1.
- 내부: `searchFiles` (`:390-429`) — 16-concurrent batch readFile. queryPhrase trim-punctuation.
- `RRF_K = 60` (`:53`) — Cormack et al. SIGIR 2009 canonical.
- `SearchResult` (`:20-33`) / `ImageRef` (`:15-18`).

#### RRF (Reciprocal Rank Fusion) 공식

```
fused(p) = sum over lists L of  1 / (K + rank_L(p))
```

K=60 (canonical Cormack et al. 2009 상수), `rank_L(p)` 는 list L 안에서 page p 의 1-indexed 순위. token list 와 vector list 모두에 등장한 page 는 양쪽에서 점수를 받고, 한쪽에만 등장한 page 는 그 list 의 1/(K+rank) 만 받아요. **rank-기반 fusion** 이 핵심: 절대 점수 (token: 1-400, vector cosine: 0-1) 가 incommensurable 이라 그냥 더하면 큰 숫자 쪽이 dominate. RRF 는 순위 정보만 사용해서 두 retrieval 의 신호를 공정하게 결합. tie 는 알파벳 path 순서로 deterministic break (`src/lib/search.ts:357-360`).

### Wiki graph (community + relevance)

- `buildWikiGraph — async (projectPath) => Promise<{nodes, edges, communities}> — src/lib/wiki-graph.ts:159-286 — wiki/ 의 모든 .md 를 노드로 + `[[wikilink]]` 를 edge 로. graphology Louvain community detection (resolution=1). retrievalGraph 로 edge weight 계산.`
- `detectCommunities — :31-113 — Louvain 후 nodeCount 기준 sort + 재번호.`
- `resolveTarget — :288-304 — wikilink 정규화 (case-insensitive, space→hyphen).`
- 내부 type filter: `HIDDEN_TYPES = {"query"}` (`:204`) — research result 는 graph 에서 제외.
- `findSurprisingConnections — (nodes, edges, communities, limit=5) => SurprisingConnection[] — src/lib/graph-insights.ts:31-102 — cross-community / cross-type / peripheral-to-hub / weak-edge 4 가지 signal 가산점.`
- `detectKnowledgeGaps — (nodes, edges, communities, limit=8) => KnowledgeGap[] — :114-193 — isolated-node / sparse-community (cohesion < 0.15) / bridge-node (≥3 communities).`
- `buildRetrievalGraph — async (projectPath, dataVersion=0) => Promise<RetrievalGraph> — src/lib/graph-relevance.ts:155-245 — module-level cache (`cachedGraph`) keyed by `dataVersion`. immutable nodes (Object.freeze).`
- `calculateRelevance — (nodeA, nodeB, graph) => number — :247-287 — 4 signal: directLink × 3.0, sourceOverlap × 4.0, commonNeighbor (Adamic-Adar) × 1.5, typeAffinity × 1.0.`
- `getRelatedNodes — (nodeId, graph, limit=5) => readonly Array<{node, relevance}> — :289-308`
- `clearGraphCache — () => void — :310-312 — reset-project-state (06 도메인) 가 호출.`

### Deep research (gap-fill)

- `queueResearch — (projectPath, topic, llmConfig, searchConfig, searchQueries?) => string — src/lib/deep-research.ts:13-33 — research-store 에 task 추가 후 `processQueue` setTimeout 50ms.`
- `executeResearch — async (...) => Promise<void> — :54-232 — webSearch (multi-query union dedup) → streamChat synthesis (incremental store update) → write `wiki/queries/research-<slug>-<date>.md` → autoIngest.`
- `optimizeResearchTopic — async (llmConfig, gapTitle, gapDescription, gapType, overview, purpose) => Promise<OptimizedTopic> — src/lib/optimize-research-topic.ts:14-75 — gap → context-aware research topic + 3 search queries. "TOPIC: ..." / "QUERY: ..." 4-line strict format.`
- `webSearch — async (query, config, maxResults=10) => Promise<WebSearchResult[]> — src/lib/web-search.ts:11-26 — provider switch (`tavily` only).`
- `tavilySearch — async (query, apiKey, maxResults) => Promise<WebSearchResult[]> — :28-72 — Tavily search_depth=advanced, source = url hostname.`

### Sweep reviews (post-drain auto-resolve)

- `sweepResolvedReviews — async (projectPath, signal?) => Promise<number> — src/lib/sweep-reviews.ts:341-461 — Stage 1 rule-based (filename / title / affectedPages) + Stage 2 LLM batch judgment.`
- `extractJsonObject — (raw: string) => string — :135-177 — fence/prose tolerant balanced-brace JSON extractor (enrich-wikilinks 와 비슷한 형태이지만 별도 구현).`
- 내부: `judgeBatch` (`:190-282`) — JUDGE_BATCH_SIZE=40, abort 시 즉시 빈 set, JSON parse 실패 시 console.warn + 빈 set. `MAX_JUDGE_BATCHES=5` (`:180`), `MAX_PAGES_IN_PROMPT=300` (`:181`).
- 내부: `matchesCurrentProject` (`:326-330`) — 매 await 후 stale-context guard.
- `normalizeReviewTitle — (title: string) => string — src/lib/review-utils.ts:21-27 — "Missing page:" / "缺失页面:" 등 prefix strip + lowercase + collapse whitespace.`
- `decidePageFate — (frontmatterSources, deletingSource) => DeleteDecision — src/lib/source-delete-decision.ts:33-58 — case-insensitive. skip / keep / delete 3 분기. findRelatedWikiPages 의 loose match 로부터 보호.`

### Misc

- `getHttpFetch — () => Promise<typeof globalThis.fetch> — src/lib/tauri-fetch.ts:41-53 — Tauri plugin-http (browser) vs node fetch (test) lazy import. cached promise.`
- `isFetchNetworkError — (err: unknown) => boolean — :68-79 — WebKit "Load failed" + Chromium TypeError + "Failed to fetch" + "network error" 4 종 cross-webview 통합.`
- `startClipWatcher — () => void — src/lib/clip-watcher.ts:12-59 — 3s polling `http://127.0.0.1:19827/clips/pending` (clip-server). 새 clip 이 현재 프로젝트면 file tree refresh + `enqueueIngest`.`
- `stopClipWatcher — () => void — :61-66`
- `checkForUpdates — async (opts) => Promise<UpdateStatus> — src/lib/update-check.ts:144-167 — GitHub Releases API.`
- `fetchLatestRelease — async (repo) => Promise<GithubRelease | null> — :101-136`
- `isNewer — (remote: string, local: string) => boolean — :72-88 — strict `MAJOR.MINOR.PATCH` 비교.`
- `toLatestReleaseUrl — (htmlUrl: string) => string — :42-46 — `/releases/tag/<tag>` → `/releases/latest` 정규화.`
- `UPDATE_CHECK_CACHE_MS = 60 * 60 * 1000` (`:176`) — 1 hour cache, 60 req/h GitHub anonymous limit 미만.

## Internal Risk

### unsafe blocks (Rust)

None observed in this domain. `claude_cli.rs` 는 tokio + serde + which crate 만 사용하고 `unsafe` 블록 0건.

### `.unwrap()` / `.expect()` chains (Rust)

production 경로의 `.unwrap()` 은 1 건이에요 — `.unwrap_or_default()` 형태로 panic 회피.

```rust src-tauri/src/commands/claude_cli.rs:300
let stderr_text = stderr_task.await.unwrap_or_default();
```
`stderr_task` (`:262-270`) 가 panic / abort 되면 빈 string 으로 fallback. **위험은 isolated**: stderr collect task 가 죽었어도 done event 는 emit 됨.

추가로 spawn-time `.ok_or_else` (`:194-205`) 가 stdin/stdout/stderr handle 결손 시 안전한 Err return. `.expect()` 는 production 0 건. test 모듈도 없음 (`#[cfg(test)]` 블록 부재).

### `panic!` / `unreachable!` / `todo!` (Rust)

None observed in this domain. `claude_cli.rs` 매크로 0 건.

### `Mutex::lock` / `RwLock::write` acquisition + drop discipline (Rust)

```rust src-tauri/src/commands/claude_cli.rs:32-35
#[derive(Default)]
pub struct ClaudeCliState {
    children: Arc<Mutex<HashMap<String, Child>>>,
}
```
`tokio::sync::Mutex` (async). 잠금 지점 3 군데:

```rust src-tauri/src/commands/claude_cli.rs:238-242
state
    .children
    .lock()
    .await
    .insert(stream_id.clone(), child);
```
spawn 후 child 등록. 단일 op, lock 즉시 drop.

```rust src-tauri/src/commands/claude_cli.rs:289-298
let child_opt = children.lock().await.remove(&stream_id_task);
let exit_code = if let Some(mut child) = child_opt {
    match child.wait().await {
        Ok(status) => status.code(),
        Err(_) => None,
    }
} else {
    // Already removed by claude_cli_kill — leave code as None.
    None
};
```
**핵심 discipline**: 주석이 명시 — "Don't hold the map lock across .wait() — kill could race." `child` 를 lock 안에서 `remove` 한 다음 lock 을 drop 하고 그 다음에 `.wait()` 호출. lock-while-await 회피.

```rust src-tauri/src/commands/claude_cli.rs:321-327
if let Some(mut child) = state.children.lock().await.remove(&stream_id) {
    let _ = child.start_kill();
    // Don't wait() here — the stdout-drain task already holds a
    // wait future elsewhere when it can. Dropping the handle is
    // enough; kill_on_drop ensures the SIGKILL is sent.
}
```
Single lock, immediate drop after `remove`. **micro-race**: spawn 의 `state.children.lock().await.insert(...)` (`:238`) 와 kill 의 `lock().await.remove(...)` (`:322`) 사이 — 하지만 동일 stream_id 면 frontend 가 spawn 끝나기 전에 kill 보내지 않는 워크플로우라 실용적으로 안전. tokio Mutex 가 fair queue 라 queue order 도 보장.

### FFI loads, `extern "C"`, dlopen-style (Rust → pdfium et al.)

None observed in this domain. claude_cli 는 `tokio::process::Command::new("claude")` subprocess 만 — child process 는 standard `wait/kill` API. PDFium FFI 는 [08-pdf-ocr-pipeline.md](08-pdf-ocr-pipeline.md) 참고.

### Result swallow (TypeScript)

#### 핵심 위험 1 — `claude-cli-transport` 의 silent abort dispatch

```typescript src/lib/claude-cli-transport.ts:158-166
const abortListener = () => {
    void invoke("claude_cli_kill", { streamId }).catch(() => {
      // Kill is best-effort; if the process already exited, the Rust
      // side returns Ok and the done handler fires normally.
    })
    finishWith(onDone)
  }
  signal?.addEventListener("abort", abortListener)
```
abort 시 kill 호출 실패가 silently 묻혀요. **결과**: kill 이 실패 (예: stream_id 미스매치, 또는 Rust 측 lock contention) 해도 `onDone` 즉시 호출 → frontend 는 정상 종료라 보지만 child process 는 여전히 살아 있음. 다음 stream 시작 시 stale child 가 stdout 에 뭐든 emit 하면 새 listener 가 그걸 다른 stream 의 data 로 오인할 수 있어요. test coverage 0 건.

```typescript src/lib/claude-cli-transport.ts:42-45
try {
      evt = JSON.parse(line)
    } catch {
      return null
    }
```
JSON parse 실패한 line silently drop. 정상 (CLI 가 prefix 데이터 emit 시 첫 line 이 partial). 단, 실제 stream-json schema 변경 시 사용자에게는 "왜 token 이 안 흘러요" 만 보임.

#### 핵심 위험 2 — `llm-client` 의 stream reader cancel race

```typescript src/lib/llm-client.ts:174-187
} catch (err) {
    if (err instanceof Error && (err.name === "AbortError" || (signal?.aborted))) {
      onDone()
      return
    }
    if (isFetchNetworkError(err)) {
      onError(new Error("Connection lost during streaming. Try again."))
      return
    }
    onError(err instanceof Error ? err : new Error(String(err)))
  } finally {
    reader.releaseLock()
  }
```
streaming 중 abort 시 `onDone` 호출 — 이미 partial 응답이 token 으로 흘러갔는데 caller 는 정상 종료라 받음. 사용자가 cancel 후 다시 보낼 때 partial generation 이 디스크에 남아 있을 수 있음 (`autoIngest` 쪽이 hardFailures 로 catch). test 는 `llm-client.test.ts` 의 isFetchNetworkError 만 픽스 — 실제 stream cancel 은 unit test 미존재.

```typescript src/lib/llm-client.ts:130-140
if (!response.ok) {
    let errorDetail = `HTTP ${response.status}: ${response.statusText}`
    try {
      const body = await response.text()
      if (body) errorDetail += ` — ${body}`
    } catch {
      // ignore body read failure
    }
    onError(new Error(errorDetail))
    return
  }
```
HTTP 4xx/5xx 의 body read 실패 silent — 사용자에게는 "HTTP 401: Unauthorized" 만 보이고 server 에서 보낸 hint 메시지 (잘못된 model id 등) 는 사라져요.

#### 핵심 위험 3 — `llm-providers` SSE parser 의 silent drop

```typescript src/lib/llm-providers.ts:118-127
try {
    const parsed = JSON.parse(data) as {
      choices: Array<{ delta: { content?: string } }>
    }
    return parsed.choices?.[0]?.delta?.content ?? null
  } catch {
    return null
  }
```
OpenAI SSE 의 corrupt JSON line silently drop. provider 가 schema breaking change 를 했을 때 사용자는 "stream 이 하다 만 것 같음" 만 보임.

```typescript src/lib/llm-providers.ts:138-146
if (
      parsed.type === "content_block_delta" &&
      parsed.delta?.type === "text_delta"
    ) {
      return parsed.delta.text ?? null
    }
    return null
  } catch {
    return null
  }
```
Anthropic SSE 동일 패턴.

```typescript src/lib/llm-providers.ts:165-175
const parts = parsed.candidates?.[0]?.content?.parts
    if (!parts || parts.length === 0) return null
    let out = ""
    for (const p of parts) {
      if (p.thought) continue
      if (p.text) out += p.text
    }
    return out.length > 0 ? out : null
  } catch {
    return null
  }
```
Gemini SSE — `thought: true` part 는 의도적으로 skip (chain-of-thought 누설 방지). 단, `parts` 형태가 새 schema 로 바뀌면 silently 빈 문자열.

#### 핵심 위험 4 — `ingest.ts` 의 catch swallow 6 군데

```typescript src/lib/ingest.ts:638-644
try {
      await writeFile(sourceSummaryFullPath, fallbackContent)
      writtenPaths.push(sourceSummaryPath)
    } catch {
      // non-critical
    }
```
fallback source-summary write 실패 시 silent. 사용자는 source page 가 wiki 에 없다는 사실만 보고 ingest 를 다시 돌려야 함.

```typescript src/lib/ingest.ts:656-662
try {
      const tree = await listDirectory(pp)
      useWikiStore.getState().setFileTree(tree)
      useWikiStore.getState().bumpDataVersion()
    } catch {
      // ignore
    }
```
Tree refresh 실패 silent — UI 에 새 페이지가 안 보일 수 있음.

```typescript src/lib/ingest.ts:696-707
try {
          const content = await readFile(`${pp}/${wpath}`)
          const titleMatch = content.match(/^---\n[\s\S]*?^title:\s*["']?(.+?)["']?\s*$/m)
          const title = titleMatch ? titleMatch[1].trim() : pageId
          await embedPage(pp, pageId, title, content, embCfg)
        } catch {
          // non-critical
        }
      }
    } catch {
      // embedding module not available
    }
```
ingest 후 embedding 실패가 page-by-page silent — 일부 페이지만 embedding 빠진 채 인덱스가 부분 갱신됨. 사용자는 "왜 RRF 의 vector half 가 어떤 페이지는 미스 하지" 만 보임.

```typescript src/lib/ingest.ts:1089-1095
async function tryReadFile(path: string): Promise<string> {
  try {
    return await readFile(path)
  } catch {
    return ""
  }
}
```
schema/purpose/index/overview 의 부재 (정상 케이스) + 실제 read 실패 (디스크 오류) 를 구분 없이 빈 string 반환. 후자가 발생하면 prompt 에 빠진 컨텍스트로 LLM 이 잘못된 wiki 를 만들어요.

#### 핵심 위험 5 — `ingest-queue` 의 stale-context guard 다중

```typescript src/lib/ingest-queue.ts:435-500
async function processNext(projectId: string): Promise<void> {
  if (processing) return
  if (currentProjectId !== projectId) return
  ...
  if (currentProjectId !== projectId) return
  ...
  if (currentProjectId !== projectId) return
  ...
  if (currentProjectId !== projectId) return
  ...
  if (currentProjectId !== projectId) return
```
processNext 가 매 await (registry lookup, saveQueue, autoIngest 호출 후) 마다 5 군데에서 `currentProjectId` mismatch 검사. 의도적이지만 — orphan 된 promise 가 그냥 silent return 으로 끝남. 호출자 (예: enqueue 가 fire-and-forget `processNext` 호출) 는 어떤 task 가 어떻게 끝났는지 모름. test `sweep-reviews.race.test.ts` 의 `clearQueueState` 호출이 abort 까지 fan out 시키는 게 보호장치.

#### 핵심 위험 6 — `embedding.ts` 의 layered swallow

```typescript src/lib/embedding.ts:115-120
let bodyText = ""
      try {
        bodyText = await resp.text()
      } catch {
        // ignore — some servers return empty bodies on error
      }
```
embedding error body read 실패 silent. fetchEmbedding 의 retry 결정에 들어가는 oversize 감지가 body 없이 돌아 — `resp.status === 413` 만 trigger.

```typescript src/lib/embedding.ts:223-231
export async function legacyVectorRowCount(projectPath: string): Promise<number> {
  try {
    return await invoke("vector_legacy_row_count", {
      projectPath: normalizePath(projectPath),
    })
  } catch {
    return 0
  }
}
```
legacy 카운트 실패 시 0 반환 — Settings UI 의 "re-index to v2" prompt 가 안 떠서 사용자는 v1 stale 인덱스에 영원히 머무를 수 있음.

```typescript src/lib/embedding.ts:453-462
export async function removePageEmbedding(
  projectPath: string,
  pageId: string,
): Promise<void> {
  try {
    await vectorDeletePage(projectPath, pageId)
  } catch {
    // non-critical
  }
}
```
embedding cascade delete 실패 silent. 06 도메인의 `cascadeDeleteWikiPage` 가 이걸 통해 호출하므로 — orphaned chunk 가 vector index 에 남고 search 에 phantom 등장 가능. test `wiki-page-delete.test.ts:78-91` 가 실패 propagation 을 확인하지만, `removePageEmbedding` 자체는 swallow 라 caller 한테는 성공으로 보임.

#### 핵심 위험 7 — `sweep-reviews` 의 race 케이스 (race.test 픽스)

`sweep-reviews.race.test.ts:106-156` 는 3 개 race 시나리오를 픽스해 — 프로젝트 path mismatch 즉시 bail, mid-sweep buildWikiIndex 후 project switch 시 resolveItem 안 적용, mid-stage2 LLM 중 project switch + abort 시 결과 안 적용. 핵심 보호:

```typescript src/lib/sweep-reviews.ts:404-431
if (stillPending.length > 0 && !signal?.aborted && matchesCurrentProject(projectPath)) {
    activityId = activity.addItem({...})

    try {
      const resolvedIds = await llmJudgeReviews(stillPending, index, signal)
      if (!signal?.aborted && matchesCurrentProject(projectPath)) {
        for (const id of resolvedIds) {
          store.resolveItem(id, "llm-judged")
          llmResolved++
        }
      }
    } catch (err) {
      activity.updateItem(activityId, {
        status: "error",
        detail: `Review cleanup failed: ${err instanceof Error ? err.message : String(err)}`,
      })
      activityId = null
    }
  }
```
**Sharp edge**: rule-stage 와 LLM-stage 사이 매 await 후 `matchesCurrentProject` 다시 검사 — 그러나 rule-stage 안 (`sweep-reviews.ts:363-397`) 에서 `store.resolveItem` 을 직접 호출하면서 매 iteration 마다 guard 가 있긴 한데 (`:366`), guard 자체가 동기 store check 라 실제 race 가 발생하면 일부 item 만 처리되고 일부는 안 됨 — 부분 적용 상태가 남을 수 있어요. property test `sweep-reviews.property.test.ts:88-94` 는 extractJsonObject 가 garbage input 에 throw 안 하는 것만 픽스.

```typescript src/lib/sweep-reviews.ts:259-281
if (hadError || signal?.aborted || !raw.trim()) return new Set()

  try {
    const cleaned = extractJsonObject(raw)
    if (!cleaned) {
      console.warn("[Sweep Reviews] No JSON object in response:", raw.slice(0, 300))
      return new Set()
    }
    const parsed = JSON.parse(cleaned) as { resolved?: unknown }
    if (!parsed || !Array.isArray(parsed.resolved)) return new Set()

    const validIds = new Set(batch.map((i) => i.id))
    const resolved = new Set<string>()
    for (const id of parsed.resolved) {
      if (typeof id === "string" && validIds.has(id)) {
        resolved.add(id)
      }
    }
    return resolved
  } catch (err) {
    console.warn("[Sweep Reviews] Failed to parse LLM response:", err, raw.slice(0, 300))
    return new Set()
  }
```
LLM JSON parse 실패 시 console.warn + 빈 set — 사용자에게는 silent. judgeBatch 의 모든 실패 (network, parse, abort, no-config) 가 같은 빈 set 으로 collapse 돼 root cause 추적이 console 만으로 가능.

#### 핵심 위험 8 — `clip-watcher` 의 폴링 silent

```typescript src/lib/clip-watcher.ts:31-58
try {
            const tree = await listDirectory(project.path)
            store.setFileTree(tree)
          } catch {
            // ignore
          }

          ...
          if (hasLlm) {
            enqueueIngest(project.id, clipFilePath).catch((err) => {
              console.error("Failed to enqueue web clip:", err)
            })
          }
        }
      }
    } catch {
      // Server not running or network error — silently ignore
    }
```
Clip server 가 다운되면 watcher 가 silently 폴링만 계속 — 사용자는 webclip 이 도착 안 했다는 사실만 보고 왜인지 몰라요. `enqueueIngest` 자체의 catch (`:49-51`) 만 console.error — 실제 enqueue 실패도 logging 외에는 무음.

#### 핵심 위험 9 — `wiki-graph` / `graph-relevance` / `deep-research` 의 read swallow

```typescript src/lib/wiki-graph.ts:165-169
let tree: FileNode[]
  try {
    tree = await listDirectory(wikiRoot)
  } catch {
    return { nodes: [], edges: [], communities: [] }
  }
```
wiki/ 디렉터리 부재 → 빈 그래프. 정상 케이스 (새 프로젝트) + 실제 디스크 오류 구분 안 됨.

```typescript src/lib/wiki-graph.ts:185-189
try {
      content = await readFile(file.path)
    } catch {
      // Skip unreadable files
      continue
    }
```
graph 빌드 도중 한 파일 read 실패 silent — graph 가 부분 상태로 community detection 실행. 사용자는 "왜 X 노드가 안 보임" 만 보임.

```typescript src/lib/wiki-graph.ts:247-253
let retrievalGraph: Awaited<ReturnType<typeof buildRetrievalGraph>> | null = null
  try {
    const { useWikiStore } = await import("@/stores/wiki-store")
    const dv = useWikiStore.getState().dataVersion
    retrievalGraph = await buildRetrievalGraph(normalizePath(projectPath), dv)
  } catch {
    // ignore — weights will default to 1
  }
```
edge weight 계산 실패 silent — 모든 edge 가 weight=1 균등이 돼서 graph layout 이 "왜 다른 모습이지" 사용자에게 unexplained.

```typescript src/lib/deep-research.ts:211-217
try {
      const tree = await listDirectory(pp)
      useWikiStore.getState().setFileTree(tree)
      useWikiStore.getState().bumpDataVersion()
    } catch {
      // ignore
    }
```
research 후 tree refresh 실패 silent — 새 query page 가 file tree 에 안 보일 수 있음. 사용자는 reload 까지 손가락질.

#### 핵심 위험 10 — `as any` / `as unknown as` 캐스트

```typescript src/lib/tauri-fetch.ts:46-49
pluginFetchPromise = import("@tauri-apps/plugin-http")
        .then((m) => m.fetch as unknown as typeof globalThis.fetch)
        .catch(() => globalThis.fetch.bind(globalThis))
```
plugin-http 의 `fetch` 타입이 native fetch 와 정확히 호환된다고 단언. 만약 plugin 의 signature 가 미세하게 달라지면 (response body API, signal 처리, 등) — runtime 에서만 발견되고 caller (llm-client, embedding, web-search) 에 전파됨. **이 도메인의 가장 wide-reach cast** — 거의 모든 외부 HTTP 호출이 이 함수를 통해요.

```typescript src/lib/claude-cli-transport.ts:55-77
const event = obj.event as Record<string, unknown> | undefined
      if (event?.type === "content_block_delta") {
        const delta = event.delta as Record<string, unknown> | undefined
        if (delta?.type === "text_delta" && typeof delta.text === "string") {
          sawDelta = true
          return delta.text
        }
      }
      return null
    }

    if (type === "assistant") {
      const message = obj.message as Record<string, unknown> | undefined
      const content = message?.content
      if (!Array.isArray(content)) return null
      const text = content
        .map((c) => {
          const cc = c as Record<string, unknown>
          return cc.type === "text" && typeof cc.text === "string" ? cc.text : ""
        })
        .join("")
```
**stream chunk handler 의 핵심 cast**. `obj`, `event`, `delta`, `message`, `content`, 각 `c` 모두 `Record<string, unknown>` 으로 narrow — 본질적으로 `as any` 패턴이지만 strict 모드 우회. 검증 0 건. Anthropic SDK 의 stream-json 형식이 변경되면 (예: `text_delta` → `text` rename, 또는 `assistant` event 의 `message.content` block array 에서 string 으로 collapse) 사용자에게는 "claude CLI 에서 token 이 안 와요" 만 보이고 코드 추적이 어려움.

```typescript src/lib/claude-cli-transport.ts:106-110
type SpawnPayload = Record<string, unknown> & {
  streamId: string
  model: string
  messages: ChatMessage[]
}
```
Tauri invoke 의 index signature 우회용 인터섹션 타입 — 캐스트는 없지만 runtime 에서 Tauri 가 받는 payload 가 실제로 string 키 필드 외에 뭘 가졌는지 컴파일 타임에 모름.

```typescript src/lib/ingest.ts:862-866
const type = (
      ["contradiction", "duplicate", "missing-page", "suggestion"].includes(rawType)
        ? rawType
        : "confirm"
    ) as ReviewItem["type"]
```
4 종 외 input 은 "confirm" 으로 mapping — 의도. 단, `ReviewItem["type"]` 에 새 분기가 추가되면 silent drift 가능.

#### project-mutex 와의 결합

`autoIngest` 가 `withProjectLock` (06 도메인) 으로 직렬화 — 단, 그 lock 안에서 autoIngestImpl 이 6 단계 await 을 한다는 건 **한 ingest 의 catastrophic hang** (LLM 응답 무한 대기, claude CLI subprocess deadlock, etc) 이 같은 프로젝트의 모든 후속 ingest / save-to-wiki / deep-research 의 auto-ingest 를 블록한다는 뜻. project-mutex.ts 의 주석 자체가 "no timeouts, no fairness" 라 인정. queue 의 `cancelTask` 가 abort signal 을 propagate 하지만 — `withProjectLock` 의 `await fn()` 자체는 abort 를 모르고 fn 의 결과를 기다려요.

#### ingest-queue 크래시 복구 표면

`ingest-queue.ts:388-395` 의 restoreQueue 가 `processing` → `pending` 로 되돌리고, `processNext` 가 retry 시 `MAX_RETRIES=3` 까지 시도. 하지만 — `cleanupWrittenFiles` (`:92-107`) 가 사실상 cancel 시에만 실행돼요. 앱이 mid-ingest 에 crash 하면 disk 의 partial wiki page (Stage 2 에서 일부만 쓰인) 가 그대로 남고, 다음 부팅시 retry 가 성공해도 *이전* run 이 만든 phantom page 가 그대로 sitting. 사용자가 직접 lint view 의 orphan-page sweep 을 돌리지 않으면 stale 상태 유지.

## Cross-refs

- [04-backend-rust.md](04-backend-rust.md) — `claude_cli_spawn / claude_cli_kill / claude_cli_detect` 가 `panic_guard::run_guarded_async` (vectorstore.rs / fs.rs 와 같은 panic 캡처 layer) 적용. `vector_*` 명령 호출자는 모두 이 도메인 (embedding.ts).
- [06-data-layer.md](06-data-layer.md) — `withProjectLock` (project-mutex), `cascadeDeleteWikiPage`, `mergeSourcesIntoContent`, `decidePageFate` 가 ingest/sweep/cleanup 흐름의 직접 의존성. `vector_upsert_chunks` 등 LanceDB IPC 도 06 의 schema 정의와 묶임.
- [03-frontend.md](03-frontend.md) — `useWikiStore` (project, llmConfig, embeddingConfig, multimodalConfig, outputLanguage, dataVersion), `useChatStore` (ingest mode + streaming), `useReviewStore` (sweep target), `useResearchStore` (deep-research progress), `useActivityStore` (long-running 진행 표시) 가 모두 이 도메인의 LLM 호출이 mutate 하는 zustand store.
- [08-pdf-ocr-pipeline.md](08-pdf-ocr-pipeline.md) — `extractAndSaveSourceImages`, `captionMarkdownImages`, `loadCaptionCache`, `buildImageMarkdownSection` 가 ingest pipeline 의 image-cascade hop. PDFium FFI 의 결과를 이 도메인이 prompt 에 주입.
- [01-tech-stack.md](01-tech-stack.md) — graphology / graphology-communities-louvain 의존성, `@tauri-apps/plugin-http` / `@tauri-apps/plugin-store` / `@tauri-apps/api/event` / `@tauri-apps/api/core` 버전, `which::which("claude")` 의 which crate 가 모두 이 도메인의 외부 의존성.

50-source-mapping.md 행 링크:

- [src/lib/llm-client.ts](50-source-mapping.md#srclibllm-clientts)
- [src/lib/llm-providers.ts](50-source-mapping.md#srclibllm-providersts)
- [src/lib/claude-cli-transport.ts](50-source-mapping.md#srclibclaude-cli-transportts)
- [src/lib/endpoint-normalizer.ts](50-source-mapping.md#srclibendpoint-normalizerts)
- [src/lib/context-budget.ts](50-source-mapping.md#srclibcontext-budgetts)
- [src/lib/templates.ts](50-source-mapping.md#srclibtemplatests)
- [src/lib/ingest.ts](50-source-mapping.md#srclibingestts)
- [src/lib/ingest-queue.ts](50-source-mapping.md#srclibingest-queuets)
- [src/lib/ingest-cache.ts](50-source-mapping.md#srclibingest-cachets)
- [src/lib/text-chunker.ts](50-source-mapping.md#srclibtext-chunkerts)
- [src/lib/enrich-wikilinks.ts](50-source-mapping.md#srclibenrich-wikilinksts)
- [src/lib/embedding.ts](50-source-mapping.md#srclibembeddingts)
- [src/lib/search.ts](50-source-mapping.md#srclibsearchts)
- [src/lib/wiki-graph.ts](50-source-mapping.md#srclibwiki-graphts)
- [src/lib/graph-insights.ts](50-source-mapping.md#srclibgraph-insightsts)
- [src/lib/graph-relevance.ts](50-source-mapping.md#srclibgraph-relevancets)
- [src/lib/deep-research.ts](50-source-mapping.md#srclibdeep-researchts)
- [src/lib/optimize-research-topic.ts](50-source-mapping.md#srcliboptimize-research-topicts)
- [src/lib/web-search.ts](50-source-mapping.md#srclibweb-searchts)
- [src/lib/sweep-reviews.ts](50-source-mapping.md#srclibsweep-reviewsts)
- [src/lib/review-utils.ts](50-source-mapping.md#srclibreview-utilsts)
- [src/lib/source-delete-decision.ts](50-source-mapping.md#srclibsource-delete-decisionts)
- [src/lib/tauri-fetch.ts](50-source-mapping.md#srclibtauri-fetchts)
- [src/lib/clip-watcher.ts](50-source-mapping.md#srclibclip-watcherts)
- [src/lib/update-check.ts](50-source-mapping.md#srclibupdate-checkts)
- [src-tauri/src/commands/claude_cli.rs](50-source-mapping.md#src-taurisrccommandsclaude_clirs)

## Evidence

### Provider abstraction

- `src/lib/llm-client.ts:36-58` — `streamChat` 진입 + claude-code subprocess dispatch 분기
- `src/lib/llm-client.ts:68` — 30 분 long-horizon timeout
- `src/lib/llm-client.ts:73-87` — combined abort signal (user + timeout)
- `src/lib/llm-client.ts:99-128` — fetch error 분기 (signal abort / timeout / network / generic)
- `src/lib/llm-client.ts:147-189` — stream reader loop + finally `releaseLock`
- `src/lib/llm-providers.ts:18-35` — `ContentBlock` 와 `ChatMessage` 정의
- `src/lib/llm-providers.ts:46-52` — `RequestOverrides` (wire-agnostic sampling)
- `src/lib/llm-providers.ts:111-113` — `localLlmOriginHeader`
- `src/lib/llm-providers.ts:115-176` — 3 SSE parser
- `src/lib/llm-providers.ts:194-208` — `toOpenAiContent` (single-text-array → flat string fast path)
- `src/lib/llm-providers.ts:236-252` / `:261-266` — `toAnthropicContent` + `flattenAnthropicSystem`
- `src/lib/llm-providers.ts:268-295` — `buildAnthropicBody` (top_p / top_k / max_tokens / stop_sequences naming)
- `src/lib/llm-providers.ts:308-319` — `requiresBearerAuth` (MiniMax / Alibaba)
- `src/lib/llm-providers.ts:334-339` — `buildAnthropicUrl` (v1 double-append 가드)
- `src/lib/llm-providers.ts:362-373` — `toGoogleParts` (inline_data)
- `src/lib/llm-providers.ts:408-419` — Gemini generationConfig naming (topP / topK / maxOutputTokens / stopSequences)
- `src/lib/llm-providers.ts:530-537` — claude-code throw branch
- `src/lib/llm-providers.ts:498-509` — qwen3 chat_template_kwargs auto-inject
- `src/lib/llm-providers.test.ts:45-79` — OpenAI vision body 픽스
- `src/lib/llm-providers.test.ts:82-111` — Anthropic vision + system flatten 픽스
- `src/lib/llm-providers.test.ts:114-127` — Google vision parts 픽스

### Claude CLI subprocess transport

- `src/lib/claude-cli-transport.ts:30-99` — stream parser closure (sawDelta + emittedFromAssistant)
- `src/lib/claude-cli-transport.ts:117-217` — streamClaudeCodeCli 흐름
- `src/lib/claude-cli-transport.ts:131-138` — DEV-only override warning
- `src/lib/claude-cli-transport.ts:158-166` — abort listener (kill best-effort)
- `src/lib/claude-cli-transport.ts:170-192` — listen FIRST then spawn ordering
- `src/lib/claude-cli-transport.ts:200-213` — spawn 실패 → "CLI not installed" actionable message
- `src-tauri/src/commands/claude_cli.rs:32-35` — `ClaudeCliState` Arc<Mutex<HashMap>>
- `src-tauri/src/commands/claude_cli.rs:58-124` — `claude_cli_detect` (which + version + macOS quarantine)
- `src-tauri/src/commands/claude_cli.rs:132-312` — `claude_cli_spawn` 전체 흐름
- `src-tauri/src/commands/claude_cli.rs:144-149` — system preamble 를 first user turn 에 inline
- `src-tauri/src/commands/claude_cli.rs:175-188` — Command flag list
- `src-tauri/src/commands/claude_cli.rs:217-230` — content array 형식 강제 ("W is not an Object" CLI 회귀 가드)
- `src-tauri/src/commands/claude_cli.rs:262-270` — stderr collection task
- `src-tauri/src/commands/claude_cli.rs:289-298` — child wait 전 lock drop discipline
- `src-tauri/src/commands/claude_cli.rs:302-308` — done event + stderr 첨부
- `src-tauri/src/commands/claude_cli.rs:317-329` — claude_cli_kill (start_kill + kill_on_drop)

### Endpoint normalization

- `src/lib/endpoint-normalizer.ts:40-53` — empty/protocol fast path
- `src/lib/endpoint-normalizer.ts:64-73` — URL parse 실패 분기
- `src/lib/endpoint-normalizer.ts:78-93` — IPv4 octet validation
- `src/lib/endpoint-normalizer.ts:102-127` — ALWAYS_WRONG_TAILS / MESSAGES_TAIL strip + version-segment hint
- `src/lib/endpoint-normalizer.test.ts:18-26` — chat/completions strip 픽스
- `src/lib/endpoint-normalizer.test.ts:131-141` — 5-octet IP 픽스

### Context budget

- `src/lib/context-budget.ts:54-59` — DEFAULT_MAX_CTX 204800 + 4 frac 상수
- `src/lib/context-budget.ts:67-99` — computeContextBudget 분배 + per-page floor/cap
- `src/lib/context-budget.test.ts:23-33` — 8K config (per-page == pageBudget) 픽스
- `src/lib/context-budget.test.ts:35-42` — 32K config (5K floor binds) 픽스
- `src/lib/context-budget.test.ts:59-67` — 1M config (per-page 150K, no 30K cap) 픽스

### Templates

- `src/lib/templates.ts:11-67` — BASE_* 5 종 공통 블록
- `src/lib/templates.ts:69-187` — research template
- `src/lib/templates.ts:189-304` — reading template
- `src/lib/templates.ts:306-432` — personal template
- `src/lib/templates.ts:434-570` — business template
- `src/lib/templates.ts:572-638` — general template
- `src/lib/templates.ts:640-654` — templates array + getTemplate (throw on unknown)

### Ingest pipeline

- `src/lib/ingest.ts:53` — legacy FILE_BLOCK_REGEX (test 호환)
- `src/lib/ingest.ts:74-75` — OPENER_LINE / CLOSER_LINE 정규식
- `src/lib/ingest.ts:101-115` — isSafeIngestPath 가드
- `src/lib/ingest.ts:119` — FENCE_LINE CommonMark
- `src/lib/ingest.ts:146-239` — parseFileBlocks 본체 (H1-H6 hazard 핸들링)
- `src/lib/ingest.ts:208-216` — truncation warning
- `src/lib/ingest.ts:226-233` — path traversal reject
- `src/lib/ingest.ts:261-271` — autoIngest withProjectLock wrapper
- `src/lib/ingest.ts:293-299` — 5-source parallel read
- `src/lib/ingest.ts:311-312` — cache check + diag log
- `src/lib/ingest.ts:336-370` — cache-hit 시 image cascade (multimodal disable strip + skip)
- `src/lib/ingest.ts:454-504` — 풀-pipeline image strip + caption
- `src/lib/ingest.ts:506-508` — 50K char truncation (analysis input)
- `src/lib/ingest.ts:517-540` — Stage 1 streamChat + abort-as-error 보호
- `src/lib/ingest.ts:548-591` — Stage 2 streamChat
- `src/lib/ingest.ts:595-606` — writeFileBlocks 호출 + warning surface
- `src/lib/ingest.ts:608-644` — fallback source-summary write (signal.aborted 체크)
- `src/lib/ingest.ts:680-686` — hardFailures cache skip
- `src/lib/ingest.ts:730-752` — contentMatchesTargetLanguage CJK/Latin 분류
- `src/lib/ingest.ts:782-801` — language guard skip (log / sources / entities)
- `src/lib/ingest.ts:830-834` — mergeSourcesIntoContent dynamic import (06 도메인 호출)
- `src/lib/ingest.ts:850-911` — parseReviewBlocks (REVIEW_BLOCK_REGEX + OPTIONS/PAGES/SEARCH)
- `src/lib/ingest.ts:917-962` — buildAnalysisPrompt
- `src/lib/ingest.ts:967-1083` — buildGenerationPrompt + duplicate language directive (마지막에 한 번 더)
- `src/lib/ingest.ts:1111-1174` — injectImagesIntoSourceSummary marker pair
- `src/lib/ingest.ts:1190-1210` — reembedSourceSummary

### Ingest queue

- `src/lib/ingest-queue.ts:9-21` — IngestTask 정의 (projectId by UUID)
- `src/lib/ingest-queue.ts:25-42` — module-level state (queue, processing, currentProjectId, etc.)
- `src/lib/ingest-queue.ts:60-74` — loadQueue projectId 백필
- `src/lib/ingest-queue.ts:92-107` — cleanupWrittenFiles cascadeDeleteWikiPage 호출
- `src/lib/ingest-queue.ts:114-142` — enqueueIngest 검증
- `src/lib/ingest-queue.ts:200-227` — cancelTask abort + cleanup
- `src/lib/ingest-queue.ts:245-264` — cancelAllTasks
- `src/lib/ingest-queue.ts:317-351` — pauseQueue (await disk flush 필수)
- `src/lib/ingest-queue.ts:361-407` — restoreQueue (cross-project contamination 가드)
- `src/lib/ingest-queue.ts:411` — MAX_RETRIES = 3
- `src/lib/ingest-queue.ts:413-433` — onQueueDrained sweep trigger
- `src/lib/ingest-queue.ts:435-539` — processNext 본체 (5 군데 stale guard)
- `src/lib/ingest-queue.ts:454-466` — registry path lookup (project moved 케이스)
- `src/lib/ingest-queue.ts:476-483` — LLM not configured → fail 처리
- `src/lib/ingest-queue.ts:507-509` — zero-files safety net (`return []` masquerade 방지)
- `src/lib/ingest-queue.ts:526-532` — retry / fail 분기

### Ingest cache

- `src/lib/ingest-cache.ts:20-26` — sha256
- `src/lib/ingest-cache.ts:61-93` — checkIngestCache (모든 cached file 디스크 존재 확인 → ghost 회귀 가드)

### Text chunker

- `src/lib/text-chunker.ts:46-64` — ChunkingOptions defaults
- `src/lib/text-chunker.ts:67-84` — Chunk 정의 (oversized flag)
- `src/lib/text-chunker.ts:131-144` — stripFrontmatter
- `src/lib/text-chunker.ts:163-238` — splitIntoSections (heading + fence-aware)
- `src/lib/text-chunker.ts:282-289` — Atom (indivisible: code/table)
- `src/lib/text-chunker.ts:291-367` — tokenizeAtoms
- `src/lib/text-chunker.ts:399-406` — SENTENCE_SPLITTERS 3-tier
- `src/lib/text-chunker.ts:413-471` — recursiveSplit fallthrough → hard slice
- `src/lib/text-chunker.ts:498-529` — sizePieces greedy packer
- `src/lib/text-chunker.ts:564-578` — applyOverlap (forward snap, 회귀 fix)
- `src/lib/text-chunker.ts:580-601` — snapOverlapHead (backward → forward 변경 fix)

### Wikilink enrichment

- `src/lib/enrich-wikilinks.ts:25-101` — enrichWithWikilinks (LLM JSON list 만 받아서 first-occurrence replace)
- `src/lib/enrich-wikilinks.ts:108-157` — parseLinkResponse (balanced-brace + fence tolerant)
- `src/lib/enrich-wikilinks.ts:166-192` — applyLinks (frontmatter split + linkedTargets dedup)
- `src/lib/enrich-wikilinks.ts:195-210` — findUnlinkedOccurrence (`[[` 우회 가드)

### Embedding

- `src/lib/embedding.ts:39-43` — getLastEmbeddingError (Settings UI 용)
- `src/lib/embedding.ts:55-68` — looksLikeOversizeError (HTTP 413 + 8 phrasing)
- `src/lib/embedding.ts:80-161` — fetchEmbedding auto-halve retry loop
- `src/lib/embedding.ts:172-188` — vectorUpsertChunks Math.fround f32 변환
- `src/lib/embedding.ts:248-257` — enrichChunkForEmbedding (title + heading + chunk)
- `src/lib/embedding.ts:268-313` — embedPage (transient 실패 시 기존 인덱스 보존)
- `src/lib/embedding.ts:321-367` — embedAllPages (structural pages skip)
- `src/lib/embedding.ts:390-447` — searchByEmbedding (top-K × 3 over-fetch + max + 0.3*tail blended)

### Search (RRF)

- `src/lib/search.ts:35-36` — MAX_RESULTS=20, SNIPPET_CONTEXT=80
- `src/lib/search.ts:53` — `RRF_K = 60`
- `src/lib/search.ts:73-78` — scoring weight 상수
- `src/lib/search.ts:80-86` — STOP_WORDS 23 종 (영문 + 중문)
- `src/lib/search.ts:88-125` — tokenizeQuery (CJK bigram + char + token)
- `src/lib/search.ts:218-368` — searchWiki RRF 본체
- `src/lib/search.ts:230-263` — wiki/ 만 search (raw/sources/ 5-15s 회귀 fix)
- `src/lib/search.ts:268-272` — tokenRank 스냅샷 (vector add 전)
- `src/lib/search.ts:299-336` — vectorRank materialize (knownIds dedup)
- `src/lib/search.ts:345-366` — RRF 계산 (1/(K+rank) 합)
- `src/lib/search.ts:357-360` — 알파벳 path tie-break (deterministic)
- `src/lib/search.ts:379` — SEARCH_READ_CONCURRENCY=16
- `src/lib/search.ts:390-429` — searchFiles batch readFile
- `src/lib/search.ts:436-494` — scoreFile 순수 점수 계산
- `src/lib/search-rrf.test.ts:76-107` — vector-only match 가 token-only weak 를 이긴다
- `src/lib/search-rrf.test.ts:109-144` — RRF 수치 픽스 (1/61 + 1/61 vs 1/62)
- `src/lib/search-rrf.test.ts:191-230` — score-magnitude 가 아니라 rank 기반 (`vr.score * 5` 회귀 가드)
- `src/lib/search-rrf.test.ts:232-252` — phantom (디스크 부재) 페이지 silent drop

### Wiki graph

- `src/lib/wiki-graph.ts:8-28` — GraphNode / GraphEdge / CommunityInfo
- `src/lib/wiki-graph.ts:31-113` — detectCommunities (Louvain resolution=1, cohesion ratio, sequential 재번호)
- `src/lib/wiki-graph.ts:115` — WIKILINK_REGEX
- `src/lib/wiki-graph.ts:159-286` — buildWikiGraph
- `src/lib/wiki-graph.ts:204` — HIDDEN_TYPES = {"query"}
- `src/lib/wiki-graph.ts:234-243` — edge dedup (양방향 캐노니컬)
- `src/lib/wiki-graph.ts:255-265` — retrievalGraph 로 weight 계산
- `src/lib/wiki-graph.ts:288-304` — resolveTarget (case-insensitive, space-hyphen)
- `src/lib/graph-insights.ts:31-102` — findSurprisingConnections (4 signal)
- `src/lib/graph-insights.ts:42` — STRUCTURAL_IDS={"index","log","overview"}
- `src/lib/graph-insights.ts:114-193` — detectKnowledgeGaps (3 카테고리)
- `src/lib/graph-relevance.ts:28` — WIKILINK_REGEX (wiki-graph 와 동일 — 재정의)
- `src/lib/graph-relevance.ts:30-43` — WEIGHTS + TYPE_AFFINITY 매트릭스
- `src/lib/graph-relevance.ts:49` — module-level cachedGraph
- `src/lib/graph-relevance.ts:71-112` — extractFrontmatter (sources YAML 양쪽 형태)
- `src/lib/graph-relevance.ts:155-245` — buildRetrievalGraph (dataVersion 캐시)
- `src/lib/graph-relevance.ts:230-240` — Object.freeze 로 immutable nodes
- `src/lib/graph-relevance.ts:247-287` — calculateRelevance (Adamic-Adar)
- `src/lib/graph-relevance.ts:310-312` — clearGraphCache (06 의 reset-project-state 가 호출)

### Deep research

- `src/lib/deep-research.ts:13-33` — queueResearch (50ms setTimeout)
- `src/lib/deep-research.ts:38-52` — processQueue concurrency
- `src/lib/deep-research.ts:64-91` — multi-query webSearch + url dedup
- `src/lib/deep-research.ts:114-133` — synthesis system prompt
- `src/lib/deep-research.ts:137-156` — streamChat synthesis (incremental store update)
- `src/lib/deep-research.ts:166-201` — research-<slug>-<date>.md 생성
- `src/lib/deep-research.ts:178-181` — `<think>` 블록 strip
- `src/lib/deep-research.ts:220-222` — autoIngest fire-and-forget
- `src/lib/optimize-research-topic.ts:14-75` — optimizeResearchTopic (4-line strict format)
- `src/lib/web-search.ts:11-26` — webSearch provider switch
- `src/lib/web-search.ts:28-72` — tavilySearch advanced

### Sweep reviews

- `src/lib/sweep-reviews.ts:50-83` — buildWikiIndex
- `src/lib/sweep-reviews.ts:89-106` — extractCandidateNames
- `src/lib/sweep-reviews.ts:109-121` — pageExists (id+kebab+title 3 형태 매칭)
- `src/lib/sweep-reviews.ts:135-177` — extractJsonObject balanced-brace
- `src/lib/sweep-reviews.ts:179-181` — JUDGE_BATCH_SIZE=40, MAX_JUDGE_BATCHES=5, MAX_PAGES_IN_PROMPT=300
- `src/lib/sweep-reviews.ts:190-282` — judgeBatch
- `src/lib/sweep-reviews.ts:259` — abort + hadError + empty raw → empty set
- `src/lib/sweep-reviews.ts:291-316` — llmJudgeReviews early-break (배치 0 resolve 시)
- `src/lib/sweep-reviews.ts:326-330` — matchesCurrentProject guard
- `src/lib/sweep-reviews.ts:341-461` — sweepResolvedReviews 본체
- `src/lib/sweep-reviews.ts:357` — 매 await 후 guard 재검사
- `src/lib/sweep-reviews.ts:402-431` — Stage 2 LLM stage + activity indicator
- `src/lib/sweep-reviews.ts:419-424` — pre-resolve guard (project switch race)
- `src/lib/sweep-reviews.race.test.ts:106-156` — 3 race scenario 픽스
- `src/lib/sweep-reviews.race.test.ts:165-196` — abort signal 케이스
- `src/lib/sweep-reviews.race.test.ts:228-303` — LLM batch loop early-break + max-batch cap
- `src/lib/sweep-reviews.property.test.ts:88-94` — extractJsonObject never-throw 픽스
- `src/lib/review-utils.ts:9-10` — REVIEW_TITLE_PREFIX_RE (영문 + 중문 prefix)
- `src/lib/review-utils.ts:21-27` — normalizeReviewTitle
- `src/lib/source-delete-decision.ts:11-19` — DeleteDecision union
- `src/lib/source-delete-decision.ts:33-58` — decidePageFate (skip / keep / delete)

### Misc (clip-watcher, update-check, tauri-fetch)

- `src/lib/tauri-fetch.ts:20-29` — pluginFetchPromise + isNodeEnv
- `src/lib/tauri-fetch.ts:41-53` — getHttpFetch lazy import + fallback
- `src/lib/tauri-fetch.ts:48` — `as unknown as typeof globalThis.fetch` cast
- `src/lib/tauri-fetch.ts:68-79` — isFetchNetworkError (4 backend 통합)
- `src/lib/llm-client.test.ts:11-52` — cross-webview fetch error 분류 픽스
- `src/lib/clip-watcher.ts:5-6` — POLL_INTERVAL=3000
- `src/lib/clip-watcher.ts:12-59` — startClipWatcher
- `src/lib/clip-watcher.ts:43-52` — hasLlm 게이트 + enqueueIngest 호출
- `src/lib/update-check.ts:42-46` — toLatestReleaseUrl (tag → latest)
- `src/lib/update-check.ts:72-88` — isNewer strict semver 비교
- `src/lib/update-check.ts:101-136` — fetchLatestRelease (Tauri plugin-http)
- `src/lib/update-check.ts:144-167` — checkForUpdates 분기 (available / up-to-date / error)
- `src/lib/update-check.ts:176` — UPDATE_CHECK_CACHE_MS = 1h (60 req/h GitHub limit 미만)

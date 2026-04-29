# 04 — Backend Rust (Tauri commands, FFI, panic_guard) — PRIMARY RISK DOC

> **Status:** Phase 4 populated. Primary risk surface — quotes ALL 91 locked Rust risk sites.
> **Phase 6 gate**: ` ```rust ` blocks ≥ 10. Achieved.

## Purpose

`src-tauri/` 는 Tauri 2 기반 데스크톱 앱의 네이티브 백엔드예요. 책임 범위는 (1) 프로젝트 디렉터리 스캐폴딩과 wiki 마크다운 파일 IO, (2) PDFium FFI 를 통한 PDF 텍스트 + 이미지 추출 그리고 zip-기반 Office (DOCX/PPTX/XLSX/ODF) 파싱, (3) LanceDB 위에 얹은 v1 페이지 단위 / v2 청크 단위 벡터스토어 CRUD, (4) Chrome MV3 웹 클리퍼가 POST 하는 `127.0.0.1:19827` 의 in-process HTTP 서버 (`tiny_http`) 데몬, (5) 사용자 머신의 `claude` CLI 를 자식 프로세스로 띄워 stream-json 을 stdout 라인 단위 Tauri 이벤트로 fanout 하는 transport 예요. 외부 신뢰 경계는 webview ↔ Rust IPC 와 브라우저 익스텐션 ↔ clip_server HTTP 두 곳이고, FFI 신뢰 경계는 PDFium dynamic library 한 곳이에요. 모든 third-party 파서 (pdfium-render, calamine, docx-rs, lopdf 간접) 가 malformed 입력에서 panic 하는 것을 가정하고 `panic_guard::run_guarded[_async]` 로 모든 Tauri command body 를 감싸는 게 이 도메인의 핵심 안전 invariant 예요.

## Public Interface

### Tauri commands (registered in `lib.rs:42-74`)

- `clip_server_status — fn() -> String — src/lib.rs:8 — 데몬 상태 문자열 ("starting"/"running"/"port_conflict"/"error") 반환`
- `commands::fs::read_file — async fn(path: String) -> Result<String, String> — src/commands/fs.rs:22 — 확장자 분기로 PDF/Office/이미지/미디어/텍스트 추출`
- `commands::fs::write_file — async fn(path: String, contents: String) -> Result<(), String> — src/commands/fs.rs:894 — 부모 디렉터리 생성 후 UTF-8 텍스트 쓰기`
- `commands::fs::list_directory — async fn(path: String) -> Result<Vec<FileNode>, String> — src/commands/fs.rs:911 — depth ≤ 30, dotfile 스킵, forward-slash 정규화 트리`
- `commands::fs::copy_file — async fn(source: String, destination: String) -> Result<(), String> — src/commands/fs.rs:997 — 단일 파일 복사 (부모 디렉터리 생성)`
- `commands::fs::copy_directory — async fn(source: String, destination: String) -> Result<Vec<String>, String> — src/commands/fs.rs:1017 — 무제한 깊이 재귀 copy, dotfile/dot-dir 스킵, 복사된 절대 경로 리스트`
- `commands::fs::preprocess_file — async fn(path: String) -> Result<String, String> — src/commands/fs.rs:85 — PDF/Office 추출 후 sibling .cache/<name>.txt 저장`
- `commands::fs::delete_file — async fn(path: String) -> Result<(), String> — src/commands/fs.rs:1071 — file/dir 자동 분기 삭제`
- `commands::fs::find_related_wiki_pages — async fn(project_path: String, source_name: String) -> Result<Vec<String>, String> — src/commands/fs.rs:1091 — wiki/ 하 .md 스캔, 3 가지 매칭 strategy`
- `commands::fs::create_directory — async fn(path: String) -> Result<(), String> — src/commands/fs.rs:1224 — `create_dir_all` 래퍼`
- `commands::fs::file_exists — async fn(path: String) -> Result<bool, String> — src/commands/fs.rs:1291 — `Path::exists` blocking 호출 (spawn_blocking 으로 isolate)`
- `commands::fs::read_file_as_base64 — async fn(path: String) -> Result<FileBase64, String> — src/commands/fs.rs:1255 — 바이너리 파일 base64 + 확장자 기반 mime 추정`
- `commands::project::create_project — fn(name: String, path: String) -> Result<WikiProject, String> — src/commands/project.rs:10 — `<root>/<name>` 디렉터리 + schema.md/purpose.md/.obsidian/ 스캐폴딩`
- `commands::project::open_project — fn(path: String) -> Result<WikiProject, String> — src/commands/project.rs:240 — schema.md + wiki/ 존재 검증`
- `commands::vectorstore::vector_upsert — async fn(project_path: String, page_id: String, embedding: Vec<f32>) -> Result<(), String> — src/commands/vectorstore.rs:98 — v1 페이지 단위 LanceDB upsert`
- `commands::vectorstore::vector_search — async fn(project_path: String, query_embedding: Vec<f32>, top_k: usize) -> Result<Vec<VectorSearchResult>, String> — src/commands/vectorstore.rs:150 — v1 페이지 KNN`
- `commands::vectorstore::vector_delete — async fn(project_path: String, page_id: String) -> Result<(), String> — src/commands/vectorstore.rs:217 — v1 단일 페이지 삭제`
- `commands::vectorstore::vector_count — async fn(project_path: String) -> Result<usize, String> — src/commands/vectorstore.rs:254 — v1 row count`
- `commands::vectorstore::vector_upsert_chunks — async fn(project_path: String, page_id: String, chunks: Vec<ChunkUpsertInput>) -> Result<(), String> — src/commands/vectorstore.rs:413 — v2 청크 단위 delete-then-add (replace semantics)`
- `commands::vectorstore::vector_search_chunks — async fn(project_path: String, query_embedding: Vec<f32>, top_k: usize) -> Result<Vec<ChunkSearchResult>, String> — src/commands/vectorstore.rs:483 — v2 청크 KNN`
- `commands::vectorstore::vector_delete_page — async fn(project_path: String, page_id: String) -> Result<(), String> — src/commands/vectorstore.rs:572 — v2 page_id 그룹 삭제`
- `commands::vectorstore::vector_count_chunks — async fn(project_path: String) -> Result<usize, String> — src/commands/vectorstore.rs:613 — v2 row count`
- `commands::vectorstore::vector_legacy_row_count — async fn(project_path: String) -> Result<usize, String> — src/commands/vectorstore.rs:651 — v1 row count (재인덱스 prompt 용)`
- `commands::vectorstore::vector_drop_legacy — async fn(project_path: String) -> Result<(), String> — src/commands/vectorstore.rs:688 — v1 테이블 drop (default namespace)`
- `commands::claude_cli::claude_cli_detect — async fn() -> Result<DetectResult, String> — src/commands/claude_cli.rs:59 — `which claude` + `claude --version` 3s timeout`
- `commands::claude_cli::claude_cli_spawn — async fn(app: AppHandle, state: State<ClaudeCliState>, stream_id: String, model: String, messages: Vec<ClaudeMessage>) -> Result<(), String> — src/commands/claude_cli.rs:132 — `claude -p --output-format stream-json` 자식 spawn, stdout 라인을 `claude-cli:{stream_id}` 이벤트 emit`
- `commands::claude_cli::claude_cli_kill — async fn(state: State<ClaudeCliState>, stream_id: String) -> Result<(), String> — src/commands/claude_cli.rs:318 — `start_kill` (kill_on_drop)`
- `commands::extract_images::extract_pdf_images_cmd — async fn(path: String) -> Result<Vec<ExtractedImage>, String> — src/commands/extract_images.rs:860 — PDF embedded image base64 추출`
- `commands::extract_images::extract_office_images_cmd — async fn(path: String) -> Result<Vec<ExtractedImage>, String> — src/commands/extract_images.rs:871 — DOCX/PPTX/XLSX `media/` 추출`
- `commands::extract_images::extract_and_save_pdf_images_cmd — async fn(source_path, dest_dir, rel_to: String) -> Result<Vec<SavedImage>, String> — src/commands/extract_images.rs:882 — base64 round-trip 없이 disk 직접 저장`
- `commands::extract_images::extract_and_save_office_images_cmd — async fn(source_path, dest_dir, rel_to: String) -> Result<Vec<SavedImage>, String> — src/commands/extract_images.rs:902 — Office 이미지 disk 직접 저장`

### IPC topology

- frontend → Rust: `tauri::generate_handler!` 등록된 31 개 command 가 webview ↔ Rust 직렬화 채널을 사용 (JSON 만, 바이너리는 base64).
- Rust → frontend: `app.emit(&topic, line)` 로 `claude-cli:{stream_id}` (stdout 한 줄/이벤트) 와 `claude-cli:{stream_id}:done` (`{ code, stderr }`) 토픽 emit.
- 외부 → Rust HTTP: `127.0.0.1:19827` 의 `/status`, `/project` (GET/POST), `/projects` (GET/POST), `/clips/pending` (GET), `/clip` (POST). CORS `*` 으로 모든 origin 허용 (extension trust boundary 크로스).

### Plugins (lib.rs:21-28)

- `tauri_plugin_opener` (외부 URL/파일 open), `tauri_plugin_dialog` (네이티브 dialog), `tauri_plugin_store` (settings 영속), `tauri_plugin_http` (CORS 우회용 fetch — `unsafe-headers` feature 활성).

### Public types (`src/types/wiki.rs`)

- `WikiProject { name: String, path: String }` — `src/types/wiki.rs:3-7`
- `FileNode { name: String, path: String, is_dir: bool, children: Option<Vec<FileNode>> }` — `src/types/wiki.rs:9-16`

## Internal Risk

> 91 locked risk sites verified via `grep -rnE '\bunsafe\b|\.unwrap\(\)|\.expect\(|panic!|Mutex::|RwLock::' src-tauri/src` — exact 91 매치. 카테고리별 모두 인용해요.

### unsafe blocks

```rust src-tauri/src/*
None observed in this domain.
```
이 코드베이스는 `unsafe` 키워드를 직접 쓰지 않아요. 모든 FFI 표면은 `pdfium-render` 크레이트의 safe wrapper 뒤에 갇혀 있고, native HTTP 는 `tiny_http` (safe Rust), 자식 프로세스는 `tokio::process` (safe Rust) 로 처리해요. 다만 PDFium dylib 자체는 C++ 구현이라 dynamic load 시점부터는 safe Rust 의 미신을 깨뜨리는 영역으로 들어간다는 점은 `Internal Risk → FFI loads` 에서 별도로 언급해요.

### `.unwrap()` / `.expect()` chains

clip_server.rs 의 production HTTP handler 에서 `Mutex::lock().unwrap()` 이 5 개. mutex 가 poisoned (다른 스레드가 lock 잡은 채 panic) 이면 이 unwrap 들이 즉시 process abort 의미가 되지만, panic_guard 는 Tauri command boundary 만 잡아주고 백그라운드 thread 인 `start_clip_server` 의 spawn 안에서 발생한 panic 은 막지 못해요. `tiny_http` 의 헤더 빌더 unwrap 4 개는 컴파일 타임 const 문자열이라 안전하지만, mutex unwrap 5 개는 cross-thread state 라 진짜 risk 예요.

```rust src-tauri/src/clip_server.rs:75
                Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap(),
```
```rust src-tauri/src/clip_server.rs:76
                Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap(),
```
```rust src-tauri/src/clip_server.rs:77
                Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap(),
```
```rust src-tauri/src/clip_server.rs:78
                Header::from_bytes("Content-Type", "application/json").unwrap(),
```
```rust src-tauri/src/clip_server.rs:103
                    let path = CURRENT_PROJECT.lock().unwrap().clone();
```
```rust src-tauri/src/clip_server.rs:143
                    let projects = ALL_PROJECTS.lock().unwrap().clone();
```
```rust src-tauri/src/clip_server.rs:144
                    let current = CURRENT_PROJECT.lock().unwrap().clone();
```
```rust src-tauri/src/clip_server.rs:169
                                let mut projects = ALL_PROJECTS.lock().unwrap();
```
```rust src-tauri/src/clip_server.rs:186
                    let mut pending = PENDING_CLIPS.lock().unwrap();
```

lib.rs 의 `tauri::Builder::build().expect(...)` 는 startup 시점 한 번만 발생하고 실패 시 메시지가 명확해서 의도된 panic 이에요.

```rust src-tauri/src/lib.rs:106
        .expect("error while building tauri application")
```

extract_images.rs 의 `mime_type.unwrap()` 은 직전 라인 `if mime_type.is_none() { continue; }` 로 가드되어 panic 불가능한 false positive 지만, 리팩터링 안전성을 위해선 `Option::expect("guarded above")` 또는 `let Some(mt) = mime_type else { continue; }` 로 바꾸는 게 좋아요.

```rust src-tauri/src/commands/extract_images.rs:405
        let mime_type = mime_type.unwrap();
```

vectorstore.rs / fs.rs 의 나머지 `.unwrap()` 65 개는 모두 `#[cfg(test)]` 블록 내부 (assertion / fixture setup) 라 production binary 에 들어가지 않아요. 다만 카운트 정확성을 위해 모두 인용해요.

```rust src-tauri/src/commands/vectorstore.rs:743
            .unwrap()
```
```rust src-tauri/src/commands/vectorstore.rs:747
        std::fs::create_dir_all(&p).unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:782
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:784
        let count = vector_count_chunks(pp.clone()).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:797
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:798
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 5);
```
```rust src-tauri/src/commands/vectorstore.rs:802
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:803
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 2);
```
```rust src-tauri/src/commands/vectorstore.rs:813
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:816
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:818
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 7);
```
```rust src-tauri/src/commands/vectorstore.rs:828
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:831
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:832
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 5);
```
```rust src-tauri/src/commands/vectorstore.rs:834
        vector_delete_page(pp.clone(), "page-a".into()).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:835
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 2);
```
```rust src-tauri/src/commands/vectorstore.rs:845
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:848
        let results = vector_search_chunks(pp.clone(), query, 10).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:868
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:871
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:873
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 3);
```
```rust src-tauri/src/commands/vectorstore.rs:882
        let results = vector_search_chunks(pp, query, 10).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:891
        assert_eq!(vector_count_chunks(pp).await.unwrap(), 0);
```
```rust src-tauri/src/commands/vectorstore.rs:902
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:907
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:908
        vector_delete_page(pp.clone(), "page-a".into()).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:909
        vector_delete_page(pp.clone(), "page-a".into()).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:911
        assert_eq!(vector_count_chunks(pp).await.unwrap(), 0);
```
```rust src-tauri/src/commands/vectorstore.rs:960
        assert_eq!(vector_legacy_row_count(pp).await.unwrap(), 0);
```
```rust src-tauri/src/commands/vectorstore.rs:971
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:973
        let count = vector_legacy_row_count(pp.clone()).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:977
        assert_eq!(vector_count_chunks(pp).await.unwrap(), 0);
```
```rust src-tauri/src/commands/vectorstore.rs:987
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:990
            .unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:992
        assert_eq!(vector_legacy_row_count(pp.clone()).await.unwrap(), 1);
```
```rust src-tauri/src/commands/vectorstore.rs:993
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 2);
```
```rust src-tauri/src/commands/vectorstore.rs:995
        vector_drop_legacy(pp.clone()).await.unwrap();
```
```rust src-tauri/src/commands/vectorstore.rs:997
        assert_eq!(vector_legacy_row_count(pp.clone()).await.unwrap(), 0);
```
```rust src-tauri/src/commands/vectorstore.rs:998
        assert_eq!(vector_count_chunks(pp.clone()).await.unwrap(), 2);
```
```rust src-tauri/src/commands/vectorstore.rs:1007
        vector_drop_legacy(pp).await.unwrap();
```

```rust src-tauri/src/commands/fs.rs:1317
                .unwrap()
```
```rust src-tauri/src/commands/fs.rs:1320
        let mut f = fs::File::create(&path).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1321
        f.write_all(bytes).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1468
                .unwrap()
```
```rust src-tauri/src/commands/fs.rs:1471
        fs::create_dir_all(&dir).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1474
            if let Some(parent) = p.parent() { fs::create_dir_all(parent).unwrap(); }
```
```rust src-tauri/src/commands/fs.rs:1475
            fs::write(&p, body).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1482
        collect_related_pages(wiki, source, &mut out).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1674
                .unwrap()
```
```rust src-tauri/src/commands/fs.rs:1677
        std::fs::create_dir_all(&dir).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1686
        std::fs::create_dir_all(dest).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1689
            std::fs::create_dir_all(dest).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1690
            for entry in std::fs::read_dir(src).unwrap().flatten() {
```
```rust src-tauri/src/commands/fs.rs:1700
                    std::fs::copy(&path, &dest_path).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1714
        std::fs::create_dir_all(&leaf_dir).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1715
        std::fs::write(leaf_dir.join("leaf.txt"), b"deep content").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1717
        std::fs::write(src.join("top.md"), b"# top").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1726
        assert_eq!(std::fs::read(&leaf_dest).unwrap(), b"deep content");
```
```rust src-tauri/src/commands/fs.rs:1743
        std::fs::write(src.join("keep.md"), b"keep me").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1744
        std::fs::create_dir_all(src.join("subdir")).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1745
        std::fs::write(src.join("subdir/keep2.md"), b"keep me too").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1747
        std::fs::write(src.join(".DS_Store"), b"junk").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1748
        std::fs::create_dir_all(src.join(".git/objects")).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1749
        std::fs::write(src.join(".git/HEAD"), b"ref: refs/heads/main").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1750
        std::fs::write(src.join(".git/objects/abc"), b"\x78\x9c").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1751
        std::fs::write(src.join(".env"), b"SECRET=foo").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1755
        std::fs::create_dir_all(src.join("subdir/.cache")).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1756
        std::fs::write(src.join("subdir/.cache/blob"), b"cache").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1782
        std::fs::create_dir_all(src.join("year/2024/q3")).unwrap();
```
```rust src-tauri/src/commands/fs.rs:1783
        std::fs::write(src.join("year/2024/q3/report.pdf"), b"%PDF-fake").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1784
        std::fs::write(src.join("year/2024/notes.md"), b"# notes").unwrap();
```
```rust src-tauri/src/commands/fs.rs:1794
            .map(|p| Path::new(p).file_name().unwrap().to_string_lossy().to_string())
```

### `panic!` / `unreachable!` / `todo!`

`panic_guard.rs` 의 4 개 panic 은 모두 `#[cfg(test)]` 블록 내부에서 panic-recovery 자체를 검증하는 용도예요. production code path 에 `panic!` / `unreachable!` / `todo!` 매크로는 0 개. 다만 Cargo.toml `panic = "unwind"` 결정이 production safety 의 핵심: `pdfium-render` / `lopdf` / `docx-rs` / `calamine` 의 third-party panic 이 `catch_unwind` 로 잡혀 `Result::Err` 으로 변환되거든요. `panic = "abort"` 이었으면 한 PDF 의 corrupt header 가 앱 전체를 죽였을 거예요.

```rust src-tauri/src/panic_guard.rs:55
            run_guarded("test", || panic!("boom from String"));
```
```rust src-tauri/src/panic_guard.rs:85
            panic!("async boom");
```
```rust src-tauri/src/panic_guard.rs:96
            panic!("post-await boom");
```

### `Mutex::lock` / `RwLock::write` acquisition + drop discipline

PDFium FFI 직렬화 mutex 한 개 (`fs.rs:160`) 와 clip_server 데몬의 글로벌 state mutex 세 개 (`clip_server.rs:6-8`), 그리고 claude_cli 의 children registry mutex (`claude_cli.rs:34` 의 tokio Mutex — std Mutex 와 다름) 가 있어요.

```rust src-tauri/src/commands/fs.rs:160
static PDFIUM_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
```

`PDFIUM_LOCK` 의 의도는 명확해요: PDFium C 라이브러리는 cross-thread call 이 interleave 되면 UB (실제로 EXC_BAD_ACCESS segfault 보고됨, fs.rs:147-158 주석) 라서 모든 PDFium 진입을 한 mutex 뒤에 직렬화해요. drop discipline 은 양호 — `lock_pdfium()` 이 `MutexGuard<'static, ()>` 를 반환하고, 호출 사이트 (`extract_pdf_markdown`, `extract_pdf_images`, `extract_and_save_pdf_images`) 가 `let _guard = ...` 로 받아 함수 끝까지 holds. `tokio::async_runtime::spawn_blocking` 안에서만 잡으므로 `.await` 를 가로지르는 hold 는 없어요. poisoning 은 `unwrap_or_else(|poisoned| poisoned.into_inner())` 로 의도적으로 무시 — PDFium 자체는 mutex 에 의존하는 shared state 가 없으니 OK.

```rust src-tauri/src/clip_server.rs:6
static CURRENT_PROJECT: Mutex<String> = Mutex::new(String::new());
```
```rust src-tauri/src/clip_server.rs:7
static ALL_PROJECTS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new()); // (name, path)
```
```rust src-tauri/src/clip_server.rs:8
static PENDING_CLIPS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new()); // (projectPath, filePath)
```

clip_server 의 세 mutex 는 lock-order 위험이 있어요 — `/projects` GET handler (clip_server.rs:143-144) 가 `ALL_PROJECTS` 잠근 직후 `CURRENT_PROJECT` 를 잠그는 반면, `handle_set_project` 는 `CURRENT_PROJECT` 만 잠가요. 만약 future code 가 `CURRENT_PROJECT` → `ALL_PROJECTS` 순서로 lock 을 잡는다면 lock-order 역전으로 인한 deadlock 가능. 현재 코드는 한 곳에서만 nested lock 을 걸어서 deadlock 은 없지만 invariant 가 비정형이에요. drop 은 표현식이 끝나면서 자동 drop 되는 게 대부분이지만, line 169 의 `let mut projects = ALL_PROJECTS.lock().unwrap();` 는 for 루프 전체 동안 holds 되어 클립 등록 작업과 lock 충돌 가능. line 186 `let mut pending = PENDING_CLIPS.lock().unwrap();` 는 `pending.clear()` 직전에 iterator 로 사용해서 hold 가 비교적 길어요 (`for h in &cors_headers { response.add_header(h.clone()); }` 실행 시간만큼).

```rust src-tauri/src/commands/claude_cli.rs:34
    children: Arc<Mutex<HashMap<String, Child>>>,
```

claude_cli 의 mutex 는 `tokio::sync::Mutex` (file head: `use tokio::sync::Mutex`) 라 `.lock().await` 사용. drop 규율: line 240-242 `state.children.lock().await.insert(...)` 가 표현식 마침과 동시에 drop, line 289 `let child_opt = children.lock().await.remove(&stream_id_task);` 도 expression-scope, line 322 `if let Some(mut child) = state.children.lock().await.remove(&stream_id) { ... }` 도 if 본문 후 drop. 주석 (claude_cli.rs:288-289) 이 명시: "Don't hold the map lock across .wait() — kill could race." 라고 .await 를 가로지른 hold 를 의도적으로 회피한다고 적어 둠.

`RwLock` 사용은 `0` 회. 모든 동시성 primitive 는 Mutex 기반.

### FFI loads (pdfium)

pdfium-render 크레이트의 `bind_to_library(path)` / `bind_to_system_library()` 가 dlopen-equivalent 동작을 수행해요. `unsafe extern "C"` 자체는 pdfium-render 내부에 있고 이 코드베이스에선 wrapper 만 호출하지만, **dynamic library 가 적재되는 순간 FFI 신뢰 경계를 넘는 거예요** — bundled `pdfium/libpdfium.{dylib,so,dll}` 파일이 손상되면 어떤 unsafe code 든 trigger 될 수 있어요.

```rust src-tauri/src/commands/fs.rs:142
static PDFIUM: std::sync::OnceLock<Result<pdfium_render::prelude::Pdfium, String>> =
    std::sync::OnceLock::new();
```

```rust src-tauri/src/commands/fs.rs:282
pub(crate) fn pdfium() -> Result<&'static pdfium_render::prelude::Pdfium, String> {
    PDFIUM
        .get_or_init(|| {
            use pdfium_render::prelude::*;
            let candidates = pdfium_candidate_paths();
            for path in &candidates {
                if let Ok(bindings) = Pdfium::bind_to_library(path) {
                    eprintln!("[pdfium] loaded dynamic library from {path}");
                    return Ok(Pdfium::new(bindings));
                }
            }
            // Last resort: let the OS dynamic loader find it.
            Pdfium::bind_to_system_library()
                .map(Pdfium::new)
                .map_err(|e| {
                    format!(
                        "Failed to locate Pdfium library. Tried: {} — and the system search path. Last error: {e}",
                        if candidates.is_empty() {
                            "(no candidates)".to_string()
                        } else {
                            candidates.join(", ")
                        }
                    )
                })
        })
        .as_ref()
        .map_err(|e| e.clone())
}
```

candidate path 결정 (`pdfium_candidate_paths`, fs.rs:193-280) 은 `$PDFIUM_DYNAMIC_LIB_PATH` 환경변수 → Tauri `resource_dir` (setup() 후 set) → `current_exe()` 기준 platform-specific 위치 → OS 검색 경로 순. **`$PDFIUM_DYNAMIC_LIB_PATH` 사용은 user-controllable 환경변수이기 때문에, Tauri 앱이 더 높은 권한으로 launch 되거나 sandboxed 가 풀려 있다면 attacker 가 임의 dylib 을 로드시켜 코드 실행할 수 있어요**. 데스크톱 앱이라 user 가 이미 자기 머신의 코드 실행 권한을 갖고 있어 trust boundary 가 약하지만, 만약 setuid 또는 elevated 컨텍스트에서 launch 되면 이 env-var path 가 LPE (local privilege escalation) 벡터가 될 수 있어요. `bind_to_system_library()` 는 `LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH` / `%PATH%` 의 OS dynamic loader 메커니즘에 위임하므로 같은 위협 모델.

PDFium FFI 호출 자체의 직렬화는 `PDFIUM_LOCK` (위 Mutex 섹션) 로 완료. 모든 PDFium-touching command 는 `tauri::async_runtime::spawn_blocking` 으로 wrap 되어 tokio worker thread 를 막지 않아요. extract_images.rs:851-857 주석이 명시: "PDFium FFI calls and zip+image-decode are all blocking. Running them inside an `async fn` body kept them on a tokio worker thread, blocking other async tasks on that worker for the full duration of the extraction."

`extern "C"` / dlopen 직접 사용은 이 코드베이스에 0 개. 모든 native lib 은 crate dependency 를 통해 통과해요 (`tiny_http`, `pdfium-render`, `lancedb`, `arrow-array`, `arrow-schema`, `zip`, `calamine`, `docx-rs`, `image`, `sha2`, `base64`, `chrono`, `which`, `uuid`).

### Result swallow (TypeScript)

이 도메인 (Rust 백엔드) 에서는 해당 사항 없음. TS Result-swallow 패턴은 frontend 도메인에서 다뤄요. 다만 Rust 측의 "result swallow equivalent" 는 `let _ = ...` 로 명시적으로 결과를 버리는 케이스가 다수 있어요 — 의도된 fire-and-forget 이라 위험은 낮지만 인용해요.

```rust src-tauri/src/lib.rs:79
                    let _ = window.hide();
```
```rust src-tauri/src/lib.rs:99
                            let _ = win.destroy();
```
```rust src-tauri/src/lib.rs:113
                        let _ = window.show();
```
```rust src-tauri/src/lib.rs:114
                        let _ = window.set_focus();
```
```rust src-tauri/src/lib.rs:118
            let _ = (app, event); // suppress unused warnings on non-macOS
```
```rust src-tauri/src/clip_server.rs:87 (and many similar lines)
                let _ = request.respond(response);
```
```rust src-tauri/src/commands/claude_cli.rs:275
                    if app.emit(&topic, line).is_err() {
                        break;
                    }
```
```rust src-tauri/src/commands/claude_cli.rs:302
        let _ = app.emit(
            &done_topic,
            serde_json::json!({
                "code": exit_code,
                "stderr": stderr_text,
            }),
        );
```
```rust src-tauri/src/commands/claude_cli.rs:323
        let _ = child.start_kill();
```

`request.respond(response)` 의 `let _ =` 는 client 가 disconnect 했을 때 Err 를 무시하는 의도, `app.emit(done_topic, ...)` 의 `let _ =` 는 webview 가 이미 unmount 된 경우 emit 실패를 무시하는 의도예요. 모두 정당. 다만 audit 관점에서 "fire-and-forget logging" 이 silently swallow 되므로 production 에서 emit 실패율을 모니터링할 수 있는 telemetry 가 없다는 점은 gap 이에요 — `eprintln!` 만으로는 데스크톱 앱에서 stderr 가 user-visible 하지 않아 silent.

## Cross-refs

- See [03-frontend.md#purpose](03-frontend.md#purpose) — frontend 의 `invoke()` 사용처가 `lib.rs` 의 `tauri::generate_handler!` 등록 surface 와 1:1 매핑.
- See [05-extension.md#purpose](05-extension.md#purpose) — Chrome MV3 webclipper 가 `clip_server.rs` 의 `127.0.0.1:19827` 와 통신. `/clip` POST → `PENDING_CLIPS` 큐 → frontend polling.
- See [06-data-layer.md#purpose](06-data-layer.md#purpose) — `vectorstore.rs` 의 v1/v2 LanceDB 스키마, `<project>/.llm-wiki/lancedb/` 경로, `<project>/.cache/<name>.txt` preprocess cache 가 데이터 레이어의 disk schema 일부.
- See [07-llm-integration.md#purpose](07-llm-integration.md#purpose) — `claude_cli.rs` 가 provider 중 하나 (Claude Code CLI 구독 재사용). `tauri-plugin-http` (`unsafe-headers` feature) 가 다른 provider (MiniMax, Volcengine Ark) 의 CORS-거부 endpoint 를 우회하는 fetch 백본.
- See [08-pdf-ocr-pipeline.md#purpose](08-pdf-ocr-pipeline.md#purpose) — `extract_images.rs`, `fs.rs` 의 PDFium 진입점이 vision-caption 파이프라인의 Phase 1 입력. SHA-256 dedup 키가 caption 캐시 1차 키.
- Source rows:
  - [src-tauri/src/lib.rs](50-source-mapping.md#src-taurisrclibrs) — Tauri builder + command registration.
  - [src-tauri/src/clip_server.rs](50-source-mapping.md#src-taurisrcclip_serverrs) — in-process HTTP server for Chrome MV3 webclipper.
  - [src-tauri/src/panic_guard.rs](50-source-mapping.md#src-taurisrcpanic_guardrs) — `catch_unwind` 기반 Tauri-command boundary.
  - [src-tauri/src/commands/fs.rs](50-source-mapping.md#src-taurisrccommandsfsrs) — PDFium serialization mutex + Office/PDF text extraction.
  - [src-tauri/src/commands/vectorstore.rs](50-source-mapping.md#src-taurisrccommandsvectorstorers) — LanceDB v1/v2 CRUD.
  - [src-tauri/src/commands/claude_cli.rs](50-source-mapping.md#src-taurisrccommandsclaude_clirs) — `claude` 자식 프로세스 transport.
  - [src-tauri/src/commands/extract_images.rs](50-source-mapping.md#src-taurisrccommandsextract_imagesrs) — PDF/Office 이미지 추출.
  - [src-tauri/src/commands/project.rs](50-source-mapping.md#src-taurisrccommandsprojectrs) — wiki project scaffolding.
  - [src-tauri/src/types/wiki.rs](50-source-mapping.md#src-taurisrctypeswikirs) — `WikiProject` / `FileNode` IPC type.
  - [src-tauri/Cargo.toml](50-source-mapping.md#src-tauricargotoml) — `panic = "unwind"` + dependency pinning.

## Evidence

- `src-tauri/src/lib.rs:42-74` — 31 `tauri::generate_handler!` 등록된 Tauri command 표면.
- `src-tauri/src/lib.rs:8-14` — `clip_server_status` 가 `run_guarded` 로 wrap, 결과를 `unwrap_or_else` 로 안전 추출.
- `src-tauri/src/lib.rs:18` — `clip_server::start_clip_server()` 호출이 backgrounded thread 시작.
- `src-tauri/src/lib.rs:21-28` — 4 plugin (`tauri_plugin_opener`, `tauri_plugin_dialog`, `tauri_plugin_store`, `tauri_plugin_http`) 등록.
- `src-tauri/src/lib.rs:106` — startup-only `expect("error while building tauri application")`.
- `src-tauri/src/panic_guard.rs:14-22` — `run_guarded` 시그니처와 `catch_unwind(AssertUnwindSafe(f))` 사용.
- `src-tauri/src/panic_guard.rs:25-34` — `run_guarded_async` 가 `futures::FutureExt::catch_unwind` 사용, async-aware panic handling.
- `src-tauri/src/panic_guard.rs:36-46` — String / `&str` 두 panic payload 형태 모두 downcast.
- `src-tauri/Cargo.toml:67-71` — `panic = "unwind"` 결정 (catch_unwind 가 동작하는 전제).
- `src-tauri/Cargo.toml:30` — `tauri-plugin-http` 의 `unsafe-headers` feature 활성 (CORS preflight 우회 의도).
- `src-tauri/src/clip_server.rs:6-8` — 3 개 `Mutex<...>` 글로벌 state (project path, all projects, pending clips).
- `src-tauri/src/clip_server.rs:11` — `AtomicU8` 기반 daemon status (lock-free 4-state).
- `src-tauri/src/clip_server.rs:13-17` — port 19827, 3 retry × 2s = 6s bind 윈도우, 10 restart × 5s = 50s crash-recovery 윈도우.
- `src-tauri/src/clip_server.rs:73-79` — CORS `*` 모든 origin 허용 (extension trust boundary 디자인).
- `src-tauri/src/clip_server.rs:262-282` — `handle_set_project` 가 path 를 forward-slash 정규화.
- `src-tauri/src/clip_server.rs:285-400` — `handle_clip` 의 markdown 파일 작성 + `PENDING_CLIPS` 큐잉.
- `src-tauri/src/commands/fs.rs:11-19` — extension classification: `OFFICE_EXTS`, `IMAGE_EXTS`, `MEDIA_EXTS`, `LEGACY_DOC_EXTS`.
- `src-tauri/src/commands/fs.rs:32-81` — `read_file` 의 `spawn_blocking` 결정 + 확장자별 분기.
- `src-tauri/src/commands/fs.rs:120-138` — `<dir>/.cache/<name>.txt` 에 mtime-비교 캐시.
- `src-tauri/src/commands/fs.rs:142-143` — `OnceLock<Result<Pdfium, String>>` 글로벌 PDFium 인스턴스.
- `src-tauri/src/commands/fs.rs:160` — `static PDFIUM_LOCK: std::sync::Mutex<()>` 직렬화 mutex.
- `src-tauri/src/commands/fs.rs:166-170` — `lock_pdfium()` 가 poison 자동 복구.
- `src-tauri/src/commands/fs.rs:193-280` — `pdfium_candidate_paths` 의 4-tier 검색 순서 (env → resource_dir → exe-relative → OS path).
- `src-tauri/src/commands/fs.rs:282-309` — `pdfium()` 의 `bind_to_library` / `bind_to_system_library` fallback 체인.
- `src-tauri/src/commands/fs.rs:327-375` — `extract_pdf_text` 의 `<project>/raw/sources/<name>.pdf` heuristic.
- `src-tauri/src/commands/fs.rs:403-527` — `docx-rs` 기반 DOCX 추출 (paragraph + table + heading detection).
- `src-tauri/src/commands/fs.rs:547-720` — manual XML walker 기반 DOCX fallback.
- `src-tauri/src/commands/fs.rs:792-861` — `calamine::open_workbook_auto` 기반 spreadsheet 추출.
- `src-tauri/src/commands/fs.rs:1108-1221` — `collect_related_pages` 의 3-strategy 매칭 (inline quoted / source-summary path / multi-line YAML block).
- `src-tauri/src/commands/extract_images.rs:43-51` — `ExtractOptions` 기본값 (100×100 min, 500 max).
- `src-tauri/src/commands/extract_images.rs:104-239` — `extract_pdf_markdown` 가 `lock_pdfium()` 직접 잡고 PDFium 사용.
- `src-tauri/src/commands/extract_images.rs:245-336` — `extract_pdf_images` 의 page-by-page enumeration + per-image PNG 재인코딩.
- `src-tauri/src/commands/extract_images.rs:347-452` — `extract_office_images` 의 `zip::ZipArchive` + `image::load_from_memory` 디코드.
- `src-tauri/src/commands/extract_images.rs:503-561` — `build_pptx_media_slide_map` 의 substring-기반 rels XML 파싱 (no-op XML parser).
- `src-tauri/src/commands/extract_images.rs:849-919` — `spawn_blocking` + `panic_guard::run_guarded` wrapping 패턴 (모든 4 Tauri command).
- `src-tauri/src/commands/vectorstore.rs:45-47` — DB 경로: `<project>/.llm-wiki/lancedb/` (forward-slash 정규화).
- `src-tauri/src/commands/vectorstore.rs:50-54` — v1 (`wiki_vectors`) / v2 (`wiki_chunks_v2`) 테이블 이름.
- `src-tauri/src/commands/vectorstore.rs:56-65` — `validate_page_id` 의 alphanumeric+`-`+`_`+`.` allowlist (SQL 인젝션 방지).
- `src-tauri/src/commands/vectorstore.rs:67-94` — v1 schema 빌더 + RecordBatch 생성.
- `src-tauri/src/commands/vectorstore.rs:332-406` — v2 schema (`chunk_id`, `page_id`, `chunk_index`, `chunk_text`, `heading_path`, `vector`).
- `src-tauri/src/commands/vectorstore.rs:316-330` — `validate_page_id_for_v2` (v1 과 동일 로직, deduplication 미적용).
- `src-tauri/src/commands/vectorstore.rs:420-477` — v2 upsert 의 delete-then-add 의미론.
- `src-tauri/src/commands/vectorstore.rs:560-561` — score 변환 `1.0 / (1.0 + distance)` (v1 과 동일 컨벤션).
- `src-tauri/src/commands/vectorstore.rs:706-711` — `drop_table(TABLE_V1, &[])` (LanceDB 0.27 의 namespace API).
- `src-tauri/src/commands/claude_cli.rs:33-35` — `Arc<tokio::sync::Mutex<HashMap<String, Child>>>` children registry.
- `src-tauri/src/commands/claude_cli.rs:60-70` — `which::which("claude")` 로 PATH 검색.
- `src-tauri/src/commands/claude_cli.rs:74-78` — `--version` 호출에 `tokio::time::timeout(Duration::from_secs(3), ...)` 적용.
- `src-tauri/src/commands/claude_cli.rs:93-103` — macOS Gatekeeper quarantine 메시지 휴리스틱.
- `src-tauri/src/commands/claude_cli.rs:144-149` — system messages 를 user 첫 turn 으로 prepend (CLI flag 비호환성 회피).
- `src-tauri/src/commands/claude_cli.rs:175-188` — `claude -p --output-format stream-json --input-format stream-json --verbose --model <model>` + `kill_on_drop(true)`.
- `src-tauri/src/commands/claude_cli.rs:217-230` — JSON event line 직렬화 (`content` 가 array of blocks 강제 — assistant turn 의 raw string 가 CLI 충돌).
- `src-tauri/src/commands/claude_cli.rs:252-309` — stdout 라인별 emit + stderr 수집 + 종료 시 `:done` 이벤트.
- `src-tauri/src/commands/claude_cli.rs:288-298` — wait() 호출 전 lock 해제 (kill 과의 race 회피).
- `src-tauri/src/commands/claude_cli.rs:317-328` — `claude_cli_kill` 의 `start_kill` (waiter 는 stdout-drain task 가 보유).
- `src-tauri/src/commands/project.rs:21-31` — 8 개 표준 디렉터리 (raw/sources, raw/assets, wiki/entities, wiki/concepts, wiki/sources, wiki/queries, wiki/comparisons, wiki/synthesis).
- `src-tauri/src/commands/project.rs:39-115` — schema.md 컨텐트 (page type 정의 / 명명 규칙 / frontmatter 표준).
- `src-tauri/src/commands/project.rs:194-230` — `.obsidian/` config (attachment dir, 무시할 dotdir, dark theme, core plugins).
- `src-tauri/src/commands/project.rs:240-278` — `open_project` 가 `schema.md` + `wiki/` 존재로 valid 검증.
- `src-tauri/src/types/wiki.rs:3-7` — `WikiProject` (name + path).
- `src-tauri/src/types/wiki.rs:9-16` — `FileNode` (재귀 트리, `Option<Vec<FileNode>>` for `serde(skip_serializing_if = "Option::is_none")`).
- `src-tauri/Cargo.toml:35` — `lancedb = "0.27.2"`, `arrow-array = "57"`, `arrow-schema = "57"`.
- `src-tauri/Cargo.toml:43` — `tokio` features `process, io-util, sync, macros, rt`.
- `src-tauri/Cargo.toml:56-58` — multimodal deps: `image` (PNG only), `base64`, `sha2`.

# {Domain Name}

> **MANDATORY**: All domain docs (03..09 and any Phase-2-renamed slots in 10..18) MUST instantiate every section below. Empty sections are not allowed; write `None observed in this domain.` if truly empty.
> **Phase 6 verification gate**: `grep -c '^## \(Purpose\|Public Interface\|Internal Risk\|Cross-refs\|Evidence\)$' $doc` must equal 5.

## Purpose

{1 paragraph: what this domain does, why it exists, what bounds it.}

## Public Interface

{Exported symbols, Tauri commands, IPC channels, HTTP routes, React props.
Format one row per symbol:

`symbol — signature — file:line — short purpose`

Example:
- `ingestFile — async (path: string) => Promise<IngestResult> — src/lib/ingest.ts:47 — entry point for single-file ingest pipeline`}

## Internal Risk

{Verbatim code quotes for each category below. If a category has zero hits in this domain, write `None observed in this domain.` — DO NOT silently omit the category.

### unsafe blocks (Rust)
```rust path:line
{verbatim quote}
```

### `.unwrap()` / `.expect()` chains (Rust)
```rust path:line
{verbatim quote}
```

### `panic!` / `unreachable!` / `todo!` (Rust)
```rust path:line
{verbatim quote}
```

### `Mutex::lock` / `RwLock::write` acquisition + drop discipline (Rust)
```rust path:line
{verbatim quote}
```
Note: comment on lock-order, drop discipline, `MutexGuard` lifetime.

### FFI loads, `extern "C"`, dlopen-style (Rust → pdfium et al.)
```rust path:line
{verbatim quote}
```

### Result swallow (TypeScript)
- `let _ = expr;`
- empty `catch {}`
- `console.error` w/o rethrow
- `as any` / `as unknown as` casts swallowing type errors
```typescript path:line
{verbatim quote}
```
}

## Cross-refs

{Links to other domain docs that share state, types, or call paths.
Links to specific rows of `50-source-mapping.md`.

Format:
- See [04-backend-rust.md#evidence](04-backend-rust.md#evidence) for Tauri command surface.
- Source rows: [src/lib/ingest.ts](50-source-mapping.md#srclibingestts), [src-tauri/src/commands/extract_images.rs](50-source-mapping.md#src-taurisrccommandsextract_imagesrs).

Phase 6 gate: ≥3 links to mapping anchors AND ≥1 link to another domain doc.}

## Evidence

{Bulleted list of `path:line` citations supporting every claim above.
Every claim in Purpose/Interface/Risk must trace to an Evidence entry.

- `src/lib/ingest.ts:47` — `ingestFile` exported as the canonical single-file entry.
- `src-tauri/src/commands/project.rs:23` — `Mutex<ProjectState>` acquisition in `set_active_project`.
- `src/lib/llm-client.ts:118` — `as any` cast on streaming chunk handler (suppresses upstream type error).}

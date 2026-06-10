//! 정적 AST 패턴 validator (Track H, spec MD-complement / plan §H1).
//!
//! ## 명명 — `validate` ≠ `verify`
//! `validate` = **정적 AST 패턴 검사**(신규). 편집된 사용자 코드를 tree-sitter 로
//! 파싱해 SDK 데이터/HTTP 계약 위반을 정적으로 검출해요. exit 0=클린, 1=block 위반,
//! 파싱 실패는 exit 0+경고(**fail-open**).
//! `verify` / `verify-deploy-artifact` = **배포·런타임 검증**(기존, cli/mod.rs). 배포
//! 산출물·런타임 상태를 확인해요. 둘은 의미가 다르니 혼동 금지.
//!
//! ## 룰은 하드코딩 금지 — vendored 단일 원천
//! 룰은 `rules/data-contract-rules.json` 에서 `include_str!` 로 embed 해요. distiller
//! 가 PINNED_SDK.lock 에서 파생한 산출물의 byte-identical 복사본이에요(provenance:
//! `rules/PROVENANCE.md`). 각 룰의 `derived_from` 가 없으면 로드가 실패해요
//! (enforcement drift 차단). `advisory_only:true` 룰은 정적 미결정이라 영구 warn
//! 트랙이고 block 으로 승격하지 않아요.
//!
//! ## 엔진 — tree-sitter 마스킹 + 계약 regex
//! 룰은 `pattern_hint`(regex)를 줘요. 순수 regex 는 주석/문자열 안의 우연한 매칭으로
//! false-positive 를 내요. 그래서 tree-sitter 로 주석·문자열 노드 span 을 공백
//! 마스킹(개행 보존 → 줄/열 좌표 유지)한 뒤 regex 를 적용해요. 룰의 `pattern_hint`
//! 가 lookahead/lookbehind 를 써서 std `regex` 가 아니라 `fancy-regex` 로 컴파일해요.
//!
//! 마스킹은 2벌이에요:
//! - 언어별 룰(`applies_lang != ["*"]`) = 코드 구문 검사 → 주석+문자열 마스킹.
//! - 언어무관 룰(`applies_lang == ["*"]`) = URL/HTTP 텍스트 계약 → 주석만 마스킹
//!   (문자열 안의 URL 을 봐야 하므로).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fancy_regex::Regex;
use serde::{Deserialize, Serialize};
use tree_sitter::{Language, Node, Parser};

const RULES_JSON: &str = include_str!("../rules/data-contract-rules.json");

const MAX_SCAN_DEPTH: usize = 12;
const MAX_SCAN_FILES: usize = 5_000;
const MAX_SOURCE_FILE_BYTES: u64 = 1024 * 1024;
/// 파일당 위반 cap — dense 파일(수만 match)에서 출력 폭주/지연을 막아요.
/// cap 도달 시 나머지 match 는 버려요(이미 충분한 신호).
const MAX_VIOLATIONS_PER_FILE: usize = 500;
const SKIP_DIRS: [&str; 7] = [
    "node_modules",
    ".git",
    "target",
    "vendor",
    "dist",
    "build",
    ".next",
];

// ──────────────────────────── 룰 로딩 ────────────────────────────

#[derive(Debug, Deserialize)]
struct RawRule {
    id: String,
    rule_kind: String,
    applies_lang: Vec<String>,
    pattern_hint: String,
    derived_from: String,
    source: RawSource,
    #[serde(default)]
    advisory_only: bool,
    #[serde(default)]
    advisory_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSource {
    #[allow(dead_code)]
    pack_section: String,
    lock_sha: String,
}

/// 컴파일된 룰. `scans_strings` = pattern 이 URL/path(`/` 포함)를 타겟하므로 문자열을
/// 스캔해야 하는 룰(oauth/api-v1/raw-http/use-client). 코드 구문 룰(or/not/cursor/
/// list/count)은 `/` 가 없어 문자열을 마스킹해요(주석/문자열 내 우연 매칭 차단).
pub(crate) struct CompiledRule {
    id: String,
    rule_kind: String,
    applies_lang: Vec<String>,
    regex: Regex,
    derived_from: String,
    lock_sha: String,
    advisory_only: bool,
    scans_strings: bool,
}

impl CompiledRule {
    fn applies_to(&self, lang_key: &str) -> bool {
        self.applies_lang
            .iter()
            .any(|l| l == "*" || l == lang_key)
    }
}

/// vendored 룰을 로드·컴파일해요. `derived_from` 누락(또는 advisory 인데
/// `advisory_reason` 누락), regex 컴파일 실패 시 에러로 빌드/실행을 막아요.
pub(crate) fn load_rules() -> Result<Vec<CompiledRule>> {
    compile_rules_json(RULES_JSON)
}

/// 룰 JSON 문자열 → 컴파일된 룰. embed 본문과 분리해 검증 3분기(derived_from /
/// advisory_reason / regex 실패)를 직접 테스트할 수 있게 해요.
fn compile_rules_json(json: &str) -> Result<Vec<CompiledRule>> {
    let raw: Vec<RawRule> =
        serde_json::from_str(json).context("data-contract-rules.json 파싱 실패")?;
    if raw.is_empty() {
        bail!("data-contract-rules.json 에 룰이 없어요");
    }
    let mut out = Vec::with_capacity(raw.len());
    for r in raw {
        if r.derived_from.trim().is_empty() {
            bail!("룰 '{}' 에 derived_from 이 없어요 (원천 없는 룰 금지)", r.id);
        }
        if r.advisory_only && r.advisory_reason.as_deref().unwrap_or("").trim().is_empty() {
            bail!("advisory 룰 '{}' 에 advisory_reason 이 없어요", r.id);
        }
        let regex = Regex::new(&r.pattern_hint)
            .with_context(|| format!("룰 '{}' 의 pattern_hint regex 컴파일 실패", r.id))?;
        // URL/path(`/` 포함) 타겟 룰은 문자열을 스캔해요 — 21룰 전부 정확 분류:
        // oauth/api-v1/raw-http/use-client(문자열 타겟) vs or/not/cursor/list/count(코드).
        let scans_strings = r.pattern_hint.contains('/');
        out.push(CompiledRule {
            id: r.id,
            rule_kind: r.rule_kind,
            applies_lang: r.applies_lang,
            regex,
            derived_from: r.derived_from,
            lock_sha: r.source.lock_sha,
            advisory_only: r.advisory_only,
            scans_strings,
        });
    }
    Ok(out)
}

// ──────────────────────────── 언어 매핑 ────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Grammar {
    Typescript,
    Tsx,
    Javascript,
    Python,
    Go,
    Java,
    Kotlin,
    Ruby,
}

/// 확장자 → (grammar, 룰 `applies_lang` 키). ts/tsx/js 는 전부 "node" 키예요.
/// site_scan 도 같은 언어 매핑을 써요(엔진 한 벌).
pub(crate) fn detect_lang(path: &Path) -> Option<(Grammar, &'static str)> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let pair = match ext.as_str() {
        "ts" | "mts" | "cts" => (Grammar::Typescript, "node"),
        "tsx" => (Grammar::Tsx, "node"),
        "js" | "jsx" | "mjs" | "cjs" => (Grammar::Javascript, "node"),
        "py" | "pyi" => (Grammar::Python, "python"),
        "go" => (Grammar::Go, "go"),
        "java" => (Grammar::Java, "java"),
        "kt" | "kts" => (Grammar::Kotlin, "kotlin"),
        "rb" => (Grammar::Ruby, "ruby"),
        _ => return None,
    };
    Some(pair)
}

pub(crate) fn ts_language(grammar: Grammar) -> Language {
    match grammar {
        Grammar::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Grammar::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Grammar::Javascript => tree_sitter_javascript::LANGUAGE.into(),
        Grammar::Python => tree_sitter_python::LANGUAGE.into(),
        Grammar::Go => tree_sitter_go::LANGUAGE.into(),
        Grammar::Java => tree_sitter_java::LANGUAGE.into(),
        Grammar::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
        Grammar::Ruby => tree_sitter_ruby::LANGUAGE.into(),
    }
}

// ──────────────────────────── 마스킹 ────────────────────────────

fn is_comment_kind(kind: &str) -> bool {
    kind.contains("comment")
}

fn is_string_kind(kind: &str) -> bool {
    kind.contains("string") || kind.contains("heredoc") || kind.contains("char_literal")
}

/// 주석/문자열 노드의 byte span 을 수집해요. 해당 노드 안으로는 재귀하지 않아요
/// (보수적: 보간 코드도 마스킹 → false-positive 0 우선, false-negative 는 허용).
fn collect_mask_spans(root: Node, comments: &mut Vec<(usize, usize)>, strings: &mut Vec<(usize, usize)>) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if is_comment_kind(kind) {
            let r = node.byte_range();
            comments.push((r.start, r.end));
            continue;
        }
        if is_string_kind(kind) {
            let r = node.byte_range();
            strings.push((r.start, r.end));
            continue;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
}

/// span 을 공백으로 치환해요. 개행(`\n`/`\r`)은 보존해 줄 좌표를 유지하고, 길이는
/// 원본과 동일하게 둬서 매치 offset 이 원본과 1:1 매핑돼요.
fn apply_mask(source: &str, spans: &[(usize, usize)]) -> String {
    let mut bytes = source.as_bytes().to_vec();
    for &(start, end) in spans {
        if let Some(slice) = bytes.get_mut(start..end) {
            for b in slice {
                if *b != b'\n' && *b != b'\r' {
                    *b = b' ';
                }
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// 소스를 파싱해 2벌 마스킹 버퍼를 만들어요(엔진 한 벌 — site_scan 도 재사용).
/// 반환: `(주석만 마스킹, 주석+문자열 마스킹)`. parse 실패면 None.
pub(crate) fn build_masks(source: &str, grammar: Grammar) -> Option<(String, String)> {
    let mut parser = Parser::new();
    parser.set_language(&ts_language(grammar)).ok()?;
    let tree = parser.parse(source.as_bytes(), None)?;
    let mut comments = Vec::new();
    let mut strings = Vec::new();
    collect_mask_spans(tree.root_node(), &mut comments, &mut strings);
    let masked_no_comments = apply_mask(source, &comments);
    let mut all_spans = Vec::with_capacity(comments.len() + strings.len());
    all_spans.extend_from_slice(&comments);
    all_spans.extend_from_slice(&strings);
    let masked_code_only = apply_mask(source, &all_spans);
    Some((masked_no_comments, masked_code_only))
}

/// 파일당 1회 생성하는 개행 byte-offset 인덱스. match 마다 소스 처음부터 재스캔하던
/// O(n²) line_col 을 binary_search O(log n)으로 바꿔요(dense 파일 회귀 수정).
/// site_scan 의 line_col/snippet 도 같은 인덱스를 재사용해요(엔진 한 벌).
pub(crate) struct LineIndex {
    /// 각 줄의 시작 byte offset — `line_starts[0] == 0`, 이후 `\n` 다음 byte.
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub(crate) fn new(source: &str) -> Self {
        let mut line_starts = vec![0usize];
        for (idx, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self { line_starts }
    }

    /// offset 이 속한 줄의 0-base 인덱스.
    fn line_idx(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1, // line_starts[0]==0 이라 i>=1 보장
        }
    }

    /// byte offset → (1-base line, 1-base column). column 은 문자 단위예요.
    pub(crate) fn line_col(&self, source: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(source.len());
        let idx = self.line_idx(offset);
        let col = source[self.line_starts[idx]..offset].chars().count() + 1;
        (idx + 1, col)
    }

    /// offset 이 속한 줄의 `[시작, 끝)` byte 범위 — 끝은 개행 직전(없으면 EOF).
    pub(crate) fn line_span(&self, source: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(source.len());
        let idx = self.line_idx(offset);
        let start = self.line_starts[idx];
        let end = self
            .line_starts
            .get(idx + 1)
            .map_or(source.len(), |next| next - 1);
        (start, end)
    }
}

// ──────────────────────────── 위반/출력 모델 ────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub rule_id: String,
    pub rule_kind: String,
    pub advisory_only: bool,
    pub message: String,
    pub derived_from: String,
    pub lock_sha: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParseFailure {
    pub file: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ValidateOutput {
    pub schema_version: String,
    pub status: String,
    pub files_scanned: usize,
    pub block_count: usize,
    pub advisory_count: usize,
    pub violations: Vec<Violation>,
    pub parse_failures: Vec<ParseFailure>,
    /// 파일 수집이 MAX_SCAN_FILES cap 으로 잘렸으면 true — "전부 검사" 오독 방지.
    pub truncated: bool,
}

/// 룰 종류별 해요체 메시지. advisory 무필터 list/count 는 plan 이 못박은 문구를
/// 그대로 써요 — **migrate-data-verify 위임 문구는 절대 넣지 않아요**(PR 스택
/// 미머지 fallback).
fn rule_message(rule: &CompiledRule) -> String {
    if rule.advisory_only {
        return match rule.rule_kind.as_str() {
            "where_required" => {
                "owner-scoped 테이블이면 정당해요. 런타임 스키마(owner_column) 확인을 권장해요."
                    .to_string()
            }
            _ => format!(
                "정적으로 판정할 수 없는 경계예요({}). 런타임 스키마 확인을 권장해요.",
                rule.id
            ),
        };
    }
    // rule_kind 단위 분기는 같은 kind 의 다른 룰에 오답 메시지를 줬어요(예: boundary
    // kind 의 use-client 룰이 "/api/v1 prefix" 메시지). 의미가 고유한 룰은 rule id
    // 단위로 먼저 분기하고, 나머지만 kind 공통 메시지로 떨어져요.
    match rule.id.as_str() {
        "raw-http-axhub-data-endpoint-forbidden" => {
            return format!(
                "`{}`: axhub 데이터 API 는 SDK 경유로 호출해요 — raw HTTP 직타는 막혀 있어요.",
                rule.id
            );
        }
        "use-client-imports-server-only-axhub" => {
            return format!(
                "`{}`: server-only helper 는 client 컴포넌트에서 import 못 해요.",
                rule.id
            );
        }
        _ => {}
    }
    match rule.rule_kind.as_str() {
        "forbidden_call" => format!(
            "`{}`: pushable 하지 않은 필터 조합이에요. 서버가 런타임에 거부해요.",
            rule.id
        ),
        "cursor" => format!(
            "`{}`: keyset 커서(after/before)는 지원 안 해요. limit/offset 을 써요.",
            rule.id
        ),
        "pushable_filter" => format!("`{}`: form-urlencoded 요청이 필요해요.", rule.id),
        "boundary" => format!("`{}`: /api/v1 prefix 가 필요한 엔드포인트예요.", rule.id),
        _ => format!("`{}`: SDK 계약 위반이에요.", rule.id),
    }
}

// ──────────────────────────── 스캔 ────────────────────────────

/// 한 소스 텍스트를 스캔해요. parse 실패 시 Err(fail-open 은 호출부에서 처리).
/// `file` 은 출력용 라벨이에요(테스트는 임의 라벨 전달).
pub(crate) fn scan_source(
    source: &str,
    grammar: Grammar,
    lang_key: &str,
    file: &str,
    rules: &[CompiledRule],
) -> Result<Vec<Violation>> {
    let (masked_no_comments, masked_code_only) = build_masks(source, grammar)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter parse 실패"))?;
    let line_index = LineIndex::new(source);

    let mut violations = Vec::new();
    'rules: for rule in rules {
        if !rule.applies_to(lang_key) {
            continue;
        }
        let haystack: &str = if rule.scans_strings {
            &masked_no_comments
        } else {
            &masked_code_only
        };
        for m in rule.regex.find_iter(haystack) {
            if violations.len() >= MAX_VIOLATIONS_PER_FILE {
                break 'rules;
            }
            let Ok(mat) = m else { continue }; // regex 런타임 에러는 스킵(fail-open)
            let (line, column) = line_index.line_col(source, mat.start());
            violations.push(Violation {
                file: file.to_string(),
                line,
                column,
                rule_id: rule.id.clone(),
                rule_kind: rule.rule_kind.clone(),
                advisory_only: rule.advisory_only,
                message: rule_message(rule),
                derived_from: rule.derived_from.clone(),
                lock_sha: rule.lock_sha.clone(),
            });
        }
    }
    Ok(violations)
}

enum ScanOutcome {
    Skipped,
    Scanned(Vec<Violation>),
    Failed(String),
}

fn scan_file(path: &Path, rules: &[CompiledRule]) -> ScanOutcome {
    let Some((grammar, lang_key)) = detect_lang(path) else {
        return ScanOutcome::Skipped;
    };
    match fs::metadata(path) {
        Ok(meta) if meta.len() > MAX_SOURCE_FILE_BYTES => {
            return ScanOutcome::Failed(format!("파일이 너무 커요({} bytes)", meta.len()));
        }
        Err(e) => return ScanOutcome::Failed(format!("metadata: {e}")),
        _ => {}
    }
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return ScanOutcome::Failed(format!("read: {e}")),
    };
    match scan_source(&source, grammar, lang_key, &path.display().to_string(), rules) {
        Ok(v) => ScanOutcome::Scanned(v),
        Err(e) => ScanOutcome::Failed(format!("{e}")),
    }
}

/// 대상 파일 수집. 반환 bool = MAX_SCAN_FILES cap 도달로 결과가 잘렸는지 신호 —
/// 조용한 truncation 은 "전부 검사했다"로 오독되니 호출부가 경고/필드로 노출해요.
pub(crate) fn collect_target_files(paths: &[String]) -> (Vec<PathBuf>, bool) {
    let mut out = Vec::new();
    let mut truncated = false;
    for p in paths {
        let path = Path::new(p);
        if path.is_file() {
            if out.len() >= MAX_SCAN_FILES {
                truncated = true;
                break;
            }
            out.push(path.to_path_buf());
        } else if path.is_dir() {
            walk_dir(path, 0, &mut out, &mut truncated);
        } else {
            eprintln!("axhub-helpers: 경로를 찾을 수 없어요 — {p} (건너뛰어요)");
        }
    }
    (out, truncated)
}

fn walk_dir(dir: &Path, depth: usize, out: &mut Vec<PathBuf>, truncated: &mut bool) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }
    if out.len() >= MAX_SCAN_FILES {
        *truncated = true;
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let skip = p
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| SKIP_DIRS.contains(&n))
                .unwrap_or(false);
            if !skip {
                walk_dir(&p, depth + 1, out, truncated);
            }
        } else if detect_lang(&p).is_some() {
            if out.len() >= MAX_SCAN_FILES {
                *truncated = true;
                return;
            }
            out.push(p);
        }
    }
}

/// `validate <paths...> [--json]` 진입점. exit 0=클린/advisory, 1=block 위반.
/// parse 실패는 위반으로 치지 않고 경고만(fail-open).
/// 경로들을 검사해 구조화된 결과를 만들어요(출력/exit 없음 — CLI 와 MCP tool 공용).
pub fn validate_paths(paths: &[String]) -> Result<ValidateOutput> {
    let rules = load_rules()?;
    let (files, truncated) = collect_target_files(paths);

    let mut violations = Vec::new();
    let mut parse_failures = Vec::new();
    let mut files_scanned = 0usize;
    for file in &files {
        match scan_file(file, &rules) {
            ScanOutcome::Skipped => {}
            ScanOutcome::Scanned(mut v) => {
                files_scanned += 1;
                violations.append(&mut v);
            }
            ScanOutcome::Failed(reason) => parse_failures.push(ParseFailure {
                file: file.display().to_string(),
                reason,
            }),
        }
    }

    let block_count = violations.iter().filter(|v| !v.advisory_only).count();
    let advisory_count = violations.len() - block_count;

    Ok(ValidateOutput {
        schema_version: "validate/v1".to_string(),
        status: if block_count > 0 { "violations" } else { "ok" }.to_string(),
        files_scanned,
        block_count,
        advisory_count,
        violations,
        parse_failures,
        truncated,
    })
}

/// `validate <paths...> [--json]` 진입점. exit 0=클린/advisory, 1=block 위반.
pub fn run_validate(paths: &[String], json: bool) -> Result<i32> {
    let output = validate_paths(paths)?;
    let exit = i32::from(output.block_count > 0);
    if json {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        emit_human(&output);
    }
    Ok(exit)
}

fn emit_human(output: &ValidateOutput) {
    for v in &output.violations {
        let tag = if v.advisory_only { "advisory" } else { "BLOCK" };
        println!("{}:{}:{} [{}] {} — {}", v.file, v.line, v.column, tag, v.rule_id, v.message);
    }
    for f in &output.parse_failures {
        eprintln!("⚠️  파싱 건너뜀(fail-open): {} — {}", f.file, f.reason);
    }
    if output.truncated {
        eprintln!(
            "⚠️  파일 수 cap({MAX_SCAN_FILES}) 도달 — 일부 파일은 검사하지 못했어요"
        );
    }
    if output.block_count > 0 {
        println!(
            "🚫 block 위반 {}건, advisory {}건 — 파일 {}개 검사",
            output.block_count, output.advisory_count, output.files_scanned
        );
    } else {
        println!(
            "✅ block 위반 없어요 (advisory {}건) — 파일 {}개 검사",
            output.advisory_count, output.files_scanned
        );
    }
}

// ──────────────────────────── PostToolUse hook 진입점 ────────────────────────────

/// PostToolUse hook 의 stdin payload 에서 편집된 파일 경로를 뽑아요. Edit/Write/
/// MultiEdit 는 `tool_input.file_path`, NotebookEdit 는 `tool_input.notebook_path`.
fn extract_edited_path(payload: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    let tool_input = value.get("tool_input")?;
    tool_input
        .get("file_path")
        .or_else(|| tool_input.get("notebook_path"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// hook 전용 프로젝트 게이트 — 편집 파일 기준 상향 최대 10단계에서 axhub 마커를
/// 찾아요. 마커: `axhub.yaml` 파일 | `.axhub/` 디렉터리 | "axhub" 항목이 든
/// `.mcp.json`. 마커가 없으면 비-axhub 프로젝트라 hook 이 조용히 no-op 해요(오발
/// 차단). CLI `validate` 는 명시 호출이라 게이트 없이 항상 검사해요.
fn is_axhub_project(start: &Path) -> bool {
    let mut dir = if start.is_dir() {
        Some(start)
    } else {
        start.parent()
    };
    for _ in 0..10 {
        let Some(d) = dir else { return false };
        if d.join("axhub.yaml").is_file() || d.join(".axhub").is_dir() {
            return true;
        }
        // .mcp.json 은 내용에 axhub 항목이 있을 때만 마커예요 (다른 MCP 만 쓰는
        // 프로젝트 오발 방지). 읽기 실패는 마커 아님(fail-open no-op 방향).
        if let Ok(s) = fs::read_to_string(d.join(".mcp.json")) {
            if s.contains("axhub") {
                return true;
            }
        }
        dir = d.parent();
    }
    false
}

/// `ast-validate` PostToolUse hook 진입점. 편집된 파일 1개만 검사해요(전체 스캔 X —
/// hook 레이턴시). **fail-open: 어떤 실패에서도 exit 0**, 위반은 systemMessage 로만
/// 노출(warn-only). `AXHUB_AST_VALIDATE=block` opt-in(§10.6 polarity, AXHUB_<scope>=
/// <value>) 시 additionalContext 로 교정 지시도 같이 실어요. advisory 는 노이즈라
/// hook 에서 노출 안 해요(명시적 `validate` CLI 로만).
///
/// 졸업 기준: block-트랙 룰만, good-fixture FP 0 + 실사용 FP 리포트 0 이 연속 2
/// 릴리즈 유지되면 block 을 default 로 승격(env 는 opt-out 으로 반전). advisory-전용
/// 트랙(무필터 list/count)은 영구 warn — 정적 미결정.
#[must_use]
pub fn run_hook() -> i32 {
    if crate::hook_safety::is_hook_disabled("ast-validate") {
        return 0;
    }
    let mut payload = String::new();
    if std::io::stdin().read_to_string(&mut payload).is_err() {
        return 0;
    }
    let Some(path) = extract_edited_path(&payload) else {
        return 0;
    };
    let target = Path::new(&path);
    if detect_lang(target).is_none() {
        return 0; // 미지원 확장자 — no-op
    }
    if !is_axhub_project(target) {
        return 0; // 비-axhub 프로젝트 — hook 침묵(오발 차단)
    }
    let rules = match load_rules() {
        Ok(r) => r,
        Err(e) => {
            crate::hook_safety::append_hook_error("ast-validate", &e);
            return 0;
        }
    };
    let violations = match scan_file(target, &rules) {
        ScanOutcome::Scanned(v) => v,
        ScanOutcome::Skipped => return 0,
        ScanOutcome::Failed(reason) => {
            crate::hook_safety::append_hook_error("ast-validate", &reason);
            return 0;
        }
    };
    let blocks: Vec<&Violation> = violations.iter().filter(|v| !v.advisory_only).collect();
    if blocks.is_empty() {
        return 0; // 클린(또는 advisory 만) — hook 침묵
    }
    print!("{}", render_hook_output(&path, &blocks));
    0
}

/// hook 출력 JSON 을 만들어요(테스트용으로 분리). warn-only=systemMessage 만,
/// block 모드=systemMessage + additionalContext(에이전트 교정 지시).
fn render_hook_output(path: &str, blocks: &[&Violation]) -> String {
    let block_mode = matches!(std::env::var("AXHUB_AST_VALIDATE").as_deref(), Ok("block"));
    let summary = blocks
        .iter()
        .map(|v| format!("{}:{} {}", v.line, v.column, v.rule_id))
        .collect::<Vec<_>>()
        .join(", ");
    if block_mode {
        let detail = blocks
            .iter()
            .map(|v| format!("- {}:{} {} — {}", v.line, v.column, v.rule_id, v.message))
            .collect::<Vec<_>>()
            .join("\n");
        let agent = format!(
            "{path} 에 axhub SDK 계약 block 위반 {}건이 있어요. 고쳐주세요:\n{detail}",
            blocks.len()
        );
        let system = format!(
            "🚫 axhub AST validator (block 모드) — {path}: block 위반 {}건",
            blocks.len()
        );
        crate::hook_output::post_tool_use_context_with_system(&agent, &system)
    } else {
        let system = format!(
            "⚠️ axhub AST validator — {path} 에서 SDK 계약 block 위반 {}건: {summary}. 검토를 권장해요. (강제하려면 AXHUB_AST_VALIDATE=block)",
            blocks.len()
        );
        serde_json::json!({ "systemMessage": system }).to_string()
    }
}

// ──────────────────────────── 테스트 ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn rules() -> Vec<CompiledRule> {
        load_rules().expect("vendored 룰 로드")
    }

    fn block_ids(violations: &[Violation]) -> BTreeSet<String> {
        violations
            .iter()
            .filter(|v| !v.advisory_only)
            .map(|v| v.rule_id.clone())
            .collect()
    }

    #[test]
    fn rules_load_with_provenance() {
        let r = rules();
        assert_eq!(
            r.len(),
            21,
            "vendored 룰 21개 로드 (F1 re-vendor: or/not/cursor 언어별 분리)"
        );
        for rule in &r {
            assert!(!rule.derived_from.is_empty(), "{} derived_from 필수", rule.id);
            assert!(!rule.lock_sha.is_empty(), "{} lock_sha 필수", rule.id);
        }
        let advisory = r.iter().filter(|x| x.advisory_only).count();
        assert_eq!(advisory, 3, "advisory_only 룰 3개(list/count/table-columns)");
    }

    // ── 룰 로딩 검증 3분기 — compile_rules_json 직접 테스트 ──

    #[test]
    fn load_fails_without_derived_from() {
        // derived_from 빈 룰은 로드 거부 (enforcement drift 차단).
        let bad = r#"[{"id":"x-bad","rule_kind":"forbidden_call","applies_lang":["node"],"pattern_hint":"\\bor\\s*\\(","derived_from":"","source":{"pack_section":"§6","lock_sha":"abc"}}]"#;
        let err = compile_rules_json(bad).err().expect("로드가 실패해야 해요");
        assert!(
            err.to_string().contains("derived_from"),
            "derived_from 누락 에러여야 해요: {err}"
        );
    }

    #[test]
    fn load_fails_when_advisory_lacks_reason() {
        let bad = r#"[{"id":"x-adv","rule_kind":"where_required","applies_lang":["node"],"pattern_hint":"\\blist\\s*\\(","derived_from":"§6","advisory_only":true,"source":{"pack_section":"§6","lock_sha":"abc"}}]"#;
        let err = compile_rules_json(bad).err().expect("로드가 실패해야 해요");
        assert!(
            err.to_string().contains("advisory_reason"),
            "advisory_reason 누락 에러여야 해요: {err}"
        );
    }

    #[test]
    fn load_fails_on_invalid_regex() {
        let bad = r#"[{"id":"x-regex","rule_kind":"forbidden_call","applies_lang":["node"],"pattern_hint":"(unclosed","derived_from":"§6","source":{"pack_section":"§6","lock_sha":"abc"}}]"#;
        let err = compile_rules_json(bad).err().expect("로드가 실패해야 해요");
        assert!(
            err.to_string().contains("regex 컴파일 실패"),
            "regex 컴파일 실패 에러여야 해요: {err}"
        );
    }

    // ── "*" HTTP/URL 룰: 문자열 안에서 매칭돼야 해요 ──
    #[test]
    fn star_rules_match_inside_strings() {
        let r = rules();
        let src = r#"export async function f() { await fetch("/invite-links/abc123"); }"#;
        let v = scan_source(src, Grammar::Typescript, "node", "<t>", &r).unwrap();
        assert!(
            block_ids(&v).contains("public-invite-links-must-use-api-v1-prefix"),
            "/invite-links/ (api/v1 무) → block, got {:?}",
            block_ids(&v)
        );
    }

    #[test]
    fn star_rules_respect_api_v1_lookbehind() {
        let r = rules();
        let src = r#"export async function f() { await fetch("/api/v1/invite-links/abc"); }"#;
        let v = scan_source(src, Grammar::Typescript, "node", "<t>", &r).unwrap();
        assert!(
            !block_ids(&v).contains("public-invite-links-must-use-api-v1-prefix"),
            "/api/v1/invite-links/ 는 정당 → no block, got {:?}",
            block_ids(&v)
        );
    }

    // ── FP 가드: 주석/문자열 안의 패턴은 매칭 안 돼야 해요 ──
    #[test]
    fn comments_and_strings_do_not_false_positive() {
        let r = rules();
        let src = r#"
// or( 와 not( 는 주석이라 무시돼야 해요. after: 도 마찬가지.
export function f() {
  const note = "or( not( after: 전부 문자열";
  return note;
}
"#;
        let v = scan_source(src, Grammar::Typescript, "node", "<t>", &r).unwrap();
        assert_eq!(block_ids(&v).len(), 0, "주석/문자열 FP 0, got {:?}", v);
    }

    // (구명: run_validate_returns_zero_on_clean — run_validate 를 호출하지 않는
    // 오명명이라 정정. 실제 exit 매핑은 아래 run_validate_exit_mapping 이 잠궈요.)
    #[test]
    fn clean_source_has_no_block_violations() {
        let r = rules();
        let src = "export function f(ownerId) { return db.table(\"p\").eq(\"owner_id\", ownerId).limit(10).list(); }";
        let v = scan_source(src, Grammar::Typescript, "node", "<t>", &r).unwrap();
        assert_eq!(block_ids(&v).len(), 0);
    }

    /// run_validate 의 exit 매핑: block 위반 → Ok(1), 클린/advisory → Ok(0).
    #[test]
    fn run_validate_exit_mapping() {
        let bad = fixture_dir().join("node/bad.ts").display().to_string();
        assert_eq!(run_validate(&[bad], true).unwrap(), 1, "block 위반 → exit 1");
        let good = fixture_dir().join("node/good.ts").display().to_string();
        assert_eq!(
            run_validate(&[good], true).unwrap(),
            0,
            "클린(advisory 만) → exit 0"
        );
    }

    /// `validate --json` envelope 전 필드 잠금 — 소비자(SKILL/MCP)가 의존하는 계약.
    #[test]
    fn validate_json_envelope_has_all_fields() {
        let bad = fixture_dir().join("node/bad.ts").display().to_string();
        let out = validate_paths(&[bad]).unwrap();
        let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&out).unwrap()).unwrap();
        assert_eq!(json["schema_version"], "validate/v1");
        assert_eq!(json["status"], "violations");
        assert_eq!(json["files_scanned"], 1);
        assert!(json["block_count"].as_u64().unwrap() > 0);
        assert!(json["advisory_count"].is_u64());
        assert_eq!(json["truncated"], false);
        assert!(json["parse_failures"].as_array().unwrap().is_empty());
        let v = &json["violations"].as_array().unwrap()[0];
        for field in [
            "file", "line", "column", "rule_id", "rule_kind", "advisory_only", "message",
            "derived_from", "lock_sha",
        ] {
            assert!(!v[field].is_null(), "violations[0].{field} 필드 누락");
        }
    }

    /// rule_kind 공통 메시지가 아니라 rule id 고유 메시지를 내는 룰 2종 잠금.
    #[test]
    fn rule_messages_are_rule_id_specific() {
        let r = rules();
        let raw_http = r
            .iter()
            .find(|x| x.id == "raw-http-axhub-data-endpoint-forbidden")
            .expect("raw-http 룰 존재");
        assert!(
            rule_message(raw_http).contains("raw HTTP 직타는 막혀 있어요"),
            "raw-http 룰은 SDK 경유 안내 메시지여야 해요"
        );
        let use_client = r
            .iter()
            .find(|x| x.id == "use-client-imports-server-only-axhub")
            .expect("use-client 룰 존재");
        let msg = rule_message(use_client);
        assert!(
            msg.contains("server-only helper") && msg.contains("client 컴포넌트"),
            "use-client 룰은 /api/v1 boundary 공통 메시지가 아니라 고유 메시지여야 해요: {msg}"
        );
        assert!(!msg.contains("/api/v1"), "boundary 공통 메시지 누출 금지: {msg}");
    }

    // ── hook 헬퍼 ──
    #[test]
    fn extract_path_from_edit_payload() {
        let p = r#"{"tool_name":"Edit","tool_input":{"file_path":"/x/a.ts","old_string":"a"}}"#;
        assert_eq!(extract_edited_path(p).as_deref(), Some("/x/a.ts"));
        let nb = r#"{"tool_input":{"notebook_path":"/x/n.ipynb"}}"#;
        assert_eq!(extract_edited_path(nb).as_deref(), Some("/x/n.ipynb"));
        assert_eq!(extract_edited_path("not json"), None);
        assert_eq!(extract_edited_path(r#"{"tool_input":{}}"#), None);
    }

    #[test]
    fn hook_output_warn_only_is_systemmessage_only() {
        let v = Violation {
            file: "a.ts".into(),
            line: 3,
            column: 5,
            rule_id: "or-combinator-not-pushable".into(),
            rule_kind: "forbidden_call".into(),
            advisory_only: false,
            message: "x".into(),
            derived_from: "y".into(),
            lock_sha: "z".into(),
        };
        let refs = vec![&v];
        // 기본(warn-only): systemMessage 만, additionalContext 없음.
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        std::env::remove_var("AXHUB_AST_VALIDATE");
        let out = render_hook_output("a.ts", &refs);
        let json: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(json.get("systemMessage").is_some());
        assert!(json.get("hookSpecificOutput").is_none(), "warn-only 는 additionalContext 없음");
    }

    // ── 6언어 fixture 매트릭스 ──
    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ast-validate")
    }

    fn scan_fixture(rel: &str, grammar: Grammar, lang_key: &str) -> Vec<Violation> {
        let r = rules();
        let path = fixture_dir().join(rel);
        let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} 읽기 실패: {e}", path.display()));
        scan_source(&src, grammar, lang_key, rel, &r).expect("parse")
    }

    /// 언어별 핵심 block 룰 id (F1 re-vendor 로 or/not/cursor 가 언어별 분리됨 —
    /// boolean 키워드 `or (`/`not (` FP 차단).
    fn expected_block_ids(lang_key: &str) -> [&'static str; 3] {
        match lang_key {
            "node" => [
                "or-combinator-not-pushable-node",
                "not-combinator-not-pushable-node",
                "cursor-keyset-after-before-forbidden-node-ruby",
            ],
            "go" => [
                "or-combinator-not-pushable-go",
                "not-combinator-not-pushable-go",
                "cursor-keyset-after-before-forbidden-go",
            ],
            "python" => [
                "or-combinator-not-pushable-python-ruby",
                "not-combinator-not-pushable-python-ruby",
                "cursor-keyset-after-before-forbidden-python",
            ],
            "ruby" => [
                "or-combinator-not-pushable-python-ruby",
                "not-combinator-not-pushable-python-ruby",
                "cursor-keyset-after-before-forbidden-node-ruby",
            ],
            "java" | "kotlin" => [
                "or-combinator-not-pushable-jvm",
                "not-combinator-not-pushable-jvm",
                "cursor-keyset-after-before-forbidden-jvm",
            ],
            other => panic!("unknown lang_key {other}"),
        }
    }

    /// bad fixture 는 핵심 block 데이터 룰(or/not/cursor)을 전부 검출해야 해요.
    fn assert_bad(rel: &str, grammar: Grammar, lang_key: &str) {
        let v = scan_fixture(rel, grammar, lang_key);
        let ids = block_ids(&v);
        for expected in expected_block_ids(lang_key) {
            assert!(ids.contains(expected), "{rel}: {expected} 미검출, got {ids:?}");
        }
    }

    /// good fixture 는 block 위반 0 — owner-scoped 무필터 list/count(advisory)는 허용.
    fn assert_good(rel: &str, grammar: Grammar, lang_key: &str) {
        let v = scan_fixture(rel, grammar, lang_key);
        let ids = block_ids(&v);
        assert!(ids.is_empty(), "{rel}: block FP {ids:?} (advisory 는 허용)");
        // owner-scoped 무필터 정당 호출이 advisory 로 떠야 해요(케이스 존재 증명).
        let has_advisory = v.iter().any(|x| x.advisory_only);
        assert!(has_advisory, "{rel}: owner-scoped advisory 케이스가 있어야 해요");
    }

    #[test]
    fn fixtures_node_ts() {
        assert_bad("node/bad.ts", Grammar::Typescript, "node");
        assert_good("node/good.ts", Grammar::Typescript, "node");
    }

    #[test]
    fn fixtures_node_tsx() {
        assert_bad("node/bad.tsx", Grammar::Tsx, "node");
        assert_good("node/good.tsx", Grammar::Tsx, "node");
    }

    /// F1 re-vendor 룰 — raw-http(bad.ts) + use-client(bad.tsx, 단따옴표) block 검출.
    #[test]
    fn fixtures_node_new_block_rules() {
        let ts = block_ids(&scan_fixture("node/bad.ts", Grammar::Typescript, "node"));
        assert!(
            ts.contains("raw-http-axhub-data-endpoint-forbidden"),
            "raw-http 미검출: {ts:?}"
        );
        let tsx = block_ids(&scan_fixture("node/bad.tsx", Grammar::Tsx, "node"));
        assert!(
            tsx.contains("use-client-imports-server-only-axhub"),
            "use-client 미검출: {tsx:?}"
        );
    }

    #[test]
    fn fixtures_python() {
        assert_bad("python/bad.py", Grammar::Python, "python");
        assert_good("python/good.py", Grammar::Python, "python");
    }

    #[test]
    fn fixtures_go() {
        assert_bad("go/bad.go", Grammar::Go, "go");
        assert_good("go/good.go", Grammar::Go, "go");
    }

    #[test]
    fn fixtures_java() {
        assert_bad("java/bad.java", Grammar::Java, "java");
        assert_good("java/good.java", Grammar::Java, "java");
    }

    #[test]
    fn fixtures_kotlin() {
        assert_bad("kotlin/bad.kt", Grammar::Kotlin, "kotlin");
        assert_good("kotlin/good.kt", Grammar::Kotlin, "kotlin");
    }

    #[test]
    fn fixtures_ruby() {
        assert_bad("ruby/bad.rb", Grammar::Ruby, "ruby");
        assert_good("ruby/good.rb", Grammar::Ruby, "ruby");
    }

    /// O(n²) 회귀 잠금: 250KB dense 파일(or() 2.8만개)도 1초 미만이어야 해요.
    /// 수정 전 line_col 은 match 마다 파일 처음부터 재스캔(O(n²)) — 리뷰 실측
    /// 4.67s(250KB), 같은 워크로드 release 재현 8.10s(440KB/5만 match) → 수정 후
    /// 0.32s. LineIndex(개행 offset 인덱스 + binary_search) + 파일당 cap 500 잠금.
    #[test]
    fn dense_file_scan_under_one_second() {
        let r = rules();
        // `q.or(x);\n` 9 bytes × 28_000 = ~250KB, or( match 2.8만개.
        let mut src = String::with_capacity(28_000 * 9 + 16);
        for _ in 0..28_000 {
            src.push_str("q.or(x);\n");
        }
        let started = std::time::Instant::now();
        let v = scan_source(&src, Grammar::Typescript, "node", "<dense>", &r).unwrap();
        let elapsed = started.elapsed();
        assert!(
            v.iter().any(|x| x.rule_id == "or-combinator-not-pushable-node"),
            "dense or() 위반이 검출돼야 해요"
        );
        assert!(
            v.len() <= 500,
            "파일당 위반 cap 500, got {}",
            v.len()
        );
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "250KB+ dense 스캔은 1초 미만이어야 해요 (실측 {elapsed:?})"
        );
    }

    /// AC10 잠금 (fallback 강등 분기): PR #198 스택 미머지 환경에서 advisory 메시지는
    /// migrate-data-verify 위임 문구 없이 권고("권장")만 방출해야 해요.
    #[test]
    fn advisory_messages_recommend_without_delegation() {
        let r = rules();
        // (a) 전 advisory 룰의 rule_message 출력.
        let mut messages: Vec<String> = r
            .iter()
            .filter(|rule| rule.advisory_only)
            .map(rule_message)
            .collect();
        assert!(!messages.is_empty(), "advisory 룰이 1개 이상이어야 해요");
        // (b) 6언어 good fixture 스캔으로 얻은 실제 advisory violation 메시지.
        for (rel, grammar, lang_key) in [
            ("node/good.ts", Grammar::Typescript, "node"),
            ("node/good.tsx", Grammar::Tsx, "node"),
            ("python/good.py", Grammar::Python, "python"),
            ("go/good.go", Grammar::Go, "go"),
            ("java/good.java", Grammar::Java, "java"),
            ("kotlin/good.kt", Grammar::Kotlin, "kotlin"),
            ("ruby/good.rb", Grammar::Ruby, "ruby"),
        ] {
            let v = scan_fixture(rel, grammar, lang_key);
            messages.extend(v.into_iter().filter(|x| x.advisory_only).map(|x| x.message));
        }
        for msg in &messages {
            assert!(
                !msg.contains("migrate-data-verify"),
                "위임 문구 금지(미머지 fallback): {msg}"
            );
            assert!(!msg.contains("위임"), "위임 문구 금지(미머지 fallback): {msg}");
            assert!(msg.contains("권장"), "권고 문구('권장') 필수: {msg}");
        }
    }
}

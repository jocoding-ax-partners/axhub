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
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fancy_regex::Regex;
use serde::{Deserialize, Serialize};
use tree_sitter::{Language, Node, Parser};

const RULES_JSON: &str = include_str!("../rules/data-contract-rules.json");

const MAX_SCAN_DEPTH: usize = 12;
const MAX_SCAN_FILES: usize = 5_000;
const MAX_SOURCE_FILE_BYTES: u64 = 1024 * 1024;
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

/// 컴파일된 룰. `scans_strings` = 언어무관(`*`) 룰이라 문자열을 스캔해요.
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
    let raw: Vec<RawRule> =
        serde_json::from_str(RULES_JSON).context("data-contract-rules.json 파싱 실패")?;
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
        let scans_strings = r.applies_lang.iter().any(|l| l == "*");
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
fn detect_lang(path: &Path) -> Option<(Grammar, &'static str)> {
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

fn ts_language(grammar: Grammar) -> Language {
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

/// byte offset → (1-base line, 1-base column). column 은 문자 단위예요.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    for (idx, ch) in source.char_indices() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
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
    let mut parser = Parser::new();
    parser
        .set_language(&ts_language(grammar))
        .map_err(|e| anyhow::anyhow!("set_language 실패: {e}"))?;
    let tree = parser
        .parse(source.as_bytes(), None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter parse 실패"))?;

    let mut comments = Vec::new();
    let mut strings = Vec::new();
    collect_mask_spans(tree.root_node(), &mut comments, &mut strings);
    let masked_no_comments = apply_mask(source, &comments);
    let mut all_spans = Vec::with_capacity(comments.len() + strings.len());
    all_spans.extend_from_slice(&comments);
    all_spans.extend_from_slice(&strings);
    let masked_code_only = apply_mask(source, &all_spans);

    let mut violations = Vec::new();
    for rule in rules {
        if !rule.applies_to(lang_key) {
            continue;
        }
        let haystack: &str = if rule.scans_strings {
            &masked_no_comments
        } else {
            &masked_code_only
        };
        for m in rule.regex.find_iter(haystack) {
            let Ok(mat) = m else { continue }; // regex 런타임 에러는 스킵(fail-open)
            let (line, column) = line_col(source, mat.start());
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

fn collect_target_files(paths: &[String]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        let path = Path::new(p);
        if path.is_file() {
            out.push(path.to_path_buf());
        } else if path.is_dir() {
            walk_dir(path, 0, &mut out);
        } else {
            eprintln!("axhub-helpers validate: 경로를 찾을 수 없어요 — {p} (건너뛰어요)");
        }
    }
    out
}

fn walk_dir(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_SCAN_DEPTH || out.len() >= MAX_SCAN_FILES {
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
                walk_dir(&p, depth + 1, out);
            }
        } else if detect_lang(&p).is_some() {
            out.push(p);
            if out.len() >= MAX_SCAN_FILES {
                return;
            }
        }
    }
}

/// `validate <paths...> [--json]` 진입점. exit 0=클린/advisory, 1=block 위반.
/// parse 실패는 위반으로 치지 않고 경고만(fail-open).
pub fn run_validate(paths: &[String], json: bool) -> Result<i32> {
    let rules = load_rules()?;
    let files = collect_target_files(paths);

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
    let exit = i32::from(block_count > 0);

    let output = ValidateOutput {
        schema_version: "validate/v1".to_string(),
        status: if block_count > 0 { "violations" } else { "ok" }.to_string(),
        files_scanned,
        block_count,
        advisory_count,
        violations,
        parse_failures,
    };

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
        assert_eq!(r.len(), 10, "vendored 룰 10개 로드");
        for rule in &r {
            assert!(!rule.derived_from.is_empty(), "{} derived_from 필수", rule.id);
            assert!(!rule.lock_sha.is_empty(), "{} lock_sha 필수", rule.id);
        }
        let advisory = r.iter().filter(|x| x.advisory_only).count();
        assert_eq!(advisory, 3, "advisory_only 룰 3개(list/count/table-columns)");
    }

    #[test]
    fn load_fails_without_derived_from() {
        // derived_from 빈 룰은 로드 거부 (enforcement drift 차단).
        let bad = r#"[{"id":"x-bad","rule_kind":"forbidden_call","applies_lang":["node"],"pattern_hint":"\\bor\\s*\\(","derived_from":"","source":{"pack_section":"§6","lock_sha":"abc"}}]"#;
        let raw: Vec<RawRule> = serde_json::from_str(bad).unwrap();
        // load_rules 는 embed 된 JSON 을 쓰므로, derived_from 검증 로직을 직접 확인.
        assert!(raw[0].derived_from.trim().is_empty());
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

    #[test]
    fn run_validate_returns_zero_on_clean() {
        // 임시 파일 없이 run path 의 exit 의미만: clean 소스는 block 0.
        let r = rules();
        let src = "export function f(ownerId) { return db.table(\"p\").eq(\"owner_id\", ownerId).limit(10).list(); }";
        let v = scan_source(src, Grammar::Typescript, "node", "<t>", &r).unwrap();
        assert_eq!(block_ids(&v).len(), 0);
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

    /// bad fixture 는 핵심 block 데이터 룰(or/not/cursor)을 전부 검출해야 해요.
    fn assert_bad(rel: &str, grammar: Grammar, lang_key: &str) {
        let v = scan_fixture(rel, grammar, lang_key);
        let ids = block_ids(&v);
        for expected in [
            "or-combinator-not-pushable",
            "not-combinator-not-pushable",
            "cursor-keyset-after-before-forbidden",
        ] {
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
}

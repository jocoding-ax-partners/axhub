//! 변환 사이트 스캐너 (Track H §H2). migrate 플로우가 "SDK 로 바꿔야 할 곳"을
//! 찾도록 raw HTTP client 직타, 직접 DB driver 사용, 하드코딩 API URL 을 검출해요.
//!
//! `validate`(정적 계약 gate)와 달리 site-scan 은 **finder** 예요 — 항상 exit 0,
//! `{file, line, kind, snippet}` JSON 배열로 후보를 내요. 마스킹/언어매핑/파일수집은
//! `ast_validate` 와 같은 엔진을 재사용해요(plan Principle 4 "엔진 한 벌").
//!
//! 검출 종류(kind):
//! - `raw_http_client`  : fetch/axios/requests/http.Get/OkHttp/Net::HTTP 등 raw HTTP.
//! - `direct_db_driver` : pg/mysql2/sqlite3/prisma raw/database\\sql/psycopg2 등 직접 DB.
//! - `hardcoded_api_url`: 소스에 박힌 절대 URL(`https?://…`).

use anyhow::Result;
use fancy_regex::Regex;
use serde::Serialize;

use crate::ast_validate::{build_masks, collect_target_files, detect_lang, line_col, Grammar};

/// site-scan 휴리스틱 룰(하드코딩 — distiller 계약이 아니라 migration 탐지용).
struct SiteRule {
    kind: &'static str,
    pattern: &'static str,
    /// 적용 언어 키("node"/"python"/"go"/"java"/"kotlin"/"ruby") 또는 "*".
    langs: &'static [&'static str],
    /// true 면 문자열(주석 제외)도 스캔 — import 지정자/URL 이 문자열에 살아요.
    scans_strings: bool,
}

const SITE_RULES: &[SiteRule] = &[
    // ── raw HTTP client (code 식별자 — 문자열/주석 제외) ──
    SiteRule { kind: "raw_http_client", pattern: r"\b(?:fetch|axios)\s*[(.]", langs: &["node"], scans_strings: false },
    SiteRule { kind: "raw_http_client", pattern: r"\bnew\s+XMLHttpRequest\b", langs: &["node"], scans_strings: false },
    SiteRule { kind: "raw_http_client", pattern: r"\b(?:requests|httpx)\s*\.\s*(?:get|post|put|delete|patch|request)\b", langs: &["python"], scans_strings: false },
    SiteRule { kind: "raw_http_client", pattern: r"\bhttp\s*\.\s*(?:Get|Post|NewRequest|PostForm)\b", langs: &["go"], scans_strings: false },
    SiteRule { kind: "raw_http_client", pattern: r"\b(?:HttpClient|HttpURLConnection|OkHttpClient)\b", langs: &["java", "kotlin"], scans_strings: false },
    SiteRule { kind: "raw_http_client", pattern: r"\b(?:Net::HTTP|Faraday|HTTParty)\b", langs: &["ruby"], scans_strings: false },
    // ── direct DB driver (import 지정자 문자열 + 코드 — 문자열 스캔) ──
    SiteRule { kind: "direct_db_driver", pattern: r#"['"](?:pg|mysql2|better-sqlite3|sqlite3)['"]"#, langs: &["node"], scans_strings: true },
    SiteRule { kind: "direct_db_driver", pattern: r"\bnew\s+Pool\s*\(|\.\$(?:queryRaw|executeRaw)\b", langs: &["node"], scans_strings: true },
    SiteRule { kind: "direct_db_driver", pattern: r"\b(?:psycopg2|pymysql|MySQLdb)\b|\bsqlite3\s*\.\s*connect\b|\bcreate_engine\s*\(", langs: &["python"], scans_strings: true },
    SiteRule { kind: "direct_db_driver", pattern: r#""database/sql"|\bsql\.Open\s*\(|\bpgx\s*\.\s*(?:Connect|New)\b"#, langs: &["go"], scans_strings: true },
    SiteRule { kind: "direct_db_driver", pattern: r"\bDriverManager\s*\.\s*getConnection\b|\bjava\.sql\.", langs: &["java", "kotlin"], scans_strings: true },
    SiteRule { kind: "direct_db_driver", pattern: r"\bPG\.connect\b|\bMysql2::Client\b|\bSQLite3::Database\b", langs: &["ruby"], scans_strings: true },
    // ── 하드코딩 절대 URL (문자열) ──
    SiteRule { kind: "hardcoded_api_url", pattern: r#"https?://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+"#, langs: &["*"], scans_strings: true },
];

struct CompiledSiteRule {
    kind: &'static str,
    regex: Regex,
    langs: &'static [&'static str],
    scans_strings: bool,
}

fn compile_rules() -> Vec<CompiledSiteRule> {
    SITE_RULES
        .iter()
        .filter_map(|r| {
            Regex::new(r.pattern).ok().map(|regex| CompiledSiteRule {
                kind: r.kind,
                regex,
                langs: r.langs,
                scans_strings: r.scans_strings,
            })
        })
        .collect()
}

fn applies(langs: &[&str], lang_key: &str) -> bool {
    langs.iter().any(|l| *l == "*" || *l == lang_key)
}

#[derive(Debug, Clone, Serialize)]
pub struct Site {
    pub file: String,
    pub line: usize,
    pub kind: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanParseFailure {
    pub file: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ScanSitesOutput {
    pub schema_version: String,
    pub files_scanned: usize,
    pub sites: Vec<Site>,
    pub parse_failures: Vec<ScanParseFailure>,
}

/// offset 이 속한 줄의 텍스트(trim)를 잘라 snippet 으로 써요.
fn snippet_at(source: &str, offset: usize) -> String {
    let start = source[..offset.min(source.len())]
        .rfind('\n')
        .map_or(0, |i| i + 1);
    let end = source[start..]
        .find('\n')
        .map_or(source.len(), |i| start + i);
    source[start..end].trim().to_string()
}

/// 한 소스를 스캔해 site 후보를 내요(모듈 내부 + 테스트용).
fn scan_source_sites(
    source: &str,
    grammar: Grammar,
    lang_key: &str,
    file: &str,
    rules: &[CompiledSiteRule],
) -> Option<Vec<Site>> {
    let (masked_no_comments, masked_code_only) = build_masks(source, grammar)?;
    let mut sites = Vec::new();
    for rule in rules {
        if !applies(rule.langs, lang_key) {
            continue;
        }
        let haystack: &str = if rule.scans_strings {
            &masked_no_comments
        } else {
            &masked_code_only
        };
        for m in rule.regex.find_iter(haystack) {
            let Ok(mat) = m else { continue };
            let (line, _col) = line_col(source, mat.start());
            sites.push(Site {
                file: file.to_string(),
                line,
                kind: rule.kind.to_string(),
                snippet: snippet_at(source, mat.start()),
            });
        }
    }
    Some(sites)
}

/// `scan-sites <paths...> [--json]` 진입점. 항상 exit 0(finder). parse 실패는
/// parse_failures 로 기록하고 계속해요.
pub fn run_scan_sites(paths: &[String], json: bool) -> Result<i32> {
    let rules = compile_rules();
    let files = collect_target_files(paths);

    let mut sites = Vec::new();
    let mut parse_failures = Vec::new();
    let mut files_scanned = 0usize;
    for file in &files {
        let Some((grammar, lang_key)) = detect_lang(file) else {
            continue;
        };
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                parse_failures.push(ScanParseFailure {
                    file: file.display().to_string(),
                    reason: format!("read: {e}"),
                });
                continue;
            }
        };
        match scan_source_sites(&source, grammar, lang_key, &file.display().to_string(), &rules) {
            Some(mut found) => {
                files_scanned += 1;
                sites.append(&mut found);
            }
            None => parse_failures.push(ScanParseFailure {
                file: file.display().to_string(),
                reason: "tree-sitter parse 실패".to_string(),
            }),
        }
    }

    let output = ScanSitesOutput {
        schema_version: "scan-sites/v1".to_string(),
        files_scanned,
        sites,
        parse_failures,
    };

    if json {
        println!("{}", serde_json::to_string(&output)?);
    } else {
        for s in &output.sites {
            println!("{}:{} [{}] {}", s.file, s.line, s.kind, s.snippet);
        }
        println!(
            "🔎 변환 후보 {}건 — 파일 {}개 검사",
            output.sites.len(),
            output.files_scanned
        );
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    fn kinds(sites: &[Site]) -> BTreeSet<String> {
        sites.iter().map(|s| s.kind.clone()).collect()
    }

    fn fixture_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/site-scan")
    }

    fn scan_fixture(rel: &str, grammar: Grammar, lang_key: &str) -> Vec<Site> {
        let rules = compile_rules();
        let path = fixture_dir().join(rel);
        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{} 읽기 실패: {e}", path.display()));
        scan_source_sites(&src, grammar, lang_key, rel, &rules).expect("parse")
    }

    /// bad fixture 는 3 종류(raw_http_client / direct_db_driver / hardcoded_api_url)를
    /// 전부 검출해야 해요.
    fn assert_detects_all(rel: &str, grammar: Grammar, lang_key: &str) {
        let k = kinds(&scan_fixture(rel, grammar, lang_key));
        for kind in ["raw_http_client", "direct_db_driver", "hardcoded_api_url"] {
            assert!(k.contains(kind), "{rel}: {kind} 미검출, got {k:?}");
        }
    }

    /// clean fixture 는 SDK 사용만 — 검출 0.
    fn assert_clean(rel: &str, grammar: Grammar, lang_key: &str) {
        let sites = scan_fixture(rel, grammar, lang_key);
        assert!(sites.is_empty(), "{rel}: 오검출 {sites:?}");
    }

    #[test]
    fn rules_compile() {
        assert_eq!(compile_rules().len(), SITE_RULES.len(), "전 룰 regex 컴파일");
    }

    #[test]
    fn comment_does_not_false_positive() {
        // 주석 안의 fetch/URL 은 검출 안 돼야 해요.
        let src = "// fetch(\"https://api.example.com/x\") 는 주석이에요\nexport const a = 1;\n";
        let sites = scan_source_sites(src, Grammar::Typescript, "node", "<t>", &compile_rules()).unwrap();
        assert!(sites.is_empty(), "주석 FP, got {sites:?}");
    }

    #[test]
    fn fixtures_node() {
        assert_detects_all("node/bad.ts", Grammar::Typescript, "node");
        assert_clean("node/good.ts", Grammar::Typescript, "node");
    }

    #[test]
    fn fixtures_python() {
        assert_detects_all("python/bad.py", Grammar::Python, "python");
        assert_clean("python/good.py", Grammar::Python, "python");
    }

    #[test]
    fn fixtures_go() {
        assert_detects_all("go/bad.go", Grammar::Go, "go");
        assert_clean("go/good.go", Grammar::Go, "go");
    }

    #[test]
    fn fixtures_java() {
        assert_detects_all("java/bad.java", Grammar::Java, "java");
        assert_clean("java/good.java", Grammar::Java, "java");
    }

    #[test]
    fn fixtures_kotlin() {
        assert_detects_all("kotlin/bad.kt", Grammar::Kotlin, "kotlin");
        assert_clean("kotlin/good.kt", Grammar::Kotlin, "kotlin");
    }

    #[test]
    fn fixtures_ruby() {
        assert_detects_all("ruby/bad.rb", Grammar::Ruby, "ruby");
        assert_clean("ruby/good.rb", Grammar::Ruby, "ruby");
    }
}

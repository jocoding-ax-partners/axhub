//! Deterministic discover()-verify gate for `data_patch_plan` conversion.
//!
//! The reliability problem this closes: an LLM expert converts a user's
//! ORM/raw-SQL data access to AxHub SDK data calls, but docs + LLM codegen
//! cannot guarantee the table/column names it emits actually exist. A wrong
//! name COMPILES and the build-level verify passes — it just silently queries
//! the wrong thing, and a vibe-coder will not catch it in review.
//!
//! The fix is a deterministic referee that is NOT the LLM. The expert calls the
//! SDK's own `discover()` (the battle-tested introspection in the node SDK's
//! `resources/data/discover.ts` — slug inspect + appId fallback + error
//! normalization) to get the REAL schema per table, and declares the refs it
//! used. This module does the pure set-diff: every referenced table must exist,
//! and every referenced column must exist in that table's real schema. A miss is
//! a hard-stop — surfaced in the Korean preview, apply blocked.
//!
//! Pure + network-free on purpose: the SDK already owns the introspection, so
//! re-implementing inspect/auth/fallback in Rust would only add a second,
//! drift-prone source of truth. The helper stays the deterministic judge.

use std::collections::BTreeMap;

use serde::Serialize;

/// `table name -> column names`. BTreeMap keeps output stable for snapshot/diff.
pub type RefMap = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationKind {
    /// A table the conversion references is absent from the app.
    MissingTable,
    /// A column the conversion references is absent from the table's real schema.
    MissingColumn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub table: String,
    pub kind: ViolationKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Verdict {
    /// True only when the conversion references nothing the real schema lacks.
    pub ok: bool,
    pub violations: Vec<Violation>,
    /// Count of distinct tables the conversion referenced (for the preview line).
    pub tables_checked: usize,
    /// Count of column references checked across all tables.
    pub columns_checked: usize,
}

/// The pure deterministic gate: `refs` (what the conversion uses) vs `schemas`
/// (the real, discover()'d schema). Violations are emitted in a stable order
/// (refs are a BTreeMap; columns keep ref order) so the preview is reproducible.
pub fn verify_data_refs(refs: &RefMap, schemas: &RefMap) -> Verdict {
    let mut violations = Vec::new();
    let mut columns_checked = 0usize;
    for (table, cols) in refs {
        match schemas.get(table) {
            None => violations.push(Violation {
                table: table.clone(),
                kind: ViolationKind::MissingTable,
                column: None,
            }),
            Some(real) => {
                for col in cols {
                    columns_checked += 1;
                    if !real.iter().any(|rc| rc == col) {
                        violations.push(Violation {
                            table: table.clone(),
                            kind: ViolationKind::MissingColumn,
                            column: Some(col.clone()),
                        });
                    }
                }
            }
        }
    }
    Verdict {
        ok: violations.is_empty(),
        violations,
        tables_checked: refs.len(),
        columns_checked,
    }
}

/// Korean (해요체) preview line(s) for the migrate preview. A pass is one line;
/// a failure lists every violation so the user sees exactly what would silently
/// break before any apply.
pub fn render_verdict_kr(verdict: &Verdict) -> String {
    if verdict.ok {
        return format!(
            "✅ data-verify: 참조한 table {}개 · column {}개 가 모두 실제 schema 에 있어요.",
            verdict.tables_checked, verdict.columns_checked
        );
    }
    let mut lines =
        vec!["🛑 data-verify 실패 — 참조가 실제 schema 와 안 맞아서 apply 를 막아요:".to_string()];
    for v in &verdict.violations {
        match v.kind {
            ViolationKind::MissingTable => {
                lines.push(format!("- table `{}` 가 앱에 없어요.", v.table));
            }
            ViolationKind::MissingColumn => {
                let col = v.column.as_deref().unwrap_or("?");
                lines.push(format!(
                    "- `{}.{}` column 이 실제 schema 에 없어요.",
                    v.table, col
                ));
            }
        }
    }
    lines.push(
        "변환 코드가 빌드는 통과해도 틀린 table·column 을 조회해요. \
         table·column 이름을 실제 schema 에 맞추거나, discover() 결과를 다시 확인해요."
            .to_string(),
    );
    lines.join("\n")
}

/// Parse a refs/schemas JSON document: `{ "table": ["col", ...], ... }`.
pub fn parse_ref_map(json: &str) -> anyhow::Result<RefMap> {
    let map: RefMap = serde_json::from_str(json)?;
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &[&str])]) -> RefMap {
        pairs
            .iter()
            .map(|(t, cols)| (t.to_string(), cols.iter().map(|c| c.to_string()).collect()))
            .collect()
    }

    #[test]
    fn passes_when_every_ref_exists() {
        let refs = map(&[("orders", &["id", "total"]), ("users", &["email"])]);
        let schemas = map(&[
            ("orders", &["id", "total", "status", "notes"]),
            ("users", &["id", "email", "name"]),
        ]);
        let v = verify_data_refs(&refs, &schemas);
        assert!(v.ok);
        assert!(v.violations.is_empty());
        assert_eq!(v.tables_checked, 2);
        assert_eq!(v.columns_checked, 3);
    }

    #[test]
    fn flags_missing_table_as_hard_stop() {
        let refs = map(&[("orderz", &["id"])]); // typo'd table
        let schemas = map(&[("orders", &["id"])]);
        let v = verify_data_refs(&refs, &schemas);
        assert!(!v.ok);
        assert_eq!(v.violations.len(), 1);
        assert_eq!(v.violations[0].kind, ViolationKind::MissingTable);
        assert_eq!(v.violations[0].column, None);
    }

    #[test]
    fn flags_missing_column_but_keeps_table() {
        let refs = map(&[("orders", &["id", "totals"])]); // typo'd column
        let schemas = map(&[("orders", &["id", "total"])]);
        let v = verify_data_refs(&refs, &schemas);
        assert!(!v.ok);
        assert_eq!(v.violations.len(), 1);
        assert_eq!(v.violations[0].kind, ViolationKind::MissingColumn);
        assert_eq!(v.violations[0].column.as_deref(), Some("totals"));
        assert_eq!(v.columns_checked, 2);
    }

    #[test]
    fn missing_table_does_not_also_report_its_columns() {
        // A missing table is ONE violation, not one-per-column — avoids noise.
        let refs = map(&[("ghost", &["a", "b", "c"])]);
        let schemas = map(&[("real", &["a"])]);
        let v = verify_data_refs(&refs, &schemas);
        assert_eq!(v.violations.len(), 1);
        assert_eq!(v.violations[0].kind, ViolationKind::MissingTable);
    }

    #[test]
    fn empty_refs_is_a_pass() {
        let v = verify_data_refs(&RefMap::new(), &map(&[("orders", &["id"])]));
        assert!(v.ok);
        assert_eq!(v.tables_checked, 0);
    }

    #[test]
    fn column_match_is_case_sensitive() {
        // Backend column names are exact identifiers; "Total" != "total".
        let refs = map(&[("orders", &["Total"])]);
        let schemas = map(&[("orders", &["total"])]);
        let v = verify_data_refs(&refs, &schemas);
        assert!(!v.ok);
        assert_eq!(v.violations[0].kind, ViolationKind::MissingColumn);
    }

    #[test]
    fn render_pass_is_single_line() {
        let v = verify_data_refs(&map(&[("orders", &["id"])]), &map(&[("orders", &["id"])]));
        let kr = render_verdict_kr(&v);
        assert!(kr.starts_with("✅ data-verify"));
        assert!(!kr.contains('\n'));
    }

    #[test]
    fn render_failure_lists_each_violation() {
        let refs = map(&[("ghost", &["x"]), ("orders", &["totals"])]);
        let schemas = map(&[("orders", &["total"])]);
        let v = verify_data_refs(&refs, &schemas);
        let kr = render_verdict_kr(&v);
        assert!(kr.contains("🛑 data-verify 실패"));
        assert!(kr.contains("table `ghost` 가 앱에 없어요"));
        assert!(kr.contains("`orders.totals` column"));
    }

    #[test]
    fn parses_ref_map_json() {
        let refs = parse_ref_map(r#"{"orders":["id","total"],"users":["email"]}"#).unwrap();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs["orders"], vec!["id", "total"]);
    }

    #[test]
    fn verdict_serializes_violation_without_null_column() {
        let v = verify_data_refs(&map(&[("ghost", &["x"])]), &RefMap::new());
        let json = serde_json::to_string(&v).unwrap();
        // missing_table violation omits the column field entirely
        assert!(json.contains("\"kind\":\"missing_table\""));
        assert!(!json.contains("\"column\""));
    }
}

//! Plan v6 §14 — layering rule enforcement for `diagnose/*`.
//!
//! `diagnose/*` MAY import: consent::{decision,jwt,key}, audit_ledger, redact,
//!                          recovery_scan, event_log, telemetry, hook_safety,
//!                          atomic_jsonl, runtime_paths, plus std/external deps.
//! `diagnose/*` MUST NOT import: bootstrap, list_deployments, deploy_prep,
//!                                statusline, the existing
//!                                `crate::preflight` (deploy preflight; the
//!                                diagnose preflight lives at
//!                                `crate::diagnose::preflight`).
//!
//! This test scans every `.rs` file under `crates/axhub-helpers/src/diagnose/`
//! and rejects forbidden `use crate::<name>` paths.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN_CRATE_MODULES: &[&str] = &[
    "bootstrap",
    "list_deployments",
    "deploy_prep",
    "statusline",
    // Deploy preflight — the diagnose preflight is `crate::diagnose::preflight`.
    "preflight",
];

fn diagnose_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join("diagnose")
}

fn walk_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !dir.exists() {
        return out;
    }
    for entry in fs::read_dir(dir).expect("read diagnose dir") {
        let entry = entry.expect("entry");
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_rust_files(&path));
        } else if path.extension().map_or(false, |e| e == "rs") {
            out.push(path);
        }
    }
    out
}

#[test]
fn diagnose_modules_dont_import_forbidden_layers() {
    let dir = diagnose_dir();
    assert!(dir.exists(), "diagnose dir must exist at {dir:?}");
    let files = walk_rust_files(&dir);
    assert!(!files.is_empty(), "must find at least one .rs file in diagnose/");

    let mut violations = Vec::new();

    for file in &files {
        let contents = fs::read_to_string(file).expect("read source");
        for (line_no, line) in contents.lines().enumerate() {
            let trimmed = line.trim_start();
            // Skip comments & doc-comments.
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            // We only care about `use crate::X` and direct `crate::X::...` references.
            for forbidden in FORBIDDEN_CRATE_MODULES {
                // Match `use crate::forbidden` or `use crate::forbidden::` or `crate::forbidden::`
                let pat_use = format!("use crate::{forbidden}");
                let pat_path = format!("crate::{forbidden}::");
                let pat_path_end = format!("crate::{forbidden};");
                if trimmed.starts_with(&pat_use)
                    || trimmed.contains(&pat_path)
                    || trimmed.contains(&pat_path_end)
                {
                    violations.push(format!(
                        "{}:{}: forbidden import of crate::{} → {}",
                        file.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                            .unwrap_or(file)
                            .display(),
                        line_no + 1,
                        forbidden,
                        trimmed
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "diagnose/ modules violate layering rule:\n  {}",
        violations.join("\n  ")
    );
}

#[test]
fn diagnose_dir_contains_expected_modules() {
    let dir = diagnose_dir();
    let must_exist = [
        "mod.rs",
        "state.rs",
        "signal.rs",
        "loop_builder.rs",
        "hitl.rs",
        "hypothesis.rs",
        "probe.rs",
        "instrument.rs",
        "fix.rs",
        "postmortem.rs",
        "learning.rs",
        "recurrence.rs",
        "preflight.rs",
    ];
    for f in must_exist {
        let path = dir.join(f);
        assert!(path.exists(), "expected diagnose/{f} to exist");
    }
    // Probe impls live in subdir.
    let probe_dir = dir.join("probe");
    for f in ["env_var.rs", "loop_shadow.rs", "manifest.rs"] {
        let path = probe_dir.join(f);
        assert!(path.exists(), "expected diagnose/probe/{f} to exist");
    }
}

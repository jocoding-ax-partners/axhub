// Phase 3 — migrate-guard deterministic git safety net (D2 = git-guarded
// preview-first). These tests cover the branchy failure modes the guard exists
// to handle so the LLM expert never has to: clean / dirty / non-git, plus the
// rollback-command surface. The expert's generated wrapper is NOT tested here
// (it is an LLM gated by preview + approval at runtime) — this is the
// deterministic ring around it.

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_axhub-helpers");

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.email=test@axhub.local", "-c", "user.name=test"])
        .args(args)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?} failed in {dir:?}");
}

fn guard(dir: &Path, extra: &[&str]) -> serde_json::Value {
    let mut args: Vec<&str> = vec!["migrate-guard", "--dir", dir.to_str().unwrap(), "--json"];
    args.extend_from_slice(extra);
    let out = Command::new(BIN).args(&args).output().unwrap();
    assert!(
        out.status.success(),
        "migrate-guard must exit 0 (fail-open)"
    );
    serde_json::from_slice(&out.stdout).expect("migrate-guard emits JSON")
}

fn git_repo_with_commit() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    git(tmp.path(), &["init", "-q"]);
    std::fs::write(tmp.path().join("f.py"), "x\n").unwrap();
    git(tmp.path(), &["add", "-A"]);
    git(tmp.path(), &["commit", "-q", "-m", "init"]);
    tmp
}

#[test]
fn non_git_probe_needs_decision() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("main.py"), "x").unwrap();
    let v = guard(tmp.path(), &[]);
    assert_eq!(v["mode"], "no_git");
    assert_eq!(v["inside_git"], false);
    assert_eq!(v["needs_decision"], true);
    assert!(v["checkpoint_ref"].is_null());
}

#[test]
fn non_git_checkpoint_requires_init_ok() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("main.py"), "x").unwrap();
    let v = guard(tmp.path(), &["--checkpoint"]);
    assert_eq!(v["ok"], false);
    assert_eq!(v["mode"], "no_git");
    assert!(v["checkpoint_ref"].is_null());
}

#[test]
fn non_git_init_ok_creates_checkpoint() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("main.py"), "x").unwrap();
    let v = guard(tmp.path(), &["--checkpoint", "--init-ok"]);
    assert_eq!(v["ok"], true);
    assert_eq!(v["mode"], "no_git_init");
    assert_eq!(v["checkpoint_ref"].as_str().unwrap().len(), 40);
    assert!(v["rollback_command"]
        .as_str()
        .unwrap()
        .contains("reset --hard"));
    assert!(tmp.path().join(".git").exists());
}

#[test]
fn git_clean_checkpoints_head() {
    let tmp = git_repo_with_commit();
    let v = guard(tmp.path(), &["--checkpoint"]);
    assert_eq!(v["ok"], true);
    assert_eq!(v["mode"], "git_clean");
    assert_eq!(v["checkpoint_ref"].as_str().unwrap().len(), 40);
    assert!(v["rollback_command"]
        .as_str()
        .unwrap()
        .contains("reset --hard"));
}

#[test]
fn git_dirty_refuses_without_allow_dirty() {
    let tmp = git_repo_with_commit();
    std::fs::write(tmp.path().join("f.py"), "changed\n").unwrap();
    let probe = guard(tmp.path(), &[]);
    assert_eq!(probe["mode"], "git_dirty");
    assert_eq!(probe["dirty"], true);
    let refused = guard(tmp.path(), &["--checkpoint"]);
    assert_eq!(refused["ok"], false);
    assert_eq!(refused["mode"], "git_dirty");
    assert!(refused["checkpoint_ref"].is_null());
}

#[test]
fn git_dirty_allow_dirty_wip_commits_and_cleans_tree() {
    let tmp = git_repo_with_commit();
    std::fs::write(tmp.path().join("f.py"), "changed\n").unwrap();
    let v = guard(tmp.path(), &["--checkpoint", "--allow-dirty"]);
    assert_eq!(v["ok"], true);
    assert_eq!(v["mode"], "git_dirty_wip");
    assert_eq!(v["checkpoint_ref"].as_str().unwrap().len(), 40);
    // the pre-existing change is saved in the WIP commit → tree is clean again
    let after = guard(tmp.path(), &[]);
    assert_eq!(after["mode"], "git_clean");
}

#[test]
fn missing_dir_fails_open() {
    let out = Command::new(BIN)
        .args([
            "migrate-guard",
            "--dir",
            "/nonexistent/axhub/guard/path",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["mode"], "missing_dir");
    assert_eq!(v["ok"], false);
}

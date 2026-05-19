// Phase 3.1 (T7) post-install regression — verifies the Rust handler that
// absorbed `.gitignore` + post-commit hook + disclosure marker write from
// install.{sh,ps1}.
//
// Tests assert:
//   - .gitignore creation when absent
//   - .gitignore idempotent append when `.axhub-state/` entry missing
//   - .gitignore skip when entry already present (idempotency)
//   - post-commit hook creation when absent
//   - post-commit hook detect existing → stderr warning + skip
//   - AXHUB_POSTCOMMIT_INSTALL=append → append to existing hook
//   - AXHUB_NO_DISCLOSURE=1 / AXHUB_SKIP_AUTODOWNLOAD=1 → no marker write
//   - Disclosure marker version = current release (drift detection — codex finding #9)

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn make_repo() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    let git_dir = repo.path().join(".git");
    fs::create_dir_all(git_dir.join("hooks")).unwrap();
    // Minimal .git/HEAD so `git rev-parse` recognises this as a repo (callers
    // pass --repo-root explicitly, so no live git invocation runs).
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    repo
}

fn make_target(repo: &std::path::Path) -> (PathBuf, PathBuf, String) {
    let bin_dir = repo.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let target_name = "axhub-helpers-test-arm64".to_string();
    let target_path = bin_dir.join(&target_name);
    fs::write(&target_path, b"#!/bin/sh\necho stub\n").unwrap();
    let link_path = bin_dir.join(if cfg!(windows) {
        "axhub-helpers.exe"
    } else {
        "axhub-helpers"
    });
    (bin_dir, link_path, target_name)
}

fn run_post_install(repo: &std::path::Path, envs: &[(&str, &str)]) -> Output {
    let (bin_dir, link_path, target_name) = make_target(repo);
    let mut cmd = Command::new(bin());
    cmd.arg("post-install")
        .args(["--target-name", &target_name])
        .args(["--bin-dir", bin_dir.to_str().unwrap()])
        .args(["--link-path", link_path.to_str().unwrap()])
        .args(["--repo-root", repo.to_str().unwrap()]);
    for key in [
        "AXHUB_NO_DISCLOSURE",
        "AXHUB_SKIP_AUTODOWNLOAD",
        "AXHUB_POSTCOMMIT_INSTALL",
        "XDG_STATE_HOME",
        "HOME",
    ] {
        cmd.env_remove(key);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

#[test]
fn gitignore_created_when_absent() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    let out = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
        ],
    );
    assert_eq!(out.status.code(), Some(0));
    let body = fs::read_to_string(repo.path().join(".gitignore")).unwrap();
    assert!(
        body.contains(".axhub-state/"),
        ".gitignore should contain entry: {body:?}"
    );
}

#[test]
fn gitignore_idempotent_skip_when_entry_present() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    fs::write(
        repo.path().join(".gitignore"),
        "# user content\n.axhub-state/\n",
    )
    .unwrap();
    let before = fs::read_to_string(repo.path().join(".gitignore")).unwrap();
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
        ],
    );
    let after = fs::read_to_string(repo.path().join(".gitignore")).unwrap();
    assert_eq!(
        before, after,
        ".gitignore should not change when entry already present"
    );
}

#[test]
fn gitignore_appends_entry_when_missing() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    fs::write(
        repo.path().join(".gitignore"),
        "# user content\nnode_modules/\n",
    )
    .unwrap();
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
        ],
    );
    let body = fs::read_to_string(repo.path().join(".gitignore")).unwrap();
    assert!(
        body.contains("node_modules/") && body.contains(".axhub-state/"),
        "existing entries should be preserved + new entry appended: {body:?}"
    );
    // 두 번 호출 → 여전히 한 번만 추가됨 (idempotent).
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
        ],
    );
    let body2 = fs::read_to_string(repo.path().join(".gitignore")).unwrap();
    let count = body2.matches(".axhub-state/").count();
    assert_eq!(
        count, 1,
        "should appear exactly once after re-run, got {count}"
    );
}

#[test]
fn post_commit_hook_created_when_absent() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
        ],
    );
    let body = fs::read_to_string(repo.path().join(".git/hooks/post-commit")).unwrap();
    assert!(body.contains("state-update --post-commit-promote"));
}

#[test]
fn post_commit_hook_detect_existing_warns_and_skips() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    let hook = repo.path().join(".git/hooks/post-commit");
    fs::write(&hook, "#!/usr/bin/env bash\n# user hook\necho hello\n").unwrap();
    let before = fs::read_to_string(&hook).unwrap();
    let out = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("기존 .git/hooks/post-commit 감지됨"),
        "expected detection warning, got: {stderr}"
    );
    let after = fs::read_to_string(&hook).unwrap();
    assert_eq!(before, after, "user hook should be left intact");
}

#[test]
fn post_commit_hook_append_mode_opts_in() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    let hook = repo.path().join(".git/hooks/post-commit");
    fs::write(&hook, "#!/usr/bin/env bash\n# user hook\necho hello\n").unwrap();
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
            ("AXHUB_POSTCOMMIT_INSTALL", "append"),
        ],
    );
    let body = fs::read_to_string(&hook).unwrap();
    assert!(body.contains("echo hello"), "user content should remain");
    assert!(
        body.contains("state-update --post-commit-promote"),
        "axhub line should be appended"
    );
}

#[test]
fn disclosure_marker_written_with_release_version_when_not_suppressed() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
        ],
    );
    let marker = state
        .path()
        .join("axhub-plugin/install-disclosure-shown.txt");
    assert!(
        marker.exists(),
        "disclosure marker should be written when AXHUB_NO_DISCLOSURE is unset"
    );
    let body = fs::read_to_string(&marker).unwrap();
    let pkg_version = env!("CARGO_PKG_VERSION");
    assert!(
        body.trim().contains(&format!("v{pkg_version}")),
        "marker version should match CARGO_PKG_VERSION ({pkg_version}), got: {body:?}"
    );
}

#[test]
fn disclosure_marker_skipped_when_axhub_no_disclosure_set() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_NO_DISCLOSURE", "1"),
        ],
    );
    let marker = state
        .path()
        .join("axhub-plugin/install-disclosure-shown.txt");
    assert!(
        !marker.exists(),
        "AXHUB_NO_DISCLOSURE=1 should suppress marker write"
    );
}

#[test]
fn disclosure_marker_skipped_when_axhub_skip_autodownload_set() {
    let repo = make_repo();
    let state = tempfile::tempdir().unwrap();
    let _ = run_post_install(
        repo.path(),
        &[
            ("HOME", repo.path().to_str().unwrap()),
            ("XDG_STATE_HOME", state.path().to_str().unwrap()),
            ("AXHUB_SKIP_AUTODOWNLOAD", "1"),
        ],
    );
    let marker = state
        .path()
        .join("axhub-plugin/install-disclosure-shown.txt");
    assert!(
        !marker.exists(),
        "AXHUB_SKIP_AUTODOWNLOAD=1 should suppress marker write (codex finding #10)"
    );
}

//! Integration tests for settings_merge — 10 cases covering all 7 branches.
//!
//! Each test uses a TempDir as the fake HOME to isolate file state.
//! ENV_LOCK serializes env-var mutations so parallel test threads don't race.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use axhub_helpers::settings_merge::{
    command_for_platform_script, default_command_path, merge, MergeOptions, MergeOutcome, Scope,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct HomeGuard {
    dir: tempfile::TempDir,
    old_home: Option<std::ffi::OsString>,
    old_userprofile: Option<std::ffi::OsString>,
    // Keep lock alive for the duration of the test.
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl HomeGuard {
    fn new() -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let dir = tempfile::tempdir().expect("TempDir creation failed");
        let old_home = std::env::var_os("HOME");
        let old_userprofile = std::env::var_os("USERPROFILE");
        unsafe {
            std::env::set_var("HOME", dir.path());
            std::env::set_var("USERPROFILE", dir.path());
        }
        Self {
            dir,
            old_home,
            old_userprofile,
            _lock: lock,
        }
    }

    fn claude_dir(&self) -> PathBuf {
        self.dir.path().join(".claude")
    }

    fn settings_path(&self) -> PathBuf {
        self.claude_dir().join("settings.json")
    }

    fn ensure_claude_dir(&self) {
        fs::create_dir_all(self.claude_dir()).expect("claude dir creation failed");
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        unsafe {
            match self.old_home.take() {
                Some(h) => std::env::set_var("HOME", h),
                None => std::env::remove_var("HOME"),
            }
            match self.old_userprofile.take() {
                Some(h) => std::env::set_var("USERPROFILE", h),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
    }
}

fn user_opts(dry_run: bool) -> MergeOptions {
    MergeOptions {
        silent: true,
        command_path_override: None,
        scope: Scope::User,
        dry_run,
    }
}

fn user_apply() -> MergeOptions {
    user_opts(false)
}

fn user_dry_run() -> MergeOptions {
    user_opts(true)
}

#[test]
fn windows_script_paths_use_explicit_powershell_bypass_command() {
    let command = command_for_platform_script("${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1", true);
    assert_eq!(
        command,
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1\""
    );

    let orphan_stub = command_for_platform_script(
        r"C:\Users\me\AppData\Local\axhub-plugin\orphan-stub-statusline.ps1",
        true,
    );
    assert!(
        orphan_stub.starts_with("powershell.exe -NoProfile -ExecutionPolicy Bypass -File "),
        "Windows orphan stub command must not be a bare .ps1 path"
    );
}

#[test]
fn unix_script_paths_stay_literal_for_shell_execution() {
    assert_eq!(
        command_for_platform_script("${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh", false),
        "${CLAUDE_PLUGIN_ROOT}/bin/statusline.sh"
    );
}

// ---------------------------------------------------------------------------
// Branch 1 — file absent → Created
// ---------------------------------------------------------------------------

#[test]
fn branch_1_file_absent_creates() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();

    let result = merge(user_apply()).unwrap();
    assert_eq!(result, MergeOutcome::Created);

    let content = fs::read_to_string(home.settings_path()).expect("settings.json created");
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(v["statusLine"]["type"], "command");
    assert!(v["statusLine"]["command"]
        .as_str()
        .unwrap()
        .contains("statusline."));
    assert_eq!(v["statusLine"]["padding"], 0);
}

// ---------------------------------------------------------------------------
// Branch 2 — empty file → Created (same path as Branch 1)
// ---------------------------------------------------------------------------

#[test]
fn branch_2_empty_file_creates() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();
    fs::write(home.settings_path(), "   \n\t  ").unwrap(); // whitespace only

    let result = merge(user_apply()).unwrap();
    assert_eq!(result, MergeOutcome::Created);

    let content = fs::read_to_string(home.settings_path()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(v["statusLine"].is_object());
}

// ---------------------------------------------------------------------------
// Branch 3 — valid JSON, statusLine absent → Merged (other keys preserved)
// ---------------------------------------------------------------------------

#[test]
fn branch_3_merge_adds_statusline() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();
    fs::write(
        home.settings_path(),
        r#"{"otherKey":"preserved","nested":{"x":1}}"#,
    )
    .unwrap();

    let result = merge(user_apply()).unwrap();
    assert_eq!(result, MergeOutcome::Merged);

    let content = fs::read_to_string(home.settings_path()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&content).unwrap();
    // statusLine added
    assert!(v["statusLine"].is_object());
    assert_eq!(v["statusLine"]["type"], "command");
    // pre-existing keys preserved (shallow merge — no mutation of other keys)
    assert_eq!(v["otherKey"], "preserved");
    assert_eq!(v["nested"]["x"], 1);
    // .bak created
    assert!(home.settings_path().with_extension("json.bak").exists());
}

// ---------------------------------------------------------------------------
// Branch 4 — statusLine.command matches default → NoOp (mtime unchanged)
// ---------------------------------------------------------------------------

#[test]
fn branch_4_idempotent_noop() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();

    let cmd = default_command_path(None);
    let fixture = serde_json::json!({
        "statusLine": { "type": "command", "command": cmd, "padding": 0 }
    })
    .to_string();
    fs::write(home.settings_path(), &fixture).unwrap();

    let mtime_before = fs::metadata(home.settings_path())
        .unwrap()
        .modified()
        .unwrap();

    let result = merge(user_apply()).unwrap();
    assert_eq!(result, MergeOutcome::NoOp);

    let mtime_after = fs::metadata(home.settings_path())
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(mtime_before, mtime_after, "NoOp must not change mtime");
}

// ---------------------------------------------------------------------------
// Branch 5 — statusLine.command is a different value → PreservedOther
// ---------------------------------------------------------------------------

#[test]
fn branch_5_preserve_other_plugin() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();

    fs::write(
        home.settings_path(),
        r#"{"statusLine":{"type":"command","command":"/usr/local/bin/other-plugin.sh","padding":0}}"#,
    )
    .unwrap();

    let result = merge(user_apply()).unwrap();
    assert_eq!(result, MergeOutcome::PreservedOther);

    // File must be untouched — original command still there.
    let content = fs::read_to_string(home.settings_path()).unwrap();
    assert!(content.contains("other-plugin.sh"));
}

// ---------------------------------------------------------------------------
// Branch 6 — invalid JSON → InvalidJson (never writes)
// ---------------------------------------------------------------------------

#[test]
fn branch_6_invalid_json_aborts() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();
    fs::write(home.settings_path(), r#"{ "broken json: "#).unwrap();

    let result = merge(user_apply()).unwrap();
    assert_eq!(result, MergeOutcome::InvalidJson);

    // File content must be unchanged.
    let content = fs::read_to_string(home.settings_path()).unwrap();
    assert_eq!(content, r#"{ "broken json: "#);
    // No .bak should be created.
    assert!(!home.settings_path().with_extension("json.bak").exists());
}

// ---------------------------------------------------------------------------
// Branch 7 — statusLine object but missing required fields → PartialSchema
// ---------------------------------------------------------------------------

#[test]
fn branch_7_partial_schema_preserved() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();

    // statusLine has type but no command.
    fs::write(home.settings_path(), r#"{"statusLine":{"type":"command"}}"#).unwrap();

    let result = merge(user_apply()).unwrap();
    assert_eq!(result, MergeOutcome::PartialSchema);

    // Also test: statusLine is not an object at all.
    fs::write(
        home.settings_path(),
        r#"{"statusLine":"string-not-object"}"#,
    )
    .unwrap();
    let result2 = merge(user_apply()).unwrap();
    assert_eq!(result2, MergeOutcome::PartialSchema);

    // File untouched in both cases.
    let content = fs::read_to_string(home.settings_path()).unwrap();
    assert!(content.contains("string-not-object"));
}

// ---------------------------------------------------------------------------
// Branch 8 — parent dir not writable → PermissionDenied  (POSIX only)
// ---------------------------------------------------------------------------

#[test]
#[cfg(unix)]
fn branch_8_readonly_parent_aborts() {
    use std::os::unix::fs::PermissionsExt;

    let home = HomeGuard::new();
    let claude_dir = home.claude_dir();
    fs::create_dir_all(&claude_dir).unwrap();

    // Pre-create settings.json and lock file so fslock can open them even
    // after the directory is made non-writable.
    let settings = home.settings_path();
    let lock_file = settings.with_file_name("settings.json.lock");
    fs::write(&settings, r#"{"otherKey":true}"#).unwrap();
    fs::write(&lock_file, "").unwrap();

    // Remove write bit from the directory.
    fs::set_permissions(&claude_dir, fs::Permissions::from_mode(0o555)).unwrap();

    let result = merge(user_apply()).unwrap();

    // Restore before assertions so cleanup can remove the dir.
    fs::set_permissions(&claude_dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(result, MergeOutcome::PermissionDenied);
}

// ---------------------------------------------------------------------------
// dry_run — outcome returned but no file written
// ---------------------------------------------------------------------------

#[test]
fn dry_run_no_write() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();

    // File absent: dry_run should return Created but not create the file.
    let result = merge(user_dry_run()).unwrap();
    assert_eq!(result, MergeOutcome::Created);
    assert!(
        !home.settings_path().exists(),
        "dry_run must not create the file"
    );

    // File with valid JSON but no statusLine: dry_run should return Merged but not write.
    fs::write(home.settings_path(), r#"{"other":1}"#).unwrap();
    let mtime_before = fs::metadata(home.settings_path())
        .unwrap()
        .modified()
        .unwrap();
    let result2 = merge(user_dry_run()).unwrap();
    assert_eq!(result2, MergeOutcome::Merged);
    let mtime_after = fs::metadata(home.settings_path())
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(mtime_before, mtime_after, "dry_run must not modify file");
    assert!(!home.settings_path().with_extension("json.bak").exists());
}

// ---------------------------------------------------------------------------
// scope_auto_ambiguous — Auto scope with no matching plugin root → Err
// ---------------------------------------------------------------------------

#[test]
fn scope_auto_ambiguous_fails_closed() {
    let home = HomeGuard::new();
    home.ensure_claude_dir();
    // CLAUDE_PLUGIN_ROOT points to a path that matches neither user nor project plugins.
    unsafe {
        std::env::set_var("CLAUDE_PLUGIN_ROOT", "/tmp/__no_such_plugin_root__");
    }

    let opts = MergeOptions {
        silent: true,
        command_path_override: None,
        scope: Scope::Auto,
        dry_run: true,
    };
    let result = merge(opts);
    // Must return an Err (ambiguous scope → fail-closed).
    assert!(
        result.is_err(),
        "Auto scope with no matching plugin root must fail-closed: {result:?}"
    );

    // Cleanup env (HomeGuard Drop will restore HOME/USERPROFILE but not CLAUDE_PLUGIN_ROOT).
    unsafe {
        std::env::remove_var("CLAUDE_PLUGIN_ROOT");
    }
}

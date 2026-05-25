use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

fn run(args: &[&str]) -> Output {
    Command::new(bin()).args(args).output().unwrap()
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn snippet_mode_a_typescript_uses_cookie_auth_without_authorization_header() {
    let output = run(&[
        "snippet",
        "--mode",
        "A",
        "--language",
        "typescript",
        "--target",
        "web-axhub",
        "--connector",
        "snowflake",
        "--path",
        "공개/employees",
        "--sql",
        "SELECT id, name FROM employees",
        "--allowed-columns",
        "id,name,email",
        "--masked",
        "email",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        stderr_text(&output)
    );
    let stdout = stdout_text(&output);
    assert!(stdout.contains("mode=A"));
    assert!(
        stdout.contains("credentials: 'include'") || stdout.contains("credentials: \"include\"")
    );
    assert!(!stdout.contains("Authorization"));
    assert!(!stdout.contains("Bearer"));
    assert!(stdout.contains("allowed_columns=id,name,email"));
    assert!(stdout.contains("masked=email"));
    assert!(
        stdout.contains("%EA%B3%B5%EA%B0%9C%2Femployees") || stdout.contains("encodeURIComponent")
    );
}

#[test]
fn snippet_mode_b_python_uses_x_api_key_env_without_literal_pat_or_bearer() {
    let output = run(&[
        "snippet",
        "--mode",
        "B",
        "--language",
        "python",
        "--target",
        "local-python",
        "--tenant",
        "tenant-a",
        "--connector",
        "snowflake",
        "--path",
        "analytics/orders",
        "--sql",
        "SELECT id FROM orders WHERE note = 'don\\'t leak'",
        "--allowed-columns",
        "id,total",
        "--masked",
        "total",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        stderr_text(&output)
    );
    let stdout = stdout_text(&output);
    assert!(stdout.contains("mode=B"));
    assert!(stdout.contains("X-Api-Key"));
    assert!(stdout.contains("AXHUB_PAT"));
    assert!(!stdout.contains("Authorization: Bearer"));
    assert!(!stdout.contains("Bearer ey"));
    assert!(!stdout.contains("axhub_pat_"));
    assert!(stdout.contains("allowed_columns=id,total"));
    assert!(stdout.contains("masked=total"));
}

#[test]
fn snippet_local_bash_prefers_cli_keychain_backed_invoke() {
    let output = run(&[
        "snippet",
        "--mode",
        "B",
        "--language",
        "shell",
        "--target",
        "local-bash",
        "--connector",
        "snowflake",
        "--path",
        "analytics/orders",
        "--sql",
        "SELECT id FROM orders",
        "--allowed-columns",
        "id",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        stderr_text(&output)
    );
    let stdout = stdout_text(&output);
    assert!(stdout.contains("axhub catalog invoke"));
    assert!(stdout.contains("--execute"));
    assert!(stdout.contains("--json"));
    assert!(!stdout.contains("AXHUB_PAT"));
    assert!(!stdout.contains("X-Api-Key"));
}

#[cfg(unix)]
fn write_fake_axhub(dir: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("axhub");
    std::fs::write(
        &path,
        r#"#!/bin/sh
if [ "$1" = "catalog" ] && [ "$2" = "resources" ]; then
  cat <<'JSON'
{"resources":[{"connector":"snowflake","path":"analytics/orders","kind":"table","tags":["finance"]}]}
JSON
  exit 0
fi
if [ "$1" = "catalog" ] && [ "$2" = "get" ]; then
  cat <<'JSON'
{"connector":"snowflake","path":"analytics/orders","kind":"table","permissions":{"read":{"allowed_columns":["id","total","email"],"masked":[{"column":"email","mask":"Hash"}]}}}
JSON
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "whoami" ]; then
  cat <<'JSON'
{"tenant_id":"ten_1","tenant_slug":"acme","user_email":"dev@example.com","endpoint":"https://api.example.test"}
JSON
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "pat" ] && [ "$3" = "whoami" ]; then
  cat <<'JSON'
{"source":"env:AXHUB_PAT","fingerprint":"fp_123"}
JSON
  exit 0
fi
echo "unexpected fake axhub args: $*" >&2
exit 64
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
fn write_fake_axhub_detail_failure(dir: &std::path::Path) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("axhub-detail-failure");
    std::fs::write(
        &path,
        r#"#!/bin/sh
if [ "$1" = "catalog" ] && [ "$2" = "resources" ]; then
  cat <<'JSON'
{"resources":[{"connector":"snowflake","path":"analytics/orders","kind":"table"}]}
JSON
  exit 0
fi
if [ "$1" = "catalog" ] && [ "$2" = "get" ]; then
  echo '{"error":{"code":"catalog.internal_error","message":"detail failed"}}' >&2
  exit 70
fi
if [ "$1" = "auth" ] && [ "$2" = "whoami" ]; then
  echo '{"tenant_id":"ten_1","user_email":"dev@example.com"}'
  exit 0
fi
if [ "$1" = "auth" ] && [ "$2" = "pat" ] && [ "$3" = "whoami" ]; then
  echo '{"fingerprint":"fp_123"}'
  exit 0
fi
exit 64
"#,
    )
    .unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(unix)]
#[test]
fn sync_writes_git_root_catalog_with_private_snapshot_and_gitignore() {
    let temp = tempfile::tempdir().unwrap();
    let fake = write_fake_axhub(temp.path());
    let repo = temp.path().join("repo");
    let nested = repo.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo)
        .output()
        .unwrap();

    let output = Command::new(bin())
        .args(["sync", "--target", "local-python", "--json"])
        .current_dir(&nested)
        .env("AXHUB_BIN", &fake)
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    let axhub_dir = repo.join(".axhub");
    assert!(axhub_dir.join("AXHUB.md").exists());
    assert_eq!(
        std::fs::read_to_string(axhub_dir.join("AXHUB_TARGET")).unwrap(),
        "local-python\n"
    );
    let catalog_path = axhub_dir.join("catalog.json");
    let catalog: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&catalog_path).unwrap()).unwrap();
    assert_eq!(catalog["schema_version"], "1");
    assert_eq!(catalog["target"], "local-python");
    assert_eq!(catalog["principal"]["tenant_id"], "ten_1");
    assert_eq!(catalog["principal"]["user_email"], "dev@example.com");
    assert_eq!(
        catalog["resources"][0]["permissions"]["read"]["allowed_columns"][1],
        "total"
    );
    assert_eq!(
        catalog["resources"][0]["permissions"]["read"]["masked"][0]["mask"],
        "hash"
    );
    assert!(catalog["identity_fingerprint"]
        .as_str()
        .is_some_and(|s| !s.is_empty()));
    assert!(std::fs::read_to_string(repo.join(".gitignore"))
        .unwrap()
        .contains(".axhub/catalog.json"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(catalog_path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[cfg(unix)]
#[test]
fn sync_fails_closed_when_detail_fetch_fails_instead_of_writing_partial_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let fake = write_fake_axhub_detail_failure(temp.path());
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo)
        .output()
        .unwrap();

    let output = Command::new(bin())
        .args(["sync", "--target", "local-python", "--json"])
        .current_dir(&repo)
        .env("AXHUB_BIN", &fake)
        .output()
        .unwrap();

    assert_ne!(output.status.code(), Some(0));
    let stdout = stdout_text(&output);
    let body: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(body["status"], "error");
    assert!(
        body["command"].as_str().unwrap().contains("catalog get"),
        "stdout={stdout}"
    );
    assert_eq!(body["exit_code"], 70);
    assert!(
        body["cause"].as_str().unwrap().contains("catalog")
            || body["message"].as_str().unwrap().contains("catalog"),
        "stdout={stdout}"
    );
    assert!(
        !repo.join(".axhub/catalog.json").exists(),
        "partial catalog must not be written after detail failure"
    );
}

#[cfg(unix)]
#[test]
fn sync_auto_target_uses_out_dir_and_detects_web_project() {
    let temp = tempfile::tempdir().unwrap();
    let fake = write_fake_axhub(temp.path());
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    std::fs::write(repo.join("package.json"), r#"{"name":"web"}"#).unwrap();

    let output = Command::new(bin())
        .args([
            "sync",
            "--target",
            "auto",
            "--out",
            repo.to_str().unwrap(),
            "--json",
        ])
        .current_dir(temp.path())
        .env("AXHUB_BIN", &fake)
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        stdout_text(&output),
        stderr_text(&output)
    );
    let body: serde_json::Value = serde_json::from_str(&stdout_text(&output)).unwrap();
    assert_eq!(body["target"], "web-axhub");
    assert_eq!(
        std::fs::read_to_string(repo.join(".axhub/AXHUB_TARGET")).unwrap(),
        "web-axhub\n"
    );
}

#[test]
fn snippet_escapes_shell_and_typescript_sql_without_expanding_user_content() {
    let shell = run(&[
        "snippet",
        "--mode",
        "B",
        "--language",
        "shell",
        "--target",
        "local-bash",
        "--connector",
        "snowflake",
        "--path",
        "analytics/orders",
        "--sql",
        "SELECT id FROM orders WHERE note = 'don'",
        "--allowed-columns",
        "id",
    ]);
    assert_eq!(
        shell.status.code(),
        Some(0),
        "stderr={}",
        stderr_text(&shell)
    );
    let shell_out = stdout_text(&shell);
    assert!(shell_out.contains("\\''don"), "stdout={shell_out}");

    let ts = run(&[
        "snippet",
        "--mode",
        "B",
        "--language",
        "typescript",
        "--target",
        "local-node",
        "--connector",
        "snowflake",
        "--path",
        "analytics/orders",
        "--sql",
        "SELECT '${notAJsInterpolation}' AS value",
        "--allowed-columns",
        "value",
    ]);
    assert_eq!(ts.status.code(), Some(0), "stderr={}", stderr_text(&ts));
    let ts_out = stdout_text(&ts);
    assert!(ts_out.contains(r#"SELECT '${notAJsInterpolation}' AS value"#));
}

#[cfg(unix)]
#[test]
fn sync_blocks_silent_refresh_when_identity_changes_without_confirmation() {
    let temp = tempfile::tempdir().unwrap();
    let fake = write_fake_axhub(temp.path());
    let repo = temp.path().join("repo");
    std::fs::create_dir_all(repo.join(".axhub")).unwrap();
    std::fs::write(repo.join(".gitignore"), ".axhub/catalog.json\n").unwrap();
    std::fs::write(repo.join(".axhub/AXHUB_TARGET"), "local-python\n").unwrap();
    std::fs::write(
        repo.join(".axhub/catalog.json"),
        r#"{"schema_version":"1","target":"local-python","identity_fingerprint":"different","resources":[]}"#,
    )
    .unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&repo)
        .output()
        .unwrap();

    let output = Command::new(bin())
        .args(["sync", "--target", "local-python", "--json"])
        .current_dir(&repo)
        .env("AXHUB_BIN", &fake)
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(65));
    let body = stdout_text(&output);
    assert!(body.contains("identity_changed"), "stdout={body}");
}

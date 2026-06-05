use std::fs;

#[cfg(unix)]
#[test]
fn unix_path_line_uses_home_literal_for_default_axhub_dir() {
    let home = tempfile::tempdir().unwrap();
    let install_dir = home.path().join(".axhub/bin");

    let line = axhub_helpers::repair_path::unix_path_line(&install_dir, home.path());

    assert_eq!(line, r#"export PATH="$HOME/.axhub/bin:$PATH""#);
}

#[cfg(unix)]
#[test]
fn unix_rc_repair_is_idempotent_and_creates_backup() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let install_dir = home.join(".axhub/bin");
    let rc_path = home.join(".zshrc");
    fs::create_dir_all(&install_dir).unwrap();
    fs::write(&rc_path, "export PATH=\"/usr/local/bin:$PATH\"\n").unwrap();

    let first = axhub_helpers::repair_path::repair_unix_shell_rc(&rc_path, &install_dir, &home)
        .expect("first repair should succeed");
    assert!(first.repaired);
    assert!(!first.already_present);
    assert!(
        first.backup_path.as_deref().is_some_and(|p| p.exists()),
        "repair must back up an existing rc file before appending PATH"
    );
    let first_content = fs::read_to_string(&rc_path).unwrap();
    assert!(first_content.contains(r#"export PATH="$HOME/.axhub/bin:$PATH""#));

    let second = axhub_helpers::repair_path::repair_unix_shell_rc(&rc_path, &install_dir, &home)
        .expect("second repair should succeed");
    assert!(!second.repaired);
    assert!(second.already_present);
    let second_content = fs::read_to_string(&rc_path).unwrap();
    assert_eq!(
        first_content, second_content,
        "repair must be idempotent once the axhub PATH line exists"
    );
}

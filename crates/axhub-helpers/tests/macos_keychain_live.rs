#[test]
#[ignore = "requires a real macOS Keychain axhub item"]
fn macos_keychain_reads_existing_axhub_item() {
    if std::env::consts::OS != "macos" {
        eprintln!("skipping: macOS-only live keychain smoke");
        return;
    }

    let result = axhub_helpers::keychain::read_keychain_token();

    let token = result
        .token
        .as_deref()
        .expect("expected an axhub token from macOS Keychain");
    assert!(token.len() >= 16, "token should pass parser length floor");
    assert_eq!(result.source.as_deref(), Some("macos-keychain"));
    assert_eq!(result.error, None);
}

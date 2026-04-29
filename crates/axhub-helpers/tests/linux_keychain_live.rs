#[test]
#[ignore = "requires Linux Secret Service with a seeded axhub keychain item"]
fn linux_secret_service_reads_go_keyring_envelope() {
    if std::env::consts::OS != "linux" {
        eprintln!("skipping: Linux-only live keychain smoke");
        return;
    }

    let result = axhub_helpers::keychain::read_keychain_token();

    assert_eq!(
        result.token.as_deref(),
        Some("axhub_pat_linux_secret_service_live_smoke_token_0123456789")
    );
    assert_eq!(result.source.as_deref(), Some("linux-secret-service"));
    assert_eq!(result.error, None);
}

//! Phase 1 — `session_bundle` integration tests.
//!
//! Spec: `.plan/deploy-time-reduction/phase-1-rest-dedup-statusline.md` §8.1.

use std::time::{Duration, SystemTime};

use axhub_helpers::session_bundle::{
    read_session_bundle, read_session_bundle_at, read_session_bundle_or_empty,
    write_session_bundle, AuthStatusBundle, LastDeployBundle, SessionBundle,
    SESSION_BUNDLE_SCHEMA_VERSION,
};

fn fixture_bundle() -> SessionBundle {
    SessionBundle {
        schema_version: SESSION_BUNDLE_SCHEMA_VERSION.to_string(),
        auth_status: AuthStatusBundle {
            ok: true,
            user_email: Some("dev@jocodingax.ai".into()),
            user_id: Some(7),
            expires_at: Some("2099-01-01T00:00:00Z".into()),
            scopes: vec!["deploy:write".into()],
        },
        current_app: Some("paydrop".into()),
        current_env: Some("prod".into()),
        last_deploy: Some(LastDeployBundle {
            deployment_id: "deploy-xyz".into(),
            status: "success".into(),
            commit_sha: Some("abc123".into()),
        }),
        plugin_version: "0.5.4".into(),
        helper_version: "0.5.4".into(),
        written_at: "2026-05-08T09:00:00Z".into(),
    }
}

#[test]
fn write_and_read_roundtrip_preserves_fields() {
    let dir = std::env::temp_dir().join(format!("axhub-bundle-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session-bundle.json");

    let bundle = fixture_bundle();
    write_session_bundle(&bundle, &path).unwrap();
    let read_back = read_session_bundle(&path).unwrap();

    assert_eq!(read_back, bundle);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_session_bundle_rejects_expired_files() {
    let dir = std::env::temp_dir().join(format!("axhub-bundle-ttl-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session-bundle.json");

    write_session_bundle(&fixture_bundle(), &path).unwrap();

    // Inject a future "now" to force the file's mtime past the TTL without
    // touching filetime crates or sleeping in the test.
    let future_now = SystemTime::now() + Duration::from_secs(600);
    let err = read_session_bundle_at(&path, future_now)
        .unwrap_err()
        .to_string();
    assert!(err.contains("expired"), "expected expiry error, got {err}");

    // The default `read_session_bundle` (real `now`) should still succeed
    // for the same fresh file.
    let fresh = read_session_bundle(&path).unwrap();
    assert_eq!(fresh.current_app.as_deref(), Some("paydrop"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_session_bundle_or_empty_falls_back_on_corrupt_file() {
    let dir = std::env::temp_dir().join(format!("axhub-bundle-corrupt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("session-bundle.json");
    std::fs::write(&path, b"{ not json }").unwrap();

    let bundle = read_session_bundle_or_empty(&path);
    assert_eq!(bundle.schema_version, SESSION_BUNDLE_SCHEMA_VERSION);
    assert!(!bundle.auth_status.ok);
    assert!(bundle.current_app.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_session_bundle_or_empty_falls_back_on_missing_file() {
    let path =
        std::env::temp_dir().join(format!("axhub-missing-bundle-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let bundle = read_session_bundle_or_empty(&path);
    assert_eq!(bundle.schema_version, SESSION_BUNDLE_SCHEMA_VERSION);
    assert!(!bundle.auth_status.ok);
}

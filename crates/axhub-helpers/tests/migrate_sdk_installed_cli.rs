// Item 1C — runtime installed-SDK check. Reports whether the user app already
// depends on an AxHub SDK and at what version, so the conversion stage can warn
// on a version mismatch vs the vendored pack. Gated on presence: at first
// conversion the app has not adopted the SDK, so `present: false` is the
// expected state (not an error), and the 6-ecosystem parser only runs when an
// SDK dependency is actually there.

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_axhub-helpers");

fn installed(dir: &Path, lang: &str) -> serde_json::Value {
    let out = Command::new(BIN)
        .args([
            "migrate-sdk-installed",
            "--dir",
            dir.to_str().unwrap(),
            "--lang",
            lang,
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "migrate-sdk-installed must exit 0");
    serde_json::from_slice(&out.stdout).expect("emits JSON")
}

#[test]
fn absent_is_the_expected_first_conversion_state() {
    let tmp = tempfile::tempdir().unwrap();
    let v = installed(tmp.path(), "node");
    assert_eq!(v["present"], false);
    assert!(v["installed_version"].is_null());
    assert_eq!(v["ok"], true); // not an error — the normal pre-conversion state
}

#[test]
fn node_present_reports_version() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"dependencies":{"@ax-hub/sdk":"^2.0.0"}}"#,
    )
    .unwrap();
    let v = installed(tmp.path(), "node");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "2.0.0");
}

#[test]
fn python_present_from_requirements() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("requirements.txt"), "axhub-sdk==0.2.0\n").unwrap();
    let v = installed(tmp.path(), "python");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "0.2.0");
}

#[test]
fn go_present_from_go_mod() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("go.mod"),
        "module x\nrequire github.com/jocoding-ax-partners/axhub-sdk-go v0.2.0\n",
    )
    .unwrap();
    let v = installed(tmp.path(), "go");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "0.2.0");
}

#[test]
fn ruby_present_from_gemfile_lock() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Gemfile.lock"), "    axhub-sdk (0.2.0)\n").unwrap();
    let v = installed(tmp.path(), "ruby");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "0.2.0");
}

#[test]
fn java_present_from_gradle() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("build.gradle"),
        "implementation 'ai.axhub:axhub-sdk-java:0.2.0'\n",
    )
    .unwrap();
    let v = installed(tmp.path(), "java");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "0.2.0");
}

#[test]
fn kotlin_present_from_gradle_kts() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("build.gradle.kts"),
        "implementation(\"ai.axhub:axhub-sdk-kotlin:0.2.1\")\n",
    )
    .unwrap();
    let v = installed(tmp.path(), "kotlin");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "0.2.1");
}

#[test]
fn present_without_parseable_version_is_still_present() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"dependencies":{"@ax-hub/sdk":"latest"}}"#,
    )
    .unwrap();
    let v = installed(tmp.path(), "node");
    assert_eq!(v["present"], true);
    assert!(v["installed_version"].is_null());
}

#[test]
fn unsupported_lang_is_handled() {
    let tmp = tempfile::tempdir().unwrap();
    let v = installed(tmp.path(), "rust");
    assert_eq!(v["ok"], false);
    assert_eq!(v["present"], false);
}

// Fallback-manifest coverage: version comes from the secondary source (lockfile /
// pom) when the primary manifest entry has no parseable version.

#[test]
fn node_version_from_package_lock_v3_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    // npm lockfileVersion 3 keys deps as "node_modules/@ax-hub/sdk" (no leading quote
    // immediately before @) — the fallback regex must still find the version.
    std::fs::write(
        tmp.path().join("package-lock.json"),
        r#"{"packages":{"node_modules/@ax-hub/sdk":{"version":"2.0.0"}}}"#,
    )
    .unwrap();
    let v = installed(tmp.path(), "node");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "2.0.0");
}

#[test]
fn java_version_from_pom_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("pom.xml"),
        "<dependency><groupId>ai.axhub</groupId><artifactId>axhub-sdk-java</artifactId><version>0.2.0</version></dependency>",
    )
    .unwrap();
    let v = installed(tmp.path(), "java");
    assert_eq!(v["present"], true);
    assert_eq!(v["installed_version"], "0.2.0");
}

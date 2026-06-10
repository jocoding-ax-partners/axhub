//! `.mcp.json` idempotent 설치/머지 helper (Track H §D.2).
//!
//! 사용자 프로젝트 루트의 `.mcp.json` 에 axhub 두 서버 항목만 추가/갱신하고 사용자
//! 항목은 byte 보존해요. `settings_merge.rs`(Claude settings 전용)와 별개예요.
//!
//! 서버 2종:
//! - **local** (`axhub-helpers`): `axhub-helpers mcp-serve` stdio — 로컬 코드 정적 검증.
//! - **remote** (`axhub`): ax-mcp streamable-http — 원격 SDK 지식/스키마 검색.
//!   URL 은 `AXHUB_MCP_URL` env 로 override 가능, 기본값은 현 prod Cloud Run.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

/// 원격 ax-mcp 기본 URL — **canonical 도메인 확정** (2026-06-10, 사용자 승인).
// canonical 주소 = https://mcp.jocodingax.ai/mcp. probe(2026-06-10): DNS ✓ / TCP 443
// ✓ / TLS handshake reset(인증서 아직 미활성). TLS 활성화 대기 중 — 미활성 환경에서는
// `AXHUB_MCP_URL` env 로 다른 엔드포인트(예: Cloud Run 직주소)를 override 할 수 있어요.
const DEFAULT_REMOTE_MCP_URL: &str = "https://mcp.jocodingax.ai/mcp";

/// remote MCP URL override env. 비어있지 않을 때만 적용.
const ENV_REMOTE_MCP_URL: &str = "AXHUB_MCP_URL";

const LOCAL_SERVER_KEY: &str = "axhub-helpers";
const REMOTE_SERVER_KEY: &str = "axhub";

/// 적용할 remote URL — env override 우선, 없으면 기본값.
pub fn remote_mcp_url() -> String {
    std::env::var(ENV_REMOTE_MCP_URL)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_REMOTE_MCP_URL.to_string())
}

/// 기존 `.mcp.json` 문자열(없으면 None)에 우리 두 서버 항목만 추가/갱신하고 나머지는
/// 보존한 결과 JSON 을 만들어요. 멱등(같은 입력 → 같은 출력).
pub fn merge_mcp_json(existing: Option<&str>, local_command: &str) -> Result<String> {
    let mut root: Value = match existing {
        Some(s) if !s.trim().is_empty() => {
            serde_json::from_str(s).context(".mcp.json 파싱 실패 (수동 확인 필요)")?
        }
        _ => json!({}),
    };
    if !root.is_object() {
        bail!(".mcp.json 최상위가 JSON object 가 아니에요");
    }
    let obj = root.as_object_mut().context(".mcp.json object 접근 실패")?;
    let servers_entry = obj.entry("mcpServers").or_insert_with(|| json!({}));
    if !servers_entry.is_object() {
        bail!(".mcp.json 의 mcpServers 가 object 가 아니에요");
    }
    let servers = servers_entry
        .as_object_mut()
        .context("mcpServers object 접근 실패")?;

    // 우리 두 키만 set/갱신 — 사용자의 다른 mcpServers 항목은 그대로 보존돼요.
    servers.insert(
        LOCAL_SERVER_KEY.to_string(),
        json!({ "command": local_command, "args": ["mcp-serve"] }),
    );
    servers.insert(
        REMOTE_SERVER_KEY.to_string(),
        json!({ "type": "http", "url": remote_mcp_url() }),
    );

    Ok(serde_json::to_string_pretty(&root)?)
}

/// `merged` 를 temp 파일에 쓴 뒤 rename 으로 원자 교체해요 — 쓰기 도중 크래시가
/// 나도 기존 `.mcp.json` 이 반쯤 깨진 상태로 남지 않아요.
fn write_atomic(path: &std::path::Path, content: &str) -> Result<()> {
    let tmp = path.with_file_name(".mcp.json.tmp");
    std::fs::write(&tmp, content).with_context(|| format!("{} 쓰기 실패", tmp.display()))?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp); // best-effort 청소
        return Err(e).with_context(|| format!("{} 교체 실패", path.display()));
    }
    Ok(())
}

/// `mcp-install [--dir <d>] [--command <c>]` 진입점. `<dir>/.mcp.json` 을 머지(없으면
/// 생성)해요. 미설치/미연결 환경은 차단하지 않고 안내만 — packs floor 무손상.
pub fn run_mcp_install(dir: Option<String>, local_command: Option<String>) -> Result<i32> {
    let dir = match dir {
        Some(d) => PathBuf::from(d),
        None => std::env::current_dir()?,
    };
    let path = dir.join(".mcp.json");
    // NotFound 만 "신규 생성" — 그 외(비UTF8 InvalidData, PermissionDenied 등)는
    // 기존 파일이 있는데 못 읽은 것이라 덮어쓰면 사용자 설정이 파괴돼요 → bail.
    let existing = match std::fs::read_to_string(&path) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => bail!(
            "{} 읽기 실패({e}) — 기존 파일을 덮어쓰지 않으려고 중단해요. 인코딩/권한을 확인해 주세요.",
            path.display()
        ),
    };
    let command = local_command.unwrap_or_else(|| "axhub-helpers".to_string());
    let merged = merge_mcp_json(existing.as_deref(), &command)?;
    write_atomic(&path, &format!("{merged}\n"))?;
    println!(
        "✅ .mcp.json 갱신: {} (local: `{} mcp-serve`, remote: {})",
        path.display(),
        command,
        remote_mcp_url()
    );
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn servers(s: &str) -> serde_json::Map<String, Value> {
        let v: Value = serde_json::from_str(s).unwrap();
        v.get("mcpServers").unwrap().as_object().unwrap().clone()
    }

    #[test]
    fn creates_both_servers_from_empty() {
        let out = merge_mcp_json(None, "axhub-helpers").unwrap();
        let m = servers(&out);
        assert!(m.contains_key("axhub-helpers"), "local 항목");
        assert!(m.contains_key("axhub"), "remote 항목");
        assert_eq!(m["axhub-helpers"]["args"][0], "mcp-serve");
        assert_eq!(m["axhub"]["type"], "http");
    }

    #[test]
    fn preserves_existing_user_servers() {
        let existing =
            r#"{"mcpServers":{"my-tool":{"command":"my-tool","args":["x"]}},"other":42}"#;
        let out = merge_mcp_json(Some(existing), "axhub-helpers").unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        // 사용자 항목 보존
        assert_eq!(v["mcpServers"]["my-tool"]["command"], "my-tool");
        assert_eq!(v["mcpServers"]["my-tool"]["args"][0], "x");
        // 최상위 다른 키도 보존
        assert_eq!(v["other"], 42);
        // 우리 항목 추가
        assert!(v["mcpServers"]["axhub-helpers"].is_object());
        assert!(v["mcpServers"]["axhub"].is_object());
    }

    #[test]
    fn idempotent_second_merge_equals_first() {
        // merge_mcp_json 은 remote_mcp_url()로 env 를 읽어요. 병렬 env_override 테스트와
        // 레이스하지 않게 두 merge 를 같은 lock 안에서 수행해요(일관된 env 관찰).
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let first = merge_mcp_json(None, "axhub-helpers").unwrap();
        let second = merge_mcp_json(Some(&first), "axhub-helpers").unwrap();
        assert_eq!(first, second, "멱등이어야 해요");
    }

    #[test]
    fn updates_our_keys_without_touching_user_keys() {
        // 우리 키가 이미 있고 사용자가 옆에 항목을 둔 경우 — 우리 것만 갱신.
        let existing = r#"{"mcpServers":{"axhub-helpers":{"command":"OLD","args":["stale"]},"keep":{"command":"k"}}}"#;
        let out = merge_mcp_json(Some(existing), "axhub-helpers").unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["mcpServers"]["axhub-helpers"]["args"][0], "mcp-serve");
        assert_eq!(v["mcpServers"]["keep"]["command"], "k");
    }

    #[test]
    fn rejects_non_object_root() {
        assert!(merge_mcp_json(Some("[1,2,3]"), "axhub-helpers").is_err());
    }

    #[test]
    fn install_bails_on_unreadable_existing_and_preserves_original() {
        // 비UTF8 기존 파일(read_to_string → InvalidData) → bail + 원본 byte 보존.
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".mcp.json");
        let original: &[u8] = &[0xFF, 0xFE, b'{', 0x80, b'}'];
        std::fs::write(&path, original).unwrap();
        let r = run_mcp_install(Some(dir.path().to_string_lossy().into_owned()), None);
        assert!(r.is_err(), "비UTF8 기존 파일은 bail 이어야 해요");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            original,
            "bail 시 원본이 byte 그대로 보존돼야 해요"
        );
    }

    #[test]
    fn install_creates_new_file_when_missing() {
        // NotFound 는 "신규 생성" 경로 — bail 하지 않고 양 서버 항목을 써요.
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        std::env::remove_var(ENV_REMOTE_MCP_URL);
        let dir = tempfile::tempdir().unwrap();
        let code = run_mcp_install(
            Some(dir.path().to_string_lossy().into_owned()),
            Some("axhub-helpers".to_string()),
        )
        .unwrap();
        assert_eq!(code, 0);
        let written = std::fs::read_to_string(dir.path().join(".mcp.json")).unwrap();
        let m = servers(&written);
        assert!(m.contains_key("axhub-helpers"), "local 항목");
        assert!(m.contains_key("axhub"), "remote 항목");
        assert!(
            !dir.path().join(".mcp.json.tmp").exists(),
            "temp 파일이 남으면 안 돼요(rename 완료)"
        );
    }

    #[test]
    fn env_override_changes_remote_url() {
        let _guard = crate::PROCESS_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        std::env::set_var(ENV_REMOTE_MCP_URL, "https://custom.example/mcp");
        let out = merge_mcp_json(None, "axhub-helpers").unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(
            v["mcpServers"]["axhub"]["url"],
            "https://custom.example/mcp"
        );
        std::env::remove_var(ENV_REMOTE_MCP_URL);
    }
}

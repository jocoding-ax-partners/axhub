// Track H frontend 3 — `mcp-serve` stdio JSON-RPC 통합 테스트.
//
// 실제 바이너리를 spawn 해 initialize → initialized → tools/list 핸드셰이크를
// stdin 으로 밀어넣고(then EOF), 서버가 두 tool(`validate`/`scan_sites`)을
// 광고하는지 검증해요. stdin EOF 로 transport 가 닫히면 서버가 종료돼요 —
// 행 방지를 위해 별도 wait 루프 없이 wait_with_output 으로 충분해요.

#![cfg(feature = "mcp")]

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_axhub-helpers")
}

#[test]
fn mcp_serve_initialize_and_tools_list() {
    let mut child = Command::new(bin())
        .arg("mcp-serve")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let requests = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e-test","version":"0.0.0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        "\n",
    );
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(requests.as_bytes())
        .unwrap();
    // stdin drop(EOF) → transport close → 서버 종료.
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "mcp-serve 는 EOF 후 정상 종료해야 해요"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut saw_initialize = false;
    let mut saw_tools = false;
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(msg) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        match msg["id"].as_i64() {
            Some(1) => {
                // initialize 응답 — serverInfo + tools capability.
                assert!(
                    msg["result"]["capabilities"]["tools"].is_object(),
                    "initialize 응답에 tools capability 가 있어야 해요: {line}"
                );
                assert!(
                    msg["result"]["serverInfo"]["name"].is_string(),
                    "initialize 응답에 serverInfo 가 있어야 해요: {line}"
                );
                saw_initialize = true;
            }
            Some(2) => {
                let tools = msg["result"]["tools"].as_array().unwrap_or_else(|| {
                    panic!("tools/list 응답에 tools 배열이 있어야 해요: {line}")
                });
                let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
                assert!(
                    names.contains(&"validate") && names.contains(&"scan_sites"),
                    "validate/scan_sites 두 tool 이 광고돼야 해요, got {names:?}"
                );
                saw_tools = true;
            }
            _ => {}
        }
    }
    assert!(saw_initialize, "initialize 응답 미수신, stdout: {stdout}");
    assert!(saw_tools, "tools/list 응답 미수신, stdout: {stdout}");
}

use std::io::{self, Read};

use axhub_helpers::catalog::classify;
use axhub_helpers::consent::{mint_token, parse_axhub_command, verify_token, ConsentBinding};
use axhub_helpers::list_deployments::{run_list_deployments, ListDeploymentsArgs};
use axhub_helpers::preflight::run_preflight;
use axhub_helpers::redact::redact;
use axhub_helpers::resolve::run_resolve;
use axhub_helpers::telemetry::emit_meta_envelope;
use serde_json::{json, Map, Value};

const HOOK_SCHEMA_VERSION: &str = "v0";
const USAGE: &str = "axhub-helpers - axhub plugin adapter binary (Rust)\n\nUsage:\n  axhub-helpers <subcommand> [args]\n\nSubcommands:\n  session-start\n  preauth-check\n  prompt-route\n  consent-mint\n  consent-verify\n  resolve\n  preflight\n  classify-exit\n  redact\n  list-deployments\n  version\n  help";

fn main() {
    std::process::exit(match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{e}");
            1
        }
    });
}

fn run() -> anyhow::Result<i32> {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        eprintln!("{USAGE}");
        return Ok(64);
    };
    let rest: Vec<String> = args.collect();
    match cmd.as_str() {
        "version" | "--version" | "-v" => {
            println!(
                "axhub-helpers {} (plugin v{}, schema {HOOK_SCHEMA_VERSION})",
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_VERSION")
            );
            Ok(0)
        }
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(0)
        }
        "redact" => {
            let raw = read_stdin()?;
            print!("{}", redact(&raw));
            Ok(0)
        }
        "classify-exit" => cmd_classify_exit(&rest),
        "preflight" => {
            let run = run_preflight();
            println!("{}", serde_json::to_string(&run.output)?);
            Ok(run.exit_code)
        }
        "resolve" => {
            let run = run_resolve(&rest);
            println!("{}", serde_json::to_string(&run.output)?);
            Ok(run.exit_code)
        }
        "list-deployments" => cmd_list_deployments(&rest),
        "consent-mint" => cmd_consent_mint(),
        "consent-verify" => cmd_consent_verify(),
        "preauth-check" => cmd_preauth_check(),
        "prompt-route" => cmd_prompt_route(),
        "session-start" => {
            println!(
                "{}",
                json!({"systemMessage":"axhub helper Rust runtime이에요."})
            );
            let mut m = Map::new();
            m.insert("event".into(), Value::String("session_start".into()));
            emit_meta_envelope(m).ok();
            Ok(0)
        }
        _ => {
            eprintln!("axhub-helpers: unknown subcommand \"{cmd}\"\n\n{USAGE}");
            Ok(64)
        }
    }
}

fn read_stdin() -> anyhow::Result<String> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;
    Ok(s)
}
fn out_json(v: Value) {
    println!("{}", v);
}

fn cmd_classify_exit(args: &[String]) -> anyhow::Result<i32> {
    let mut exit_code = 1;
    let mut stdout = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--exit-code" if i + 1 < args.len() => {
                i += 1;
                exit_code = args[i].parse().unwrap_or(1);
            }
            "--stdout" if i + 1 < args.len() => {
                i += 1;
                stdout = args[i].clone();
            }
            _ => {}
        }
        i += 1;
    }
    out_json(serde_json::to_value(classify(exit_code, &stdout))?);
    Ok(0)
}

fn parse_binding(raw: &str) -> anyhow::Result<ConsentBinding> {
    Ok(serde_json::from_str(raw)?)
}
fn cmd_consent_mint() -> anyhow::Result<i32> {
    let b = parse_binding(&read_stdin()?)?;
    let result = mint_token(b, 60)?;
    out_json(serde_json::to_value(result)?);
    Ok(0)
}
fn cmd_consent_verify() -> anyhow::Result<i32> {
    let b = parse_binding(&read_stdin()?)?;
    let result = verify_token(b);
    out_json(serde_json::to_value(&result)?);
    Ok(if result.valid { 0 } else { 65 })
}

fn cmd_preauth_check() -> anyhow::Result<i32> {
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if let Some(sid) = payload.get("session_id").and_then(Value::as_str) {
        if std::env::var("CLAUDE_SESSION_ID").is_err() {
            std::env::set_var("CLAUDE_SESSION_ID", sid);
        }
    }
    if payload.get("tool_name").and_then(Value::as_str) != Some("Bash") {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        return Ok(0);
    }
    let cmd = payload
        .pointer("/tool_input/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    let parsed = parse_axhub_command(cmd);
    if !parsed.is_destructive {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        return Ok(0);
    }
    let is_identity = parsed.action.as_deref() == Some("auth_login");
    let binding = ConsentBinding {
        tool_call_id: format!(
            "{}:{}",
            payload
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or(""),
            payload
                .get("tool_call_id")
                .and_then(Value::as_str)
                .unwrap_or("")
        ),
        action: parsed.action.unwrap_or_default(),
        app_id: parsed
            .app_id
            .unwrap_or_else(|| if is_identity { "_".into() } else { "".into() }),
        profile: parsed.profile.unwrap_or_else(|| {
            if is_identity {
                std::env::var("AXHUB_PROFILE").unwrap_or_else(|_| "default".into())
            } else {
                "".into()
            }
        }),
        branch: parsed
            .branch
            .unwrap_or_else(|| if is_identity { "_".into() } else { "".into() }),
        commit_sha: parsed.commit_sha.unwrap_or_else(|| {
            if is_identity {
                "_".into()
            } else {
                "".into()
            }
        }),
    };
    let result = verify_token(binding);
    if result.valid {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        Ok(0)
    } else {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":format!("destructive axhub command requires fresh consent token ({})", result.reason.unwrap_or_else(|| "unknown".into()))}}),
        );
        Ok(65)
    }
}

fn prompt_matches_doctor_intent(prompt: &str) -> bool {
    let p = prompt.trim().to_lowercase();
    if p.is_empty() {
        return false;
    }
    [
        "/axhub:doctor",
        "doctor",
        "diagnose",
        "health check",
        "sanity check",
        "setup check",
        "env check",
        "진단",
        "닥터",
        "환경 점검",
        "환경점검",
        "헬스체크",
        "헬스 체크",
        "설치 상태",
        "잘 깔렸",
        "axhub 점검",
    ]
    .iter()
    .any(|needle| p.contains(needle))
}

fn cmd_prompt_route() -> anyhow::Result<i32> {
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");
    if !prompt_matches_doctor_intent(prompt) {
        out_json(json!({}));
        return Ok(0);
    }

    let preflight = run_preflight();
    let version = preflight
        .output
        .cli_version
        .clone()
        .unwrap_or_else(|| "unknown".into());
    let guidance = if preflight.output.cli_too_old {
        format!(
            "axhub 버전 확인 결과, axhub가 너무 오래된 버전이에요 ({version}). 'axhub 업그레이드해줘'라고 말씀해주세요."
        )
    } else if preflight.output.cli_too_new {
        format!(
            "axhub 버전 확인 결과, 검증 범위보다 새 버전이에요 ({version}). 플러그인 업데이트 확인이 필요해요."
        )
    } else if !preflight.output.cli_present {
        "axhub 설치 확인 결과, CLI를 찾지 못했어요. axhub 설치 후 다시 점검해주세요.".into()
    } else {
        format!("axhub 버전 확인 결과, CLI {version} 상태를 확인했어요.")
    };
    let preflight_json = json!({
        "cli_version": preflight.output.cli_version,
        "cli_present": preflight.output.cli_present,
        "in_range": preflight.output.in_range,
        "cli_too_old": preflight.output.cli_too_old,
        "cli_too_new": preflight.output.cli_too_new,
        "auth_ok": preflight.output.auth_ok,
        "auth_error_code": preflight.output.auth_error_code,
        "exit_code": preflight.exit_code,
    });
    let context = [
        "[axhub prompt routing]".to_string(),
        format!("사용자 발화 \"{prompt}\"는 axhub doctor/환경 점검 의도예요."),
        "일반 저장소 환경 체크로 답하지 말고 axhub doctor 워크플로우를 적용해요.".into(),
        format!("Preflight 결과: {preflight_json}"),
        guidance,
    ]
    .join("\n");
    out_json(json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": context,
        }
    }));
    Ok(0)
}

fn cmd_list_deployments(args: &[String]) -> anyhow::Result<i32> {
    let mut app_id = None;
    let mut limit = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--app-id" | "--app" if i + 1 < args.len() => {
                i += 1;
                app_id = Some(args[i].clone());
            }
            "--limit" if i + 1 < args.len() => {
                i += 1;
                limit = args[i].parse().ok();
            }
            _ => {}
        }
        i += 1;
    }
    let Some(app_id) = app_id else {
        eprintln!("--app-id is required");
        return Ok(64);
    };
    let result = run_list_deployments(ListDeploymentsArgs { app_id, limit });
    let code = result.exit_code;
    out_json(serde_json::to_value(result)?);
    Ok(code)
}

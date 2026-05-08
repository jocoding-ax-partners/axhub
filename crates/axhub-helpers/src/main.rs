use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use axhub_helpers::bootstrap::{cmd_bootstrap_dependency_plan, run_bootstrap};
use axhub_helpers::catalog::classify;
use axhub_helpers::consent::{
    format_preauth_deny_hint, mint_token, parse_axhub_command, validate_binding_schema,
    verify_or_claim_token, verify_token, write_private_file_no_follow, ConsentBinding,
};
use axhub_helpers::keychain::{parse_keyring_value, read_keychain_token};
use axhub_helpers::list_deployments::{run_list_deployments, ListDeploymentsArgs};
use axhub_helpers::preflight::{run_preflight, PreflightRun};
use axhub_helpers::redact::redact;
use axhub_helpers::resolve::run_resolve;
use axhub_helpers::runtime_paths::{last_deploy_file, state_dir, token_file};
use axhub_helpers::statusline::current_statusline;
use axhub_helpers::telemetry::emit_meta_envelope;
use serde_json::{json, Map, Value};

const HOOK_SCHEMA_VERSION: &str = "v0";
const USAGE: &str = "axhub-helpers - axhub plugin adapter binary (Rust)\n\nUsage:\n  axhub-helpers <subcommand> [args]\n\nSubcommands:\n  session-start\n  preauth-check\n  prompt-route\n  consent-mint [--validate-only]\n  consent-verify\n  resolve\n  preflight\n  classify-exit\n  redact\n  statusline\n  path <token-file|last-deploy-file|state-dir>\n  token-init [--json]\n  token-import [--json]\n  list-deployments\n  bootstrap [--json] [--dry-run|--plan-only|--auto-chain|--record <event>|dependency-plan]\n  cleanup-audit [--all] [--yes]\n  version\n  help";

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
        "statusline" => {
            print!("{}", current_statusline());
            Ok(0)
        }
        "path" => cmd_path(&rest),
        "token-init" => cmd_token_init(&rest),
        "token-import" => cmd_token_import(&rest),
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
        "bootstrap" => cmd_bootstrap(&rest),
        "cleanup-audit" => cmd_cleanup_audit(&rest),
        "consent-mint" => cmd_consent_mint(&rest),
        "consent-verify" => cmd_consent_verify(),
        "preauth-check" => cmd_preauth_check(),
        "prompt-route" => cmd_prompt_route(),
        "session-start" => {
            println!(
                "{}",
                json!({"systemMessage":"axhub helper Rust runtime이에요.\naudit log 로컬 7일 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1. 삭제: axhub-helpers cleanup-audit --all"})
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

fn cmd_path(args: &[String]) -> anyhow::Result<i32> {
    let Some(kind) = args.first().map(String::as_str) else {
        eprintln!("axhub-helpers path: expected one of token-file, last-deploy-file, state-dir");
        return Ok(64);
    };
    let path = match kind {
        "token-file" => token_file(),
        "last-deploy-file" => last_deploy_file(),
        "state-dir" => state_dir(),
        _ => {
            eprintln!("axhub-helpers path: unknown path kind \"{kind}\"");
            return Ok(64);
        }
    };
    let Some(path) = path else {
        eprintln!("axhub-helpers path: cannot resolve {kind}");
        return Ok(65);
    };
    println!("{}", path.display());
    Ok(0)
}

fn cmd_token_init(args: &[String]) -> anyhow::Result<i32> {
    let json_output = match parse_json_flag(args, "token-init") {
        Ok(value) => value,
        Err(code) => return Ok(code),
    };
    let (token, source) = match env_token() {
        Some(token) => (token, "env:AXHUB_TOKEN".to_string()),
        None => {
            let keychain = read_keychain_token();
            match keychain.token {
                Some(token) => (
                    token,
                    keychain
                        .source
                        .unwrap_or_else(|| "platform-keychain".to_string()),
                ),
                None => {
                    return emit_token_error(
                        json_output,
                        keychain.error.unwrap_or_else(|| {
                            "axhub token을 찾을 수 없어요. axhub auth login 또는 AXHUB_TOKEN을 사용해주세요."
                                .to_string()
                        }),
                    );
                }
            }
        }
    };
    store_and_report_token(json_output, &token, &source)
}

fn cmd_token_import(args: &[String]) -> anyhow::Result<i32> {
    let json_output = match parse_json_flag(args, "token-import") {
        Ok(value) => value,
        Err(code) => return Ok(code),
    };
    let raw = read_stdin()?;
    let Some(token) = extract_token_from_import_payload(&raw) else {
        return emit_token_error(
            json_output,
            "token-import 입력에서 access_token/token 값을 찾을 수 없어요.".to_string(),
        );
    };
    store_and_report_token(json_output, &token, "stdin")
}

fn parse_json_flag(args: &[String], command: &str) -> Result<bool, i32> {
    let mut json_output = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json_output = true,
            _ => {
                eprintln!("axhub-helpers {command}: unknown option");
                return Err(64);
            }
        }
    }
    Ok(json_output)
}

fn env_token() -> Option<String> {
    std::env::var("AXHUB_TOKEN")
        .ok()
        .and_then(|value| normalize_token_candidate(&value))
}

fn extract_token_from_import_payload(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(token) = parse_keyring_value(trimmed) {
        return Some(token);
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return token_from_json_value(&value).and_then(normalize_token_candidate);
    }
    normalize_token_candidate(trimmed)
}

fn token_from_json_value(value: &Value) -> Option<&str> {
    if let Some(token) = value.as_str() {
        return Some(token);
    }
    ["access_token", "token", "AXHUB_TOKEN"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_str))
}

fn normalize_token_candidate(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let candidate = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .unwrap_or(trimmed)
        .trim();
    if candidate.len() < 16
        || candidate
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return None;
    }
    Some(candidate.to_string())
}

fn store_and_report_token(json_output: bool, token: &str, source: &str) -> anyhow::Result<i32> {
    let path = store_plugin_token(token)?;
    if json_output {
        out_json(json!({
            "stored": true,
            "source": source,
            "token_file": path,
        }));
    } else {
        println!("axhub token stored at {} ({source})", path.display());
    }
    Ok(0)
}

fn emit_token_error(json_output: bool, message: String) -> anyhow::Result<i32> {
    if json_output {
        out_json(json!({
            "stored": false,
            "error": message,
        }));
    } else {
        eprintln!("{message}");
    }
    Ok(65)
}

fn store_plugin_token(token: &str) -> anyhow::Result<PathBuf> {
    let path = token_file()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve axhub plugin token file path"))?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve axhub plugin token directory"))?
        .to_path_buf();
    fs::create_dir_all(&parent)?;
    axhub_helpers::consent::set_private_dir_mode(&parent).ok();
    write_private_file_no_follow(&path, token.as_bytes())?;
    Ok(path)
}

fn cmd_bootstrap(args: &[String]) -> anyhow::Result<i32> {
    if args.first().map(String::as_str) == Some("dependency-plan") {
        return cmd_bootstrap_dependency_plan(&args[1..]);
    }
    let stdin = if bootstrap_record_event(args).is_some() {
        Some(read_stdin()?)
    } else {
        None
    };
    let run = run_bootstrap(args, stdin.as_deref());
    println!("{}", serde_json::to_string(&run.output)?);
    Ok(run.exit_code)
}

fn bootstrap_record_event(args: &[String]) -> Option<&str> {
    let index = args.iter().position(|arg| arg == "--record")?;
    let event = args.get(index + 1)?.as_str();
    if event.starts_with("--") || !matches!(event, "apps_create" | "deploy_create") {
        return None;
    }
    Some(event)
}

fn cmd_classify_exit(args: &[String]) -> anyhow::Result<i32> {
    let raw = read_stdin()?;
    if !raw.trim().is_empty() {
        let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let command = payload
            .pointer("/tool_input/command")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !command.starts_with("axhub ") {
            out_json(json!({}));
            return Ok(0);
        }
        let exit_code = payload
            .pointer("/tool_response/exit_code")
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32;
        let stdout = payload
            .pointer("/tool_response/stdout")
            .and_then(Value::as_str)
            .unwrap_or("");
        if exit_code == 0 && !command.starts_with("axhub deploy create") {
            out_json(json!({}));
            return Ok(0);
        }
        let entry = classify(exit_code, stdout);
        let mut system_message = format!(
            "{}\n\n원인: {}\n\n해결: {}",
            entry.emotion, entry.cause, entry.action
        );
        if let Some(button) = entry.button {
            system_message.push_str(&format!("\n\n선택: {button}"));
        }
        out_json(json!({ "systemMessage": system_message }));
        return Ok(0);
    }

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
fn cmd_consent_mint(args: &[String]) -> anyhow::Result<i32> {
    let validate_only = match args {
        [] => false,
        [flag] if flag == "--validate-only" => true,
        [flag, ..] => {
            eprintln!("axhub-helpers consent-mint: unknown option \"{flag}\"");
            return Ok(64);
        }
    };
    let b = parse_binding(&read_stdin()?)?;
    validate_binding_schema(&b)?;
    if validate_only {
        out_json(json!({"valid": true, "action": b.action}));
        return Ok(0);
    }
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
    let deny_hint = format_preauth_deny_hint(parsed.action.as_deref(), parsed.app_id.as_deref());
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
        context: parsed.context,
        synthesized_by_helper: false,
    };
    let result = verify_or_claim_token(binding);
    if result.valid {
        out_json(
            json!({"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}),
        );
        Ok(0)
    } else {
        out_json(json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny"
            },
            "systemMessage": deny_hint
        }));
        Ok(0)
    }
}

const MAX_LIST_DEPLOYMENTS_LIMIT: usize = 100;

const CLEANUP_AUDIT_HELP: &str = "axhub-helpers cleanup-audit — audit log 삭제

USAGE:
  axhub-helpers cleanup-audit          # 7일 이상 된 파일만 삭제 (rotation)
  axhub-helpers cleanup-audit --all    # 전체 삭제 (확인 prompt)
  axhub-helpers cleanup-audit --all --yes   # 확인 우회

OPTIONS:
  --all      전체 삭제 (default 는 7일 이상만)
  --yes -y   확인 prompt 우회
  -h --help  도움말
";

fn cmd_cleanup_audit(args: &[String]) -> anyhow::Result<i32> {
    use axhub_helpers::audit;

    let mut all = false;
    let mut yes = false;
    for arg in args {
        match arg.as_str() {
            "--all" => all = true,
            "--yes" | "-y" => yes = true,
            "-h" | "--help" => {
                print!("{CLEANUP_AUDIT_HELP}");
                return Ok(0);
            }
            other => {
                eprintln!("axhub-helpers cleanup-audit: 알 수 없는 flag: {other}");
                return Ok(64);
            }
        }
    }

    if all {
        if !yes {
            print!("audit log 전체 삭제할까요? (y/N): ");
            use std::io::Write;
            std::io::stdout().flush().ok();
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("취소했어요.");
                return Ok(0);
            }
        }
        let count = audit::cleanup_all()?;
        println!("audit log {count} 파일 삭제했어요.");
    } else {
        let count = audit::rotate(7)?;
        println!("7일 이상 된 audit log {count} 파일 삭제했어요. 전체 삭제는 --all 사용해요.");
    }
    Ok(0)
}

// Approach E (Phase 2): cmd_prompt_route is preflight + audit only.
// No keyword chain, no skill enforcement, no `skills/<X>/SKILL.md` paths in
// additionalContext. Claude Code matches skills via SKILL.md frontmatter
// description natively (Phase 1 codegen merged main.rs phrases into descriptions).
fn cmd_prompt_route() -> anyhow::Result<i32> {
    use axhub_helpers::audit::{append as audit_append, now_iso8601, sha256_hex, AuditRecord};

    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");

    let preflight = run_preflight();

    let record = AuditRecord {
        ts: now_iso8601(),
        prompt_hash: sha256_hex(prompt),
        // UTF-8 byte length, not char count. Korean prompts run ~3x byte size of visible chars.
        prompt_len: prompt.len() as u32,
        cli_version: preflight.output.cli_version.clone(),
        auth_ok: preflight.output.auth_ok,
        is_axhub_related: heuristic_axhub_keyword(prompt),
    };
    let _ = audit_append(record);

    // format_preflight_context always emits at least one Korean line (cli status branch);
    // additionalContext is therefore always non-empty in normal operation.
    let context = format_preflight_context(&preflight);
    out_json(json!({
        "hookSpecificOutput": {
            "hookEventName": "UserPromptSubmit",
            "additionalContext": context,
        }
    }));
    Ok(0)
}

/// Single substring check for measurement only. NOT intent classification.
fn heuristic_axhub_keyword(prompt: &str) -> bool {
    prompt.to_lowercase().contains("axhub")
}

/// Render preflight result as a Korean systemMessage block. Always emits at
/// least one line (one of cli_too_old / cli_too_new / !cli_present / healthy)
/// plus an optional auth_ok=false annotation. Fail-soft: never returns an Err
/// that blocks the hook.
fn format_preflight_context(preflight: &PreflightRun) -> String {
    let mut lines = Vec::new();
    let cli_version = preflight
        .output
        .cli_version
        .clone()
        .unwrap_or_else(|| "unknown".into());
    if preflight.output.cli_too_old {
        lines.push(format!(
            "axhub 버전 확인 결과, axhub가 너무 오래된 버전이에요 ({cli_version}). 'axhub 업그레이드해줘'라고 말해요."
        ));
    } else if preflight.output.cli_too_new {
        lines.push(format!(
            "axhub 버전 확인 결과, 검증 범위보다 새 버전이에요 ({cli_version}). 플러그인 업데이트 확인이 필요해요."
        ));
    } else if !preflight.output.cli_present {
        lines
            .push("axhub 설치 확인 결과, CLI를 찾지 못했어요. axhub 설치 후 다시 점검해요.".into());
    } else {
        lines.push(format!(
            "axhub 버전 확인 결과, CLI {cli_version} 상태를 확인했어요."
        ));
    }
    if !preflight.output.auth_ok {
        if let Some(code) = preflight.output.auth_error_code.as_deref() {
            lines.push(format!("auth 상태 비정상 ({code}). 로그인 확인 필요해요."));
        }
    }
    lines.join("\n")
}

fn cmd_list_deployments(args: &[String]) -> anyhow::Result<i32> {
    let mut app_id = None;
    let mut limit = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--app-id" | "--app" => {
                if i + 1 >= args.len() {
                    eprintln!("{} requires a value", args[i]);
                    return Ok(64);
                }
                i += 1;
                app_id = Some(args[i].clone());
            }
            "--limit" => {
                if i + 1 >= args.len() {
                    eprintln!("--limit requires a value");
                    return Ok(64);
                }
                i += 1;
                let parsed = match args[i].parse::<usize>() {
                    Ok(value) => value,
                    Err(_) => {
                        eprintln!("invalid --limit: {}", args[i]);
                        return Ok(64);
                    }
                };
                if !(1..=MAX_LIST_DEPLOYMENTS_LIMIT).contains(&parsed) {
                    eprintln!("--limit must be between 1 and {MAX_LIST_DEPLOYMENTS_LIMIT}");
                    return Ok(64);
                }
                limit = Some(parsed);
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

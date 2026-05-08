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
use axhub_helpers::runtime_paths::{last_deploy_file, state_dir, token_file, welcome_marker_path};
use axhub_helpers::statusline::current_statusline;
use axhub_helpers::telemetry::{
    append_phase_marker_to_file, emit_deploy_complete, emit_meta_envelope,
};
use serde_json::{json, Map, Value};

const HOOK_SCHEMA_VERSION: &str = "v0";
const USAGE: &str = "axhub-helpers - axhub plugin adapter binary (Rust)\n\nUsage:\n  axhub-helpers <subcommand> [args]\n\nSubcommands:\n  session-start\n  preauth-check\n  prompt-route\n  consent-mint [--validate-only]\n  consent-verify\n  resolve\n  preflight\n  classify-exit\n  redact\n  statusline\n  path <token-file|last-deploy-file|state-dir>\n  token-init [--json]\n  token-import [--json]\n  list-deployments\n  bootstrap [--json] [--dry-run|--plan-only|--auto-chain|--record <event>|dependency-plan]\n  routing-stats [--since <D>] [--json] [--top <N>] [--confused]\n  cleanup-audit [--all] [--yes]\n  audit-clarify (--hash <H>|--prompt <P>) --chosen <S>\n  routing-dashboard [--html]\n  mark <phase_name>\n  emit-deploy-complete [<exit_code> [<command_class>]]\n  version\n  help";

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
        "routing-stats" => cmd_routing_stats(&rest),
        "cleanup-audit" => cmd_cleanup_audit(&rest),
        "audit-clarify" => cmd_audit_clarify(&rest),
        "routing-dashboard" => cmd_routing_dashboard(&rest),
        "bootstrap" => cmd_bootstrap(&rest),
        "consent-mint" => cmd_consent_mint(&rest),
        "consent-verify" => cmd_consent_verify(),
        "preauth-check" => cmd_preauth_check(),
        "prompt-route" => cmd_prompt_route(),
        "session-start" => cmd_session_start(),
        "mark" => cmd_mark(&rest),
        "emit-deploy-complete" => cmd_emit_deploy_complete(&rest),
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

fn consent_mint_json_stdin_help() -> &'static str {
    r#"PowerShell example: $binding | ConvertTo-Json -Compress | & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" consent-mint
Temp-file fallback: Get-Content -Raw "$Path" | & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" consent-mint"#
}

fn parse_consent_mint_binding(raw: &str) -> Result<ConsentBinding, i32> {
    let binding_json = raw.trim().trim_start_matches('\u{feff}').trim();
    if binding_json.is_empty() {
        eprintln!(
            "axhub-helpers consent-mint: empty stdin; no JSON binding was provided.\n{}",
            consent_mint_json_stdin_help()
        );
        return Err(65);
    }
    serde_json::from_str(binding_json).map_err(|err| {
        eprintln!(
            "axhub-helpers consent-mint: invalid JSON; consent-mint expects one JSON object binding on stdin.\nError: {err}\n{}",
            consent_mint_json_stdin_help()
        );
        65
    })
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
    let raw = read_stdin()?;
    let b = match parse_consent_mint_binding(&raw) {
        Ok(binding) => binding,
        Err(code) => return Ok(code),
    };
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
        clarify_invoked: false,
        chosen_skill: None,
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
    // Phase 9 sub-task 9.2 — examples context marker (env-gated, default off).
    // Claude Code 가 SKILL.md frontmatter examples field 를 native 인식하면
    // 별도 hook 주입 불필요. 미지원 환경의 fallback marker 만 emit.
    if std::env::var("AXHUB_INJECT_EXAMPLES").is_ok() {
        lines.push(
            "[examples context] AXHUB_INJECT_EXAMPLES enabled — 각 SKILL.md frontmatter 의 examples field 를 매칭 시 참고해요."
                .to_string(),
        );
    }
    lines.join("\n")
}

// Approach E (Phase 4): routing-stats + cleanup-audit subcommands.
//
// Local-only audit log analytics. AXHUB_NO_AUDIT respected. Silent skip on
// disk read errors. Always Korean default + --json machine-readable.

const ROUTING_STATS_HELP: &str = "axhub-helpers routing-stats — 라우팅 audit log 통계 조회

USAGE:
  axhub-helpers routing-stats [OPTIONS]

OPTIONS:
  --since <DURATION>    조회 기간 (예: 1d, 7d, 30d, all). 기본: 7d
  --json                machine-readable JSON 출력
  --top <N>             top N axhub-related prompt hash 표시. 기본: 10
  --confused            clarify_invoked=true 인 records 만 표시 (사용자 disambiguation 발동)
  -h, --help            도움말

PRIVACY:
  prompt content 저장 X. sha256 hash + length + cli_version + auth_ok 만 기록.
  외부 전송 X. 모두 로컬 ~/.local/share/axhub-plugin/ 또는 동등 경로.
  끄려면: AXHUB_NO_AUDIT=1 환경 변수 설정.
  삭제: axhub-helpers cleanup-audit --all
";

fn parse_routing_stats_args(
    args: &[String],
) -> anyhow::Result<(chrono::Duration, bool, u32, bool)> {
    let mut since = chrono::Duration::days(7);
    let mut json = false;
    let mut top: u32 = 10;
    let mut confused = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--since" if i + 1 < args.len() => {
                i += 1;
                since = parse_duration(&args[i])?;
            }
            "--json" => json = true,
            "--top" if i + 1 < args.len() => {
                i += 1;
                top = args[i]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("--top 은 숫자여야 해요"))?;
            }
            "--confused" => confused = true,
            "-h" | "--help" => {
                print!("{}", ROUTING_STATS_HELP);
                std::process::exit(0);
            }
            other => anyhow::bail!("알 수 없는 flag: {other}"),
        }
        i += 1;
    }
    Ok((since, json, top, confused))
}

fn parse_duration(s: &str) -> anyhow::Result<chrono::Duration> {
    if s == "all" {
        return Ok(chrono::Duration::days(36500));
    }
    if s.is_empty() {
        anyhow::bail!("duration 비어 있어요");
    }
    let last = s.chars().last().unwrap();
    let (num_str, unit) = s.split_at(s.len() - last.len_utf8());
    let num: i64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("duration 숫자 부분 파싱 실패: {s}"))?;
    match unit {
        "d" => Ok(chrono::Duration::days(num)),
        "h" => Ok(chrono::Duration::hours(num)),
        "m" => Ok(chrono::Duration::minutes(num)),
        _ => anyhow::bail!("duration 단위는 d/h/m 또는 'all' 만 (받은 값: {s})"),
    }
}

fn percentile(sorted: &[u32], p: f64) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn cmd_routing_stats(args: &[String]) -> anyhow::Result<i32> {
    use axhub_helpers::audit;

    let (since, json, top, confused) = match parse_routing_stats_args(args) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("axhub-helpers routing-stats: {e}");
            return Ok(64);
        }
    };

    // 매 호출마다 7-day rotation 자동 trigger (silent).
    let _ = audit::rotate(7);

    if std::env::var("AXHUB_NO_AUDIT").is_ok() {
        if json {
            println!(
                "{}",
                json!({
                    "audit_disabled": true,
                    "message": "AXHUB_NO_AUDIT 환경 변수가 설정되어 audit 가 비활성이에요."
                })
            );
        } else {
            println!("audit log 가 비활성이에요 (AXHUB_NO_AUDIT 환경 변수 설정).");
            println!("끄려면 변수 unset 후 다음 prompt 부터 기록해요.");
        }
        return Ok(0);
    }

    let mut records = audit::read_since(since)?;
    if confused {
        records.retain(|r| r.clarify_invoked);
    } else {
        // Clarify sentinel records are feedback events, not regular prompt-route samples.
        // Keeping them in default stats inflates auth_failed because sentinel records have
        // auth_ok=false by construction, and can depress axhub_related counts.
        records.retain(|r| !r.clarify_invoked);
    }
    if records.is_empty() {
        if json {
            println!(
                "{}",
                json!({"records": [], "total_prompts": 0, "confused_prompts": []})
            );
        } else if confused {
            println!("최근 {since:?} 동안 clarify 발동 prompt 가 없어요.");
        } else {
            println!("아직 audit 데이터가 없어요. axhub 사용하다 보면 자동 누적돼요.");
        }
        return Ok(0);
    }

    let total = records.len() as u32;
    let axhub_related = records.iter().filter(|r| r.is_axhub_related).count() as u32;
    let auth_failed = records.iter().filter(|r| !r.auth_ok).count() as u32;

    let mut lengths: Vec<u32> = records.iter().map(|r| r.prompt_len).collect();
    lengths.sort_unstable();
    let p50 = percentile(&lengths, 0.50);
    let p95 = percentile(&lengths, 0.95);

    let mut versions: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for r in &records {
        if let Some(v) = &r.cli_version {
            *versions.entry(v.clone()).or_insert(0) += 1;
        }
    }

    let mut hash_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for r in records.iter().filter(|r| r.is_axhub_related) {
        *hash_counts.entry(r.prompt_hash.clone()).or_insert(0) += 1;
    }
    let mut top_hashes: Vec<(String, u32)> = hash_counts.into_iter().collect();
    top_hashes.sort_by(|a, b| b.1.cmp(&a.1));
    top_hashes.truncate(top as usize);

    let mut confused_counts: std::collections::HashMap<(String, Option<String>), (u32, String)> =
        std::collections::HashMap::new();
    for r in records.iter().filter(|r| r.clarify_invoked) {
        let entry = confused_counts
            .entry((r.prompt_hash.clone(), r.chosen_skill.clone()))
            .or_insert((0, r.ts.clone()));
        entry.0 += 1;
        if r.ts.as_str() > entry.1.as_str() {
            entry.1 = r.ts.clone();
        }
    }
    let mut confused_rows: Vec<(String, Option<String>, u32, String)> = confused_counts
        .into_iter()
        .map(|((hash, chosen_skill), (count, latest_ts))| (hash, chosen_skill, count, latest_ts))
        .collect();
    confused_rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));

    if json {
        let summary = json!({
            "total_prompts": total,
            "axhub_related": axhub_related,
            "axhub_related_rate": axhub_related as f64 / total as f64,
            "auth_failed": auth_failed,
            "prompt_length_p50": p50,
            "prompt_length_p95": p95,
            "cli_versions": versions,
            "top_axhub_hashes": top_hashes.iter().map(|(h, c)| json!({"hash": h, "count": c})).collect::<Vec<_>>(),
            "confused_prompts": confused_rows.iter().map(|(hash, chosen_skill, count, latest_ts)| json!({
                "hash": hash,
                "count": count,
                "chosen_skill": chosen_skill,
                "latest_ts": latest_ts,
            })).collect::<Vec<_>>(),
        });
        println!("{}", summary);
        return Ok(0);
    }

    // Korean default output
    println!("[지난 prompt 통계]");
    println!("==========================================");
    println!("총 prompt:           {total}");
    let rate_pct = 100.0 * axhub_related as f64 / total as f64;
    println!("axhub 관련 가능성:    {axhub_related} ({rate_pct:.1}%)");
    println!("auth 실패:           {auth_failed}");
    println!("prompt 길이 p50/p95: {p50} / {p95} bytes");
    println!();
    println!("CLI 버전:");
    for (v, c) in &versions {
        println!("  {v}: {c}");
    }
    if !top_hashes.is_empty() {
        println!();
        println!("상위 axhub 관련 prompt (hash):");
        for (h, c) in &top_hashes {
            println!("  {h}: {c:>4}");
        }
    }
    println!();
    if let Some(dir) = axhub_helpers::runtime_paths::state_dir() {
        println!("audit log 위치: {}", dir.display());
    }
    println!("끄려면: AXHUB_NO_AUDIT=1");
    println!("삭제: axhub-helpers cleanup-audit --all");
    Ok(0)
}

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

// Phase 10 — audit-clarify subcommand: clarify SKILL fires this command after the
// user picks a final disambiguation. Adds an audit record with clarify_invoked=true
// + chosen_skill=Some(name). routing-stats --confused filters on this signal.

const AUDIT_CLARIFY_HELP: &str = "axhub-helpers audit-clarify — clarify feedback record

USAGE:
  axhub-helpers audit-clarify (--hash <prompt-hash>|--prompt <prompt>) --chosen <skill-name>

OPTIONS:
  --hash <H>       원본 prompt 의 sha256 hash (e.g. sha256:abc...)
  --prompt <P>     원본 prompt. helper 가 로컬에서 sha256 hash 로 변환해요.
  --chosen <S>     사용자가 final 선택한 skill name (또는 'null')
  -h, --help       도움말
";

fn cmd_audit_clarify(args: &[String]) -> anyhow::Result<i32> {
    use axhub_helpers::audit::{self, sha256_hex, AuditRecord};
    let mut hash: Option<String> = None;
    let mut prompt: Option<String> = None;
    let mut chosen: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--hash" if i + 1 < args.len() => {
                hash = Some(args[i + 1].clone());
                i += 1;
            }
            "--prompt" if i + 1 < args.len() => {
                prompt = Some(args[i + 1].clone());
                i += 1;
            }
            "--chosen" if i + 1 < args.len() => {
                chosen = Some(args[i + 1].clone());
                i += 1;
            }
            "-h" | "--help" => {
                print!("{AUDIT_CLARIFY_HELP}");
                return Ok(0);
            }
            other => {
                eprintln!("axhub-helpers audit-clarify: 알 수 없는 flag: {other}");
                return Ok(64);
            }
        }
        i += 1;
    }
    if hash.is_some() && prompt.is_some() {
        eprintln!("axhub-helpers audit-clarify: --hash 또는 --prompt 하나만 사용해요");
        return Ok(64);
    }
    let (prompt_hash, prompt_len) = match (hash, prompt) {
        (Some(prompt_hash), None) => (prompt_hash, 0),
        (None, Some(prompt)) => (sha256_hex(&prompt), prompt.len() as u32),
        (None, None) => {
            eprintln!("axhub-helpers audit-clarify: --hash 또는 --prompt 필요해요");
            return Ok(64);
        }
        (Some(_), Some(_)) => unreachable!(),
    };
    let chosen_skill = chosen.and_then(|s| if s == "null" { None } else { Some(s) });
    let record = AuditRecord {
        ts: audit::now_iso8601(),
        prompt_hash,
        prompt_len,
        cli_version: None,
        auth_ok: false,
        is_axhub_related: false,
        clarify_invoked: true,
        chosen_skill,
    };
    audit::append(record).ok();
    println!("audit-clarify 기록했어요.");
    Ok(0)
}

// Phase 10 — routing-dashboard subcommand: per-skill stats HTML render.

const ROUTING_DASHBOARD_HELP: &str = "axhub-helpers routing-dashboard — per-skill drift dashboard

USAGE:
  axhub-helpers routing-dashboard [--html]

OPTIONS:
  --html      inline HTML render (per-skill table + drift trend + failing prompts)
  -h, --help  도움말
";

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn cmd_routing_dashboard(args: &[String]) -> anyhow::Result<i32> {
    use axhub_helpers::audit;
    let html_mode = args.iter().any(|a| a == "--html");
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{ROUTING_DASHBOARD_HELP}");
        return Ok(0);
    }
    let records = audit::read_since(chrono::Duration::days(7))?;
    let operational_records: Vec<_> = records.iter().filter(|r| !r.clarify_invoked).collect();
    let total = operational_records.len();
    let axhub_related = operational_records
        .iter()
        .filter(|r| r.is_axhub_related)
        .count();
    let auth_failed = operational_records.iter().filter(|r| !r.auth_ok).count();
    let confused = records.iter().filter(|r| r.clarify_invoked).count();
    let mut chosen_counts: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    for r in records.iter().filter(|r| r.clarify_invoked) {
        if let Some(skill) = &r.chosen_skill {
            *chosen_counts.entry(skill.clone()).or_insert(0) += 1;
        }
    }
    let mut rows: Vec<(String, u32)> = chosen_counts.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    if html_mode {
        let mut chosen_rows = String::new();
        for (skill, count) in &rows {
            chosen_rows.push_str(&format!(
                "<tr><td>{}</td><td>{count}</td><td>n/a</td><td>n/a</td></tr>",
                html_escape(skill)
            ));
        }
        if chosen_rows.is_empty() {
            chosen_rows
                .push_str("<tr><td colspan=\"4\">clarify feedback 이 아직 없어요.</td></tr>");
        }
        let mut failing_rows = String::new();
        for r in records.iter().filter(|r| r.clarify_invoked).take(25) {
            failing_rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(&r.prompt_hash),
                html_escape(r.chosen_skill.as_deref().unwrap_or("null")),
                html_escape(&r.ts),
            ));
        }
        if failing_rows.is_empty() {
            failing_rows
                .push_str("<tr><td colspan=\"3\">failing prompt hash 가 아직 없어요.</td></tr>");
        }
        let html = format!(
            include_str!("../templates/dashboard.html"),
            total = total,
            axhub_related = axhub_related,
            auth_failed = auth_failed,
            confused = confused,
            chosen_rows = chosen_rows,
            failing_rows = failing_rows,
        );
        print!("{html}");
    } else {
        println!("[axhub routing dashboard — last 7d]");
        println!("total prompts: {total}");
        println!("axhub-related: {axhub_related}");
        println!("auth failed: {auth_failed}");
        println!("clarify invoked: {confused}");
        if !rows.is_empty() {
            println!("\nUser-chosen skill (clarify feedback):");
            for (skill, count) in &rows {
                println!("  {skill:<16} {count}");
            }
        }
    }
    Ok(0)
}

// Phase 7 (Component 6): SessionStart magical-moment message.
//
// Base systemMessage (always 3 lines) + current-version first-session welcome
// (6 extra lines, one-shot, gated by welcome marker file). Marker write is
// best-effort — failure surfaces the welcome again next session, never blocks Claude.

const WELCOME_VERSION: &str = env!("CARGO_PKG_VERSION");

fn cmd_session_start() -> anyhow::Result<i32> {
    let mut lines: Vec<String> = vec![
        format!(
            "axhub helper Rust runtime 활성 (v{}).",
            env!("CARGO_PKG_VERSION")
        ),
        "막히면 /axhub:help 로 명령 메뉴를, /axhub:clarify 로 모호한 의도 확인을 부탁해요."
            .to_string(),
        "라우팅 통계는 axhub-helpers routing-stats 로 봐요.".to_string(),
        "audit log 로컬 7일 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1. 삭제: axhub-helpers cleanup-audit --all"
            .to_string(),
    ];

    let marker = welcome_marker_path(WELCOME_VERSION);
    let show_welcome = marker.as_ref().map(|p| !p.exists()).unwrap_or(false);
    if show_welcome {
        lines.push(String::new());
        lines.push(format!(
            "[axhub v{WELCOME_VERSION} 첫 세션] 라우팅 똑똑해졌어요."
        ));
        lines.push(
            "- Rust 키워드 체인 ~600줄 폐기. Claude 가 SKILL.md description 으로 직접 매칭해요."
                .to_string(),
        );
        lines.push("- 메타 질문 (\"왜 ~ 키워드 매칭이야?\") 자동 처리해요.".to_string());
        lines.push(
            "- routing audit log 7일 로컬 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1.".to_string(),
        );
        lines.push("- 변경점 보기: /axhub:whatsnew".to_string());

        if let Some(path) = marker {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(
                &path,
                format!("shown: {}\n", chrono::Utc::now().to_rfc3339()),
            );
        }
    }

    let context = lines.join("\n");
    println!("{}", json!({"systemMessage": context}));
    let mut m = Map::new();
    m.insert("event".into(), Value::String("session_start".into()));
    emit_meta_envelope(m).ok();
    Ok(0)
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

fn cmd_mark(rest: &[String]) -> anyhow::Result<i32> {
    let Some(phase_name) = rest.first() else {
        eprintln!("mark requires <phase_name>");
        return Ok(64);
    };
    let path_env = match std::env::var("AXHUB_PHASE_MARKER_FILE") {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(0),
    };
    let path = std::path::PathBuf::from(path_env);
    if let Err(err) = append_phase_marker_to_file(&path, phase_name) {
        eprintln!("mark: {err}");
        return Ok(1);
    }
    Ok(0)
}

fn cmd_emit_deploy_complete(rest: &[String]) -> anyhow::Result<i32> {
    let exit_code: i32 = rest.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let default_class = "axhub deploy create".to_string();
    let command_class = rest.get(1).unwrap_or(&default_class);
    if let Err(err) = emit_deploy_complete(exit_code, command_class) {
        eprintln!("emit-deploy-complete: {err}");
        return Ok(1);
    }
    Ok(0)
}

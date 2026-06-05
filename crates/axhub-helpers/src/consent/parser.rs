use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ParsedAxhubCommand {
    pub is_destructive: bool,
    pub action: Option<String>,
    pub app_id: Option<String>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub profile: Option<String>,
    #[serde(default)]
    pub context: HashMap<String, String>,
}

const COLLECT_MAX_DEPTH: usize = 5;

fn flag_map(flag: &str) -> Option<&'static str> {
    match flag {
        "--app" => Some("app_id"),
        "--branch" => Some("branch"),
        "--commit" => Some("commit_sha"),
        "--profile" => Some("profile"),
        "--method" => Some("method"),
        "--repo" => Some("repo"),
        "--account" => Some("account"),
        "--endpoint" => Some("endpoint"),
        "--slug" => Some("slug"),
        "--confirm" => Some("confirm"),
        "--from-file" => Some("source"),
        "--body-file" => Some("body_file"),
        "--name" => Some("name"),
        "--description" => Some("description"),
        "--visibility" => Some("visibility"),
        "--resource-tier" => Some("resource_tier"),
        "--subdomain" => Some("subdomain"),
        "--auth-mode" => Some("auth_mode"),
        "--category-id" => Some("category_id"),
        "--data-scopes" => Some("data_scopes"),
        "--icon-dark-url" => Some("icon_dark_url"),
        "--icon-url" => Some("icon_url"),
        "--note" => Some("note"),
        "--role" => Some("role"),
        "--tenant" => Some("tenant"),
        "--user" => Some("user"),
        "--table" => Some("table"),
        "--principal-id" => Some("principal_id"),
        "--principal-type" => Some("principal_type"),
        "--actions" => Some("actions"),
        "--grant-id" => Some("grant_id"),
        "--body" => Some("body"),
        "--batch" => Some("batch"),
        "--type" => Some("type"),
        "--column" => Some("column"),
        "--owner-column" => Some("owner_column"),
        "--connector-id" => Some("connector_id"),
        "--items-file" => Some("items_file"),
        "--items-json" => Some("items_json"),
        "--tag-id" => Some("tag_id"),
        "--parent-id" => Some("parent_id"),
        "--from-deployment" => Some("from_deployment"),
        "--config-file" => Some("config_file"),
        "--config-json" => Some("config_json"),
        "--credentials-file" => Some("credentials_file"),
        "--credentials-json" => Some("credentials_json"),
        "--template" => Some("template"),
        "--engine" => Some("engine"),
        "--auth-method" => Some("auth_method"),
        "--redirect-uri" => Some("redirect_uri"),
        "--scope" => Some("scope"),
        "--grant-type" => Some("grant_type"),
        "--client-id" => Some("client_id"),
        "--token-type-hint" => Some("token_type_hint"),
        "--expires-in-days" => Some("expires_in_days"),
        _ => None,
    }
}

fn extract_flags(tokens: &[String]) -> HashMap<&'static str, String> {
    let mut out = HashMap::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if let Some(eq) = t.find('=') {
            if let Some(k) = flag_map(&t[..eq]) {
                out.insert(k, t[eq + 1..].to_string());
            }
        } else if let Some(k) = flag_map(t) {
            if let Some(val) = tokens.get(i + 1).filter(|v| !v.starts_with("--")) {
                out.insert(k, val.clone());
                i += 1;
            }
        }
        i += 1;
    }
    out
}

fn positional(tokens: &[String], index: usize) -> Option<String> {
    tokens.get(index).filter(|v| !v.starts_with('-')).cloned()
}

fn has_flag(tokens: &[String], flag: &str) -> bool {
    tokens.iter().any(|token| {
        token == flag
            || token
                .strip_prefix(flag)
                .is_some_and(|suffix| suffix.starts_with('='))
    })
}

fn collect_flag_values(tokens: &[String], flag: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let token = &tokens[i];
        if let Some(value) = token
            .strip_prefix(flag)
            .and_then(|suffix| suffix.strip_prefix('='))
        {
            if !value.is_empty() {
                out.push(value.to_string());
            }
        } else if token == flag {
            if let Some(value) = tokens.get(i + 1).filter(|value| !value.starts_with("--")) {
                out.push(value.clone());
                i += 1;
            }
        }
        i += 1;
    }
    out
}

fn joined_flag_values(tokens: &[String], flag: &str) -> Option<String> {
    let values = collect_flag_values(tokens, flag);
    (!values.is_empty()).then(|| values.join(","))
}

fn insert_app_update_context(
    context: &mut HashMap<String, String>,
    tokens: &[String],
    flags: &HashMap<&'static str, String>,
) {
    let mut fields = Vec::new();
    let value_fields = [
        "name",
        "description",
        "visibility",
        "resource_tier",
        "subdomain",
        "auth_mode",
        "category_id",
        "data_scopes",
        "icon_dark_url",
        "icon_url",
    ];
    for field in value_fields {
        if let Some(value) = flags.get(field).filter(|value| !value.is_empty()) {
            fields.push(field);
            context.insert(field.into(), value.clone());
        }
    }
    for (field, flag) in [
        ("clear_subdomain", "--clear-subdomain"),
        ("clear_category", "--clear-category"),
    ] {
        if has_flag(tokens, flag) {
            fields.push(field);
            context.insert(field.into(), "true".into());
        }
    }
    if !fields.is_empty() {
        context.insert("fields".into(), fields.join(","));
    }
}

fn sha256_bytes_context_value(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut out = String::from("sha256:");
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn sha256_context_value(value: &str) -> String {
    sha256_bytes_context_value(value.as_bytes())
}

fn file_sha256_context_value(path: &str) -> Option<String> {
    fs::read(path)
        .ok()
        .map(|contents| sha256_bytes_context_value(&contents))
}

fn source_marker(tokens: &[String], flags: &HashMap<&'static str, String>) -> Option<String> {
    flags
        .get("source")
        .cloned()
        .or_else(|| flags.get("body_file").cloned().map(|_| "body_file".into()))
        .or_else(|| flags.get("body").cloned().map(|_| "body".into()))
        .or_else(|| flags.get("batch").cloned().map(|_| "batch".into()))
        .or_else(|| {
            flags
                .get("config_file")
                .cloned()
                .map(|_| "config_file".into())
        })
        .or_else(|| {
            flags
                .get("config_json")
                .cloned()
                .map(|_| "config_json".into())
        })
        .or_else(|| {
            flags
                .get("credentials_file")
                .cloned()
                .map(|_| "credentials_file".into())
        })
        .or_else(|| {
            flags
                .get("credentials_json")
                .cloned()
                .map(|_| "credentials_json".into())
        })
        .or_else(|| {
            flags
                .get("items_file")
                .cloned()
                .map(|_| "items_file".into())
        })
        .or_else(|| {
            flags
                .get("items_json")
                .cloned()
                .map(|_| "items_json".into())
        })
        .or_else(|| has_flag(tokens, "--interactive").then(|| "interactive".into()))
        .or_else(|| has_flag(tokens, "--credentials-stdin").then(|| "stdin".into()))
        .or_else(|| has_flag(tokens, "--from-stdin").then(|| "stdin".into()))
        .or_else(|| flags.get("name").cloned().map(|_| "inline".into()))
}

fn insert_source_identity(
    context: &mut HashMap<String, String>,
    tokens: &[String],
    flags: &HashMap<&'static str, String>,
) {
    if let Some(body_file) = flags.get("body_file").filter(|value| !value.is_empty()) {
        context.insert("body_file".into(), body_file.clone());
        if let Some(digest) = file_sha256_context_value(body_file) {
            context.insert("body_digest".into(), digest);
        }
    }
    if let Some(body) = flags.get("body").filter(|value| !value.is_empty()) {
        context.insert("body_digest".into(), sha256_context_value(body));
    }
    if let Some(batch) = flags.get("batch").filter(|value| !value.is_empty()) {
        context.insert("batch".into(), batch.clone());
        if let Some(digest) = file_sha256_context_value(batch) {
            context.insert("batch_digest".into(), digest);
        }
    }
    if let Some(config_file) = flags.get("config_file").filter(|value| !value.is_empty()) {
        context.insert("config_file".into(), config_file.clone());
        if let Some(digest) = file_sha256_context_value(config_file) {
            context.insert("config_digest".into(), digest);
        }
    }
    if let Some(config) = flags.get("config_json").filter(|value| !value.is_empty()) {
        context.insert("config_digest".into(), sha256_context_value(config));
    }
    if let Some(credentials_file) = flags
        .get("credentials_file")
        .filter(|value| !value.is_empty())
    {
        context.insert("credentials_file".into(), credentials_file.clone());
        if let Some(digest) = file_sha256_context_value(credentials_file) {
            context.insert("credentials_digest".into(), digest);
        }
    }
    if let Some(credentials) = flags
        .get("credentials_json")
        .filter(|value| !value.is_empty())
    {
        context.insert(
            "credentials_digest".into(),
            sha256_context_value(credentials),
        );
    }
    if has_flag(tokens, "--credentials-stdin") {
        context.insert("credentials_source".into(), "stdin".into());
    }
    if let Some(items_file) = flags.get("items_file").filter(|value| !value.is_empty()) {
        context.insert("items_file".into(), items_file.clone());
        if let Some(digest) = file_sha256_context_value(items_file) {
            context.insert("items_digest".into(), digest);
        }
    }
    if let Some(items) = flags.get("items_json").filter(|value| !value.is_empty()) {
        context.insert("items_digest".into(), sha256_context_value(items));
    }
}

fn insert_digest_if_present(
    context: &mut HashMap<String, String>,
    key: &str,
    digest_key: &str,
    value: Option<&String>,
) -> bool {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        context.insert(key.into(), value.clone());
        context.insert(digest_key.into(), sha256_context_value(value));
        true
    } else {
        false
    }
}

fn split_env_split_string(value: &str) -> Vec<String> {
    shlex::split(value).unwrap_or_else(|| value.split_whitespace().map(str::to_string).collect())
}

fn strip_env_options(tokens: &mut Vec<String>) {
    while let Some(first) = tokens.first().cloned() {
        match first.as_str() {
            "--" => {
                tokens.remove(0);
            }
            "-S" | "--split-string" => {
                tokens.remove(0);
                if !tokens.is_empty() {
                    let split = split_env_split_string(&tokens.remove(0));
                    for token in split.into_iter().rev() {
                        tokens.insert(0, token);
                    }
                }
            }
            token if token.starts_with("--split-string=") => {
                let split_value = token
                    .split_once('=')
                    .map(|(_, value)| value.to_string())
                    .unwrap_or_default();
                tokens.remove(0);
                for token in split_env_split_string(&split_value).into_iter().rev() {
                    tokens.insert(0, token);
                }
            }
            "-u" | "--unset" => {
                tokens.remove(0);
                if !tokens.is_empty() {
                    tokens.remove(0);
                }
            }
            token if token.starts_with("--unset=") => {
                tokens.remove(0);
            }
            token if token.starts_with('-') => {
                tokens.remove(0);
            }
            _ => break,
        }
    }
}

fn strip_sudo_options(tokens: &mut Vec<String>) {
    while let Some(first) = tokens.first().cloned() {
        match first.as_str() {
            "--" => {
                tokens.remove(0);
            }
            "-u" | "--user" | "-g" | "--group" | "-h" | "--host" | "-p" | "--prompt" | "-C"
            | "--close-from" | "-T" | "--command-timeout" => {
                tokens.remove(0);
                if !tokens.is_empty() {
                    tokens.remove(0);
                }
            }
            token
                if token.starts_with("--user=")
                    || token.starts_with("--group=")
                    || token.starts_with("--host=")
                    || token.starts_with("--prompt=")
                    || token.starts_with("--close-from=")
                    || token.starts_with("--command-timeout=") =>
            {
                tokens.remove(0);
            }
            token if token.starts_with('-') => {
                tokens.remove(0);
            }
            _ => break,
        }
    }
}

fn strip_time_options(tokens: &mut Vec<String>) {
    while let Some(first) = tokens.first().cloned() {
        match first.as_str() {
            "--" | "-p" | "--portability" | "-v" | "--verbose" | "-a" | "--append" => {
                tokens.remove(0);
            }
            "-o" | "--output" | "-f" | "--format" => {
                tokens.remove(0);
                if !tokens.is_empty() {
                    tokens.remove(0);
                }
            }
            token if token.starts_with("--output=") || token.starts_with("--format=") => {
                tokens.remove(0);
            }
            token if token.starts_with('-') => {
                tokens.remove(0);
            }
            _ => break,
        }
    }
}

fn strip_command_options(tokens: &mut Vec<String>) {
    while let Some(first) = tokens.first().cloned() {
        match first.as_str() {
            "--" | "-p" | "-v" | "-V" => {
                tokens.remove(0);
            }
            token if token.starts_with('-') => {
                tokens.remove(0);
            }
            _ => break,
        }
    }
}

fn strip_exec_options(tokens: &mut Vec<String>) {
    while let Some(first) = tokens.first().cloned() {
        match first.as_str() {
            "--" | "-c" | "-l" => {
                tokens.remove(0);
            }
            "-a" => {
                tokens.remove(0);
                if !tokens.is_empty() {
                    tokens.remove(0);
                }
            }
            token if token.starts_with("-a") && token.len() > 2 => {
                tokens.remove(0);
            }
            token if token.starts_with('-') => {
                tokens.remove(0);
            }
            _ => break,
        }
    }
}

fn strip_nohup_options(tokens: &mut Vec<String>) {
    while let Some(first) = tokens.first().cloned() {
        if first == "--" || first.starts_with('-') {
            tokens.remove(0);
        } else {
            break;
        }
    }
}

fn normalized_command_name(token: &str) -> &str {
    token
        .trim_matches(|ch| matches!(ch, ';' | '&' | '|' | '(' | ')' | '`' | '\'' | '"'))
        .rsplit('/')
        .next()
        .unwrap_or(token)
}

fn env_profile_from_tokens(tokens: &[String]) -> Option<String> {
    tokens.iter().find_map(|token| {
        token
            .strip_prefix("AXHUB_PROFILE=")
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn is_shell_wrapper(token: &str) -> bool {
    matches!(
        normalized_command_name(token),
        "bash" | "sh" | "zsh" | "dash" | "ksh"
    )
}

fn shell_command_body(tokens: &[String], shell_index: usize) -> Option<String> {
    let mut i = shell_index + 1;
    while i < tokens.len() {
        let token = tokens[i].as_str();
        if token == "--" {
            i += 1;
            continue;
        }
        if let Some(body) = token.strip_prefix("--command=") {
            return (!body.is_empty()).then(|| body.to_string());
        }
        if token == "--command" {
            return tokens.get(i + 1).cloned();
        }
        if token.starts_with("--") {
            i += 1;
            continue;
        }
        if token.starts_with('-') && token.len() > 1 {
            if token[1..].contains('c') {
                return tokens.get(i + 1).cloned();
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    None
}

fn collect_shell_wrapper_bodies(cmd: &str) -> Vec<String> {
    let tokens = shlex::split(cmd).unwrap_or_else(|| {
        cmd.split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>()
    });
    let mut bodies = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        match normalized_command_name(token) {
            "eval" => {
                if let Some(body) = tokens.get(i + 1).filter(|body| !body.is_empty()) {
                    bodies.push(body.clone());
                }
            }
            _ if is_shell_wrapper(token) => {
                if let Some(body) = shell_command_body(&tokens, i).filter(|body| !body.is_empty()) {
                    bodies.push(body);
                }
            }
            _ => {}
        }
    }
    bodies
}

fn xargs_command_start(tokens: &[String], xargs_index: usize) -> Option<usize> {
    let mut i = xargs_index + 1;
    while i < tokens.len() {
        let token = tokens[i].as_str();
        match token {
            "--" => {
                i += 1;
                break;
            }
            "-E" | "--eof" | "-I" | "--replace" | "-L" | "--max-lines" | "-n" | "--max-args"
            | "-P" | "--max-procs" | "-s" | "--max-chars" => {
                i += 2;
            }
            token
                if token.starts_with("--eof=")
                    || token.starts_with("--replace=")
                    || token.starts_with("--max-lines=")
                    || token.starts_with("--max-args=")
                    || token.starts_with("--max-procs=")
                    || token.starts_with("--max-chars=")
                    || token.starts_with("-E")
                    || token.starts_with("-I")
                    || token.starts_with("-L")
                    || token.starts_with("-n")
                    || token.starts_with("-P")
                    || token.starts_with("-s") =>
            {
                i += 1;
            }
            token if token.starts_with('-') => {
                i += 1;
            }
            _ => break,
        }
    }
    (i < tokens.len()).then_some(i)
}

fn collect_indirect_runner_bodies(cmd: &str) -> Vec<String> {
    let tokens = shlex::split(cmd).unwrap_or_else(|| {
        cmd.split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>()
    });
    let mut bodies = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        match normalized_command_name(token) {
            "xargs" => {
                if let Some(start) = xargs_command_start(&tokens, i) {
                    let body = tokens[start..].join(" ");
                    if contains_axhub_tokenish(&body) && contains_mutation_signal(&body) {
                        bodies.push(body);
                    }
                }
            }
            "find" => {
                let mut j = i + 1;
                while j < tokens.len() {
                    if matches!(tokens[j].as_str(), "-exec" | "-execdir") {
                        let start = j + 1;
                        let end = tokens[start..]
                            .iter()
                            .position(|part| part == ";" || part == "+")
                            .map(|offset| start + offset)
                            .unwrap_or(tokens.len());
                        if start < end {
                            let body = tokens[start..end].join(" ");
                            if contains_axhub_tokenish(&body) && contains_mutation_signal(&body) {
                                bodies.push(body);
                            }
                        }
                        j = end;
                    }
                    j += 1;
                }
            }
            _ => {}
        }
    }
    bodies
}

fn is_help_request(tokens: &[String]) -> bool {
    tokens.iter().any(|t| matches!(t.as_str(), "--help" | "-h"))
}

fn destructive(action: &str, tokens: &[String], app_id: Option<String>) -> ParsedAxhubCommand {
    let flags = extract_flags(tokens.get(2..).unwrap_or_default());
    ParsedAxhubCommand {
        is_destructive: true,
        action: Some(action.into()),
        app_id: app_id.or_else(|| flags.get("app_id").cloned()),
        branch: flags.get("branch").cloned(),
        commit_sha: flags.get("commit_sha").cloned(),
        profile: flags.get("profile").cloned(),
        context: HashMap::new(),
    }
}

fn is_mutating_verb(token: &str) -> bool {
    matches!(
        token,
        "add"
            | "apply"
            | "bulk"
            | "bulk-register"
            | "call"
            | "cancel"
            | "connect"
            | "create"
            | "credentials-set"
            | "delete"
            | "disconnect"
            | "drop"
            | "fleet"
            | "fork"
            | "grant"
            | "insert"
            | "invite"
            | "issue"
            | "login"
            | "logout"
            | "move"
            | "publish"
            | "remove"
            | "rename"
            | "resend"
            | "resume"
            | "revoke"
            | "rollback"
            | "rotate"
            | "send"
            | "set"
            | "submit"
            | "suspend"
            | "tag-attach"
            | "tag-detach"
            | "uninvite"
            | "unset"
            | "update"
            | "upsert"
            | "use"
    )
}

fn is_unknown_mutating_axhub_command(tokens: &[String]) -> bool {
    has_flag(tokens, "--execute")
        || tokens
            .iter()
            .skip(1)
            .take_while(|token| !token.starts_with('-'))
            .any(|token| is_mutating_verb(token))
}

fn unknown_destructive(tokens: &[String]) -> ParsedAxhubCommand {
    let mut parsed = destructive("unknown_axhub_mutation", tokens, None);
    parsed.context.insert(
        "command_path".into(),
        tokens
            .iter()
            .skip(1)
            .take_while(|token| !token.starts_with('-'))
            .cloned()
            .collect::<Vec<_>>()
            .join(" "),
    );
    parsed
}

fn unknown_destructive_reason(tokens: &[String], reason: &str) -> ParsedAxhubCommand {
    let mut parsed = unknown_destructive(tokens);
    parsed.context.insert("reason".into(), reason.into());
    parsed
}

fn insert_if_some(context: &mut HashMap<String, String>, key: &str, value: Option<String>) {
    if let Some(value) = value.filter(|v| !v.is_empty()) {
        context.insert(key.into(), value);
    }
}

fn collect_command_positions(cmd: &str, depth: usize) -> Vec<String> {
    let mut positions = vec![cmd.to_string()];
    if depth >= COLLECT_MAX_DEPTH {
        return positions;
    }
    let chars: Vec<char> = cmd.chars().collect();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\\' {
            i += 2;
            continue;
        }
        match ch {
            '\'' if !in_double && !in_backtick => in_single = !in_single,
            '"' if !in_single && !in_backtick => in_double = !in_double,
            '`' if !in_single && !in_double => {
                in_backtick = !in_backtick;
                positions.push(chars[i + 1..].iter().collect());
            }
            ';' | '&' | '|' | '\n' | '\r' if !in_single && !in_double && !in_backtick => {
                let mut j = i + 1;
                while j < chars.len()
                    && (chars[j] == '&' || chars[j] == '|' || chars[j] == '\n' || chars[j] == '\r')
                {
                    j += 1;
                }
                positions.push(chars[j..].iter().collect());
                i = j;
                continue;
            }
            '$' if !in_single && chars.get(i + 1) == Some(&'(') => {
                positions.push(chars[i + 2..].iter().collect());
                i += 2;
                continue;
            }
            '(' if !in_single && !in_double && !in_backtick => {
                positions.push(chars[i + 1..].iter().collect());
                i += 1;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    for body in collect_shell_wrapper_bodies(cmd) {
        positions.push(body.clone());
        let unescaped = body.replace("\\\"", "\"").replace("\\'", "'");
        if unescaped != body {
            positions.push(unescaped.clone());
            positions.extend(
                collect_command_positions(&unescaped, depth + 1)
                    .into_iter()
                    .skip(1),
            );
        }
        positions.extend(
            collect_command_positions(&body, depth + 1)
                .into_iter()
                .skip(1),
        );
    }
    for body in collect_indirect_runner_bodies(cmd) {
        positions.push(body.clone());
        positions.extend(
            collect_command_positions(&body, depth + 1)
                .into_iter()
                .skip(1),
        );
    }
    positions
}

fn contains_axhub_tokenish(value: &str) -> bool {
    shlex::split(value)
        .unwrap_or_else(|| {
            value
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .iter()
        .any(|token| !is_shell_assignment_token(token) && normalized_command_name(token) == "axhub")
        || value.contains("axhub ")
}

fn contains_mutation_signal(value: &str) -> bool {
    value.contains("--execute")
        || shlex::split(value)
            .unwrap_or_else(|| {
                value
                    .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
                    .filter(|token| !token.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .iter()
            .any(|token| is_mutating_verb(token))
}

fn contains_non_dry_run_mutation_signal(value: &str) -> bool {
    if value.contains("--dry-run")
        && !value.contains("--execute")
        && value.contains("deploy create")
    {
        let mutating_tokens = shlex::split(value)
            .unwrap_or_else(|| {
                value
                    .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
                    .filter(|token| !token.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .into_iter()
            .filter(|token| is_mutating_verb(token))
            .collect::<Vec<_>>();
        if !mutating_tokens.is_empty() && mutating_tokens.iter().all(|token| token == "create") {
            return false;
        }
    }
    contains_mutation_signal(value)
}

fn contains_dynamic_axhub_mutation(cmd: &str) -> bool {
    if !(cmd.contains("$(") || cmd.contains('`')) {
        return false;
    }
    collect_command_positions(cmd, 0)
        .into_iter()
        .skip(1)
        .any(|pos| {
            contains_structural_axhub_mutation(&pos)
                || ((pos.contains("--execute") || pos.contains("echo axhub"))
                    && contains_axhub_tokenish(&pos)
                    && contains_non_dry_run_mutation_signal(&pos))
        })
}

fn contains_indirect_runner_axhub_mutation(cmd: &str) -> bool {
    collect_indirect_runner_bodies(cmd)
        .iter()
        .any(|body| contains_non_dry_run_axhub_mutation(body))
}

fn is_variable_command_token(token: &str) -> bool {
    let trimmed =
        token.trim_matches(|ch| matches!(ch, ';' | '&' | '|' | '(' | ')' | '`' | '\'' | '"'));
    trimmed.starts_with('$')
}

fn is_shell_assignment_token(token: &str) -> bool {
    let trimmed =
        token.trim_matches(|ch| matches!(ch, ';' | '&' | '|' | '(' | ')' | '`' | '\'' | '"'));
    let Some((name, _)) = trimmed.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn strip_leading_wrappers(tokens: &mut Vec<String>) {
    loop {
        if tokens
            .first()
            .is_some_and(|token| is_shell_assignment_token(token))
        {
            tokens.remove(0);
            continue;
        }
        let Some(first_name) = tokens
            .first()
            .map(|first| normalized_command_name(first).to_string())
        else {
            break;
        };
        match first_name.as_str() {
            "do" | "then" | "else" | "elif" | "if" | "while" | "until" => {
                tokens.remove(0);
            }
            "time" => {
                tokens.remove(0);
                strip_time_options(tokens);
            }
            "command" => {
                tokens.remove(0);
                strip_command_options(tokens);
            }
            "exec" => {
                tokens.remove(0);
                strip_exec_options(tokens);
            }
            "nohup" => {
                tokens.remove(0);
                strip_nohup_options(tokens);
            }
            "env" => {
                tokens.remove(0);
                strip_env_options(tokens);
            }
            "sudo" => {
                tokens.remove(0);
                strip_sudo_options(tokens);
            }
            _ => break,
        }
    }
}

fn variable_command_has_non_dry_run_mutation(tokens: &[String]) -> bool {
    let args = tokens.iter().skip(1).cloned().collect::<Vec<_>>();
    let rest = args.join(" ");
    if args
        .first()
        .map(|token| normalized_command_name(token))
        .is_none_or(|token| !is_axhub_subcommandish(token))
        && contains_axhub_tokenish(&rest)
    {
        return contains_non_dry_run_axhub_mutation(&rest);
    }
    if args.first().map(String::as_str) == Some("deploy")
        && args.get(1).map(String::as_str) == Some("create")
        && has_flag(&args, "--dry-run")
        && !has_flag(&args, "--execute")
    {
        return false;
    }
    contains_mutation_signal(&rest)
}

fn is_axhub_subcommandish(token: &str) -> bool {
    matches!(
        token,
        "admin"
            | "apis"
            | "apps"
            | "auth"
            | "bootstrap"
            | "connectors"
            | "data"
            | "deploy"
            | "env"
            | "gateway"
            | "install-deps"
            | "logs"
            | "profile"
            | "publish"
            | "resources"
            | "tables"
            | "update"
    )
}

fn contains_variable_axhub_mutation(cmd: &str) -> bool {
    collect_command_positions(cmd, 0).into_iter().any(|pos| {
        let mut tokens = shlex::split(pos.trim_start()).unwrap_or_else(|| {
            pos.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
        strip_leading_wrappers(&mut tokens);
        tokens
            .first()
            .is_some_and(|token| is_variable_command_token(token))
            && variable_command_has_non_dry_run_mutation(&tokens)
    })
}

fn is_shell_script_runner(tokens: &[String]) -> bool {
    let mut normalized = tokens.to_vec();
    strip_leading_wrappers(&mut normalized);
    matches!(
        normalized
            .first()
            .map(|token| normalized_command_name(token)),
        Some("sh" | "bash" | "zsh" | "source" | ".")
    ) && shell_command_body(&normalized, 0).is_none()
}

fn contains_shell_script_axhub_mutation(cmd: &str) -> bool {
    if !contains_non_dry_run_axhub_mutation(cmd) {
        return false;
    }
    collect_command_positions(cmd, 0).into_iter().any(|pos| {
        let tokens = shlex::split(pos.trim_start()).unwrap_or_else(|| {
            pos.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
        is_shell_script_runner(&tokens)
    })
}

fn strip_shell_punctuation(value: &str) -> &str {
    value.trim_matches(|ch| matches!(ch, ';' | '&' | '|' | '(' | ')' | '`' | '\'' | '"'))
}

fn alias_rhs_points_to_axhub(value: &str) -> bool {
    let rhs = strip_shell_punctuation(value);
    normalized_command_name(rhs) == "axhub" || rhs.starts_with("axhub ") || rhs.contains("/axhub")
}

fn collect_axhub_alias_names(cmd: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    for pos in collect_command_positions(cmd, 0) {
        let mut tokens = shlex::split(pos.trim_start()).unwrap_or_else(|| {
            pos.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        });
        strip_leading_wrappers(&mut tokens);
        if tokens
            .first()
            .is_none_or(|token| normalized_command_name(token) != "alias")
        {
            continue;
        }
        for token in tokens.iter().skip(1) {
            if let Some((name, value)) = token.split_once('=') {
                let name = strip_shell_punctuation(name);
                if !name.is_empty() && alias_rhs_points_to_axhub(value) {
                    aliases.push(name.to_string());
                }
            }
        }
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn contains_axhub_alias_invocation(cmd: &str, aliases: &[String]) -> bool {
    !aliases.is_empty()
        && collect_command_positions(cmd, 0).into_iter().any(|pos| {
            let mut tokens = shlex::split(pos.trim_start()).unwrap_or_else(|| {
                pos.split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            });
            strip_leading_wrappers(&mut tokens);
            let Some(first) = tokens.first() else {
                return false;
            };
            let command_name = normalized_command_name(first);
            aliases.iter().any(|alias| alias == command_name)
                && variable_command_has_non_dry_run_mutation(&tokens)
        })
}

fn contains_shell_function_block_marker(cmd: &str) -> bool {
    let mut search_from = 0;
    while let Some(offset) = cmd[search_from..].find("()") {
        let after = search_from + offset + 2;
        if cmd[after..]
            .chars()
            .find(|ch| !ch.is_whitespace())
            .is_some_and(|ch| ch == '{')
        {
            return true;
        }
        search_from = after;
    }
    false
}

fn contains_shell_alias_axhub_mutation(cmd: &str) -> bool {
    let aliases = collect_axhub_alias_names(cmd);
    contains_axhub_alias_invocation(cmd, &aliases)
        || (contains_non_dry_run_axhub_mutation(cmd)
            && (contains_shell_function_block_marker(cmd) || cmd.contains("function ")))
}

fn context_payload_file_paths(context: &HashMap<String, String>) -> Vec<&str> {
    let mut paths = Vec::new();
    for key in [
        "body_file",
        "batch",
        "config_file",
        "credentials_file",
        "items_file",
    ] {
        if let Some(path) = context.get(key).filter(|value| !value.is_empty()) {
            paths.push(path.as_str());
        }
    }
    if let Some(source) = context.get("source").filter(|value| {
        !value.is_empty()
            && !matches!(
                value.as_str(),
                "body"
                    | "body_file"
                    | "batch"
                    | "config_file"
                    | "config_json"
                    | "credentials_file"
                    | "credentials_json"
                    | "items_file"
                    | "items_json"
                    | "interactive"
                    | "inline"
                    | "stdin"
            )
    }) {
        paths.push(source.as_str());
    }
    paths
}

fn prefix_has_write_to_payload_file(prefix: &str, paths: &[&str]) -> bool {
    if paths.is_empty() {
        return false;
    }
    let prefix_lower = prefix.to_ascii_lowercase();
    let has_write_operator = prefix.contains('>')
        || prefix.contains(" tee ")
        || prefix.contains(" dd ")
        || prefix.contains(" cp ")
        || prefix.contains(" mv ")
        || prefix.contains(" ln ")
        || prefix.trim_start().starts_with("ln ")
        || prefix.contains("sed -i")
        || prefix.contains("perl -i")
        || prefix_lower.contains("write_text")
        || prefix_lower.contains("write_bytes")
        || prefix_lower.contains("writefile")
        || prefix_lower.contains("writefilesync")
        || prefix_lower.contains("file.write")
        || prefix_lower.contains("file.open")
        || prefix_lower.contains("with open")
        || prefix_lower.contains("open(");
    has_write_operator
        && paths.iter().any(|path| {
            prefix.contains(path)
                || Path::new(path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.is_empty())
                    .is_some_and(|name| prefix.contains(name))
        })
}

fn has_prior_payload_file_write(
    cmd: &str,
    command_position: &str,
    parsed: &ParsedAxhubCommand,
) -> bool {
    let paths = context_payload_file_paths(&parsed.context);
    let needle = command_position.trim_start();
    cmd.rfind(needle)
        .map(|idx| prefix_has_write_to_payload_file(&cmd[..idx], &paths))
        .unwrap_or(false)
}

fn tokens_if_axhub_command(raw_position: &str) -> Option<Vec<String>> {
    let mut s = raw_position.trim_start().to_string();
    while s.starts_with(['\"', '\'', '`', '(']) {
        s.remove(0);
    }
    if s.starts_with("$(") {
        s = s[2..].to_string();
    }
    s = s.trim_start().to_string();
    let mut tokens = shlex::split(&s)
        .unwrap_or_else(|| s.split_whitespace().map(str::to_string).collect::<Vec<_>>());
    for token in &mut tokens {
        while token.ends_with([';', ')', '`', '\'', '"']) {
            token.pop();
        }
    }
    tokens.retain(|token| !token.is_empty());
    let profile_from_env = env_profile_from_tokens(&tokens);
    strip_leading_wrappers(&mut tokens);
    if tokens.first().map(|token| normalized_command_name(token)) != Some("axhub") {
        return None;
    }
    tokens[0] = "axhub".into();
    if let Some(profile) = profile_from_env.filter(|_| !has_flag(&tokens, "--profile")) {
        tokens.push("--profile".into());
        tokens.push(profile);
    }
    Some(tokens)
}

fn contains_non_dry_run_axhub_mutation(value: &str) -> bool {
    let mut saw_structural_axhub = false;
    for pos in collect_command_positions(value, 0) {
        let Some(tokens) = tokens_if_axhub_command(&pos) else {
            continue;
        };
        saw_structural_axhub = true;
        if match_known_intent(&tokens).is_some_and(|parsed| parsed.is_destructive) {
            return true;
        }
    }
    !saw_structural_axhub
        && contains_axhub_tokenish(value)
        && contains_non_dry_run_mutation_signal(value)
}

fn contains_structural_axhub_mutation(value: &str) -> bool {
    collect_command_positions(value, 0).into_iter().any(|pos| {
        tokens_if_axhub_command(&pos)
            .and_then(|tokens| match_known_intent(&tokens))
            .is_some_and(|parsed| parsed.is_destructive)
    })
}

fn match_known_intent(tokens: &[String]) -> Option<ParsedAxhubCommand> {
    if is_help_request(tokens) {
        return None;
    }
    let sub = tokens.get(1).map(String::as_str);
    let sub2 = tokens.get(2).map(String::as_str);
    let sub3 = tokens.get(3).map(String::as_str);
    let flags = extract_flags(tokens.get(2..).unwrap_or_default());
    let parsed = match (sub, sub2, sub3) {
        (Some("publish"), _, _) => {
            let mut parsed = destructive("publish_submit", tokens, None);
            let note = flags.get("note").cloned().unwrap_or_default();
            parsed
                .context
                .insert("note_length".into(), note.chars().count().to_string());
            parsed
                .context
                .insert("note_digest".into(), sha256_context_value(&note));
            parsed
        }
        (Some("deploy"), Some("create"), _)
            if has_flag(tokens, "--dry-run") && !has_flag(tokens, "--execute") =>
        {
            return None;
        }
        (Some("deploy"), Some("create"), _) => destructive("deploy_create", tokens, None),
        (Some("deploy"), Some("rollback"), _) => {
            let mut parsed = destructive("deploy_rollback", tokens, None);
            insert_if_some(
                &mut parsed.context,
                "from_deployment",
                flags.get("from_deployment").cloned(),
            );
            parsed
        }
        (Some("update"), Some("check"), _) => return None,
        (Some("update"), Some("apply"), _) => destructive("update_apply", tokens, None),
        (Some("deploy"), Some("logs"), _) if tokens.iter().any(|t| t == "--kill") => {
            destructive("deploy_logs_kill", tokens, None)
        }
        (Some("auth"), Some("login"), _) => destructive("auth_login", tokens, None),
        (Some("auth"), Some("logout"), _) => {
            let mut parsed = destructive("auth_logout", tokens, None);
            insert_if_some(
                &mut parsed.context,
                "profile",
                flags
                    .get("profile")
                    .cloned()
                    .or_else(|| Some("<active>".into())),
            );
            parsed
        }
        (Some("auth"), Some("oauth"), Some("client"))
            if tokens.get(4).map(String::as_str) == Some("create") =>
        {
            let mut parsed = destructive("auth_oauth_client_create", tokens, None);
            insert_if_some(&mut parsed.context, "name", flags.get("name").cloned());
            insert_if_some(&mut parsed.context, "type", flags.get("type").cloned());
            insert_if_some(
                &mut parsed.context,
                "auth_method",
                flags.get("auth_method").cloned(),
            );
            insert_if_some(
                &mut parsed.context,
                "redirect_uris",
                joined_flag_values(tokens, "--redirect-uri"),
            );
            insert_if_some(
                &mut parsed.context,
                "scopes",
                joined_flag_values(tokens, "--scope"),
            );
            insert_if_some(
                &mut parsed.context,
                "grant_types",
                joined_flag_values(tokens, "--grant-type"),
            );
            parsed
        }
        (Some("auth"), Some("oauth"), Some("revoke")) => {
            let mut parsed = destructive("auth_oauth_revoke", tokens, None);
            insert_if_some(
                &mut parsed.context,
                "target",
                positional(tokens, 4).or_else(|| Some("<stdin>".into())),
            );
            insert_if_some(
                &mut parsed.context,
                "client_id",
                flags
                    .get("client_id")
                    .cloned()
                    .or_else(|| Some("default".into())),
            );
            insert_if_some(
                &mut parsed.context,
                "token_type_hint",
                flags.get("token_type_hint").cloned(),
            );
            parsed
        }
        (Some("auth"), Some("oauth"), Some("consent"))
            if tokens.get(4).map(String::as_str) == Some("revoke") =>
        {
            let mut parsed = destructive("auth_oauth_consent_revoke", tokens, None);
            insert_if_some(&mut parsed.context, "client_id", positional(tokens, 5));
            parsed
        }
        (Some("auth"), Some("pat"), Some("issue")) => {
            let mut parsed = destructive("auth_pat_issue", tokens, None);
            insert_if_some(&mut parsed.context, "name", flags.get("name").cloned());
            insert_if_some(
                &mut parsed.context,
                "expires_in_days",
                flags.get("expires_in_days").cloned(),
            );
            parsed
                .context
                .insert("use".into(), has_flag(tokens, "--use").to_string());
            parsed
                .context
                .insert("no_save".into(), has_flag(tokens, "--no-save").to_string());
            parsed.context.insert(
                "show_token".into(),
                has_flag(tokens, "--show-token").to_string(),
            );
            parsed
        }
        (Some("auth"), Some("pat"), Some("revoke")) => {
            let mut parsed = destructive("auth_pat_revoke", tokens, None);
            insert_if_some(&mut parsed.context, "pat_id", positional(tokens, 4));
            parsed
        }
        (Some("auth"), Some("pat"), Some("use")) => {
            let mut parsed = destructive("auth_pat_use", tokens, None);
            insert_if_some(&mut parsed.context, "pat_id", positional(tokens, 4));
            insert_if_some(
                &mut parsed.context,
                "profile",
                flags
                    .get("profile")
                    .cloned()
                    .or_else(|| Some("<active>".into())),
            );
            parsed
        }
        (Some("auth"), Some("pat"), Some("unset")) => {
            let mut parsed = destructive("auth_pat_unset", tokens, None);
            parsed.context.insert("target".into(), "active_pat".into());
            insert_if_some(
                &mut parsed.context,
                "profile",
                flags
                    .get("profile")
                    .cloned()
                    .or_else(|| Some("<active>".into())),
            );
            parsed
        }
        (Some("auth"), Some("pat"), Some("rotate")) => {
            let mut parsed = destructive("auth_pat_rotate", tokens, None);
            insert_if_some(
                &mut parsed.context,
                "name",
                flags
                    .get("name")
                    .cloned()
                    .or_else(|| Some("<default>".into())),
            );
            insert_if_some(
                &mut parsed.context,
                "expires_in_days",
                flags.get("expires_in_days").cloned(),
            );
            parsed
        }
        (Some("env"), Some("set"), _) => {
            let mut parsed = destructive("env_set", tokens, None);
            insert_if_some(&mut parsed.context, "key", positional(tokens, 3));
            parsed
        }
        (Some("env"), Some("delete"), _) | (Some("env"), Some("unset"), _) => {
            let mut parsed = destructive("env_delete", tokens, None);
            insert_if_some(&mut parsed.context, "key", positional(tokens, 3));
            parsed
        }
        (Some("apps"), Some("create"), _) => {
            let app = flags.get("slug").cloned().or_else(|| positional(tokens, 3));
            let mut parsed = destructive("apps_create", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            parsed
        }
        (Some("apps"), Some("update"), _) => {
            let app = positional(tokens, 3).or_else(|| flags.get("slug").cloned());
            let mut parsed = destructive("apps_update", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            insert_app_update_context(&mut parsed.context, tokens, &flags);
            parsed
        }
        (Some("apps"), Some("delete"), _) | (Some("apps"), Some("rm"), _) => {
            let app = positional(tokens, 3).or_else(|| flags.get("slug").cloned());
            let mut parsed = destructive("apps_delete", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            parsed
        }
        (Some("apps"), Some("fork"), _) => {
            let app = positional(tokens, 3).or_else(|| flags.get("app_id").cloned());
            let mut parsed = destructive("apps_fork", tokens, app.clone());
            insert_if_some(&mut parsed.context, "source", app);
            insert_if_some(&mut parsed.context, "slug", flags.get("slug").cloned());
            insert_if_some(
                &mut parsed.context,
                "subdomain",
                flags.get("subdomain").cloned(),
            );
            insert_if_some(
                &mut parsed.context,
                "tenant",
                flags
                    .get("tenant")
                    .cloned()
                    .or_else(|| Some("<active>".into())),
            );
            insert_if_some(
                &mut parsed.context,
                "name",
                flags
                    .get("name")
                    .cloned()
                    .or_else(|| flags.get("slug").cloned()),
            );
            insert_if_some(
                &mut parsed.context,
                "template",
                flags
                    .get("template")
                    .cloned()
                    .or_else(|| Some("<source>".into())),
            );
            parsed.context.insert(
                "repo_public".into(),
                has_flag(tokens, "--repo-public").to_string(),
            );
            parsed
        }
        (Some("apps"), Some("suspend"), _) => {
            let app = positional(tokens, 3).or_else(|| flags.get("app_id").cloned());
            destructive("apps_suspend", tokens, app)
        }
        (Some("apps"), Some("resume"), _) => {
            let app = positional(tokens, 3).or_else(|| flags.get("app_id").cloned());
            destructive("apps_resume", tokens, app)
        }
        (Some("github"), Some("connect"), _) | (Some("apps"), Some("git"), Some("connect")) => {
            let app = if sub == Some("apps") {
                flags
                    .get("app_id")
                    .cloned()
                    .or_else(|| positional(tokens, 4))
            } else {
                positional(tokens, 3).or_else(|| flags.get("app_id").cloned())
            };
            let mut parsed = destructive("github_connect", tokens, app);
            insert_if_some(&mut parsed.context, "repo", flags.get("repo").cloned());
            insert_if_some(&mut parsed.context, "branch", flags.get("branch").cloned());
            insert_if_some(
                &mut parsed.context,
                "account",
                flags.get("account").cloned(),
            );
            parsed
        }
        (Some("github"), Some("disconnect"), _)
        | (Some("apps"), Some("git"), Some("disconnect")) => {
            let app = if sub == Some("apps") {
                flags
                    .get("app_id")
                    .cloned()
                    .or_else(|| positional(tokens, 4))
            } else {
                positional(tokens, 3).or_else(|| flags.get("app_id").cloned())
            };
            let mut parsed = destructive("github_disconnect", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            parsed
        }
        (Some("deploy"), Some("cancel"), _) => {
            let mut parsed = destructive("deploy_cancel", tokens, None);
            insert_if_some(&mut parsed.context, "deployment_id", positional(tokens, 3));
            parsed
        }
        (Some("profile"), Some("add"), _) => {
            let mut parsed = destructive("profile_add", tokens, None);
            insert_if_some(&mut parsed.context, "profile", positional(tokens, 3));
            insert_if_some(
                &mut parsed.context,
                "endpoint",
                flags.get("endpoint").cloned(),
            );
            parsed
        }
        (Some("profile"), Some("use"), _) => {
            let mut parsed = destructive("profile_use", tokens, None);
            insert_if_some(&mut parsed.context, "profile", positional(tokens, 3));
            parsed
        }
        (Some("apis"), Some("call"), _) => {
            let mut parsed = destructive("apis_call", tokens, None);
            insert_if_some(&mut parsed.context, "endpoint_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "method", flags.get("method").cloned());
            insert_if_some(
                &mut parsed.context,
                "body_file",
                flags.get("body_file").cloned(),
            );
            parsed
        }
        (Some("invitations"), Some("send"), _) => {
            let mut parsed = destructive("invitation_send", tokens, None);
            insert_if_some(&mut parsed.context, "email", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            insert_if_some(&mut parsed.context, "role", flags.get("role").cloned());
            parsed
        }
        (Some("invitations"), Some("bulk"), _) => {
            let mut parsed = destructive("invitation_bulk", tokens, None);
            insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            insert_if_some(&mut parsed.context, "role", flags.get("role").cloned());
            parsed
        }
        (Some("invitations"), Some("cancel"), _) => {
            let mut parsed = destructive("invitation_cancel", tokens, None);
            insert_if_some(&mut parsed.context, "invitation_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        (Some("invitations"), Some("resend"), _) => {
            let mut parsed = destructive("invitation_resend", tokens, None);
            insert_if_some(&mut parsed.context, "invitation_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            insert_if_some(&mut parsed.context, "role", flags.get("role").cloned());
            parsed
        }
        (Some("access"), Some("grant"), _) => destructive("access_grant", tokens, None),
        (Some("access"), Some("revoke"), _) => destructive("access_revoke", tokens, None),
        (Some("access"), Some("invite"), _) => {
            let mut parsed = destructive("access_invite", tokens, None);
            insert_if_some(&mut parsed.context, "user", flags.get("user").cloned());
            parsed
        }
        (Some("access"), Some("uninvite"), _) => {
            let mut parsed = destructive("access_uninvite", tokens, None);
            insert_if_some(&mut parsed.context, "user", flags.get("user").cloned());
            parsed
        }
        (Some("tables"), Some("create"), _) => {
            let mut parsed = destructive("tables_create", tokens, None);
            insert_if_some(&mut parsed.context, "table", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "column", flags.get("column").cloned());
            parsed
        }
        (Some("tables"), Some("drop"), _) => {
            let mut parsed = destructive("tables_drop", tokens, None);
            insert_if_some(&mut parsed.context, "table", positional(tokens, 3));
            parsed
        }
        (Some("tables"), Some("columns"), Some("add")) => {
            let mut parsed = destructive("tables_columns_add", tokens, None);
            insert_if_some(&mut parsed.context, "table", positional(tokens, 4));
            insert_if_some(&mut parsed.context, "name", flags.get("name").cloned());
            insert_if_some(&mut parsed.context, "type", flags.get("type").cloned());
            parsed
        }
        (Some("tables"), Some("columns"), Some("remove")) => {
            let mut parsed = destructive("tables_columns_remove", tokens, None);
            insert_if_some(&mut parsed.context, "table", positional(tokens, 4));
            insert_if_some(&mut parsed.context, "name", flags.get("name").cloned());
            parsed
        }
        (Some("tables"), Some("grants"), Some("issue")) => {
            let mut parsed = destructive("tables_grants_issue", tokens, None);
            insert_if_some(
                &mut parsed.context,
                "table",
                flags
                    .get("table")
                    .cloned()
                    .or_else(|| positional(tokens, 4)),
            );
            insert_if_some(
                &mut parsed.context,
                "principal_id",
                flags.get("principal_id").cloned(),
            );
            insert_if_some(
                &mut parsed.context,
                "actions",
                flags.get("actions").cloned(),
            );
            parsed
        }
        (Some("tables"), Some("grants"), Some("revoke")) => {
            let mut parsed = destructive("tables_grants_revoke", tokens, None);
            insert_if_some(
                &mut parsed.context,
                "table",
                flags
                    .get("table")
                    .cloned()
                    .or_else(|| positional(tokens, 4)),
            );
            insert_if_some(
                &mut parsed.context,
                "grant_id",
                flags.get("grant_id").cloned(),
            );
            parsed
        }
        (Some("data"), Some("insert"), _) => {
            let mut parsed = destructive("data_insert", tokens, None);
            insert_if_some(&mut parsed.context, "table", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            insert_source_identity(&mut parsed.context, tokens, &flags);
            parsed
        }
        (Some("data"), Some("update"), _) => {
            let mut parsed = destructive("data_update", tokens, None);
            insert_if_some(&mut parsed.context, "table", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "row_id", positional(tokens, 4));
            insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            insert_source_identity(&mut parsed.context, tokens, &flags);
            parsed
        }
        (Some("data"), Some("delete"), _) => {
            let mut parsed = destructive("data_delete", tokens, None);
            insert_if_some(&mut parsed.context, "table", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "row_id", positional(tokens, 4));
            parsed
        }
        (Some("connectors"), Some("create"), _) => {
            let mut parsed = destructive("connector_create", tokens, None);
            insert_if_some(&mut parsed.context, "name", flags.get("name").cloned());
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            insert_if_some(&mut parsed.context, "engine", flags.get("engine").cloned());
            insert_digest_if_present(
                &mut parsed.context,
                "description",
                "description_digest",
                flags.get("description"),
            );
            insert_source_identity(&mut parsed.context, tokens, &flags);
            parsed
        }
        (Some("connectors"), Some("update"), _) => {
            let mut parsed = destructive("connector_update", tokens, None);
            let mut fields = Vec::new();
            insert_if_some(&mut parsed.context, "connector_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            if flags.contains_key("config_file") || flags.contains_key("config_json") {
                fields.push("config");
                insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            }
            insert_source_identity(&mut parsed.context, tokens, &flags);
            if insert_digest_if_present(
                &mut parsed.context,
                "description",
                "description_digest",
                flags.get("description"),
            ) {
                fields.push("description");
            }
            if has_flag(tokens, "--enabled") {
                fields.push("enabled");
                parsed.context.insert("enabled".into(), "true".into());
            }
            if has_flag(tokens, "--disabled") {
                fields.push("disabled");
                parsed.context.insert("disabled".into(), "true".into());
            }
            if !fields.is_empty() {
                parsed.context.insert("fields".into(), fields.join(","));
            }
            parsed
        }
        (Some("connectors"), Some("credentials-set"), _) => {
            let mut parsed = destructive("connector_credentials_set", tokens, None);
            insert_if_some(&mut parsed.context, "connector_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            insert_source_identity(&mut parsed.context, tokens, &flags);
            parsed
        }
        (Some("connectors"), Some("delete"), _) => {
            let mut parsed = destructive("connector_delete", tokens, None);
            insert_if_some(&mut parsed.context, "connector_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        (Some("resources"), Some("namespace"), Some("create")) => {
            let mut parsed = destructive("resource_namespace_create", tokens, None);
            insert_if_some(&mut parsed.context, "name", flags.get("name").cloned());
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            insert_if_some(
                &mut parsed.context,
                "parent_id",
                flags.get("parent_id").cloned(),
            );
            parsed
        }
        (Some("resources"), Some("rename"), _) => {
            let mut parsed = destructive("resource_rename", tokens, None);
            insert_if_some(&mut parsed.context, "resource_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "name", flags.get("name").cloned());
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        (Some("resources"), Some("move"), _) => {
            let mut parsed = destructive("resource_move", tokens, None);
            insert_if_some(&mut parsed.context, "resource_id", positional(tokens, 3));
            insert_if_some(
                &mut parsed.context,
                "parent_id",
                flags
                    .get("parent_id")
                    .cloned()
                    .or_else(|| has_flag(tokens, "--root").then(|| "root".into())),
            );
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        (Some("resources"), Some("bulk-register"), _) => {
            let mut parsed = destructive("resource_bulk_register", tokens, None);
            insert_if_some(
                &mut parsed.context,
                "connector_id",
                flags.get("connector_id").cloned(),
            );
            insert_if_some(&mut parsed.context, "source", source_marker(tokens, &flags));
            insert_source_identity(&mut parsed.context, tokens, &flags);
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        (Some("resources"), Some("delete"), _) => {
            let mut parsed = destructive("resource_delete", tokens, None);
            insert_if_some(&mut parsed.context, "resource_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        (Some("resources"), Some("tag-attach"), _) => {
            let mut parsed = destructive("resource_tag_attach", tokens, None);
            insert_if_some(&mut parsed.context, "resource_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tag_id", flags.get("tag_id").cloned());
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        (Some("resources"), Some("tag-detach"), _) => {
            let mut parsed = destructive("resource_tag_detach", tokens, None);
            insert_if_some(&mut parsed.context, "resource_id", positional(tokens, 3));
            insert_if_some(&mut parsed.context, "tag_id", flags.get("tag_id").cloned());
            insert_if_some(&mut parsed.context, "tenant", flags.get("tenant").cloned());
            parsed
        }
        _ if is_unknown_mutating_axhub_command(tokens) => unknown_destructive(tokens),
        _ => return None,
    };
    Some(parsed)
}

pub fn parse_axhub_commands(cmd: &str) -> Vec<ParsedAxhubCommand> {
    let mut hits = Vec::new();
    let mut seen_positions = Vec::<String>::new();
    if contains_dynamic_axhub_mutation(cmd) {
        hits.push(unknown_destructive_reason(
            &["axhub".into(), "dynamic-shell".into()],
            "dynamic_shell_axhub_mutation",
        ));
    }
    if contains_variable_axhub_mutation(cmd) {
        hits.push(unknown_destructive_reason(
            &["axhub".into(), "variable-command".into()],
            "variable_axhub_command",
        ));
    }
    if contains_indirect_runner_axhub_mutation(cmd) {
        hits.push(unknown_destructive_reason(
            &["axhub".into(), "indirect-runner".into()],
            "indirect_axhub_runner",
        ));
    }
    if contains_shell_script_axhub_mutation(cmd) {
        hits.push(unknown_destructive_reason(
            &["axhub".into(), "shell-script".into()],
            "shell_script_axhub_mutation",
        ));
    }
    if contains_shell_alias_axhub_mutation(cmd) {
        hits.push(unknown_destructive_reason(
            &["axhub".into(), "shell-alias".into()],
            "shell_alias_axhub_mutation",
        ));
    }
    for pos in collect_command_positions(cmd, 0) {
        let normalized = pos.trim().to_string();
        if normalized.is_empty() || seen_positions.iter().any(|seen| seen == &normalized) {
            continue;
        }
        if let Some(tokens) = tokens_if_axhub_command(&pos) {
            if let Some(hit) = match_known_intent(&tokens).filter(|hit| hit.is_destructive) {
                if has_prior_payload_file_write(cmd, &pos, &hit) {
                    hits.push(unknown_destructive_reason(
                        &tokens,
                        "same_bash_payload_file_write",
                    ));
                } else {
                    hits.push(hit);
                }
                seen_positions.push(normalized);
            }
        }
    }
    hits
}

pub fn parse_axhub_command(cmd: &str) -> ParsedAxhubCommand {
    parse_axhub_commands(cmd)
        .into_iter()
        .next()
        .unwrap_or_else(|| ParsedAxhubCommand {
            is_destructive: false,
            ..Default::default()
        })
}

pub fn format_preauth_deny_hint(action: Option<&str>, app_id: Option<&str>) -> String {
    let app = app_id.filter(|s| !s.is_empty()).unwrap_or("앱이름");
    if matches!(
        action,
        Some(
            "tables_create"
                | "tables_drop"
                | "tables_columns_add"
                | "tables_columns_remove"
                | "tables_grants_issue"
                | "tables_grants_revoke"
                | "data_insert"
                | "data_update"
                | "data_delete"
        )
    ) {
        return format!(
            "이 axhub 테이블/행 변경 명령은 사전 승인 없이는 실행할 수 없어요. 사용자에게 다시 말하라고 하지 말고 {app} 대상 테이블 변경 미리보기와 승인 카드를 만든 뒤 같은 binding 으로 재시도해요."
        );
    }
    let phrase = match action {
        Some("deploy_create") => format!("'{} 배포해'", app),
        Some("deploy_rollback") => format!("'{} 배포 롤백해'", app),
        Some("deploy_cancel") => format!("'{} 배포 취소해'", app),
        Some("deploy_logs_kill") => format!("'{} 로그 종료해'", app),
        Some("update_apply") => "'axhub 업데이트해'".to_string(),
        Some("auth_login") => "'로그인해'".to_string(),
        Some("auth_logout") => "'로그아웃해'".to_string(),
        Some("env_set") => format!("'{} 환경변수 추가해'", app),
        Some("env_delete") => format!("'{} 환경변수 삭제해'", app),
        Some("apps_create") => format!("'{} 앱 만들어'", app),
        Some("apps_update") => format!("'{} 앱 업데이트해'", app),
        Some("apps_delete") => format!("'{} 앱 지워'", app),
        Some("auth_oauth_client_create") => format!("'{} OAuth client 만들어'", app),
        Some("auth_oauth_revoke") => "'OAuth token revoke 해'".to_string(),
        Some("auth_oauth_consent_revoke") => "'OAuth consent revoke 해'".to_string(),
        Some("auth_pat_issue") => "'PAT 발급해'".to_string(),
        Some("auth_pat_revoke") => "'PAT 폐기해'".to_string(),
        Some("auth_pat_use") => "'PAT 활성화해'".to_string(),
        Some("auth_pat_unset") => "'PAT 활성 해제해'".to_string(),
        Some("auth_pat_rotate") => "'PAT rotate 해'".to_string(),
        Some("github_connect") => format!("'{} github 연결해'", app),
        Some("github_disconnect") => format!("'{} github 끊어'", app),
        Some("profile_add") => "'profile 추가해'".to_string(),
        Some("profile_use") => "'profile 바꿔'".to_string(),
        Some("apis_call") => format!("'{} API 호출해'", app),
        Some("publish_submit") => format!("'{} publish 해'", app),
        Some("invitation_send") | Some("invitation_bulk") => "'팀 초대 보내'".to_string(),
        Some("invitation_cancel") => "'팀 초대 취소해'".to_string(),
        Some("invitation_resend") => "'팀 초대 다시 보내'".to_string(),
        Some("access_grant") | Some("access_invite") => {
            format!("'{} 접근 권한 추가해'", app)
        }
        Some("access_revoke") | Some("access_uninvite") => {
            format!("'{} 접근 권한 제거해'", app)
        }
        Some("tables_create") => format!("'{} 테이블 만들어'", app),
        Some("tables_drop") => format!("'{} 테이블 삭제해'", app),
        Some("tables_columns_add") => format!("'{} 테이블 컬럼 추가해'", app),
        Some("tables_columns_remove") => format!("'{} 테이블 컬럼 제거해'", app),
        Some("tables_grants_issue") => format!("'{} 테이블 권한 발급해'", app),
        Some("tables_grants_revoke") => format!("'{} 테이블 권한 회수해'", app),
        Some("data_insert") => format!("'{} 데이터 추가해'", app),
        Some("data_update") => format!("'{} 데이터 수정해'", app),
        Some("data_delete") => format!("'{} 데이터 삭제해'", app),
        Some("connector_create") => "'connector 만들어'".to_string(),
        Some("connector_update") => "'connector 수정해'".to_string(),
        Some("connector_credentials_set") => "'connector 인증정보 바꿔'".to_string(),
        Some("connector_delete") => "'connector 삭제해'".to_string(),
        Some("apps_fork") => format!("'{} 앱 복제해줘'", app),
        Some("apps_suspend") => format!("'{} 앱 잠깐 멈춰줘'", app),
        Some("apps_resume") => format!("'{} 앱 다시 켜줘'", app),
        Some("resource_namespace_create") => "'resource namespace 만들어'".to_string(),
        Some("resource_rename") => "'resource 이름 바꿔'".to_string(),
        Some("resource_move") => "'resource 이동해'".to_string(),
        Some("resource_bulk_register") => "'resource bulk 등록해'".to_string(),
        Some("resource_delete") => "'resource 삭제해'".to_string(),
        Some("resource_tag_attach") => "'resource 태그 붙여'".to_string(),
        Some("resource_tag_detach") => "'resource 태그 떼어'".to_string(),
        Some("unknown_axhub_mutation") => "'axhub 변경 명령 승인해'".to_string(),
        _ => format!("'{} 배포해'", app),
    };
    format!(
        "이 명령은 사전 승인이 필요해요. 먼저 {}라고 말해서 승인 카드를 받으세요.",
        phrase
    )
}

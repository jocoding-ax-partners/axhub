use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

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

static ENV_ASSIGN_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[A-Za-z_][A-Za-z0-9_]*=\S*\s+)+").unwrap());
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
        "--field" => Some("field"),
        "--body-file" => Some("body_file"),
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

fn destructive(action: &str, tokens: &[String], app_id: Option<String>) -> ParsedAxhubCommand {
    let flags = extract_flags(tokens.get(3..).unwrap_or_default());
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
    while i < chars.len() {
        let ch = chars[i];
        if ch == ';' || ch == '&' || ch == '|' {
            let mut j = i + 1;
            while j < chars.len() && (chars[j] == '&' || chars[j] == '|') {
                j += 1;
            }
            positions.push(chars[j..].iter().collect());
            i = j;
            continue;
        }
        if ch == '$' && chars.get(i + 1) == Some(&'(') {
            positions.push(chars[i + 2..].iter().collect());
            i += 2;
            continue;
        }
        if ch == '(' || ch == '`' {
            positions.push(chars[i + 1..].iter().collect());
            i += 1;
            continue;
        }
        i += 1;
    }
    let shell_re = Regex::new(r#"\b(?:bash|sh|zsh|dash|ksh|eval)\s+(?:-c\s+)?(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|(\S+))"#).unwrap();
    for caps in shell_re.captures_iter(cmd) {
        let body = caps
            .get(1)
            .or_else(|| caps.get(2))
            .or_else(|| caps.get(3))
            .map(|m| m.as_str())
            .unwrap_or("");
        if !body.is_empty() {
            positions.push(body.to_string());
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
                collect_command_positions(body, depth + 1)
                    .into_iter()
                    .skip(1),
            );
        }
    }
    positions
}

fn tokens_if_axhub_command(raw_position: &str) -> Option<Vec<String>> {
    let mut s = raw_position.trim_start().to_string();
    s = ENV_ASSIGN_PREFIX_RE.replace(&s, "").to_string();
    while s.starts_with(['\"', '\'', '`', '(']) {
        s.remove(0);
    }
    if s.starts_with("$(") {
        s = s[2..].to_string();
    }
    s = s.trim_start().to_string();
    if !Regex::new(r"^axhub(?:\s|$)").unwrap().is_match(&s) {
        return None;
    }
    Some(
        s.split_whitespace()
            .map(|t| {
                let mut v = t.to_string();
                if v.len() >= 2 {
                    let first = v.chars().next().unwrap();
                    let last = v.chars().last().unwrap();
                    if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                        v = v[1..v.len() - 1].to_string();
                    }
                }
                while v.ends_with([')', '`', '\'', '"']) {
                    v.pop();
                }
                v
            })
            .filter(|t| !t.is_empty())
            .collect(),
    )
}

fn match_known_intent(tokens: &[String]) -> Option<ParsedAxhubCommand> {
    let sub = tokens.get(1).map(String::as_str);
    let sub2 = tokens.get(2).map(String::as_str);
    let flags = extract_flags(tokens.get(3..).unwrap_or_default());
    let parsed = match (sub, sub2) {
        (Some("deploy"), Some("create")) => destructive("deploy_create", tokens, None),
        (Some("update"), Some("apply")) => destructive("update_apply", tokens, None),
        (Some("deploy"), Some("logs")) if tokens.iter().any(|t| t == "--kill") => {
            destructive("deploy_logs_kill", tokens, None)
        }
        (Some("auth"), Some("login")) => destructive("auth_login", tokens, None),
        (Some("env"), Some("set")) => {
            let mut parsed = destructive("env_set", tokens, None);
            insert_if_some(&mut parsed.context, "key", positional(tokens, 3));
            parsed
        }
        (Some("env"), Some("delete")) | (Some("env"), Some("unset")) => {
            let mut parsed = destructive("env_delete", tokens, None);
            insert_if_some(&mut parsed.context, "key", positional(tokens, 3));
            parsed
        }
        (Some("apps"), Some("create")) => {
            let app = flags.get("slug").cloned().or_else(|| positional(tokens, 3));
            let mut parsed = destructive("apps_create", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            insert_if_some(&mut parsed.context, "source", flags.get("source").cloned());
            parsed
        }
        (Some("apps"), Some("update")) => {
            let app = positional(tokens, 3).or_else(|| flags.get("slug").cloned());
            let mut parsed = destructive("apps_update", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            insert_if_some(&mut parsed.context, "field", flags.get("field").cloned());
            parsed
        }
        (Some("apps"), Some("delete")) | (Some("apps"), Some("rm")) => {
            let app = positional(tokens, 3).or_else(|| flags.get("slug").cloned());
            let mut parsed = destructive("apps_delete", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            parsed
        }
        (Some("github"), Some("connect")) => {
            let app = positional(tokens, 3).or_else(|| flags.get("app_id").cloned());
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
        (Some("github"), Some("disconnect")) => {
            let app = positional(tokens, 3).or_else(|| flags.get("app_id").cloned());
            let mut parsed = destructive("github_disconnect", tokens, app.clone());
            insert_if_some(&mut parsed.context, "slug", app);
            parsed
        }
        (Some("deploy"), Some("cancel")) => {
            let mut parsed = destructive("deploy_cancel", tokens, None);
            insert_if_some(&mut parsed.context, "deployment_id", positional(tokens, 3));
            parsed
        }
        (Some("profile"), Some("add")) => {
            let mut parsed = destructive("profile_add", tokens, None);
            insert_if_some(&mut parsed.context, "profile", positional(tokens, 3));
            insert_if_some(
                &mut parsed.context,
                "endpoint",
                flags.get("endpoint").cloned(),
            );
            parsed
        }
        (Some("profile"), Some("use")) => {
            let mut parsed = destructive("profile_use", tokens, None);
            insert_if_some(&mut parsed.context, "profile", positional(tokens, 3));
            parsed
        }
        (Some("apis"), Some("call")) => {
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
        _ => return None,
    };
    Some(parsed)
}

pub fn parse_axhub_command(cmd: &str) -> ParsedAxhubCommand {
    for pos in collect_command_positions(cmd, 0) {
        if let Some(tokens) = tokens_if_axhub_command(&pos) {
            if let Some(hit) = match_known_intent(&tokens) {
                return hit;
            }
        }
    }
    ParsedAxhubCommand {
        is_destructive: false,
        ..Default::default()
    }
}

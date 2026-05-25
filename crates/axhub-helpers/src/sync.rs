use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const EXIT_USAGE: i32 = 64;
const EXIT_NEEDS_CONSENT: i32 = 65;
const DEFAULT_LIMIT: usize = 200;
const AXHUB_MD: &str = include_str!("../assets/AXHUB.md");

#[derive(Debug, Clone)]
struct SyncArgs {
    target: String,
    out: Option<PathBuf>,
    json: bool,
    no_detail: bool,
    allow_identity_change: bool,
    limit: usize,
}

pub fn run_sync(args: &[String]) -> Result<i32> {
    let opts = match SyncArgs::parse(args) {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("axhub-helpers sync: {message}\n\n{USAGE}");
            return Ok(EXIT_USAGE);
        }
    };

    let out_root = resolve_output_root(opts.out.as_deref())?;
    let resolved_target = if opts.target == "auto" {
        match detect_target(&out_root) {
            Some(target) => target,
            None => {
                emit_json_or_stderr(
                    opts.json,
                    EXIT_NEEDS_CONSENT,
                    json!({
                        "schema_version": "1",
                        "status": "ambiguous_target",
                        "message": "target auto could not choose safely; pass --target local-python, local-bash, web-axhub, or --out",
                    }),
                );
                return Ok(EXIT_NEEDS_CONSENT);
            }
        }
    } else {
        opts.target.clone()
    };

    let axhub_bin = std::env::var("AXHUB_BIN").unwrap_or_else(|_| "axhub".to_string());
    macro_rules! axhub_json {
        ($($arg:expr),+ $(,)?) => {
            match run_axhub_json(&axhub_bin, [$($arg),+]) {
                Ok(value) => value,
                Err(error) => {
                    let exit = error.exit_code;
                    emit_axhub_failure(opts.json, &error);
                    return Ok(exit);
                }
            }
        };
    }

    let resources_raw = axhub_json!(
        "catalog",
        "resources",
        "--json",
        "--limit",
        &opts.limit.to_string(),
    );
    let resource_refs = extract_resources(&resources_raw);

    let mut resources = Vec::new();
    if opts.no_detail {
        resources = resource_refs;
    } else {
        for resource in resource_refs {
            let connector = resource
                .get("connector")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let path = resource
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if connector.is_empty() || path.is_empty() {
                resources.push(normalize_catalog_value(resource));
                continue;
            }
            let detail = axhub_json!(
                "catalog",
                "get",
                "--connector",
                &connector,
                "--path",
                &path,
                "--json",
            );
            resources.push(normalize_catalog_value(merge_resource(resource, detail)));
        }
    }

    let principal = axhub_json!("auth", "whoami", "--json");
    // PAT is optional identity metadata recorded in catalog.json. OAuth-only users
    // have no active PAT context, so `axhub auth pat whoami` exits 65 ("No active PAT
    // context."). The catalog reads + `auth whoami` above already succeeded via OAuth,
    // so a missing PAT must NOT abort the sync — otherwise .axhub/AXHUB.md (written
    // below) never gets created. Treat PAT absence as null metadata and continue.
    let pat =
        run_axhub_json(&axhub_bin, ["auth", "pat", "whoami", "--json"]).unwrap_or(Value::Null);
    let pat_metadata = sanitize_pat_metadata(&pat);
    let identity_fingerprint = identity_fingerprint(&resolved_target, &principal, &pat);

    let axhub_dir = out_root.join(".axhub");
    let catalog_path = axhub_dir.join("catalog.json");
    if let Some(existing) = read_existing_catalog(&catalog_path) {
        let existing_fp = existing
            .get("identity_fingerprint")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !existing_fp.is_empty()
            && existing_fp != identity_fingerprint
            && !opts.allow_identity_change
        {
            emit_json_or_stderr(
                opts.json,
                EXIT_NEEDS_CONSENT,
                json!({
                    "schema_version": "1",
                    "status": "identity_changed",
                    "message": "auth principal or PAT fingerprint changed; re-run with explicit consent before overwriting .axhub/catalog.json",
                    "out_dir": axhub_dir,
                    "target": resolved_target,
                }),
            );
            return Ok(EXIT_NEEDS_CONSENT);
        }
    }

    fs::create_dir_all(&axhub_dir).with_context(|| format!("create {}", axhub_dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&axhub_dir, fs::Permissions::from_mode(0o700))
            .with_context(|| format!("chmod 0700 {}", axhub_dir.display()))?;
    }

    fs::write(axhub_dir.join("AXHUB.md"), AXHUB_MD)
        .with_context(|| format!("write {}", axhub_dir.join("AXHUB.md").display()))?;
    fs::write(
        axhub_dir.join("AXHUB_TARGET"),
        format!("{resolved_target}\n"),
    )
    .with_context(|| format!("write {}", axhub_dir.join("AXHUB_TARGET").display()))?;

    let catalog = json!({
        "schema_version": "1",
        "generated_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "target": resolved_target,
        "principal": principal,
        "pat": pat_metadata,
        "identity_fingerprint": identity_fingerprint,
        "resources": resources,
    });
    atomic_write_private(
        &catalog_path,
        serde_json::to_string_pretty(&catalog)?.as_bytes(),
    )?;
    ensure_catalog_ignored(&out_root.join(".gitignore"))?;

    emit_json_or_stderr(
        opts.json,
        0,
        json!({
            "schema_version": "1",
            "status": "synced",
            "out_dir": axhub_dir,
            "target": catalog["target"],
            "resources": catalog["resources"].as_array().map(|a| a.len()).unwrap_or(0),
        }),
    );
    Ok(0)
}

const USAGE: &str = "Usage: axhub-helpers sync [--target <local-python|local-bash|web-axhub|auto>] [--out <dir>] [--json] [--no-detail] [--allow-identity-change] [--limit <n>]";

impl SyncArgs {
    fn parse(args: &[String]) -> std::result::Result<Self, String> {
        let mut target = "auto".to_string();
        let mut out = None;
        let mut json = false;
        let mut no_detail = false;
        let mut allow_identity_change = false;
        let mut limit = DEFAULT_LIMIT;
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--target" => {
                    i += 1;
                    target = args.get(i).cloned().ok_or("missing value for --target")?;
                }
                "--out" => {
                    i += 1;
                    out = Some(PathBuf::from(
                        args.get(i).cloned().ok_or("missing value for --out")?,
                    ));
                }
                "--json" => json = true,
                "--no-detail" => no_detail = true,
                "--allow-identity-change" => allow_identity_change = true,
                "--limit" => {
                    i += 1;
                    let raw = args.get(i).cloned().ok_or("missing value for --limit")?;
                    limit = raw
                        .parse::<usize>()
                        .map_err(|_| format!("--limit must be a positive integer, got {raw:?}"))?;
                    if limit == 0 {
                        return Err("--limit must be greater than 0".to_string());
                    }
                }
                "--help" | "-h" => return Err("help requested".to_string()),
                unknown => return Err(format!("unknown flag {unknown:?}")),
            }
            i += 1;
        }
        Ok(Self {
            target,
            out,
            json,
            no_detail,
            allow_identity_change,
            limit,
        })
    }
}

fn resolve_output_root(out: Option<&Path>) -> Result<PathBuf> {
    if let Some(out) = out {
        return Ok(out.canonicalize().unwrap_or_else(|_| out.to_path_buf()));
    }
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !stdout.is_empty() {
                return Ok(PathBuf::from(stdout));
            }
        }
    }
    std::env::current_dir().context("resolve current directory")
}

fn detect_target(root: &Path) -> Option<String> {
    if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
        Some("local-python".to_string())
    } else if root.join("package.json").exists() {
        Some("web-axhub".to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct AxhubCommandError {
    command: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
    message: String,
}

impl fmt::Display for AxhubCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} failed with exit {}: {}",
            self.command, self.exit_code, self.message
        )
    }
}

fn run_axhub_json<'a, I>(bin: &str, args: I) -> std::result::Result<Value, AxhubCommandError>
where
    I: IntoIterator<Item = &'a str>,
{
    let args_vec: Vec<&str> = args.into_iter().collect();
    let command = format!("{} {}", bin, args_vec.join(" "));
    let output = Command::new(bin)
        .args(&args_vec)
        .output()
        .map_err(|err| AxhubCommandError {
            command: command.clone(),
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            message: err.to_string(),
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(1);
        return Err(AxhubCommandError {
            command,
            exit_code,
            stdout,
            stderr: stderr.clone(),
            message: if stderr.trim().is_empty() {
                format!("process exited with status {exit_code}")
            } else {
                stderr
            },
        });
    }
    serde_json::from_str(&stdout).map_err(|err| AxhubCommandError {
        command,
        exit_code: EXIT_USAGE,
        stdout,
        stderr,
        message: format!("invalid JSON output: {err}"),
    })
}

fn extract_resources(value: &Value) -> Vec<Value> {
    value
        .get("resources")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .map(|items| items.to_vec())
        .unwrap_or_default()
}

fn merge_resource(summary: Value, detail: Value) -> Value {
    match (summary, detail) {
        (Value::Object(mut a), Value::Object(b)) => {
            for (k, v) in b {
                a.insert(k, v);
            }
            Value::Object(a)
        }
        (_, detail) => detail,
    }
}

fn normalize_catalog_value(mut value: Value) -> Value {
    normalize_masks(&mut value);
    value
}

fn normalize_masks(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                if key == "mask" {
                    if let Some(mask) = value.as_str() {
                        *value = Value::String(mask.to_ascii_lowercase());
                    }
                } else {
                    normalize_masks(value);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_masks(item);
            }
        }
        _ => {}
    }
}

fn read_existing_catalog(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn identity_fingerprint(target: &str, principal: &Value, pat: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(target.as_bytes());
    hasher.update(b"\n");
    hasher.update(
        serde_json::to_string(principal)
            .unwrap_or_default()
            .as_bytes(),
    );
    hasher.update(b"\n");
    hasher.update(serde_json::to_string(pat).unwrap_or_default().as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

fn sanitize_pat_metadata(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sanitized = Map::new();
            for (key, value) in map {
                if is_secret_pat_key(key) {
                    continue;
                }
                sanitized.insert(key.clone(), sanitize_pat_metadata(value));
            }
            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.iter().map(sanitize_pat_metadata).collect()),
        Value::String(s) => Value::String(crate::redact::redact(s)),
        _ => value.clone(),
    }
}

fn is_secret_pat_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', ' '], "_");
    matches!(
        normalized.as_str(),
        "access_token"
            | "refresh_token"
            | "id_token"
            | "token"
            | "pat"
            | "api_key"
            | "x_api_key"
            | "secret"
            | "client_secret"
            | "authorization"
            | "bearer"
    )
}

fn atomic_write_private(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("catalog.json");
    let tmp = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let mut opts = OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts
        .open(&tmp)
        .with_context(|| format!("open temp {}", tmp.display()))?;
    file.write_all(data)
        .with_context(|| format!("write temp {}", tmp.display()))?;
    file.write_all(b"\n")?;
    file.sync_all()
        .with_context(|| format!("sync temp {}", tmp.display()))?;
    drop(file);
    fs::rename(&tmp, path)
        .with_context(|| format!("atomic rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

fn ensure_catalog_ignored(path: &Path) -> Result<()> {
    let entry = ".axhub/catalog.json";
    let existing = fs::read_to_string(path).unwrap_or_default();
    if existing.lines().any(|line| line.trim() == entry) {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    if !existing.is_empty() && !existing.ends_with('\n') {
        writeln!(file)?;
    }
    writeln!(file, "{entry}")?;
    Ok(())
}

fn emit_axhub_failure(json_mode: bool, error: &AxhubCommandError) {
    let classifier_input = if error.stdout.trim().is_empty() {
        error.stderr.as_str()
    } else {
        error.stdout.as_str()
    };
    let entry = crate::catalog::classify(error.exit_code, classifier_input);
    let payload = json!({
        "schema_version": "1",
        "status": "error",
        "command": error.command,
        "exit_code": error.exit_code,
        "emotion": entry.emotion,
        "cause": entry.cause,
        "action": entry.action,
        "button": entry.button,
        "message": error.message.trim(),
    });
    emit_json_or_stderr(json_mode, error.exit_code, payload);
}

fn emit_json_or_stderr(json_mode: bool, _exit_code: i32, payload: Value) {
    if json_mode {
        println!("{}", payload);
    } else {
        eprintln!("{}", payload);
    }
}

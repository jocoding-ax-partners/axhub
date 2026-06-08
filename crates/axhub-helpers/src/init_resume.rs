use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub const INIT_RESUME_SCHEMA_VERSION: &str = "init-resume/v1";
pub const INIT_RESUME_STATE_RELATIVE_PATH: &str = ".axhub/init-resume.json";
const INIT_RESUME_STATE_TTL_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitResumeState {
    pub schema_version: String,
    pub template: String,
    pub app_name: String,
    pub slug: String,
    pub subdomain: Option<String>,
    pub idempotency_key: String,
    pub bootstrap_id: Option<String>,
    pub repo_full_name: Option<String>,
    pub clone_done: bool,
    pub pending_device_flow: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StateRead {
    Missing,
    Corrupt,
    Present(Box<InitResumeState>),
}

pub fn run_init_resume(args: &[String]) -> anyhow::Result<i32> {
    let Some((action, rest)) = args.split_first() else {
        eprintln!("axhub-helpers init-resume: missing action");
        return Ok(64);
    };
    let cwd = std::env::current_dir()?;
    match action.as_str() {
        "get" => {
            parse_json_flag("init-resume get", rest)?;
            println!("{}", serde_json::to_string(&cmd_get(&cwd)?)?);
            Ok(0)
        }
        "put" => {
            let state = parse_put(rest)?;
            write_state(&cwd, &state)?;
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "schema_version": "init-resume-command/v1",
                    "written": true,
                    "state": state
                }))?
            );
            Ok(0)
        }
        "route" => {
            parse_json_flag("init-resume route", rest)?;
            println!("{}", serde_json::to_string(&cmd_route(&cwd)?)?);
            Ok(0)
        }
        "clear" => {
            parse_json_flag("init-resume clear", rest)?;
            let existed = state_path(&cwd).exists();
            let _ = fs::remove_file(state_path(&cwd));
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "schema_version": "init-resume-command/v1",
                    "cleared": existed
                }))?
            );
            Ok(0)
        }
        other => {
            eprintln!("axhub-helpers init-resume: unknown action \"{other}\"");
            Ok(64)
        }
    }
}

fn cmd_get(cwd: &Path) -> anyhow::Result<Value> {
    let value = match read_state(cwd)? {
        StateRead::Present(state) => json!({
            "schema_version": "init-resume-command/v1",
            "present": true,
            "state": state
        }),
        StateRead::Corrupt => {
            let _ = fs::remove_file(state_path(cwd));
            json!({
                "schema_version": "init-resume-command/v1",
                "present": false,
                "reason": "state_corrupt"
            })
        }
        StateRead::Missing => json!({
            "schema_version": "init-resume-command/v1",
            "present": false,
            "reason": "state_missing"
        }),
    };
    Ok(value)
}

fn cmd_route(cwd: &Path) -> anyhow::Result<Value> {
    let value = match read_state(cwd)? {
        StateRead::Missing => json!({
            "schema_version": "init-resume-route/v1",
            "route": "fresh",
            "reason": "state_missing",
            "state_stale": false,
            "requires_status_authority": false
        }),
        StateRead::Corrupt => {
            let _ = fs::remove_file(state_path(cwd));
            json!({
                "schema_version": "init-resume-route/v1",
                "route": "fresh",
                "reason": "state_corrupt",
                "state_stale": false,
                "requires_status_authority": false
            })
        }
        StateRead::Present(state) if state.clone_done => json!({
            "schema_version": "init-resume-route/v1",
            "route": "fresh",
            "reason": "clone_done",
            "state_age_secs": state_age_secs(&state),
            "state_stale": state_stale(&state),
            "requires_status_authority": false,
            "state": state
        }),
        StateRead::Present(state) => {
            let state_stale = state_stale(&state);
            let state_age_secs = state_age_secs(&state);
            if let Some(bootstrap_id) = state.bootstrap_id.as_ref() {
                json!({
                    "schema_version": "init-resume-route/v1",
                    "route": "watch_status",
                    "reason": if state_stale { "bootstrap_id_present_stale" } else { "bootstrap_id_present" },
                    "state_age_secs": state_age_secs,
                    "state_stale": state_stale,
                    "requires_status_authority": true,
                    "args": {
                        "bootstrap_id": bootstrap_id,
                        "status_command": bootstrap_status_command(bootstrap_id)
                    },
                    "state": state
                })
            } else if state_stale {
                json!({
                    "schema_version": "init-resume-route/v1",
                    "route": "fresh",
                    "reason": "state_stale",
                    "state_age_secs": state_age_secs,
                    "state_stale": true,
                    "requires_status_authority": false,
                    "state": state
                })
            } else {
                json!({
                    "schema_version": "init-resume-route/v1",
                    "route": "resume_last",
                    "reason": if state.pending_device_flow { "pending_device_flow" } else { "breadcrumb_only" },
                    "state_age_secs": state_age_secs,
                    "state_stale": false,
                    "requires_status_authority": false,
                    "args": {
                        "template": state.template,
                        "name": state.app_name,
                        "slug": state.slug,
                        "idempotency_key": state.idempotency_key,
                        "resume_command": resume_last_command(
                            &state.template,
                            &state.app_name,
                            &state.slug,
                            &state.idempotency_key
                        ),
                        "fresh_retry_reasons": ["no_cached_device_flow", "expired"]
                    },
                    "state": state
                })
            }
        }
    };
    Ok(value)
}

fn bootstrap_status_command(bootstrap_id: &str) -> Vec<String> {
    [
        "axhub",
        "apps",
        "bootstrap-status",
        bootstrap_id,
        "--watch",
        "--watch-timeout",
        "9m",
        "--json",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn resume_last_command(
    template: &str,
    app_name: &str,
    slug: &str,
    idempotency_key: &str,
) -> Vec<String> {
    [
        "axhub",
        "apps",
        "bootstrap",
        "--template",
        template,
        "--name",
        app_name,
        "--slug",
        slug,
        "--execute",
        "--resume-last",
        "--watch",
        "--watch-timeout",
        "9m",
        "--idempotency-key",
        idempotency_key,
        "--json",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn state_stale(state: &InitResumeState) -> bool {
    state_age_secs(state)
        .map(|age| age > INIT_RESUME_STATE_TTL_SECS)
        .unwrap_or(true)
}

fn state_age_secs(state: &InitResumeState) -> Option<i64> {
    let parsed = DateTime::parse_from_rfc3339(&state.updated_at).ok()?;
    Some(
        Utc::now()
            .signed_duration_since(parsed.with_timezone(&Utc))
            .num_seconds()
            .max(0),
    )
}

fn parse_json_flag(command: &str, args: &[String]) -> anyhow::Result<()> {
    for arg in args {
        if arg != "--json" {
            anyhow::bail!("axhub-helpers {command}: unknown option \"{arg}\"");
        }
    }
    Ok(())
}

fn parse_put(args: &[String]) -> anyhow::Result<InitResumeState> {
    let mut template = None;
    let mut app_name = None;
    let mut slug = None;
    let mut subdomain = None;
    let mut idempotency_key = None;
    let mut bootstrap_id = None;
    let mut repo_full_name = None;
    let mut clone_done = false;
    let mut pending_device_flow = false;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--json" => {}
            "--template" => template = next_value(&mut iter, "--template")?,
            "--app-name" => app_name = next_value(&mut iter, "--app-name")?,
            "--slug" => slug = next_value(&mut iter, "--slug")?,
            "--subdomain" => subdomain = next_value(&mut iter, "--subdomain")?,
            "--idempotency-key" => idempotency_key = next_value(&mut iter, "--idempotency-key")?,
            "--bootstrap-id" => bootstrap_id = next_value(&mut iter, "--bootstrap-id")?,
            "--repo-full-name" => repo_full_name = next_value(&mut iter, "--repo-full-name")?,
            "--clone-done" => {
                clone_done = parse_bool(
                    next_value(&mut iter, "--clone-done")?
                        .as_deref()
                        .unwrap_or("false"),
                )?
            }
            "--pending-device-flow" => {
                pending_device_flow = parse_bool(
                    next_value(&mut iter, "--pending-device-flow")?
                        .as_deref()
                        .unwrap_or("false"),
                )?
            }
            other => anyhow::bail!("axhub-helpers init-resume put: unknown option \"{other}\""),
        }
    }

    let now = now_ts();
    Ok(InitResumeState {
        schema_version: INIT_RESUME_SCHEMA_VERSION.to_string(),
        template: required(template, "--template")?,
        app_name: required(app_name, "--app-name")?,
        slug: required(slug, "--slug")?,
        subdomain: empty_to_none(subdomain),
        idempotency_key: idempotency_key.unwrap_or_else(|| Uuid::new_v4().to_string()),
        bootstrap_id: empty_to_none(bootstrap_id),
        repo_full_name: empty_to_none(repo_full_name),
        clone_done,
        pending_device_flow,
        created_at: now.clone(),
        updated_at: now,
    })
}

fn next_value<'a>(
    iter: &mut impl Iterator<Item = &'a String>,
    flag: &str,
) -> anyhow::Result<Option<String>> {
    let Some(value) = iter.next() else {
        anyhow::bail!("axhub-helpers init-resume put: missing value for {flag}");
    };
    Ok(Some(value.to_string()))
}

fn required(value: Option<String>, flag: &str) -> anyhow::Result<String> {
    let value = value.with_context(|| format!("missing required {flag}"))?;
    anyhow::ensure!(!value.trim().is_empty(), "{flag} cannot be empty");
    Ok(value)
}

fn empty_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        other => anyhow::bail!("invalid boolean value \"{other}\""),
    }
}

fn state_path(cwd: &Path) -> PathBuf {
    cwd.join(INIT_RESUME_STATE_RELATIVE_PATH)
}

fn read_state(cwd: &Path) -> anyhow::Result<StateRead> {
    let path = state_path(cwd);
    if !path.exists() {
        return Ok(StateRead::Missing);
    }
    let raw = fs::read_to_string(&path)?;
    match serde_json::from_str::<InitResumeState>(&raw) {
        Ok(state) if state.schema_version == INIT_RESUME_SCHEMA_VERSION => {
            Ok(StateRead::Present(Box::new(state)))
        }
        _ => Ok(StateRead::Corrupt),
    }
}

fn write_state(cwd: &Path, state: &InitResumeState) -> anyhow::Result<()> {
    let path = state_path(cwd);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(state)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn now_ts() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn state_with_updated_at(updated_at: &str) -> InitResumeState {
        InitResumeState {
            schema_version: INIT_RESUME_SCHEMA_VERSION.to_string(),
            template: "nextjs".to_string(),
            app_name: "테스트 앱".to_string(),
            slug: "test-app".to_string(),
            subdomain: None,
            idempotency_key: "00000000-0000-4000-8000-000000000009".to_string(),
            bootstrap_id: None,
            repo_full_name: None,
            clone_done: false,
            pending_device_flow: false,
            created_at: updated_at.to_string(),
            updated_at: updated_at.to_string(),
        }
    }

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| arg.to_string()).collect()
    }

    #[test]
    fn command_builders_preserve_resume_and_status_argv_contracts() {
        assert_eq!(
            bootstrap_status_command("00000000-0000-4000-8000-000000000001"),
            vec![
                "axhub",
                "apps",
                "bootstrap-status",
                "00000000-0000-4000-8000-000000000001",
                "--watch",
                "--watch-timeout",
                "9m",
                "--json"
            ]
        );
        assert_eq!(
            resume_last_command("nextjs", "결제 앱", "pay-app", "idem-1"),
            vec![
                "axhub",
                "apps",
                "bootstrap",
                "--template",
                "nextjs",
                "--name",
                "결제 앱",
                "--slug",
                "pay-app",
                "--execute",
                "--resume-last",
                "--watch",
                "--watch-timeout",
                "9m",
                "--idempotency-key",
                "idem-1",
                "--json"
            ]
        );
    }

    #[test]
    fn parse_helpers_cover_empty_optional_and_boolean_edges() {
        assert_eq!(empty_to_none(None), None);
        assert_eq!(empty_to_none(Some("  ".to_string())), None);
        assert_eq!(
            empty_to_none(Some(" value ".to_string())),
            Some("value".to_string())
        );
        assert_eq!(parse_bool("true").unwrap(), true);
        assert_eq!(parse_bool("1").unwrap(), true);
        assert_eq!(parse_bool("yes").unwrap(), true);
        assert_eq!(parse_bool("false").unwrap(), false);
        assert_eq!(parse_bool("0").unwrap(), false);
        assert_eq!(parse_bool("no").unwrap(), false);
        assert!(parse_bool("maybe").is_err());
    }

    #[test]
    fn state_age_and_staleness_fail_closed_on_bad_or_old_timestamps() {
        let fresh = state_with_updated_at(&now_ts());
        assert_eq!(state_stale(&fresh), false);
        assert!(state_age_secs(&fresh).unwrap() >= 0);

        let old = state_with_updated_at("2000-01-01T00:00:00Z");
        assert_eq!(state_stale(&old), true);
        assert!(state_age_secs(&old).unwrap() > INIT_RESUME_STATE_TTL_SECS);

        let invalid = state_with_updated_at("not-a-date");
        assert_eq!(state_stale(&invalid), true);
        assert_eq!(state_age_secs(&invalid), None);
    }

    #[test]
    fn cmd_get_clears_corrupt_state_and_reports_missing_afterward() {
        let dir = tempdir().expect("tempdir");
        let axhub_dir = dir.path().join(".axhub");
        fs::create_dir_all(&axhub_dir).expect("mkdir .axhub");
        fs::write(axhub_dir.join("init-resume.json"), "{not json").expect("write corrupt state");

        let corrupt = cmd_get(dir.path()).expect("cmd_get corrupt");
        assert_eq!(corrupt["present"], false);
        assert_eq!(corrupt["reason"], "state_corrupt");
        assert!(!axhub_dir.join("init-resume.json").exists());

        let missing = cmd_get(dir.path()).expect("cmd_get missing");
        assert_eq!(missing["present"], false);
        assert_eq!(missing["reason"], "state_missing");
    }

    #[test]
    fn parse_put_and_write_state_roundtrip_present_state() {
        let dir = tempdir().expect("tempdir");
        let state = parse_put(&argv(&[
            "--json",
            "--template",
            "nextjs",
            "--app-name",
            "결제 앱",
            "--slug",
            "pay-app",
            "--subdomain",
            " pay ",
            "--idempotency-key",
            "idem-1",
            "--bootstrap-id",
            "boot-1",
            "--repo-full-name",
            "owner/repo",
            "--clone-done",
            "true",
            "--pending-device-flow",
            "false",
        ]))
        .expect("parse_put");

        assert_eq!(state.template, "nextjs");
        assert_eq!(state.app_name, "결제 앱");
        assert_eq!(state.slug, "pay-app");
        assert_eq!(state.subdomain.as_deref(), Some("pay"));
        assert_eq!(state.idempotency_key, "idem-1");
        assert_eq!(state.bootstrap_id.as_deref(), Some("boot-1"));
        assert_eq!(state.repo_full_name.as_deref(), Some("owner/repo"));
        assert_eq!(state.clone_done, true);
        assert_eq!(state.pending_device_flow, false);

        write_state(dir.path(), &state).expect("write_state");
        let present = cmd_get(dir.path()).expect("cmd_get present");
        assert_eq!(present["present"], true);
        assert_eq!(present["state"]["slug"], "pay-app");
    }

    #[test]
    fn cmd_route_covers_resume_clone_done_and_bootstrap_branches() {
        let dir = tempdir().expect("tempdir");

        let fresh = cmd_route(dir.path()).expect("missing route");
        assert_eq!(fresh["route"], "fresh");
        assert_eq!(fresh["reason"], "state_missing");
        assert_eq!(fresh["requires_status_authority"], false);

        let mut resume = state_with_updated_at(&now_ts());
        write_state(dir.path(), &resume).expect("write resume state");
        let resume_route = cmd_route(dir.path()).expect("resume route");
        assert_eq!(resume_route["route"], "resume_last");
        assert_eq!(resume_route["reason"], "breadcrumb_only");
        assert_eq!(
            resume_route["args"]["resume_command"][0],
            serde_json::json!("axhub")
        );

        resume.pending_device_flow = true;
        write_state(dir.path(), &resume).expect("write pending state");
        let pending_route = cmd_route(dir.path()).expect("pending route");
        assert_eq!(pending_route["route"], "resume_last");
        assert_eq!(pending_route["reason"], "pending_device_flow");

        resume.bootstrap_id = Some("boot-1".to_string());
        resume.updated_at = "2000-01-01T00:00:00Z".to_string();
        write_state(dir.path(), &resume).expect("write bootstrap state");
        let bootstrap_route = cmd_route(dir.path()).expect("bootstrap route");
        assert_eq!(bootstrap_route["route"], "watch_status");
        assert_eq!(bootstrap_route["reason"], "bootstrap_id_present_stale");
        assert_eq!(bootstrap_route["state_stale"], true);
        assert_eq!(bootstrap_route["requires_status_authority"], true);

        resume.clone_done = true;
        write_state(dir.path(), &resume).expect("write clone-done state");
        let clone_done_route = cmd_route(dir.path()).expect("clone done route");
        assert_eq!(clone_done_route["route"], "fresh");
        assert_eq!(clone_done_route["reason"], "clone_done");
    }
}

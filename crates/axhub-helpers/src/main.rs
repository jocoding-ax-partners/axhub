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
        context: parsed.context,
    };
    let result = verify_token(binding);
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
            "systemMessage": "이 명령은 사전 승인이 필요해요. 먼저 'paydrop 배포해'라고 말해서 승인 카드를 받으세요."
        }));
        Ok(0)
    }
}

#[derive(Clone, Copy)]
struct PromptRoute {
    skill: &'static str,
    label: &'static str,
    guidance: &'static str,
    needs_preflight: bool,
}

fn contains_any(prompt: &str, needles: &[&str]) -> bool {
    let compact = prompt.split_whitespace().collect::<String>();
    needles.iter().any(|needle| {
        prompt.contains(needle) || compact.contains(&needle.split_whitespace().collect::<String>())
    })
}

fn detect_prompt_route(prompt: &str) -> Option<PromptRoute> {
    let p = prompt.trim().to_lowercase();
    if p.is_empty() {
        return None;
    }
    if contains_any(
        &p,
        &[
            "새 앱 만들어",
            "결제 앱 만들어",
            "앱 만들어줘",
            "프로젝트 만들어",
            "next.js 앱",
            "nextjs 앱",
            "fastapi 앱",
            "init",
            "scaffold",
            "axhub.yaml 만들어",
            "apphub.yaml 만들어",
            "빈 디렉토리",
        ],
    ) {
        return Some(PromptRoute {
            skill: "init",
            label: "init template scaffold",
            guidance: "새 axhub 앱을 시작하는 init template 요청이에요. helper bootstrap 을 만들지 말고 skills/init/SKILL.md 흐름에서 axhub --json init --list-templates 를 source of truth 로 사용해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "profile",
            "프로필",
            "다른 회사",
            "회사 endpoint",
            "endpoint 바꿔",
            "엔드포인트 바꿔",
            "사내 endpoint",
        ],
    ) {
        return Some(PromptRoute {
            skill: "profile",
            label: "profile/endpoint",
            guidance: "axhub profile 또는 endpoint 전환 요청이에요. skills/profile/SKILL.md 흐름으로 current/list/add/use 를 구분하고 endpoint allowlist 를 확인해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
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
        ],
    ) {
        return Some(PromptRoute {
            skill: "doctor",
            label: "doctor/환경 점검",
            guidance: "일반 저장소 환경 체크로 답하지 말고 axhub doctor 워크플로우를 적용해요.",
            needs_preflight: true,
        });
    }
    if p == "환경" {
        return Some(PromptRoute {
            skill: "clarify",
            label: "모호한 환경 요청",
            guidance: "환경이라는 단어만으로는 axhub env var 조회인지 환경 점검인지 확실하지 않아요. skills/clarify/SKILL.md 흐름으로 선택지를 좁혀요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "환경변수",
            "환경 변수",
            "db url",
            "database url",
            "secret",
            "api 키",
            "api key",
            "env 봐",
            "env list",
            "env 추가",
            "env 삭제",
            "env var",
        ],
    ) {
        return Some(PromptRoute {
            skill: "env",
            label: "env var 관리",
            guidance: "axhub env var 조회/변경 요청이에요. skills/env/SKILL.md 흐름을 따르고 set 은 --from-stdin 으로만 secret 값을 전달해요.",
            needs_preflight: true,
        });
    }
    if contains_any(
        &p,
        &[
            "github 연결",
            "github repo",
            "github connect",
            "github disconnect",
            "repo 연결",
            "repo 끊",
            "내 repo 붙",
            "git 연결",
            "깃허브 연결",
        ],
    ) {
        return Some(PromptRoute {
            skill: "github",
            label: "GitHub repo 연결",
            guidance: "GitHub repo 연결 또는 해제 요청이에요. skills/github/SKILL.md 흐름으로 GitHub App 설치 상태와 connect/disconnect consent 를 확인해요.",
            needs_preflight: true,
        });
    }
    if contains_any(
        &p,
        &[
            "결과 봐",
            "라이브 봐",
            "브라우저로 열",
            "프로덕션 열",
            "deploy url",
            "open in browser",
            "open",
            "metrics 봐",
            "logs 페이지",
        ],
    ) {
        return Some(PromptRoute {
            skill: "open",
            label: "브라우저 open",
            guidance: "배포 결과를 브라우저에서 여는 요청이에요. skills/open/SKILL.md 흐름으로 axhub open 의 read-only URL 확인을 수행해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "뭐 새로",
            "release notes",
            "changelog",
            "what's new",
            "whatsnew",
            "신규 기능",
        ],
    ) {
        return Some(PromptRoute {
            skill: "whatsnew",
            label: "release notes",
            guidance: "axhub whatsnew/release notes 요청이에요. skills/whatsnew/SKILL.md read-only 흐름으로 변경점을 요약해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "api",
            "apis",
            "api catalog",
            "available api",
            "available endpoint",
            "endpoint",
            "엔드포인트",
            "쓸 수 있는 api",
            "사용 가능한 api",
            "호출 가능한 api",
            "api 뭐",
            "api 목록",
            "api 리스트",
            "api 카탈로그",
            "api 보여",
            "어떤 api",
        ],
    ) {
        return Some(PromptRoute {
            skill: "apis",
            label: "API 카탈로그 조회",
            guidance: "읽기 전용 API 카탈로그 요청이에요. 기본 scope 는 현재 앱으로 유지하고 skills/apis/SKILL.md 흐름을 따라요.",
            needs_preflight: true,
        });
    }
    if contains_any(
        &p,
        &[
            "apps",
            "my apps",
            "app list",
            "app catalog",
            "내 앱",
            "제 앱",
            "우리 앱",
            "회사 앱",
            "앱 목록",
            "앱 리스트",
            "앱 보여",
            "앱 봐",
            "앱 등록",
            "앱 생성",
            "apps create",
            "앱 뭐",
            "어떤 앱",
            "등록된 앱",
            "운영 중인 앱",
            "앱 슬러그",
            "앱 id",
        ],
    ) {
        return Some(PromptRoute {
            skill: "apps",
            label: "앱 목록 조회",
            guidance: "읽기 전용 앱 목록 요청이에요. 일반 repo 탐색 대신 skills/apps/SKILL.md 흐름으로 팀 scope 안에서만 보여줘요.",
            needs_preflight: true,
        });
    }
    if contains_any(
        &p,
        &[
            "auth",
            "login",
            "log in",
            "logout",
            "log out",
            "sign in",
            "sign out",
            "who am i",
            "whoami",
            "로그인",
            "로그아웃",
            "토큰",
            "인증",
            "권한",
            "scope",
            "누구로",
            "누구야",
            "계정",
        ],
    ) {
        return Some(PromptRoute {
            skill: "auth",
            label: "로그인/토큰/identity",
            guidance: "axhub identity 요청이에요. 저장소 사용자 확인으로 답하지 말고 skills/auth/SKILL.md 흐름으로 로그인 상태와 토큰 상태를 확인해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "logs",
            "log",
            "tail",
            "console",
            "why did",
            "why is",
            "로그",
            "빌드 로그",
            "런타임 로그",
            "왜 실패",
            "왜 안돼",
            "왜 깨졌",
            "왜 죽었",
            "에러",
            "콘솔",
            "출력",
        ],
    ) {
        return Some(PromptRoute {
            skill: "logs",
            label: "배포 로그 조회",
            guidance: "axhub 배포 로그 요청이에요. 기본값은 빌드 로그이고 skills/logs/SKILL.md 흐름으로 deployment 를 해석해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "status",
            "watch",
            "follow",
            "progress",
            "is it done",
            "배포 상태",
            "진행 상황",
            "진행 중",
            "어떻게 됐",
            "다 됐",
            "끝났",
            "어디까지",
            "어디쯤",
            "올라갔",
            "떴어",
            "라이브 됐",
            "반영 됐",
            "빌드 됐",
            "상태 봐",
        ],
    ) {
        return Some(PromptRoute {
            skill: "status",
            label: "배포 상태 조회",
            guidance: "axhub 배포 진행 상태 요청이에요. skills/status/SKILL.md 흐름으로 최근 배포나 지정 배포를 추적해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "roll back",
            "rollback",
            "revert",
            "undo",
            "restore",
            "hot fix",
            "hotfix",
            "되돌",
            "롤백",
            "이전 버전",
            "직전 버전",
            "배포 취소",
            "복구",
            "안정 버전",
            "마지막 정상",
        ],
    ) {
        return Some(PromptRoute {
            skill: "recover",
            label: "복구/rollback",
            guidance: "axhub 복구 요청이에요. 실제 rollback 이 아니라 직전 안정 commit 재배포 방식의 skills/recover/SKILL.md 흐름을 적용해요.",
            needs_preflight: true,
        });
    }
    if (contains_any(&p, &["plugin", "플러그인"]))
        && contains_any(
            &p,
            &[
                "upgrade",
                "update",
                "version",
                "업데이트",
                "업그레이드",
                "새 버전",
                "버전",
                "호환",
            ],
        )
    {
        return Some(PromptRoute {
            skill: "upgrade",
            label: "플러그인 업그레이드",
            guidance: "axhub Claude Code 플러그인 업그레이드 요청이에요. CLI 업데이트와 구분해서 skills/upgrade/SKILL.md 흐름을 따라요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "update",
            "upgrade",
            "version",
            "latest",
            "new release",
            "새 버전",
            "최신",
            "버전 확인",
            "업데이트",
            "업그레이드",
            "brew upgrade",
        ],
    ) {
        return Some(PromptRoute {
            skill: "update",
            label: "CLI 업데이트",
            guidance: "axhub CLI 버전 관리 요청이에요. plugin release 작업이 아니라 skills/update/SKILL.md 흐름으로 CLI 업데이트를 확인해요.",
            needs_preflight: false,
        });
    }
    if contains_any(
        &p,
        &[
            "deploy",
            "ship",
            "rollout",
            "launch",
            "release",
            "배포",
            "올려",
            "올리자",
            "쏘자",
            "내보내자",
            "띄워",
            "프로덕션",
            "공개해",
            "demo가 필요",
        ],
    ) {
        return Some(PromptRoute {
            skill: "deploy",
            label: "라이브 배포",
            guidance: "axhub 앱 라이브 배포 요청이에요. 저장소 release workflow, `bun run release`, git tag 작업으로 해석하지 말고 skills/deploy/SKILL.md 의 axhub deploy 안전 가드 흐름을 적용해요.",
            needs_preflight: true,
        });
    }
    if contains_any(
        &p,
        &[
            "help me with axhub",
            "axhub",
            "axhub 좀",
            "axhub 도와줘",
            "axhub 어떻게",
            "axhub 관련",
            "axhub 뭐",
        ],
    ) {
        return Some(PromptRoute {
            skill: "clarify",
            label: "모호한 axhub 요청",
            guidance: "명확한 목적지가 없는 axhub 요청이에요. 조용히 추측하지 말고 skills/clarify/SKILL.md 흐름으로 선택지를 좁혀요.",
            needs_preflight: false,
        });
    }
    None
}

fn cmd_prompt_route() -> anyhow::Result<i32> {
    let raw = read_stdin()?;
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    let prompt = payload.get("prompt").and_then(Value::as_str).unwrap_or("");
    let Some(route) = detect_prompt_route(prompt) else {
        out_json(json!({}));
        return Ok(0);
    };
    let mut lines = vec![
        "[axhub prompt routing]".to_string(),
        format!(
            "사용자 발화 \"{}\"는 axhub {} 의도예요.",
            prompt.trim(),
            route.label
        ),
        format!(
            "반드시 skills/{}/SKILL.md 워크플로우를 우선 적용해요.",
            route.skill
        ),
        route.guidance.into(),
    ];
    if route.needs_preflight {
        let preflight = run_preflight();
        let version = preflight
            .output
            .cli_version
            .clone()
            .unwrap_or_else(|| "unknown".into());
        let guidance = if preflight.output.cli_too_old {
            format!(
                "axhub 버전 확인 결과, axhub가 너무 오래된 버전이에요 ({version}). 'axhub 업그레이드해줘'라고 말해요."
            )
        } else if preflight.output.cli_too_new {
            format!(
                "axhub 버전 확인 결과, 검증 범위보다 새 버전이에요 ({version}). 플러그인 업데이트 확인이 필요해요."
            )
        } else if !preflight.output.cli_present {
            "axhub 설치 확인 결과, CLI를 찾지 못했어요. axhub 설치 후 다시 점검해요.".into()
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
        lines.push(format!("Preflight 결과: {preflight_json}"));
        lines.push(guidance);
    }
    let context = lines.join("\n");
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

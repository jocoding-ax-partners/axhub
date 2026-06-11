//! Blocker card state for the migrate remediation loop (PR1: secret class).
//!
//! 성격: append-only 이벤트 로그(`audit_ledger.rs`)가 아니라 **현재-상태 문서**
//! (idempotent overwrite + per-card revision). 카드 상태머신:
//!
//! ```text
//!              reconcile(증거 있음)            attempt(residual)
//!   [없음] ───────────────▶ OPEN ───────────▶ REMEDIATING ──┐
//!                            ▲ ▲                  │         │ attempts < cap → 재시도
//!                            │ │   attempt(verified: 재검증 PASS)
//!                            │ │                  ▼         │ attempts ≥ cap → plan_only
//!                            │ │              RESOLVED      │   종착 (카드는 OPEN/REMEDIATING
//!                            │ │                  │         │    유지 — 상태 전이 없음)
//!                            │ └── reconcile(증거 재등장: ◀──┘
//!                            │       예) secret 재커밋) ── RESOLVED → OPEN 재개방
//!                            └─ reconcile(증거 소멸) ─ OPEN/REMEDIATING → RESOLVED (stale close)
//! ```
//!
//! 불변 계약 (design doc 2026-06-11):
//! - **파생 상태**: 카드는 `migrate-plan` detect 출력에서만 파생돼요 — 자체 패턴
//!   매칭 금지. 손상/스키마 불일치 시 re-detect 재구축이 정답이라 복구가 싸요.
//! - **비권위(advisory)**: 카드 status 는 기록일 뿐이에요. 진행 판정은 항상
//!   결정론적 재검증(redact 재스캔 exit 0 등)이 내려요. reconcile 은 카드
//!   status 를 신뢰하지 않고 증거로 재판정해요 (조작된 resolved 카드 + 증거
//!   잔존 → OPEN 재개방).
//! - **attempts = best-effort UX 카운터**: 손상 재구축 시 0 리셋을 허용해요.
//!   상한(기본 3)은 무한루프 UX 보호장치지 보안 게이트가 아니에요.
//! - **secret 값 비노출**: payload 에는 env 키 *이름*만 실려요 (`EnvRef` 에
//!   값 필드 자체가 없어 구조적으로 안전).

use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// envelope 헤더의 schema_version. 읽기 시 이보다 크면 (미래 플러그인이 쓴
/// 파일) 기본값 둔갑 없이 재구축 경로로 보내요 — 롤백 forward-compat.
pub const BLOCKER_SCHEMA_VERSION: u32 = 1;

/// 클래스 공통 기본 재시도 상한 (재검증 "잔존"만 소모, 스캔 자체 실패는 미소모).
pub const DEFAULT_ATTEMPT_CAP: u32 = 3;

/// PR1 에서 카드화하는 유일한 클래스. missing_table 은 PR3, custom_auth 는
/// PR5 에서 합류해요. 그 외 hard-stop 은 기존 plan_only 경로 그대로예요.
pub const CLASS_SECRET_EXPOSURE: &str = "secret_exposure";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
    Open,
    Remediating,
    Resolved,
}

impl CardStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CardStatus::Open => "open",
            CardStatus::Remediating => "remediating",
            CardStatus::Resolved => "resolved",
        }
    }

    /// migrate_planning 의 enum 전이 검증 관례를 따라요. no-op(같은 상태)은
    /// 항상 허용, 그 외에는 다이어그램의 화살표만 허용해요.
    pub fn can_transition_to(self, next: CardStatus) -> bool {
        if self == next {
            return true;
        }
        matches!(
            (self, next),
            (CardStatus::Open, CardStatus::Remediating)
                | (CardStatus::Open, CardStatus::Resolved)
                | (CardStatus::Remediating, CardStatus::Resolved)
                | (CardStatus::Resolved, CardStatus::Open)
        )
    }
}

/// 카드 한 장. 공용 8필드는 generic stage-handoff 스키마 정의 (E1) — contract
/// 테스트가 핀해요. migrate 전용 데이터는 전부 `payload` 아래로만 들어가요.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockerCard {
    pub card_id: String,
    pub class: String,
    pub status: CardStatus,
    pub attempts: u32,
    pub revision: u64,
    pub updated_at: String,
    pub skill: String,
    pub payload: Value,
}

/// envelope v2: 문서 헤더 + 카드 배열. `repo_fingerprint` 가 어긋나면 다른
/// 레포/워크트리의 stale 카드라 재구축해요.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockerEnvelope {
    pub schema_version: u32,
    pub run_id: String,
    pub repo_fingerprint: String,
    pub cards: Vec<BlockerCard>,
}

/// 읽기 결과 3분류. Corrupt/버전 불일치는 에러가 아니라 "재구축 신호"예요 —
/// 파생 상태 원칙상 detect 재실행이 복구 경로라서요.
#[derive(Debug)]
pub enum ReadOutcome {
    Ok(BlockerEnvelope),
    Missing,
    Rebuild { reason: String },
}

/// migrate_planning::sha256_hex(repo_root) 와 동일 규칙 — 같은 fingerprint
/// 의미를 공유해요 (assert_repo_fingerprint_matches 참조).
pub fn repo_fingerprint(repo_root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(repo_root.display().to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

// ---------------------------------------------------------------------------
// 읽기 / 쓰기 (atomic, fsync, 0o600)
// ---------------------------------------------------------------------------

pub fn read_envelope(path: &Path, expected_fingerprint: &str) -> Result<ReadOutcome> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(ReadOutcome::Missing),
        Err(err) => {
            return Err(err)
                .with_context(|| format!("{} blocker 상태 파일을 읽지 못했어요", path.display()))
        }
    };
    let envelope: BlockerEnvelope = match serde_json::from_str(&raw) {
        Ok(env) => env,
        Err(err) => {
            return Ok(ReadOutcome::Rebuild {
                reason: format!("손상/스키마 불일치 — re-detect 로 재구축해요 ({err})"),
            })
        }
    };
    if envelope.schema_version > BLOCKER_SCHEMA_VERSION {
        return Ok(ReadOutcome::Rebuild {
            reason: format!(
                "schema_version {} 는 이 플러그인({BLOCKER_SCHEMA_VERSION})보다 새 버전이라 재구축해요",
                envelope.schema_version
            ),
        });
    }
    if envelope.repo_fingerprint != expected_fingerprint {
        return Ok(ReadOutcome::Rebuild {
            reason: "repo fingerprint 불일치 — 다른 레포/워크트리 카드라 재구축해요".to_string(),
        });
    }
    Ok(ReadOutcome::Ok(envelope))
}

/// 같은 디렉터리 temp + fsync + rename. temp 이름에 pid+seq 를 섞어 동시
/// 실행 시 temp 경합을 피해요. Unix 에선 0o600 강제, Windows 에선 rename
/// 대상이 이미 있으면 교체를 위해 먼저 지워요.
pub fn write_envelope(path: &Path, envelope: &BlockerEnvelope) -> Result<()> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    fs::create_dir_all(&parent)
        .with_context(|| format!("{} 디렉터리를 만들지 못했어요", parent.display()))?;
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = parent.join(format!(".blockers-{}-{}.tmp", std::process::id(), seq));
    let bytes = serde_json::to_vec_pretty(envelope)?;
    {
        let mut file = fs::File::create(&tmp)
            .with_context(|| format!("{} temp 파일을 만들지 못했어요", tmp.display()))?;
        file.write_all(&bytes)?;
        file.sync_all()
            .with_context(|| "blocker 상태 fsync 에 실패했어요".to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
    fs::rename(&tmp, path).with_context(|| {
        let _ = fs::remove_file(&tmp);
        format!("{} 로 원자적 교체를 하지 못했어요", path.display())
    })?;
    #[cfg(unix)]
    {
        if let Ok(dir) = fs::File::open(&parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// detect 출력 파싱 (좁은 lenient 뷰 — 입력은 우리 소유 포맷이지만 forward-compat)
// ---------------------------------------------------------------------------

/// migrate-plan --json 출력의 좁은 읽기 뷰. 카드 파생에 필요한 필드만 봐요.
/// (MigratePlanOutput 은 Serialize 전용이라 별도 뷰가 정석이에요.)
#[derive(Debug, Deserialize)]
struct PlanView {
    #[serde(default)]
    sdk_conversion: SdkConversionView,
}

#[derive(Debug, Default, Deserialize)]
struct SdkConversionView {
    #[serde(default)]
    candidates: Vec<CandidateView>,
}

#[derive(Debug, Deserialize)]
struct CandidateView {
    #[serde(default)]
    hard_stop_policy: Vec<PolicyView>,
    #[serde(default)]
    env_refs: Vec<EnvRefView>,
}

#[derive(Debug, Deserialize)]
struct PolicyView {
    code: String,
}

#[derive(Debug, Deserialize)]
struct EnvRefView {
    name: String,
}

/// detect 출력에서 secret_exposure 증거를 파생해요 — 이름만, 값은 구조적으로
/// 존재하지 않아요. (파생-전용 경계: 여기서 새 패턴 매칭을 하지 않고
/// hard_stop_policy 판정과 is_secretish_env 분류를 그대로 재사용해요.)
fn secret_evidence(plan: &PlanView) -> Option<Vec<String>> {
    let mut names: Vec<String> = Vec::new();
    let mut present = false;
    for candidate in &plan.sdk_conversion.candidates {
        if candidate
            .hard_stop_policy
            .iter()
            .any(|p| p.code == CLASS_SECRET_EXPOSURE)
        {
            present = true;
            names.extend(
                candidate
                    .env_refs
                    .iter()
                    .filter(|env| crate::migrate_plan::is_secretish_env(&env.name))
                    .map(|env| env.name.clone()),
            );
        }
    }
    if !present {
        return None;
    }
    names.sort();
    names.dedup();
    Some(names)
}

// ---------------------------------------------------------------------------
// reconcile — detect 증거 기준 양방향 재판정 (비권위 원칙의 구현체)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Serialize)]
pub struct ReconcileSummary {
    pub rebuilt: bool,
    pub rebuild_reason: Option<String>,
    pub opened: Vec<String>,
    pub reopened: Vec<String>,
    pub closed: Vec<String>,
    pub unchanged: Vec<String>,
}

pub fn reconcile(
    existing: ReadOutcome,
    plan: &str,
    run_id: &str,
    fingerprint: &str,
) -> Result<(BlockerEnvelope, ReconcileSummary)> {
    let plan: PlanView =
        serde_json::from_str(plan).context("migrate-plan JSON 을 해석하지 못했어요")?;
    let mut summary = ReconcileSummary::default();
    let mut envelope = match existing {
        ReadOutcome::Ok(env) => env,
        ReadOutcome::Missing => BlockerEnvelope {
            schema_version: BLOCKER_SCHEMA_VERSION,
            run_id: run_id.to_string(),
            repo_fingerprint: fingerprint.to_string(),
            cards: Vec::new(),
        },
        ReadOutcome::Rebuild { reason } => {
            summary.rebuilt = true;
            summary.rebuild_reason = Some(reason);
            BlockerEnvelope {
                schema_version: BLOCKER_SCHEMA_VERSION,
                run_id: run_id.to_string(),
                repo_fingerprint: fingerprint.to_string(),
                cards: Vec::new(),
            }
        }
    };
    envelope.run_id = run_id.to_string();
    envelope.repo_fingerprint = fingerprint.to_string();

    let evidence = secret_evidence(&plan);
    let now = now_rfc3339();
    let existing_index = envelope
        .cards
        .iter()
        .position(|c| c.card_id == CLASS_SECRET_EXPOSURE);

    match (evidence, existing_index) {
        (Some(names), None) => {
            envelope.cards.push(BlockerCard {
                card_id: CLASS_SECRET_EXPOSURE.to_string(),
                class: CLASS_SECRET_EXPOSURE.to_string(),
                status: CardStatus::Open,
                attempts: 0,
                revision: 1,
                updated_at: now,
                skill: "migrate".to_string(),
                payload: json!({ "env_names": names }),
            });
            summary.opened.push(CLASS_SECRET_EXPOSURE.to_string());
        }
        (Some(names), Some(i)) => {
            let card = &mut envelope.cards[i];
            let new_payload = json!({ "env_names": names });
            if card.status == CardStatus::Resolved {
                // 비권위 핵심: resolved 라고 적혀 있어도 증거가 살아 있으면
                // 재개방해요 — 카드 편집으로는 우회가 안 돼요.
                debug_assert!(card.status.can_transition_to(CardStatus::Open));
                card.status = CardStatus::Open;
                card.revision += 1;
                card.updated_at = now;
                card.payload = new_payload;
                summary.reopened.push(card.card_id.clone());
            } else if card.payload != new_payload {
                card.revision += 1;
                card.updated_at = now;
                card.payload = new_payload;
                summary.unchanged.push(card.card_id.clone());
            } else {
                summary.unchanged.push(card.card_id.clone());
            }
        }
        (None, Some(i)) => {
            let card = &mut envelope.cards[i];
            if card.status != CardStatus::Resolved {
                debug_assert!(card.status.can_transition_to(CardStatus::Resolved));
                card.status = CardStatus::Resolved;
                card.revision += 1;
                card.updated_at = now;
                summary.closed.push(card.card_id.clone());
            } else {
                summary.unchanged.push(card.card_id.clone());
            }
        }
        (None, None) => {}
    }
    // 미지 클래스 카드는 그대로 보존해요 (PR3/PR5 합류분 forward-compat).
    Ok((envelope, summary))
}

// ---------------------------------------------------------------------------
// attempt — 재검증 결과 기록 (잔존만 소모, 스캔 실패는 미소모)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutcome {
    /// 재검증이 돌았고 증거가 남아 있어요 → attempts 소모.
    Residual,
    /// 재검증 자체가 실패했어요 (IO 등) → attempts 미소모, 카드 무변경.
    ScanError,
    /// 결정론적 재검증(redact 재스캔 exit 0)이 통과했어요 → resolved.
    Verified,
}

impl AttemptOutcome {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "residual" => Ok(AttemptOutcome::Residual),
            "scan_error" => Ok(AttemptOutcome::ScanError),
            "verified" => Ok(AttemptOutcome::Verified),
            other => {
                bail!("--outcome 은 residual|scan_error|verified 중 하나예요 (받은 값: {other})")
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AttemptResult {
    pub card_id: String,
    pub status: CardStatus,
    pub attempts: u32,
    pub cap: u32,
    /// true 면 plan_only 종착이에요 — 카드는 열린 채 유지하고 체크리스트에
    /// 남은 행동을 실어요 (dead end 0 원칙).
    pub cap_reached: bool,
    pub consumed_attempt: bool,
}

pub fn record_attempt(
    envelope: &mut BlockerEnvelope,
    card_id: &str,
    outcome: AttemptOutcome,
    cap: u32,
) -> Result<AttemptResult> {
    let card = envelope
        .cards
        .iter_mut()
        .find(|c| c.card_id == card_id)
        .with_context(|| format!("{card_id} 카드를 찾지 못했어요 — reconcile 을 먼저 돌려요"))?;
    let now = now_rfc3339();
    let mut consumed = false;
    match outcome {
        AttemptOutcome::Residual => {
            if card.status == CardStatus::Open {
                debug_assert!(card.status.can_transition_to(CardStatus::Remediating));
                card.status = CardStatus::Remediating;
            }
            card.attempts = card.attempts.saturating_add(1);
            card.revision += 1;
            card.updated_at = now;
            consumed = true;
        }
        AttemptOutcome::ScanError => {
            // 스캔 자체 실패는 사용자 잘못이 아니에요 — 아무것도 소모하지 않아요.
        }
        AttemptOutcome::Verified => {
            if !card.status.can_transition_to(CardStatus::Resolved) {
                bail!(
                    "{} → resolved 전이가 허용되지 않아요 (현재: {})",
                    card.card_id,
                    card.status.as_str()
                );
            }
            card.status = CardStatus::Resolved;
            card.revision += 1;
            card.updated_at = now;
        }
    }
    Ok(AttemptResult {
        card_id: card.card_id.clone(),
        status: card.status,
        attempts: card.attempts,
        cap,
        cap_reached: card.status != CardStatus::Resolved && card.attempts >= cap,
        consumed_attempt: consumed,
    })
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

pub fn run_migrate_blockers(args: &[String]) -> Result<i32> {
    let Some(op) = args.first() else {
        bail!("사용법: migrate-blockers <reconcile|attempt> ...");
    };
    match op.as_str() {
        "reconcile" => run_reconcile(&args[1..]),
        "attempt" => run_attempt(&args[1..]),
        other => bail!("알 수 없는 migrate-blockers 작업이에요: {other}"),
    }
}

fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
}

fn run_reconcile(args: &[String]) -> Result<i32> {
    let plan_json = flag_value(args, "--plan-json")
        .context("--plan-json <migrate-plan 출력 파일> 이 필요해요")?;
    let file = flag_value(args, "--file").context("--file <blockers.json 경로> 가 필요해요")?;
    let run_id = flag_value(args, "--run-id").context("--run-id 가 필요해요")?;
    let repo_root = flag_value(args, "--repo-root").context("--repo-root 가 필요해요")?;
    let path = Path::new(file);
    let fingerprint = repo_fingerprint(Path::new(repo_root));
    let plan_raw =
        fs::read_to_string(plan_json).with_context(|| format!("{plan_json} 을 읽지 못했어요"))?;
    let existing = read_envelope(path, &fingerprint)?;
    let (envelope, summary) = reconcile(existing, &plan_raw, run_id, &fingerprint)?;
    write_envelope(path, &envelope)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": BLOCKER_SCHEMA_VERSION,
            "file": file,
            "summary": summary,
            "cards": envelope.cards,
        }))?
    );
    Ok(0)
}

fn run_attempt(args: &[String]) -> Result<i32> {
    let file = flag_value(args, "--file").context("--file <blockers.json 경로> 가 필요해요")?;
    let card_id = flag_value(args, "--card").context("--card <card_id> 가 필요해요")?;
    let outcome =
        AttemptOutcome::parse(flag_value(args, "--outcome").context("--outcome 이 필요해요")?)?;
    let repo_root = flag_value(args, "--repo-root").context("--repo-root 가 필요해요")?;
    let path = Path::new(file);
    let fingerprint = repo_fingerprint(Path::new(repo_root));
    let mut envelope = match read_envelope(path, &fingerprint)? {
        ReadOutcome::Ok(env) => env,
        ReadOutcome::Missing => bail!("{file} 이 없어요 — reconcile 을 먼저 돌려요"),
        ReadOutcome::Rebuild { reason } => {
            bail!("blocker 상태를 신뢰할 수 없어요 ({reason}) — migrate-plan 후 reconcile 로 재구축해요")
        }
    };
    let result = record_attempt(&mut envelope, card_id, outcome, DEFAULT_ATTEMPT_CAP)?;
    write_envelope(path, &envelope)?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(0)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn plan_with_secret(names: &[&str]) -> String {
        let env_refs: Vec<Value> = names
            .iter()
            .map(|n| json!({ "name": n, "scope": "file" }))
            .collect();
        json!({
            "sdk_conversion": { "candidates": [{
                "hard_stop_policy": [
                    { "code": "secret_exposure", "message": "m", "overridable": false }
                ],
                "env_refs": env_refs,
            }]}
        })
        .to_string()
    }

    fn plan_without_secret() -> String {
        json!({
            "sdk_conversion": { "candidates": [{
                "hard_stop_policy": [
                    { "code": "missing_verification", "message": "m", "overridable": true }
                ],
                "env_refs": [],
            }]}
        })
        .to_string()
    }

    #[test]
    fn contract_envelope_and_card_field_sets_are_pinned() {
        // E1 override 의 집행 장치예요: 공용 필드 셋이 조용히 늘어나면 여기서
        // 깨져요. 필드를 의도적으로 바꾸려면 schema_version bump + 이 테스트
        // 갱신이 한 커밋에 있어야 해요.
        let (envelope, _) = reconcile(
            ReadOutcome::Missing,
            &plan_with_secret(&["SECRET_KEY"]),
            "run-1",
            "fp",
        )
        .unwrap();
        let value = serde_json::to_value(&envelope).unwrap();
        // serde_json::Value 는 키를 BTreeMap 정렬로 들고 있어요 — 셋 비교가 정답.
        let mut header_keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        header_keys.sort_unstable();
        assert_eq!(
            header_keys,
            vec!["cards", "repo_fingerprint", "run_id", "schema_version"]
        );
        let card_keys: Vec<&str> = value["cards"][0]
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        let mut sorted = card_keys.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            vec![
                "attempts",
                "card_id",
                "class",
                "payload",
                "revision",
                "skill",
                "status",
                "updated_at"
            ]
        );
    }

    #[test]
    fn corrupt_file_signals_rebuild() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blockers.json");
        fs::write(&path, "{ not json").unwrap();
        match read_envelope(&path, "fp").unwrap() {
            ReadOutcome::Rebuild { .. } => {}
            other => panic!("expected Rebuild, got {other:?}"),
        }
    }

    #[test]
    fn unknown_future_schema_version_signals_rebuild() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blockers.json");
        fs::write(
            &path,
            json!({
                "schema_version": BLOCKER_SCHEMA_VERSION + 1,
                "run_id": "r", "repo_fingerprint": "fp", "cards": []
            })
            .to_string(),
        )
        .unwrap();
        match read_envelope(&path, "fp").unwrap() {
            ReadOutcome::Rebuild { reason } => assert!(reason.contains("schema_version")),
            other => panic!("expected Rebuild, got {other:?}"),
        }
    }

    #[test]
    fn unknown_top_level_field_signals_rebuild_not_silent_default() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blockers.json");
        fs::write(
            &path,
            json!({
                "schema_version": BLOCKER_SCHEMA_VERSION,
                "run_id": "r", "repo_fingerprint": "fp", "cards": [],
                "smuggled": true
            })
            .to_string(),
        )
        .unwrap();
        match read_envelope(&path, "fp").unwrap() {
            ReadOutcome::Rebuild { .. } => {}
            other => panic!("expected Rebuild, got {other:?}"),
        }
    }

    #[test]
    fn fingerprint_mismatch_signals_rebuild() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blockers.json");
        let envelope = BlockerEnvelope {
            schema_version: BLOCKER_SCHEMA_VERSION,
            run_id: "r".into(),
            repo_fingerprint: "other-repo".into(),
            cards: vec![],
        };
        write_envelope(&path, &envelope).unwrap();
        match read_envelope(&path, "this-repo").unwrap() {
            ReadOutcome::Rebuild { reason } => assert!(reason.contains("fingerprint")),
            other => panic!("expected Rebuild, got {other:?}"),
        }
    }

    #[test]
    fn reconcile_creates_open_card_with_env_names_only() {
        let (envelope, summary) = reconcile(
            ReadOutcome::Missing,
            &plan_with_secret(&["KAMAL_REGISTRY_PASSWORD", "SECRET_KEY_BASE"]),
            "run-1",
            "fp",
        )
        .unwrap();
        assert_eq!(summary.opened, vec![CLASS_SECRET_EXPOSURE]);
        let card = &envelope.cards[0];
        assert_eq!(card.status, CardStatus::Open);
        assert_eq!(card.attempts, 0);
        assert_eq!(card.revision, 1);
        // secret 값 비노출 — payload 는 이름 목록뿐이에요.
        assert_eq!(
            card.payload,
            json!({ "env_names": ["KAMAL_REGISTRY_PASSWORD", "SECRET_KEY_BASE"] })
        );
    }

    #[test]
    fn tampered_resolved_card_reopens_when_evidence_remains() {
        // [보안 회귀 — 4A] 카드를 손으로 resolved 로 바꿔도 증거가 살아 있으면
        // reconcile 이 재개방해요. 카드 편집은 게이트 우회가 아니에요.
        let (mut envelope, _) = reconcile(
            ReadOutcome::Missing,
            &plan_with_secret(&["SECRET_KEY"]),
            "run-1",
            "fp",
        )
        .unwrap();
        envelope.cards[0].status = CardStatus::Resolved; // 조작
        let (envelope, summary) = reconcile(
            ReadOutcome::Ok(envelope),
            &plan_with_secret(&["SECRET_KEY"]),
            "run-2",
            "fp",
        )
        .unwrap();
        assert_eq!(summary.reopened, vec![CLASS_SECRET_EXPOSURE]);
        assert_eq!(envelope.cards[0].status, CardStatus::Open);
        assert_eq!(envelope.cards[0].revision, 2);
    }

    #[test]
    fn stale_card_closes_when_evidence_gone() {
        let (envelope, _) = reconcile(
            ReadOutcome::Missing,
            &plan_with_secret(&["SECRET_KEY"]),
            "run-1",
            "fp",
        )
        .unwrap();
        let (envelope, summary) = reconcile(
            ReadOutcome::Ok(envelope),
            &plan_without_secret(),
            "run-2",
            "fp",
        )
        .unwrap();
        assert_eq!(summary.closed, vec![CLASS_SECRET_EXPOSURE]);
        assert_eq!(envelope.cards[0].status, CardStatus::Resolved);
    }

    #[test]
    fn attempt_cap_boundary_and_scan_error_does_not_consume() {
        let (mut envelope, _) = reconcile(
            ReadOutcome::Missing,
            &plan_with_secret(&["SECRET_KEY"]),
            "run-1",
            "fp",
        )
        .unwrap();
        // 스캔 실패는 소모 없음
        let r = record_attempt(
            &mut envelope,
            CLASS_SECRET_EXPOSURE,
            AttemptOutcome::ScanError,
            3,
        )
        .unwrap();
        assert!(!r.consumed_attempt);
        assert_eq!(r.attempts, 0);
        // 잔존 1·2회 — 아직 종착 아님
        for expected in 1..=2u32 {
            let r = record_attempt(
                &mut envelope,
                CLASS_SECRET_EXPOSURE,
                AttemptOutcome::Residual,
                3,
            )
            .unwrap();
            assert_eq!(r.attempts, expected);
            assert!(!r.cap_reached);
        }
        // 3회째 잔존 = plan_only 종착, 카드는 열린 채
        let r = record_attempt(
            &mut envelope,
            CLASS_SECRET_EXPOSURE,
            AttemptOutcome::Residual,
            3,
        )
        .unwrap();
        assert!(r.cap_reached);
        assert_eq!(r.status, CardStatus::Remediating);
        // 통과 시 resolved
        let r = record_attempt(
            &mut envelope,
            CLASS_SECRET_EXPOSURE,
            AttemptOutcome::Verified,
            3,
        )
        .unwrap();
        assert_eq!(r.status, CardStatus::Resolved);
        assert!(!r.cap_reached);
    }

    #[test]
    fn unknown_class_cards_are_preserved_verbatim() {
        let (mut envelope, _) = reconcile(
            ReadOutcome::Missing,
            &plan_with_secret(&["SECRET_KEY"]),
            "run-1",
            "fp",
        )
        .unwrap();
        envelope.cards.push(BlockerCard {
            card_id: "future_class".into(),
            class: "future_class".into(),
            status: CardStatus::Open,
            attempts: 1,
            revision: 7,
            updated_at: "2026-06-11T00:00:00Z".into(),
            skill: "migrate".into(),
            payload: json!({ "x": 1 }),
        });
        let (envelope, _) = reconcile(
            ReadOutcome::Ok(envelope),
            &plan_with_secret(&["SECRET_KEY"]),
            "run-2",
            "fp",
        )
        .unwrap();
        let future = envelope
            .cards
            .iter()
            .find(|c| c.card_id == "future_class")
            .unwrap();
        assert_eq!(future.revision, 7);
        assert_eq!(future.attempts, 1);
    }

    #[test]
    fn write_is_atomic_and_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blockers.json");
        let envelope = BlockerEnvelope {
            schema_version: BLOCKER_SCHEMA_VERSION,
            run_id: "r1".into(),
            repo_fingerprint: "fp".into(),
            cards: vec![],
        };
        write_envelope(&path, &envelope).unwrap();
        let mut second = envelope.clone();
        second.run_id = "r2".into();
        write_envelope(&path, &second).unwrap();
        match read_envelope(&path, "fp").unwrap() {
            ReadOutcome::Ok(env) => assert_eq!(env.run_id, "r2"),
            other => panic!("expected Ok, got {other:?}"),
        }
        // temp 잔여물 없음
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    /// os-shape 통합 회귀: kamal+ruby 모양 앱에서 detect → 카드 open →
    /// secret 해소 → 재detect → 카드 close 가 전구간으로 도는지 확인해요.
    /// (실제 detect 파이프라인을 통과하므로 PlanView 가 migrate-plan 출력과
    /// 어긋나면 여기서 깨져요.)
    #[test]
    fn os_shape_kamal_ruby_full_loop() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::write(root.join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();
        fs::write(
            root.join("app.rb"),
            r#"registry = ENV["KAMAL_REGISTRY_PASSWORD"]
db.execute("SELECT * FROM orders WHERE status = ?", status)
"#,
        )
        .unwrap();
        let plan = crate::migrate_plan::build_migrate_plan(root).unwrap();
        let plan_json = serde_json::to_string(&plan).unwrap();
        let fp = repo_fingerprint(root);
        let (envelope, summary) =
            reconcile(ReadOutcome::Missing, &plan_json, "run-1", &fp).unwrap();
        assert_eq!(summary.opened, vec![CLASS_SECRET_EXPOSURE]);
        assert!(envelope.cards[0].payload["env_names"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "KAMAL_REGISTRY_PASSWORD"));
        // 해소: secret env 참조 제거 후 재detect
        fs::write(
            root.join("app.rb"),
            "db.execute(\"SELECT * FROM orders WHERE status = ?\", status)\n",
        )
        .unwrap();
        let plan = crate::migrate_plan::build_migrate_plan(root).unwrap();
        let plan_json = serde_json::to_string(&plan).unwrap();
        let (envelope, summary) =
            reconcile(ReadOutcome::Ok(envelope), &plan_json, "run-2", &fp).unwrap();
        assert_eq!(summary.closed, vec![CLASS_SECRET_EXPOSURE]);
        assert_eq!(envelope.cards[0].status, CardStatus::Resolved);
    }

    #[test]
    fn concurrent_writers_leave_a_valid_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("blockers.json");
        std::thread::scope(|s| {
            for i in 0..8 {
                let path = path.clone();
                s.spawn(move || {
                    let envelope = BlockerEnvelope {
                        schema_version: BLOCKER_SCHEMA_VERSION,
                        run_id: format!("run-{i}"),
                        repo_fingerprint: "fp".into(),
                        cards: vec![],
                    };
                    write_envelope(&path, &envelope).unwrap();
                });
            }
        });
        // 어느 쪽이 이겼든 파일은 항상 완전한 JSON 이어야 해요 (찢긴 쓰기 금지).
        match read_envelope(&path, "fp").unwrap() {
            ReadOutcome::Ok(env) => assert!(env.run_id.starts_with("run-")),
            other => panic!("expected Ok, got {other:?}"),
        }
    }
}

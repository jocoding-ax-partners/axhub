use std::{collections::HashMap, fs, path::PathBuf};

use chrono::{SecondsFormat, TimeZone, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::key::{
    load_or_mint_key, pending_token_file_path, read_private_file, runtime_root, session_id,
    set_private_dir_mode, token_file_path, write_private_file_no_follow, HMAC_KEY_BYTES,
};

pub const JWT_ALG: Algorithm = Algorithm::HS256;
pub const PENDING_TOOL_CALL_ID: &str = "pending";
const PENDING_SESSION_ID: &str = "pending";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsentBinding {
    pub tool_call_id: String,
    pub action: String,
    pub app_id: String,
    pub profile: String,
    pub branch: String,
    pub commit_sha: String,
    #[serde(default)]
    pub context: HashMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintResult {
    pub token_id: String,
    pub expires_at: String,
    pub file_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerifyResult {
    pub valid: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    tool_call_id: String,
    action: String,
    app_id: String,
    profile: String,
    branch: String,
    commit_sha: String,
    #[serde(default)]
    context: HashMap<String, String>,
    jti: String,
    iat: i64,
    exp: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenFile {
    token_id: String,
    jwt: String,
    expires_at: String,
    session_id: String,
}

impl From<(ConsentBinding, String, i64, i64)> for Claims {
    fn from((b, jti, iat, exp): (ConsentBinding, String, i64, i64)) -> Self {
        Self {
            tool_call_id: b.tool_call_id,
            action: b.action,
            app_id: b.app_id,
            profile: b.profile,
            branch: b.branch,
            commit_sha: b.commit_sha,
            context: b.context,
            jti,
            iat,
            exp,
        }
    }
}

pub fn mint_token(binding: ConsentBinding, ttl_sec: i64) -> anyhow::Result<MintResult> {
    let key = load_or_mint_key()?;
    if binding.tool_call_id == PENDING_TOOL_CALL_ID {
        return mint_pending_token_with_key(binding, ttl_sec, &key);
    }
    match session_id() {
        Ok(sid) => mint_token_with_key(binding, ttl_sec, &key, &sid),
        Err(_) => mint_pending_token_with_key(binding, ttl_sec, &key),
    }
}

pub fn mint_token_with_key(
    binding: ConsentBinding,
    ttl_sec: i64,
    key: &[u8; HMAC_KEY_BYTES],
    sid: &str,
) -> anyhow::Result<MintResult> {
    let token_id = Uuid::new_v4().to_string();
    mint_token_to_path(binding, ttl_sec, key, sid, token_file_path(sid), token_id)
}

fn mint_pending_token_with_key(
    mut binding: ConsentBinding,
    ttl_sec: i64,
    key: &[u8; HMAC_KEY_BYTES],
) -> anyhow::Result<MintResult> {
    binding.tool_call_id = PENDING_TOOL_CALL_ID.into();
    let token_id = Uuid::new_v4().to_string();
    let file_path = pending_token_file_path(&token_id);
    mint_token_to_path(
        binding,
        ttl_sec,
        key,
        PENDING_SESSION_ID,
        file_path,
        token_id,
    )
}

fn mint_token_to_path(
    binding: ConsentBinding,
    ttl_sec: i64,
    key: &[u8; HMAC_KEY_BYTES],
    sid: &str,
    file_path: PathBuf,
    token_id: String,
) -> anyhow::Result<MintResult> {
    let now = Utc::now().timestamp();
    let exp = now + ttl_sec;
    let claims = Claims::from((binding, token_id.clone(), now, exp));
    let mut header = Header::new(JWT_ALG);
    header.typ = Some("JWT".into());
    let jwt = encode(&header, &claims, &EncodingKey::from_secret(key))?;
    std::fs::create_dir_all(runtime_root())?;
    set_private_dir_mode(&runtime_root()).ok();
    let expires_at = Utc
        .timestamp_opt(exp, 0)
        .single()
        .unwrap()
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let body = serde_json::to_vec(&TokenFile {
        token_id: token_id.clone(),
        jwt,
        expires_at: expires_at.clone(),
        session_id: sid.into(),
    })?;
    write_private_file_no_follow(&file_path, &body)?;
    Ok(MintResult {
        token_id,
        expires_at,
        file_path: file_path.display().to_string(),
    })
}

pub fn verify_token(binding: ConsentBinding) -> VerifyResult {
    match verify_token_result(binding) {
        Ok(v) => v,
        Err(reason) => VerifyResult {
            valid: false,
            reason: Some(reason.to_string()),
        },
    }
}

pub fn verify_or_claim_token(binding: ConsentBinding) -> VerifyResult {
    match verify_token_result(binding.clone()) {
        Ok(v) if v.valid => v,
        Ok(v) => match claim_pending_token(&binding) {
            Ok(()) => VerifyResult {
                valid: true,
                reason: None,
            },
            Err(_) => v,
        },
        Err(reason) => match claim_pending_token(&binding) {
            Ok(()) => VerifyResult {
                valid: true,
                reason: None,
            },
            Err(_) => VerifyResult {
                valid: false,
                reason: Some(reason.to_string()),
            },
        },
    }
}

fn verify_token_result(binding: ConsentBinding) -> Result<VerifyResult, &'static str> {
    let sid = session_id().map_err(|_| "session_id_missing")?;
    let key = load_or_mint_key().map_err(|_| "hmac_key_unreadable")?;
    let (_, claims) = decode_token_file(&token_file_path(&sid), &key)?;
    if let Some(reason) = binding_mismatch_reason(&claims, &binding, true) {
        return Ok(VerifyResult {
            valid: false,
            reason: Some(reason),
        });
    }
    Ok(VerifyResult {
        valid: true,
        reason: None,
    })
}

fn claim_pending_token(binding: &ConsentBinding) -> Result<(), &'static str> {
    let key = load_or_mint_key().map_err(|_| "hmac_key_unreadable")?;
    let entries = fs::read_dir(runtime_root()).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "no_pending_consent_token"
        } else {
            "pending_consent_unreadable"
        }
    })?;
    let mut saw_pending = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_pending_token_path(&path) {
            continue;
        }
        saw_pending = true;
        match decode_token_file(&path, &key) {
            Ok((token_file, claims))
                if token_file.session_id == PENDING_SESSION_ID
                    && claims.tool_call_id == PENDING_TOOL_CALL_ID =>
            {
                if binding_mismatch_reason(&claims, binding, false).is_none() {
                    fs::remove_file(&path).ok();
                    return Ok(());
                }
            }
            Err("token_expired") => {
                fs::remove_file(&path).ok();
            }
            _ => {}
        }
    }
    Err(if saw_pending {
        "binding_mismatch:pending_consent"
    } else {
        "no_pending_consent_token"
    })
}

fn is_pending_token_path(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("consent-pending-") && name.ends_with(".json"))
}

fn decode_token_file(
    path: &PathBuf,
    key: &[u8; HMAC_KEY_BYTES],
) -> Result<(TokenFile, Claims), &'static str> {
    let raw = read_private_file(path).map_err(|e| {
        if e.downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
        {
            "no_consent_token"
        } else {
            "token_file_unreadable"
        }
    })?;
    let parsed: TokenFile = serde_json::from_slice(&raw).map_err(|_| "token_file_corrupt")?;
    if parsed.jwt.is_empty() {
        return Err("token_file_missing_jwt");
    }
    let mut validation = Validation::new(JWT_ALG);
    validation.leeway = 0;
    validation.validate_exp = true;
    let data = decode::<Claims>(&parsed.jwt, &DecodingKey::from_secret(key), &validation).map_err(
        |e| {
            let msg = e.to_string().to_lowercase();
            if msg.contains("expired") || msg.contains("exp") {
                "token_expired"
            } else {
                "token_signature_invalid"
            }
        },
    )?;
    if data.claims.exp <= Utc::now().timestamp() {
        return Err("token_expired");
    }
    Ok((parsed, data.claims))
}

fn binding_mismatch_reason(
    claims: &Claims,
    binding: &ConsentBinding,
    include_tool_call_id: bool,
) -> Option<String> {
    if include_tool_call_id && claims.tool_call_id != binding.tool_call_id {
        return Some("binding_mismatch:tool_call_id".into());
    }
    let checks = [
        ("action", claims.action.as_str(), binding.action.as_str()),
        ("app_id", claims.app_id.as_str(), binding.app_id.as_str()),
        ("profile", claims.profile.as_str(), binding.profile.as_str()),
        ("branch", claims.branch.as_str(), binding.branch.as_str()),
        (
            "commit_sha",
            claims.commit_sha.as_str(),
            binding.commit_sha.as_str(),
        ),
    ];
    for (field, got, expected) in checks {
        if got != expected {
            return Some(format!("binding_mismatch:{field}"));
        }
    }
    if claims.context != binding.context {
        return Some("binding_mismatch:context".into());
    }
    None
}

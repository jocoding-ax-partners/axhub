use std::collections::HashMap;

use chrono::{SecondsFormat, TimeZone, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::key::{
    load_or_mint_key, read_private_file, runtime_root, session_id, set_private_dir_mode,
    token_file_path, write_private_file_no_follow, HMAC_KEY_BYTES,
};

pub const JWT_ALG: Algorithm = Algorithm::HS256;

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
    let sid = session_id()?;
    let key = load_or_mint_key()?;
    mint_token_with_key(binding, ttl_sec, &key, &sid)
}

pub fn mint_token_with_key(
    binding: ConsentBinding,
    ttl_sec: i64,
    key: &[u8; HMAC_KEY_BYTES],
    sid: &str,
) -> anyhow::Result<MintResult> {
    let now = Utc::now().timestamp();
    let exp = now + ttl_sec;
    let token_id = Uuid::new_v4().to_string();
    let claims = Claims::from((binding, token_id.clone(), now, exp));
    let mut header = Header::new(JWT_ALG);
    header.typ = Some("JWT".into());
    let jwt = encode(&header, &claims, &EncodingKey::from_secret(key))?;
    std::fs::create_dir_all(runtime_root())?;
    set_private_dir_mode(&runtime_root()).ok();
    let file_path = token_file_path(sid);
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

fn verify_token_result(binding: ConsentBinding) -> Result<VerifyResult, &'static str> {
    let sid = session_id().map_err(|_| "session_id_missing")?;
    let raw = read_private_file(&token_file_path(&sid)).map_err(|e| {
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
    let key = load_or_mint_key().map_err(|_| "hmac_key_unreadable")?;
    let mut validation = Validation::new(JWT_ALG);
    validation.leeway = 0;
    validation.validate_exp = true;
    let data = decode::<Claims>(&parsed.jwt, &DecodingKey::from_secret(&key), &validation)
        .map_err(|e| {
            let msg = e.to_string().to_lowercase();
            if msg.contains("expired") || msg.contains("exp") {
                "token_expired"
            } else {
                "token_signature_invalid"
            }
        })?;
    if data.claims.exp <= Utc::now().timestamp() {
        return Err("token_expired");
    }
    let checks = [
        (
            "tool_call_id",
            data.claims.tool_call_id.as_str(),
            binding.tool_call_id.as_str(),
        ),
        (
            "action",
            data.claims.action.as_str(),
            binding.action.as_str(),
        ),
        (
            "app_id",
            data.claims.app_id.as_str(),
            binding.app_id.as_str(),
        ),
        (
            "profile",
            data.claims.profile.as_str(),
            binding.profile.as_str(),
        ),
        (
            "branch",
            data.claims.branch.as_str(),
            binding.branch.as_str(),
        ),
        (
            "commit_sha",
            data.claims.commit_sha.as_str(),
            binding.commit_sha.as_str(),
        ),
    ];
    for (field, got, expected) in checks {
        if got != expected {
            return Ok(VerifyResult {
                valid: false,
                reason: Some(format!("binding_mismatch:{field}")),
            });
        }
    }
    if data.claims.context != binding.context {
        return Ok(VerifyResult {
            valid: false,
            reason: Some("binding_mismatch:context".into()),
        });
    }
    Ok(VerifyResult {
        valid: true,
        reason: None,
    })
}

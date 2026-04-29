pub mod errors {
    pub const TLS_PIN_MISMATCH: &str = "hub-api TLS pin 검증에 실패했어요.";
    pub const TLS_PIN_TIMEOUT: &str = "hub-api TLS pin 검증 시간이 초과됐어요.";
    pub const ENDPOINT_INVALID: &str = "잘못된 AXHUB_ENDPOINT 값이에요";
    pub const HTTPS_REQUIRED: &str = "hub-api.jocodingax.ai 는 HTTPS 로만 호출해야 해요.";
    pub const KEYCHAIN_EDR_BLOCKED: &str = "Windows 보안 솔루션이 axhub 토큰 조회를 차단했어요.";
}

#[macro_export]
macro_rules! msg {
    ($key:ident) => { $crate::messages::errors::$key };
    ($key:ident, $($arg:tt)*) => { format!("{}: {}", $crate::messages::errors::$key, format!($($arg)*)) };
}

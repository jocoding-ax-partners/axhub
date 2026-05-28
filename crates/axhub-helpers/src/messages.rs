pub mod errors {
    pub const KEYCHAIN_EDR_BLOCKED: &str = "Windows 보안 솔루션이 axhub 토큰 조회를 차단했어요.";
}

#[macro_export]
macro_rules! msg {
    ($key:ident) => { $crate::messages::errors::$key };
    ($key:ident, $($arg:tt)*) => { format!("{}: {}", $crate::messages::errors::$key, format!($($arg)*)) };
}

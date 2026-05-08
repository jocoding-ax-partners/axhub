//! Humanize timestamps for vibe-coder-friendly Korean display.

use chrono::{DateTime, FixedOffset, Utc};

/// Convert RFC3339 ISO timestamp to Korean human-friendly remaining-time string.
///
/// Returns `None` if `rfc3339` cannot be parsed.
///
/// Format X (7 case):
/// - delta >= 365d -> "약 N년 남았어요"
/// - 7d <= delta < 365d -> "N일 M시간 남았어요"
/// - 24h <= delta < 7d -> "N시간 M분 남았어요"
/// - 1h <= delta < 24h -> "N시간 M분 남았어요"
/// - 5m <= delta < 1h -> "N분 남았어요"
/// - 0 <= delta < 5m -> "곧 만료돼요 (5분 미만)"
/// - delta < 0 -> "이미 만료됐어요"
pub fn format_expires_human(rfc3339: &str, _tz: FixedOffset, now: DateTime<Utc>) -> Option<String> {
    let exp = DateTime::parse_from_rfc3339(rfc3339)
        .ok()?
        .with_timezone(&Utc);
    let delta = exp - now;
    let secs = delta.num_seconds();
    if secs < 0 {
        return Some("이미 만료됐어요".to_string());
    }
    if secs < 300 {
        return Some("곧 만료돼요 (5분 미만)".to_string());
    }
    let total_mins = delta.num_minutes();
    if total_mins < 60 {
        return Some(format!("{}분 남았어요", total_mins));
    }
    let total_hours = delta.num_hours();
    let rem_mins = total_mins - total_hours * 60;
    if total_hours < 24 {
        return Some(format!("{}시간 {}분 남았어요", total_hours, rem_mins));
    }
    let total_days = delta.num_days();
    let rem_hours = total_hours - total_days * 24;
    if total_days < 7 {
        return Some(format!("{}시간 {}분 남았어요", total_hours, rem_mins));
    }
    if total_days < 365 {
        return Some(format!("{}일 {}시간 남았어요", total_days, rem_hours));
    }
    let years = total_days / 365;
    Some(format!("약 {}년 남았어요", years))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn kst() -> FixedOffset {
        FixedOffset::east_opt(9 * 3600).unwrap()
    }

    fn now_fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 7, 0, 0, 0).unwrap()
    }

    #[test]
    fn under_five_minutes() {
        let exp = "2026-05-07T00:00:45Z";
        assert_eq!(
            format_expires_human(exp, kst(), now_fixed()),
            Some("곧 만료돼요 (5분 미만)".to_string())
        );
    }

    #[test]
    fn under_one_hour() {
        let exp = "2026-05-07T00:42:00Z";
        assert_eq!(
            format_expires_human(exp, kst(), now_fixed()),
            Some("42분 남았어요".to_string())
        );
    }

    #[test]
    fn one_day_range() {
        let exp = "2026-05-07T21:36:00Z";
        assert_eq!(
            format_expires_human(exp, kst(), now_fixed()),
            Some("21시간 36분 남았어요".to_string())
        );
    }

    #[test]
    fn one_week_to_year() {
        // 30 days 5 hours from now_fixed
        let exp = "2026-06-06T05:00:00Z";
        assert_eq!(
            format_expires_human(exp, kst(), now_fixed()),
            Some("30일 5시간 남았어요".to_string())
        );
    }

    #[test]
    fn above_year() {
        // case 13 token: ~73 years from now
        let exp = "2099-01-01T00:00:00Z";
        let result = format_expires_human(exp, kst(), now_fixed()).unwrap();
        assert!(result.starts_with("약 ") && result.ends_with("년 남았어요"));
        // 72 or 73 depending on leap year math
        assert!(result.contains("72년") || result.contains("73년"));
    }

    #[test]
    fn already_expired() {
        let exp = "2026-05-06T23:59:00Z";
        assert_eq!(
            format_expires_human(exp, kst(), now_fixed()),
            Some("이미 만료됐어요".to_string())
        );
    }

    #[test]
    fn invalid_rfc3339_returns_none() {
        assert_eq!(format_expires_human("not-a-date", kst(), now_fixed()), None);
        assert_eq!(format_expires_human("", kst(), now_fixed()), None);
    }
}

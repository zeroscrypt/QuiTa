/// Maximum length of a task title, in characters. Mirrors the `maxlength`
/// attribute in the UI: the check lives here too because the IPC boundary is
/// reachable from anything running in the webview.
pub const MAX_TITLE_CHARS: usize = 500;

/// Maximum length of a tab name, in characters.
pub const MAX_TAB_NAME_CHARS: usize = 40;

/// Maximum length of a task comment, in characters.
pub const MAX_COMMENT_CHARS: usize = 2000;

/// Trims the raw input and rejects empty or over-long titles.
pub fn normalize_title(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_TITLE_CHARS {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Validates a calendar date in the strict YYYY-MM-DD format.
/// The shape check rejects unpadded values (chrono's parser accepts "2026-1-1"),
/// and `chrono` rejects impossible dates (2025-02-30, 2025-13-01).
pub fn is_valid_due_date(value: &str) -> bool {
    if value.len() != 10 {
        return false;
    }
    let bytes = value.as_bytes();
    let digits_ok = bytes.iter().enumerate().all(|(i, b)| {
        if i == 4 || i == 7 {
            *b == b'-'
        } else {
            b.is_ascii_digit()
        }
    });
    digits_ok && chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_and_rejects_empty() {
        assert_eq!(
            normalize_title("  buy milk  "),
            Some("buy milk".to_string())
        );
        assert_eq!(normalize_title("   "), None);
        assert_eq!(normalize_title(""), None);
        assert_eq!(normalize_title("  задача  "), Some("задача".to_string()));
    }

    #[test]
    fn due_date_accepts_real_calendar_dates() {
        assert!(is_valid_due_date("2026-10-13"));
        assert!(is_valid_due_date("2024-02-29")); // leap year
        assert!(!is_valid_due_date("2025-02-29")); // not a leap year
        assert!(!is_valid_due_date("2025-13-01"));
        assert!(!is_valid_due_date("13-10-2026"));
        assert!(!is_valid_due_date("2026-10-13T10:00:00Z"));
        assert!(!is_valid_due_date("2026-1-1"));
        assert!(!is_valid_due_date("завтра"));
    }
}

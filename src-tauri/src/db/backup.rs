//! Consistent snapshots of the SQLite file, with rotation.
//!
//! The database runs in WAL mode, so copying the file with `fs::copy` can
//! race active writes and lose transactions that still live only in the
//! `-wal` companion. `VACUUM INTO` builds a fresh, self-contained copy with
//! the database's own machinery instead: either the copy is valid or it is
//! not written.
//!
//! Copies live in a `backups` directory next to `quita.db`. A name carries
//! a timestamp (`quita-backup-20261022-103500.db`), so "which is the oldest"
//! is a string comparison and the rotation simply drops the oldest names
//! past the configured count.

use chrono::{NaiveDateTime, SecondsFormat, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::db::repository::{MAX_BACKUP_COPIES, MIN_BACKUP_COPIES};
use crate::domain::task::AppSettings;

const NAME_PREFIX: &str = "quita-backup-";
const NAME_SUFFIX: &str = ".db";
/// Length of `20261022-103500`, the stamp embedded in every name.
const STAMP_LEN: usize = 15;
const STAMP_FORMAT: &str = "%Y%m%d-%H%M%S";
/// Launch copies count as daily: two starts in one morning produce one file,
/// not three identical ones.
const AUTO_BACKUP_INTERVAL_HOURS: i64 = 24;

/// One stored copy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub name: String,
    /// Rebuilt from the stamp in the name, e.g. "2026-10-22 10:35:00".
    pub created_at: String,
    pub size_bytes: u64,
}

/// The stored copies, newest first.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub files: Vec<BackupFile>,
}

/// Tauri state: where the copies live. Computed once at startup from the
/// data directory the database was opened in, so the commands cannot drift
/// to a different folder than the launch copy wrote to.
pub struct BackupDir(pub PathBuf);

/// The directory copies go to; lives next to the file it mirrors.
pub fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

/// RFC-3339 "now" for the stamp, the same shape the repository stamps rows.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// `2026-10-22T10:35:00.123Z` -> `20261022-103500`. Milliseconds are cut off
/// on purpose: the stamp names files, and a name must not depend on the
/// precision of whoever produced the timestamp.
fn rfc3339_to_stamp(now: &str) -> Option<String> {
    let b = now.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let digits = |s: &[u8]| s.iter().all(u8::is_ascii_digit);
    // YYYY MM DD T HH MM SS — positions fixed by the RFC 3339 calendar
    // date-time format, which is all this app ever stores or emits.
    if !(digits(&b[..4]) && b[4] == b'-' && digits(&b[5..7]) && b[7] == b'-') {
        return None;
    }
    if !(digits(&b[8..10]) && b[10] == b'T') {
        return None;
    }
    if !(digits(&b[11..13]) && b[13] == b':' && digits(&b[14..16]) && b[16] == b':') {
        return None;
    }
    if !(digits(&b[17..19]) && b[17..19].len() == 2) {
        return None;
    }
    Some(format!(
        "{}-{}",
        now[..10].replace('-', ""),
        now[11..19].replace(':', "")
    ))
}

/// Strip `quita-backup-` and `.db`, leaving the stamp.
fn stamp_of(name: &str) -> Option<&str> {
    let stem = name.strip_prefix(NAME_PREFIX)?.strip_suffix(NAME_SUFFIX)?;
    (stem.len() == STAMP_LEN).then_some(stem)
}

fn stamp_to_naive(stamp: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(stamp, STAMP_FORMAT).ok()
}

/// Reads the stored copies, newest first. A missing or unreadable directory
/// means "no copies": the feature must stay usable even if the folder got
/// deleted by hand.
pub fn list_backups(dir: &Path) -> BackupInfo {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return BackupInfo { files: Vec::new() };
    };
    let mut files: Vec<BackupFile> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stamp) = stamp_of(&name) else {
            continue;
        };
        let created_at = stamp_to_naive(stamp)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| stamp.to_string());
        let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
        files.push(BackupFile {
            name,
            created_at,
            size_bytes,
        });
    }
    // The stamp in the name makes "newest first" a plain reverse sort.
    files.sort_by(|a, b| b.name.cmp(&a.name));
    BackupInfo { files }
}

/// Deletes the oldest names until `keep` copies are left. No metadata is
/// parsed: the chronological order is inside the name.
fn rotate(dir: &Path, keep: i64) -> Result<(), String> {
    let keep = keep.clamp(MIN_BACKUP_COPIES, MAX_BACKUP_COPIES) as usize;
    for extra in list_backups(dir).files.iter().skip(keep) {
        std::fs::remove_file(dir.join(&extra.name)).map_err(|e| format!("{}: {e}", extra.name))?;
    }
    Ok(())
}

/// Makes a fresh copy of the database right now and enforces the rotation.
///
/// `VACUUM INTO` refuses to overwrite, and two copies taken within the same
/// second carry the same stamp: replacing is the useful answer there.
pub fn create_backup(
    conn: &Connection,
    dir: &Path,
    keep: i64,
    now: &str,
) -> Result<BackupInfo, String> {
    let stamp = rfc3339_to_stamp(now).ok_or_else(|| format!("bad timestamp: {now}"))?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{NAME_PREFIX}{stamp}{NAME_SUFFIX}"));
    let target = path.to_string_lossy().into_owned();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    conn.execute("VACUUM INTO ?1", [&target])
        .map_err(|e| e.to_string())?;
    rotate(dir, keep)?;
    Ok(list_backups(dir))
}

/// The launch-time copy: only when enabled, only once a day, always quiet.
///
/// Nothing here may keep the app from starting: a failed copy prints and
/// gets out of the way, the next launch tries again.
pub fn auto_backup(conn: &Connection, dir: &Path, settings: &AppSettings) {
    if !settings.backup_enabled {
        return;
    }
    let keep = settings
        .backup_copies
        .clamp(MIN_BACKUP_COPIES, MAX_BACKUP_COPIES);

    let newest = list_backups(dir).files.into_iter().next();
    let young_enough = newest
        .map(|f| {
            stamp_of(&f.name)
                .and_then(stamp_to_naive)
                .is_some_and(|created| {
                    Utc::now().signed_duration_since(created.and_utc())
                        < chrono::Duration::hours(AUTO_BACKUP_INTERVAL_HOURS)
                })
        })
        // A name that does not parse counts as old: a copy is due. The
        // rotation will have dealt with the oddball by the next round.
        .unwrap_or(false);
    if young_enough {
        return;
    }

    if let Err(err) = create_backup(conn, dir, keep, &now_rfc3339()) {
        eprintln!("QuiTa: auto-backup failed: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::{rfc3339_to_stamp, stamp_of};

    #[test]
    fn stamps_come_from_and_go_back_to_rfc3339() {
        // The repository stamps with millisecond precision; the stamp keeps
        // the second so file names do not depend on the precision.
        assert_eq!(
            rfc3339_to_stamp("2026-10-22T10:35:00.123Z").as_deref(),
            Some("20261022-103500")
        );
        // No milliseconds, another permitted RFC 3339 shape.
        assert_eq!(
            rfc3339_to_stamp("2026-01-02T03:04:05Z").as_deref(),
            Some("20260102-030405")
        );
        // Truncated or malformed input must produce a stamp, not a name
        // that hides a broken timestamp.
        assert_eq!(rfc3339_to_stamp("2026-10-22T10:35"), None);
        assert_eq!(rfc3339_to_stamp("2026-10-22 10:35:00"), None);
        assert_eq!(rfc3339_to_stamp("16.10.2026"), None);
    }

    #[test]
    fn stamps_are_read_only_from_names_that_look_like_copies() {
        assert_eq!(
            stamp_of("quita-backup-20261022-103500.db"),
            Some("20261022-103500")
        );
        // Foreign files and plain stray names stay out of the listing.
        assert_eq!(stamp_of("quita.db"), None);
        assert_eq!(stamp_of("quita-backup-x.db"), None);
        assert_eq!(stamp_of("quita-backup-20261022-103500.db-wal"), None);
    }
}

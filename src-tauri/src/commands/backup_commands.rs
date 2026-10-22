use tauri::State;

use crate::db::backup::{self, BackupDir, BackupInfo};
use crate::db::Db;
use crate::error::TaskError;

/// Takes a copy right now. The rotation count comes from the settings, so the
/// button and the launch copy agree on how many generations to keep.
#[tauri::command]
pub async fn backup_database(
    db: State<'_, Db>,
    dir: State<'_, BackupDir>,
) -> Result<BackupInfo, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    let settings = crate::db::repository::get_settings(&conn)?;
    backup::create_backup(
        &conn,
        &dir.0,
        settings.backup_copies,
        &backup::now_rfc3339(),
    )
    .map_err(TaskError::Internal)
}

/// The stored copies, newest first — for the line the settings tab shows.
#[tauri::command]
pub async fn backup_info(dir: State<'_, BackupDir>) -> Result<BackupInfo, TaskError> {
    Ok(backup::list_backups(&dir.0))
}

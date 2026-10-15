use tauri::State;

use crate::db::{repository, Db};
use crate::domain::task::{Tab, TaskId};
use crate::error::TaskError;

#[tauri::command]
pub async fn list_tabs(db: State<'_, Db>) -> Result<Vec<Tab>, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::list_tabs(&conn)
}

#[tauri::command]
pub async fn create_tab(name: Option<String>, db: State<'_, Db>) -> Result<Tab, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    let name = name.unwrap_or_else(|| repository::DEFAULT_TAB_NAME.to_string());
    repository::create_tab(&conn, &name)
}

#[tauri::command]
pub async fn rename_tab(id: TaskId, name: String, db: State<'_, Db>) -> Result<Tab, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::rename_tab(&mut conn, &id, &name)
}

/// Persists a new tab order (drag & drop on the tab strip).
#[tauri::command]
pub async fn reorder_tabs(
    ordered_ids: Vec<TaskId>,
    db: State<'_, Db>,
) -> Result<Vec<Tab>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::reorder_tabs(&mut conn, &ordered_ids)?;
    repository::list_tabs(&conn)
}

/// Deletes a tab together with its tasks, both through the trash.
///
/// The result carries everything the undo needs, so the UI does not have to
/// guess which tasks went with the tab.
#[tauri::command]
pub async fn delete_tab(
    id: TaskId,
    db: State<'_, Db>,
) -> Result<crate::domain::task::TabDeletion, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::delete_tab(&mut conn, &id)
}

/// Takes back a tab deletion within the undo window.
#[tauri::command]
pub async fn restore_deleted_tab(
    id: TaskId,
    task_ids: Vec<TaskId>,
    db: State<'_, Db>,
) -> Result<crate::domain::task::TabDeletion, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::restore_deleted_tab(&mut conn, &id, &task_ids)
}

/// Per-tab task counters for the tab bar badges.
#[tauri::command]
pub async fn tab_stats(db: State<'_, Db>) -> Result<Vec<crate::domain::task::TabStat>, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::tab_stats(&conn)
}

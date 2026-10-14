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

/// Deletes a tab; its tasks move to `fallback_tab_id` so nothing is lost.
#[tauri::command]
pub async fn delete_tab(
    id: TaskId,
    fallback_tab_id: Option<TaskId>,
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::delete_tab(&mut conn, &id, fallback_tab_id.as_deref())
}

/// Per-tab task counters for the tab bar badges.
#[tauri::command]
pub async fn tab_stats(db: State<'_, Db>) -> Result<Vec<crate::domain::task::TabStat>, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::tab_stats(&conn)
}

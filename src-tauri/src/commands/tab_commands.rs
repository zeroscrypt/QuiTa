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

/// Hides a tab and remembers its tasks, leaving the tasks where they are.
///
/// The undo window is what makes this honest: the tasks move to the trash when
/// the window closes, not before it, so quitting the app mid-window leaves
/// something you can still take back. `finalize_tab_deletion` does the second
/// half; a restart calls `sweep_pending_tab_deletions` for the app that closed
/// instead of reporting.
///
/// The result carries everything the undo needs, so the UI does not have to
/// guess which tasks were in the tab.
#[tauri::command]
pub async fn delete_tab(
    id: TaskId,
    db: State<'_, Db>,
) -> Result<crate::domain::task::TabDeletion, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::delete_tab(&mut conn, &id)
}

/// Sends a tab's tasks to the trash once its undo window has passed.
#[tauri::command]
pub async fn finalize_tab_deletion(
    id: TaskId,
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::finalize_tab_deletion(&mut conn, &id)
}

/// Finishes a tab deletion whose undo window was cut short by the app closing,
/// so a tab hidden ten seconds before quitting does not stay hidden for ever.
#[tauri::command]
pub async fn sweep_pending_tab_deletions(
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::sweep_pending_tab_deletions(&mut conn)
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

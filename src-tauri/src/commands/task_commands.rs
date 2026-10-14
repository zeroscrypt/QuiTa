use tauri::State;

use crate::db::{repository, Db};
use crate::domain::task::{TaskId, ToggleOutcome};
use crate::error::TaskError;

#[tauri::command]
pub async fn create_task(
    title: String,
    parent_id: Option<TaskId>,
    tab_id: Option<TaskId>,
    db: State<'_, Db>,
) -> Result<crate::domain::task::Task, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    // New root tasks go to the top or the bottom depending on the preference;
    // the database is the source of truth so a reload keeps the order.
    let newest_first = repository::get_settings(&conn)?.newest_first;
    repository::create_task(
        &mut conn,
        &title,
        parent_id.as_deref(),
        tab_id.as_deref(),
        newest_first,
    )
}

#[tauri::command]
pub async fn rename_task(
    id: TaskId,
    title: String,
    db: State<'_, Db>,
) -> Result<crate::domain::task::Task, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::rename_task(&mut conn, &id, &title)
}

#[tauri::command]
pub async fn toggle_task_completion(
    id: TaskId,
    cascade_complete: Option<bool>,
    db: State<'_, Db>,
) -> Result<ToggleOutcome, TaskError> {
    // Defaults to the cascade behaviour when the UI does not send the flag.
    let cascade = cascade_complete.unwrap_or(true);
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::toggle_completion(&mut conn, &id, cascade)
}

#[tauri::command]
pub async fn set_task_due_date(
    id: TaskId,
    due_date: Option<String>,
    db: State<'_, Db>,
) -> Result<crate::domain::task::Task, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::set_due_date(&mut conn, &id, due_date.as_deref())
}

#[tauri::command]
pub async fn set_task_comment(
    id: TaskId,
    comment: Option<String>,
    db: State<'_, Db>,
) -> Result<crate::domain::task::Task, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::set_comment(&mut conn, &id, comment.as_deref())
}

#[tauri::command]
pub async fn soft_delete_task(
    id: TaskId,
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::soft_delete_task(&mut conn, &id)
}

#[tauri::command]
pub async fn restore_task(
    id: TaskId,
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::restore_task(&mut conn, &id)
}

#[tauri::command]
pub async fn move_task(id: TaskId, direction: String, db: State<'_, Db>) -> Result<(), TaskError> {
    let up = match direction.as_str() {
        "up" => true,
        "down" => false,
        other => return Err(TaskError::InvalidDirection(other.to_string())),
    };
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::move_task(&mut conn, &id, up)
}

#[tauri::command]
pub async fn list_tasks(
    include_completed: Option<bool>,
    tab_id: Option<TaskId>,
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::TaskNode>, TaskError> {
    let include = include_completed.unwrap_or(false);
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::list_task_nodes(&conn, include, tab_id.as_deref())
}

#[tauri::command]
pub async fn list_subtasks(
    parent_id: TaskId,
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::list_subtasks(&conn, &parent_id)
}

/// Number of active subtasks, used by the delete confirmation.
#[tauri::command]
pub async fn count_subtasks(parent_id: TaskId, db: State<'_, Db>) -> Result<i64, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::count_subtasks(&conn, &parent_id)
}

/// Lazily deletes completed tasks older than the configured retention period.
/// Runs when the UI loads the list, so no background timer is required.
#[tauri::command]
pub async fn purge_completed_tasks(
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    let settings = repository::get_settings(&conn)?;
    let Some(days) = settings.auto_delete_days else {
        return Ok(Vec::new());
    };
    repository::purge_completed_tasks(&mut conn, days)
}

#[tauri::command]
pub async fn list_deleted_tasks(
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::list_deleted_all(&conn, 200)
}

#[tauri::command]
pub async fn search_tasks(
    query: String,
    tab_id: Option<TaskId>,
    db: State<'_, Db>,
) -> Result<crate::domain::task::SearchResult, TaskError> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(crate::domain::task::SearchResult::default());
    }
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::search_tasks(&conn, &query, tab_id.as_deref())
}

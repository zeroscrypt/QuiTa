/// How many deleted tasks the recycle bin shows in one go.
///
/// It is a bound, not a feature: the bin is a list a person scans, and a
/// repository with tens of thousands of deleted rows should not try to render
/// all of them at once.
const TRASH_PAGE_SIZE: i64 = 200;

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
pub async fn toggle_task_not_relevance(
    id: TaskId,
    db: State<'_, Db>,
) -> Result<ToggleOutcome, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::toggle_not_relevance(&mut conn, &id)
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

/// What a move into the trash did: the tasks that arrived, and what fell out
/// because the bin was full.
///
/// Both halves are returned because both are things the user needs to be told
/// about — the first is visible, the second is gone for good.
///
/// Every path that puts tasks in the bin answers with this, not only a deletion
/// by hand. A tree finished by the timer and the tasks of a deleted tab arrive
/// in the trash exactly the same way, so the size rule has to hold on all of
/// them, and what it pushes out has to be named on all of them.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteOutcome {
    pub deleted: Vec<crate::domain::task::Task>,
    pub evicted: Vec<crate::domain::task::Task>,
}

#[tauri::command]
pub async fn soft_delete_task(id: TaskId, db: State<'_, Db>) -> Result<DeleteOutcome, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    let deleted = repository::soft_delete_task(&mut conn, &id)?;
    let settings = repository::get_settings(&conn)?;
    let evicted = repository::trim_trash(&mut conn, settings.trash_limit)?;
    Ok(DeleteOutcome { deleted, evicted })
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
///
/// The trash limit is enforced here, exactly as it is after a deletion by hand.
/// A finished tree arrives in the bin the same way a thrown-away task does, and
/// the size rule that governs the bin has no exceptions to remember — otherwise
/// the bin silently grows past its bound the first night the timer fires.
#[tauri::command]
pub async fn purge_completed_tasks(db: State<'_, Db>) -> Result<DeleteOutcome, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    let settings = repository::get_settings(&conn)?;
    let Some(days) = settings.auto_delete_days else {
        return Ok(DeleteOutcome {
            deleted: Vec::new(),
            evicted: Vec::new(),
        });
    };
    let sweep = repository::purge_completed_tasks(&mut conn, days, settings.trash_limit)?;
    Ok(DeleteOutcome {
        deleted: sweep.moved,
        evicted: sweep.evicted,
    })
}

/// Removes what has been sitting in the trash longer than the configured
/// retention period, for good. Runs when the UI loads, so no background timer
/// is required, and the setting is empty until the user asks for it.
#[tauri::command]
pub async fn purge_trash_by_age(
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    let settings = repository::get_settings(&conn)?;
    let Some(days) = settings.trash_retention_days else {
        return Ok(Vec::new());
    };
    repository::purge_trash_by_age(&mut conn, days)
}

/// One window of the trash, together with the total it is a window of.
///
/// The total is not a convenience: the header has to say how many tasks are
/// really there before anyone agrees to empty the bin. A count of one page
/// sitting above "delete everything" is how three hundred tasks used to
/// disappear behind a dialog that promised two hundred.
#[tauri::command]
pub async fn list_deleted_tasks(
    db: State<'_, Db>,
    offset: Option<i64>,
) -> Result<TrashPage, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    let offset = offset.unwrap_or(0).max(0);
    let tasks = repository::list_deleted_all(&conn, TRASH_PAGE_SIZE, offset)?;
    let total = repository::count_deleted(&conn)?;
    Ok(TrashPage { tasks, total })
}

/// A window of the trash plus how many are in there altogether.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashPage {
    pub tasks: Vec<crate::domain::task::Task>,
    pub total: i64,
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

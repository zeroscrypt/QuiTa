use serde::Serialize;
use tauri::State;

use crate::db::{repository, Db};
use crate::domain::task::TaskId;
use crate::error::TaskError;

#[tauri::command]
pub async fn purge_task(
    id: TaskId,
    db: State<'_, Db>,
) -> Result<Vec<crate::domain::task::Task>, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::purge_task(&mut conn, &id)
}

/// Drag & drop between levels: `parent_id = null` promotes the task to root.
#[tauri::command]
pub async fn reparent_task(
    id: TaskId,
    parent_id: Option<TaskId>,
    place_at_top: Option<bool>,
    db: State<'_, Db>,
) -> Result<crate::domain::task::Task, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::reparent_task(
        &mut conn,
        &id,
        parent_id.as_deref(),
        place_at_top.unwrap_or(true),
    )
}

/// Drag & drop onto a tab.
#[tauri::command]
pub async fn move_task_to_tab(
    id: TaskId,
    tab_id: TaskId,
    db: State<'_, Db>,
) -> Result<crate::domain::task::Task, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::move_task_to_tab(&mut conn, &id, &tab_id)
}

#[tauri::command]
pub async fn purge_all_deleted(db: State<'_, Db>) -> Result<usize, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::purge_all_deleted(&mut conn)
}

#[tauri::command]
pub async fn reorder_tasks(ordered_ids: Vec<TaskId>, db: State<'_, Db>) -> Result<(), TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::reorder_tasks(&mut conn, &ordered_ids)
}

/// Static, always up-to-date product information shown in "About".
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub description_ru: &'static str,
    pub description_en: &'static str,
    pub author: &'static str,
    pub license: &'static str,
    pub repository: &'static str,
    /// Global shortcut as it is registered on this machine, e.g. "⌘⇧Q" or
    /// "Win+Alt+Q". Read from the same place that registers it, so the settings
    /// cannot claim a combination the app does not actually use.
    pub shortcut: &'static str,
    /// Input limits enforced by the repository. The UI takes them from here
    /// rather than repeating the numbers: two hand-kept copies drift apart, and
    /// the input then stops accepting what the database would have stored.
    pub limits: Limits,
}

/// Field lengths the database accepts, in characters.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub title: usize,
    pub tab_name: usize,
    pub comment: usize,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "QuiTa",
        version: env!("CARGO_PKG_VERSION"),
        description_ru: "Лёгкое приложение для быстрой фиксации задач. Всё хранится локально: без аккаунтов, сервера и телеметрии. Любую задачу можно раскрыть в подзадачи.",
        description_en: "A lightweight app for capturing tasks instantly. Everything is stored locally: no accounts, no server, no telemetry. Any task can be expanded into subtasks.",
        author: "@zeroscrypt",
        license: "MIT",
        repository: "https://github.com/zeroscrypt/QuiTa",
        shortcut: crate::platform::shortcuts::shortcut_label(std::env::consts::OS),
        limits: Limits {
            title: crate::domain::validation::MAX_TITLE_CHARS,
            tab_name: crate::domain::validation::MAX_TAB_NAME_CHARS,
            comment: crate::domain::validation::MAX_COMMENT_CHARS,
        },
    }
}

/// Statistics of the local database file (size, pages, row counts).
#[tauri::command]
pub async fn db_stats(db: State<'_, Db>) -> Result<crate::domain::task::DbStats, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::db_stats(&conn)
}

/// Compacts the database file, returning the fresh statistics.
#[tauri::command]
pub async fn vacuum_db(db: State<'_, Db>) -> Result<crate::domain::task::DbStats, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::vacuum(&conn)
}

/// Turns the task list over when the "newest first" preference is switched.
#[tauri::command]
pub async fn reverse_root_order(db: State<'_, Db>) -> Result<(), TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::reverse_root_order(&mut conn)
}

/// Nests a task that has subtasks: the children become siblings under the new
/// parent. Returns a receipt the UI needs to offer an undo.
#[tauri::command]
pub async fn nest_and_flatten(
    task_id: crate::domain::task::TaskId,
    parent_id: crate::domain::task::TaskId,
    place_at_bottom: bool,
    db: State<'_, Db>,
) -> Result<crate::domain::task::ReparentReceipt, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::nest_and_flatten(&mut conn, &task_id, &parent_id, place_at_bottom)
}

/// Restores a structure produced by `nest_and_flatten`.
#[tauri::command]
pub async fn undo_nest(
    receipt: crate::domain::task::ReparentReceipt,
    db: State<'_, Db>,
) -> Result<(), TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::undo_nest(&mut conn, &receipt)
}

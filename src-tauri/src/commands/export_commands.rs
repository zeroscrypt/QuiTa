use std::path::PathBuf;

use chrono::Utc;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::db::{repository, Db};
use crate::domain::transfer::{self, ExportPayload};
use crate::error::TaskError;

/// Version 1 held tasks only. Version 2 adds the tabs, the settings and the
/// deleted tasks, which makes a round trip reproduce the whole application
/// instead of just the visible task list.
const EXPORT_FORMAT_VERSION: u32 = 2;

/// Upper bound for an import file. The largest realistic export — a hundred
/// thousand tasks — is around 25 MB, so 64 MB is generous. Without the check a
/// user who picks a multi-gigabyte file by mistake has the whole thing pulled
/// into memory at once.
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;

/// Result of an import, shown to the user so the result is never silent.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub path: String,
    pub format: String,
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
    /// Tabs created by the import.
    pub tabs_added: usize,
    /// Tabs whose name or colour was changed by the import.
    pub tabs_updated: usize,
    /// True when the file carried settings and they were applied.
    pub settings_restored: bool,
}

fn build_payload(conn: &rusqlite::Connection) -> Result<ExportPayload, TaskError> {
    Ok(ExportPayload {
        version: EXPORT_FORMAT_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        tasks: repository::export_tasks(conn)?,
        tabs: Some(repository::list_tabs(conn)?),
        settings: Some(repository::get_settings(conn)?),
    })
}

/// Writes the whole application state — tasks with their hierarchy, the trash,
/// the tabs and the settings — through the native save dialog. Returns the
/// written path, or None when the user cancelled, which is not an error.
#[tauri::command]
pub async fn export_tasks(db: State<'_, Db>, app: AppHandle) -> Result<Option<String>, TaskError> {
    let contents = {
        let conn = db.lock().map_err(TaskError::Internal)?;
        transfer::render(&build_payload(&conn)?)?
    };

    // blocking_save_file must not run on the async runtime's main thread.
    let builder = app
        .dialog()
        .file()
        .set_file_name(transfer::EXPORT_FILE_NAME)
        .add_filter("JSON", &["json"]);
    let path = tauri::async_runtime::spawn_blocking(move || builder.blocking_save_file())
        .await
        .ok()
        .flatten();

    match path {
        Some(path) => {
            let path = path
                .into_path()
                .map_err(|e| TaskError::Internal(e.to_string()))?;
            std::fs::write(&path, contents).map_err(|e| TaskError::Internal(e.to_string()))?;
            Ok(Some(path.display().to_string()))
        }
        None => Ok(None),
    }
}

/// Reads a JSON/YAML/XML file chosen through the native open dialog and merges
/// it into the database. The format is detected from the content.
///
/// Order matters: tabs first (tasks reference them), then tasks, then the
/// settings — nothing is ever deleted, so a wrong file can only add.
#[tauri::command]
pub async fn import_tasks(
    db: State<'_, Db>,
    app: AppHandle,
) -> Result<Option<ImportSummary>, TaskError> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("QuiTa export", &["json"])
            .blocking_pick_file()
    })
    .await
    .ok()
    .flatten();

    let Some(path) = picked else { return Ok(None) };
    let path: PathBuf = path
        .into_path()
        .map_err(|e| TaskError::ImportRead(e.to_string()))?;

    let size = std::fs::metadata(&path)
        .map(|m| m.len())
        .map_err(|e| TaskError::ImportRead(e.to_string()))?;
    if size > MAX_IMPORT_BYTES {
        return Err(TaskError::ImportTooLarge(MAX_IMPORT_BYTES));
    }
    let contents =
        std::fs::read_to_string(&path).map_err(|e| TaskError::ImportRead(e.to_string()))?;
    let payload = transfer::parse(&contents)?;

    let summary = {
        let mut conn = db.lock().map_err(TaskError::Internal)?;
        let (tabs_added, tabs_updated) = match &payload.tabs {
            Some(tabs) => repository::import_tabs(&mut conn, tabs)?,
            None => (0, 0),
        };
        let (added, updated, skipped) = repository::import_tasks(&mut conn, &payload.tasks)?;
        let settings_restored = match &payload.settings {
            Some(settings) => {
                repository::import_settings(&mut conn, settings)?;
                true
            }
            None => false,
        };
        ImportSummary {
            path: path.display().to_string(),
            format: "json".to_string(),
            added,
            updated,
            skipped,
            tabs_added,
            tabs_updated,
            settings_restored,
        }
    };
    Ok(Some(summary))
}

use tauri::State;

use crate::db::{repository, Db};
use crate::domain::task::AppSettings;
use crate::error::TaskError;

#[tauri::command]
pub async fn get_settings(db: State<'_, Db>) -> Result<AppSettings, TaskError> {
    let conn = db.lock().map_err(TaskError::Internal)?;
    repository::get_settings(&conn)
}

#[tauri::command]
pub async fn update_settings(
    settings: AppSettings,
    db: State<'_, Db>,
) -> Result<AppSettings, TaskError> {
    let mut conn = db.lock().map_err(TaskError::Internal)?;
    repository::update_settings(&mut conn, &settings)
}

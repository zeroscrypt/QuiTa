pub mod backup;
pub mod migrations;
pub mod repository;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime};

/// Shared SQLite connection. All data access goes through typed repository
/// functions which open short-lived transactions; the connection itself is
/// never exposed to the UI.
pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.0
            .lock()
            .map_err(|_| "database lock poisoned".to_string())
    }
}

/// Creates the database on first launch and applies pending migrations.
/// The database file lives in the OS-specific application data directory and
/// is never deleted on updates.
pub fn init<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join("quita.db");

    let conn = Connection::open(&db_path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations::run(&conn)?;
    // Every install needs at least one tab, and tasks created before tabs
    // existed are attached to the first one.
    repository::ensure_default_tab(&conn)?;

    // The copies live next to the file they mirror. The launch copy runs
    // here, where the connection and the directory are both at hand; the
    // commands pick the same folder from managed state.
    let backups = backup::backups_dir(&data_dir);
    app.manage(backup::BackupDir(backups.clone()));
    let settings = repository::get_settings(&conn)?;
    backup::auto_backup(&conn, &backups, &settings);

    app.manage(Db(Mutex::new(conn)));
    Ok(())
}

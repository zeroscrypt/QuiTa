use chrono::Utc;
use rusqlite::Connection;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "create_tasks_and_settings",
        sql: r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    parent_id TEXT REFERENCES tasks(id),
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    deleted_at TEXT,
    due_date TEXT,
    sort_order INTEGER NOT NULL,
    completion_event_id TEXT,
    auto_completed_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_tasks_active_roots
    ON tasks(sort_order) WHERE deleted_at IS NULL AND parent_id IS NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_subtasks
    ON tasks(parent_id, sort_order) WHERE deleted_at IS NULL AND parent_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_title ON tasks(title);
CREATE INDEX IF NOT EXISTS idx_tasks_deleted ON tasks(deleted_at) WHERE deleted_at IS NOT NULL;

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Enforce the single subtask level at the database level as well:
-- a task whose parent already has a parent must not be inserted/moved.
CREATE TRIGGER IF NOT EXISTS task_no_third_level_insert
BEFORE INSERT ON tasks
WHEN NEW.parent_id IS NOT NULL
  AND EXISTS (SELECT 1 FROM tasks WHERE id = NEW.parent_id AND parent_id IS NOT NULL)
BEGIN
    SELECT RAISE(ABORT, 'only one subtask level is allowed');
END;

CREATE TRIGGER IF NOT EXISTS task_no_third_level_update
BEFORE UPDATE OF parent_id ON tasks
WHEN NEW.parent_id IS NOT NULL
  AND EXISTS (SELECT 1 FROM tasks WHERE id = NEW.parent_id AND parent_id IS NOT NULL)
BEGIN
    SELECT RAISE(ABORT, 'only one subtask level is allowed');
END;
"#,
    },
    Migration {
        version: 2,
        name: "add_tabs",
        sql: r#"
-- Tabs group tasks by topic/project (single level of grouping, orthogonal to
-- the task/subtask hierarchy).
CREATE TABLE IF NOT EXISTS tabs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

ALTER TABLE tasks ADD COLUMN tab_id TEXT REFERENCES tabs(id);

CREATE INDEX IF NOT EXISTS idx_tasks_by_tab
    ON tasks(tab_id, sort_order) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_tabs_order ON tabs(sort_order);
"#,
    },
    Migration {
        version: 3,
        name: "add_task_comment",
        sql: r#"
-- Free-form note attached to a task. NULL means "no comment", which lets the UI
-- show the icon only when there is something to read.
ALTER TABLE tasks ADD COLUMN comment TEXT;
"#,
    },
];

/// Applies all pending migrations in order. Migration records are inserted
/// after each batch; existing data is never touched.
pub fn run(conn: &Connection) -> Result<(), rusqlite::Error> {
    // The bookkeeping table must exist before we can ask what is applied.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    for migration in MIGRATIONS {
        if migration.version > current {
            conn.execute_batch(migration.sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![migration.version, migration.name, Utc::now().to_rfc3339()],
            )?;
        }
    }
    Ok(())
}

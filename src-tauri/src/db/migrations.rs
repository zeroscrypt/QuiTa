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
    Migration {
        version: 5,
        name: "add_tab_deleted_at",
        sql: r#"
-- A deleted tab keeps its row so the tasks in the trash still point at a tab
-- that exists. Hard-deleting the row would break the foreign key, and setting
-- the tasks' tab_id to NULL would lose which tab they came from, so undoing
-- the deletion could not put them back.
ALTER TABLE tabs ADD COLUMN deleted_at TEXT;
"#,
    },
    Migration {
        version: 4,
        name: "add_task_search_text",
        sql: r#"
-- A lowercased copy of the title, written by Rust, for the search to run
-- against. SQLite's LIKE folds case for ASCII only, so without this column
-- "МОЛОКО" would not find "Молоко" in a Russian interface.
--
-- The values are filled in by the Rust side after this batch: SQLite's own
-- lower() is exactly the ASCII-only function the column exists to replace, so
-- backfilling here would bake the same bug into the existing rows.
ALTER TABLE tasks ADD COLUMN search_text TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_tasks_search_text ON tasks(search_text);
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

    backfill_search_text(conn)
}

/// Fills in `search_text` for rows the column never covered.
///
/// Called on every start, not only right after the migration: the check is a
/// single indexed query that is empty in the steady state, and it repairs a
/// database whose backfill was interrupted instead of leaving those rows
/// permanently unfindable.
fn backfill_search_text(conn: &Connection) -> Result<(), rusqlite::Error> {
    let pending: Vec<(String, String)> = {
        let mut stmt = conn.prepare("SELECT id, title FROM tasks WHERE search_text = ''")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    if pending.is_empty() {
        return Ok(());
    }
    for (id, title) in &pending {
        conn.execute(
            "UPDATE tasks SET search_text = ?1 WHERE id = ?2",
            rusqlite::params![fold(title), id],
        )?;
    }
    Ok(())
}

/// Lowercases a title the way the search column expects.
///
/// `to_lowercase` is Unicode-aware, which is the whole point: SQLite's
/// `lower()` would leave Cyrillic untouched.
pub(crate) fn fold(title: &str) -> String {
    title.to_lowercase()
}

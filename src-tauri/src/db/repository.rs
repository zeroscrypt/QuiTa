use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Row};
use uuid::Uuid;

use crate::domain::rules::{compute_mark_plan, compute_toggle_plan, MarkPlan, TogglePlan};
use crate::domain::task::{
    AppSettings, ChildPlacement, DbStats, MatchedSubtask, ReparentReceipt, SearchResult, Tab,
    TabDeletion, TabStat, Task, TaskNode, ToggleOutcome, TrashSweep, TAB_COLORS,
};
use crate::domain::validation;
use crate::error::TaskError;

/// Name given to the tab created automatically on first launch.
pub const DEFAULT_TAB_NAME: &str = "Новая вкладка";

/// How far an imported `sort_order` may stray from zero. The bound is generous
/// compared with any list a person will build, and it keeps `next_sort_order`
/// and `next_sort_order_at_bottom` a step away from overflowing.
const SORT_ORDER_LIMIT: i64 = 1 << 40;

/// How many soft-deleted tasks the trash keeps by default, and the most it can
/// ever be told to keep.
///
/// This is a bound on stored data, not on the window the interface renders:
/// once the trash is full, deleting anything pushes the oldest task out
/// permanently. The default is also the ceiling, because raising it would only
/// make the silent loss slower to arrive.
pub const DEFAULT_TRASH_LIMIT: i64 = 200;
pub const MAX_TRASH_LIMIT: i64 = 200;
/// Zero is not "keep nothing" but "do not evict by size at all", which is why
/// the range starts here rather than at one. A trash that keeps nothing would
/// destroy every task the moment it was deleted, with the undo notice still on
/// screen; a trash with no bound is simply the user declining the rule, and the
/// age rule still clears it if they set one.
pub const TRASH_LIMIT_OFF: i64 = 0;

const TASK_COLUMNS: &str = "id, parent_id, tab_id, title, created_at, updated_at, completed_at, deleted_at, due_date, comment, sort_order, completion_event_id, auto_completed_by, not_relevant_at, not_relevant_event_id, auto_not_relevant_by";

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn row_to_task(row: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get("id")?,
        parent_id: row.get("parent_id")?,
        tab_id: row.get("tab_id").ok().flatten(),
        title: row.get("title")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
        deleted_at: row.get("deleted_at")?,
        due_date: row.get("due_date")?,
        comment: row.get("comment")?,
        sort_order: row.get("sort_order")?,
        completion_event_id: row.get("completion_event_id")?,
        auto_completed_by: row.get("auto_completed_by")?,
        not_relevant_at: row.get("not_relevant_at")?,
        not_relevant_event_id: row.get("not_relevant_event_id")?,
        auto_not_relevant_by: row.get("auto_not_relevant_by")?,
    })
}

fn get_task_row(conn: &Connection, id: &str, include_deleted: bool) -> Result<Task, TaskError> {
    let sql = if include_deleted {
        format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?1")
    } else {
        format!("SELECT {TASK_COLUMNS} FROM tasks WHERE id = ?1 AND deleted_at IS NULL")
    };
    conn.query_row(&sql, [id], row_to_task)
        .optional()?
        .ok_or_else(|| TaskError::NotFound(id.to_string()))
}

/// New tasks go to the top of their sibling group: sort_order = min(siblings) - 1.
fn next_sort_order(
    conn: &Connection,
    parent_id: Option<&str>,
    tab_id: Option<&str>,
) -> Result<i64, TaskError> {
    // Note: no `.optional()` here — it would make the compiler infer T = i64
    // and fail on NULL. `row.get::<_, Option<i64>>` maps SQL NULL to None.
    let min: Option<i64> = match parent_id {
        None => conn.query_row(
            "SELECT MIN(sort_order) FROM tasks
             WHERE deleted_at IS NULL AND parent_id IS NULL AND tab_id IS ?1",
            [tab_id],
            |row| row.get::<_, Option<i64>>(0),
        )?,
        Some(pid) => conn.query_row(
            "SELECT MIN(sort_order) FROM tasks WHERE deleted_at IS NULL AND parent_id = ?1",
            [pid],
            |row| row.get::<_, Option<i64>>(0),
        )?,
    };
    // Saturating, because an imported file is allowed to carry any sort_order
    // it likes. At i64::MIN this used to wrap to i64::MAX, which quietly put
    // a new task at the wrong end of the list; in a debug build it panicked.
    Ok(min.unwrap_or(0).saturating_sub(1))
}

pub fn create_task(
    conn: &mut Connection,
    title: &str,
    parent_id: Option<&str>,
    tab_id: Option<&str>,
    place_at_top: bool,
) -> Result<Task, TaskError> {
    let title = validation::normalize_title(title).ok_or(TaskError::EmptyTitle)?;

    let tx = conn.transaction()?;
    if let Some(pid) = parent_id {
        let parent = get_task_row(&tx, pid, false)
            .map_err(|_| TaskError::ParentNotFound(pid.to_string()))?;
        // A completed parent may still receive subtasks: the row becomes
        // "partially done" (done, with work left below it).
        // Defence in depth: the DB trigger also enforces the level rule.
        if !parent.is_root() {
            return Err(TaskError::ThirdLevelForbidden);
        }
    }

    let now = now_rfc3339();
    // Subtasks inherit the parent's tab so the tree never spans tabs; a task
    // without one falls back to the first tab rather than a NULL, because
    // `list_roots` filters on `tab_id` and a NULL task would be invisible.
    let inherited = match parent_id {
        Some(pid) => get_task_row(&tx, pid, false).map(|p| p.tab_id),
        None => Ok(None),
    }?;
    let resolved_tab = ensure_task_tab(&tx, inherited.or_else(|| tab_id.map(str::to_string)))?;
    let sort_tab = parent_id.is_none().then_some(resolved_tab.as_str());
    let task = Task {
        id: Uuid::new_v4().to_string(),
        parent_id: parent_id.map(|s| s.to_string()),
        tab_id: Some(resolved_tab.clone()),
        title,
        created_at: now.clone(),
        updated_at: now,
        completed_at: None,
        deleted_at: None,
        due_date: None,
        comment: None,
        // Subtasks are always appended. Root tasks follow the user's
        // "newest first" preference: on top of the tab, or at the bottom.
        sort_order: match parent_id {
            Some(_) => next_sort_order_at_bottom(&tx, parent_id)?,
            None if place_at_top => next_sort_order(&tx, parent_id, sort_tab)?,
            None => next_sort_order_at_bottom(&tx, parent_id)?,
        },
        completion_event_id: None,
        auto_completed_by: None,
        not_relevant_at: None,
        not_relevant_event_id: None,
        auto_not_relevant_by: None,
    };
    tx.execute(
        "INSERT INTO tasks (id, parent_id, tab_id, title, search_text, created_at, updated_at, completed_at, deleted_at, due_date, comment, sort_order, completion_event_id, auto_completed_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, NULL, NULL, ?8, NULL, NULL)",
        rusqlite::params![
            task.id,
            task.parent_id,
            task.tab_id,
            task.title,
            crate::db::migrations::fold(&task.title),
            task.created_at,
            task.updated_at,
            task.sort_order
        ],
    )?;
    tx.commit()?;
    Ok(task)
}

pub fn rename_task(conn: &mut Connection, id: &str, title: &str) -> Result<Task, TaskError> {
    let title = validation::normalize_title(title).ok_or(TaskError::EmptyTitle)?;
    let tx = conn.transaction()?;
    get_task_row(&tx, id, false)?;
    let now = now_rfc3339();
    tx.execute(
        "UPDATE tasks SET title = ?1, search_text = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![title, crate::db::migrations::fold(&title), now, id],
    )?;
    tx.commit()?;
    get_task_row(conn, id, false)
}

/// Sets or clears a task comment. Blank text is stored as `None` so the UI can
/// treat "empty" and "absent" the same way.
pub fn set_comment(
    conn: &mut Connection,
    id: &str,
    comment: Option<&str>,
) -> Result<Task, TaskError> {
    let cleaned = comment
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string);
    if let Some(text) = cleaned.as_deref() {
        if text.chars().count() > validation::MAX_COMMENT_CHARS {
            return Err(TaskError::CommentTooLong(
                validation::MAX_COMMENT_CHARS.to_string(),
            ));
        }
    }
    let tx = conn.transaction()?;
    get_task_row(&tx, id, false)?;
    let now = now_rfc3339();
    tx.execute(
        "UPDATE tasks SET comment = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![cleaned, now, id],
    )?;
    tx.commit()?;
    get_task_row(conn, id, false)
}

pub fn set_due_date(
    conn: &mut Connection,
    id: &str,
    due_date: Option<&str>,
) -> Result<Task, TaskError> {
    if let Some(d) = due_date {
        if !validation::is_valid_due_date(d) {
            return Err(TaskError::InvalidDueDate(d.to_string()));
        }
    }
    let tx = conn.transaction()?;
    get_task_row(&tx, id, false)?;
    let now = now_rfc3339();
    tx.execute(
        "UPDATE tasks SET due_date = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![due_date, now, id],
    )?;
    tx.commit()?;
    get_task_row(conn, id, false)
}

/// Toggles completion of a task, applying the full parent/child rules in a
/// single transaction. `cascade` mirrors the user's "complete subtasks with
/// the parent" setting. Returns every task that changed so the UI can update
/// precisely without a full reload.
pub fn toggle_completion(
    conn: &mut Connection,
    id: &str,
    cascade: bool,
) -> Result<ToggleOutcome, TaskError> {
    let tx = conn.transaction()?;
    let target = get_task_row(&tx, id, false)?;
    let children = list_subtasks_tx(&tx, id)?;

    let plan = compute_toggle_plan(&target, &children, cascade);
    apply_toggle_plan(&tx, &plan)?;

    let mut updated_ids: Vec<String> = vec![target.id.clone()];
    updated_ids.extend(plan.children_to_complete.iter().cloned());
    updated_ids.extend(plan.children_to_reopen.iter().cloned());

    let mut updated = Vec::with_capacity(updated_ids.len());
    for uid in updated_ids {
        updated.push(get_task_row(&tx, &uid, false)?);
    }
    tx.commit()?;
    Ok(ToggleOutcome { updated })
}

fn apply_toggle_plan(tx: &Connection, plan: &TogglePlan) -> Result<(), TaskError> {
    let now = now_rfc3339();
    if plan.complete {
        tx.execute(
            "UPDATE tasks SET completed_at = ?1, updated_at = ?1, completion_event_id = ?2, auto_completed_by = NULL WHERE id = ?3",
            rusqlite::params![now, plan.completion_event_id, plan.target_id],
        )?;
    } else {
        tx.execute(
            "UPDATE tasks SET completed_at = NULL, updated_at = ?1, completion_event_id = NULL, auto_completed_by = NULL WHERE id = ?2",
            rusqlite::params![now, plan.target_id],
        )?;
    }
    for cid in &plan.children_to_complete {
        tx.execute(
            "UPDATE tasks SET completed_at = ?1, updated_at = ?1, auto_completed_by = ?2 WHERE id = ?3",
            rusqlite::params![now, plan.completion_event_id, cid],
        )?;
    }
    for cid in &plan.children_to_reopen {
        tx.execute(
            "UPDATE tasks SET completed_at = NULL, updated_at = ?1, auto_completed_by = NULL WHERE id = ?2",
            rusqlite::params![now, cid],
        )?;
    }
    Ok(())
}

/// Toggles the "не актуальная" mark on a task, mirroring the completion
/// cascade. Marking a parent auto-marks its not-yet-marked subtasks under a
/// fresh event id recorded on the parent; unmarking undoes exactly the marks
/// that event made, leaving hand-marked subtasks alone. See
/// `compute_mark_plan` in `domain/rules.rs` and docs/TASK_RULES.md.
pub fn toggle_not_relevance(conn: &mut Connection, id: &str) -> Result<ToggleOutcome, TaskError> {
    let tx = conn.transaction()?;
    let target = get_task_row(&tx, id, false)?;
    let children = list_subtasks_tx(&tx, id)?;

    let plan = compute_mark_plan(&target, &children);
    apply_mark_plan(&tx, &plan)?;

    let mut updated_ids: Vec<String> = vec![target.id.clone()];
    updated_ids.extend(plan.children_to_mark.iter().cloned());
    updated_ids.extend(plan.children_to_unmark.iter().cloned());

    let mut updated = Vec::with_capacity(updated_ids.len());
    for uid in updated_ids {
        updated.push(get_task_row(&tx, &uid, false)?);
    }
    tx.commit()?;
    Ok(ToggleOutcome { updated })
}

fn apply_mark_plan(tx: &Connection, plan: &MarkPlan) -> Result<(), TaskError> {
    let now = now_rfc3339();
    if plan.mark {
        tx.execute(
            "UPDATE tasks SET not_relevant_at = ?1, not_relevant_event_id = ?2, updated_at = ?1 WHERE id = ?3",
            rusqlite::params![now, plan.mark_event_id, plan.target_id],
        )?;
    } else {
        tx.execute(
            "UPDATE tasks SET not_relevant_at = NULL, not_relevant_event_id = NULL, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, plan.target_id],
        )?;
    }
    for cid in &plan.children_to_mark {
        tx.execute(
            "UPDATE tasks SET not_relevant_at = ?1, auto_not_relevant_by = ?2, updated_at = ?1 WHERE id = ?3",
            rusqlite::params![now, plan.mark_event_id, cid],
        )?;
    }
    for cid in &plan.children_to_unmark {
        tx.execute(
            "UPDATE tasks SET not_relevant_at = NULL, auto_not_relevant_by = NULL, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, cid],
        )?;
    }
    Ok(())
}

/// Soft-deletes a task and (for parents) all of its subtasks atomically.
/// Returns the deleted tasks (parent first).
pub fn soft_delete_task(conn: &mut Connection, id: &str) -> Result<Vec<Task>, TaskError> {
    let tx = conn.transaction()?;
    let target = get_task_row(&tx, id, false)?;
    let mut deleted = vec![target];
    deleted.extend(list_subtasks_tx(&tx, id)?);
    let now = now_rfc3339();
    for task in &deleted {
        tx.execute(
            "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task.id],
        )?;
    }
    tx.commit()?;
    Ok(deleted)
}

/// Restores a soft-deleted task. Only the task itself is restored — subtasks
/// deleted independently stay deleted, and a subtask whose parent is deleted
/// cannot be restored (it would become an orphan).
/// Every soft-deleted task, root tasks *and* their subtasks.
///
/// The trash shows exactly what is in there: a subtask that was removed
/// together with its parent would otherwise be invisible and unreachable.
pub fn list_deleted_all(
    conn: &Connection,
    limit: i64,
    offset: i64,
) -> Result<Vec<Task>, TaskError> {
    // The offset is what makes the trash a window rather than a wall. Without
    // it the limit silently cut off everything past the newest rows, and those
    // tasks could not be restored one by one, because there was no row on
    // screen to press the button on.
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC, sort_order ASC LIMIT ?1 OFFSET ?2"
    ))?;
    let tasks = stmt
        .query_map(rusqlite::params![limit, offset], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

/// How many tasks are in the trash in total, ignoring any window.
///
/// The list itself is capped, and the header still has to state the real
/// number: a count of 200 sitting above "delete everything" is what made a
/// user agree to a purge that then removed three times as much.
pub fn count_deleted(conn: &Connection) -> Result<i64, TaskError> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE deleted_at IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    Ok(total)
}

/// Restores a soft-deleted task.
///
/// Restoring a root also restores the subtasks that went with it, otherwise
/// they would stay invisible in the trash forever. Restoring a subtask whose
/// parent is still deleted turns it into a root task — the alternative is
/// leaving it unreachable.
pub fn restore_task(conn: &mut Connection, id: &str) -> Result<Vec<Task>, TaskError> {
    let tx = conn.transaction()?;
    let task = get_task_row(&tx, id, true)?;
    if task.deleted_at.is_none() {
        return Ok(vec![task]);
    }
    let now = now_rfc3339();

    let parent_deleted = match &task.parent_id {
        Some(pid) => match get_task_row(&tx, pid, true) {
            Ok(parent) => parent.deleted_at.is_some(),
            // The parent row is gone: the subtask becomes a root task.
            Err(_) => true,
        },
        None => false,
    };

    if parent_deleted {
        tx.execute(
            "UPDATE tasks SET parent_id = NULL, deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
    } else {
        tx.execute(
            "UPDATE tasks SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        // Bring back the subtasks that were removed with this parent.
        // `list_subtasks` hides deleted rows, so the trash needs its own query.
        for child in list_all_subtasks_tx(&tx, id)? {
            if child.deleted_at.is_some() {
                tx.execute(
                    "UPDATE tasks SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
                    rusqlite::params![now, child.id],
                )?;
            }
        }
    }
    // A tab deleted alongside its tasks keeps its row but is hidden from the
    // strip. Restoring one of those tasks without its tab would put a task back
    // where no list can show it, so the tab comes back with it.
    let tab_of: Option<Option<String>> = tx
        .query_row("SELECT tab_id FROM tasks WHERE id = ?1", [id], |row| {
            row.get::<_, Option<String>>(0)
        })
        .optional()?;
    if let Some(Some(tab_id)) = tab_of {
        tx.execute(
            "UPDATE tabs SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NOT NULL",
            rusqlite::params![now, tab_id],
        )?;
    }

    tx.commit()?;

    // Return everything that is active again after the restore.
    let mut restored = vec![get_task_row(conn, id, false)?];
    for child in list_subtasks(conn, id)? {
        if child.deleted_at.is_none() {
            restored.push(child);
        }
    }
    Ok(restored)
}

/// Swaps sort_order with the previous/next sibling in the same group.
pub fn move_task(conn: &mut Connection, id: &str, up: bool) -> Result<(), TaskError> {
    let tx = conn.transaction()?;
    let task = get_task_row(&tx, id, false)?;
    let siblings = list_siblings_tx(&tx, &task)?;
    let pos = siblings
        .iter()
        .position(|t| t.id == task.id)
        .ok_or_else(|| TaskError::NotFound(id.to_string()))?;
    let neighbor_pos = if up {
        pos.checked_sub(1)
    } else {
        Some(pos + 1).filter(|p| *p < siblings.len())
    };
    if let Some(other_pos) = neighbor_pos {
        let other = &siblings[other_pos];
        let now = now_rfc3339();
        tx.execute(
            "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![other.sort_order, now, task.id],
        )?;
        tx.execute(
            "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![task.sort_order, now, other.id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub fn list_roots(
    conn: &Connection,
    include_completed: bool,
    tab_id: Option<&str>,
) -> Result<Vec<Task>, TaskError> {
    // `tab_id = None` means "every tab"; a specific id filters to that tab.
    // "Closed" is either completion or the "не актуальная" mark, so both
    // timestamps gate the hidden branch; the showing branch filters nothing,
    // exactly like completed tasks.
    let sql = if include_completed {
        format!("SELECT {TASK_COLUMNS} FROM tasks WHERE parent_id IS NULL AND deleted_at IS NULL AND (?1 IS NULL OR tab_id = ?1) ORDER BY sort_order ASC")
    } else {
        format!("SELECT {TASK_COLUMNS} FROM tasks WHERE parent_id IS NULL AND deleted_at IS NULL AND completed_at IS NULL AND not_relevant_at IS NULL AND (?1 IS NULL OR tab_id = ?1) ORDER BY sort_order ASC")
    };
    let mut stmt = conn.prepare(&sql)?;
    let tasks = stmt
        .query_map([tab_id], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

/// Active root tasks together with their subtasks (one level).
pub fn list_task_nodes(
    conn: &Connection,
    include_completed: bool,
    tab_id: Option<&str>,
) -> Result<Vec<TaskNode>, TaskError> {
    let roots = list_roots(conn, include_completed, tab_id)?;
    let mut nodes = Vec::with_capacity(roots.len());
    for root in roots {
        let children = list_subtasks(conn, &root.id)?;
        nodes.push(TaskNode {
            task: root,
            children,
        });
    }
    Ok(nodes)
}

/// Number of active subtasks of a task.
///
/// The delete confirmation needs this even when completed subtasks are hidden
/// from the list, so it must not be derived from what the UI has loaded.
pub fn count_subtasks(conn: &Connection, parent_id: &str) -> Result<i64, TaskError> {
    conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL",
        [parent_id],
        |row| row.get(0),
    )
    .map_err(TaskError::from)
}

pub fn list_subtasks(conn: &Connection, parent_id: &str) -> Result<Vec<Task>, TaskError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC"
    ))?;
    let tasks = stmt
        .query_map([parent_id], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

fn list_subtasks_tx(tx: &Connection, parent_id: &str) -> Result<Vec<Task>, TaskError> {
    list_subtasks(tx, parent_id)
}

/// Subtasks including the soft-deleted ones — the recycle bin must see them.
fn list_all_subtasks_tx(tx: &Connection, parent_id: &str) -> Result<Vec<Task>, TaskError> {
    let mut stmt = tx.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks WHERE parent_id = ?1 ORDER BY sort_order ASC"
    ))?;
    let tasks = stmt
        .query_map([parent_id], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

fn list_siblings_tx(tx: &Connection, task: &Task) -> Result<Vec<Task>, TaskError> {
    let (sql, param) = match &task.parent_id {
        None => (
            format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE parent_id IS NULL AND deleted_at IS NULL AND tab_id IS ?1 ORDER BY sort_order ASC"
            ),
            task.tab_id.as_deref(),
        ),
        Some(pid) => (
            format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC"
            ),
            Some(pid.as_str()),
        ),
    };
    let mut stmt = tx.prepare(&sql)?;
    let tab_value: rusqlite::types::Value = match param {
        Some(value) => rusqlite::types::Value::Text(value.to_string()),
        None => rusqlite::types::Value::Null,
    };
    let tasks = stmt
        .query_map([tab_value], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

/// Moves finished trees to the trash once the work in them has been closed for
/// `days`. Runs on startup / list refresh, so no background timer is needed.
/// Returns the moved tasks, parent first.
///
/// The unit is the tree, and the countdown starts at its *last* closure. Taking
/// every task by its own timestamp was how a tree parted with open work: a
/// parent closed weeks ago carried off a subtask nobody had finished, quietly.
pub fn purge_completed_tasks(
    conn: &mut Connection,
    days: i64,
    limit: i64,
) -> Result<TrashSweep, TaskError> {
    if days <= 0 {
        return Ok(TrashSweep::empty());
    }
    let tx = conn.transaction()?;

    // Both halves of the rule sit in one query on purpose. Walking candidate
    // parents in Rust and then asking about their children would leave a gap
    // between the two lookups — and a subtask reopened inside that gap would be
    // carried off by the very check written to protect it.
    let due: Vec<Task> = {
        let mut stmt = tx.prepare(&format!(
            "SELECT {TASK_COLUMNS}
             FROM tasks AS root
             WHERE root.parent_id IS NULL
               AND root.deleted_at IS NULL
               AND (root.completed_at IS NOT NULL OR root.not_relevant_at IS NOT NULL)
               AND NOT EXISTS (
                   SELECT 1 FROM tasks AS open
                   WHERE open.deleted_at IS NULL
                     AND open.completed_at IS NULL
                     AND open.not_relevant_at IS NULL
                     AND (open.id = root.id OR open.parent_id = root.id)
               )
               AND datetime((
                   SELECT MAX(COALESCE(closed.completed_at, closed.not_relevant_at)) FROM tasks AS closed
                   WHERE closed.deleted_at IS NULL
                     AND (closed.id = root.id OR closed.parent_id = root.id)
               )) < datetime('now', ?1)"
        ))?;
        let rows = stmt
            .query_map([format!("-{days} days")], row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let now = now_rfc3339();
    let mut moved: Vec<Task> = Vec::new();
    for root in due {
        let children = list_subtasks_tx(&tx, &root.id)?;
        moved.push(root);
        moved.extend(children);
    }
    for task in &moved {
        tx.execute(
            "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, task.id],
        )?;
    }
    tx.commit()?;
    // The trash limit is enforced here rather than in the caller: a rule that
    // only holds when every caller remembers it is not a rule. A timer that
    // arrives with thirty finished tasks would otherwise push the bin past its
    // bound and take the next manual deletion's eviction away from whoever it
    // belongs to.
    let evicted = trim_trash(conn, limit)?;
    Ok(TrashSweep { moved, evicted })
}

/// Permanently removes a soft-deleted task (and its subtasks).
/// This cannot be undone, so the UI must confirm first.
/// Collects every descendant of `root_id`, soft-deleted ones included.
///
/// `list_subtasks` only returns active rows, which is correct for the UI but
/// not for a permanent delete: leaving an already soft-deleted child behind
/// would break the `parent_id` foreign key and roll the whole delete back.
/// Nests a task that has subtasks.
///
/// Only two levels exist, so a task that carries subtasks cannot keep them:
/// the children become siblings directly under the new parent, right after
/// the moved task and in their previous order. Nothing is deleted, and the
/// caller gets everything needed to undo the move (see `ReparentReceipt`).
pub fn nest_and_flatten(
    conn: &mut Connection,
    id: &str,
    new_parent_id: &str,
    place_at_bottom: bool,
) -> Result<ReparentReceipt, TaskError> {
    let tx = conn.transaction()?;
    let task = get_task_row(&tx, id, false)?;

    if new_parent_id == id || is_descendant_of(&tx, new_parent_id, id)? {
        return Err(TaskError::WouldCreateCycle);
    }
    let parent = get_task_row(&tx, new_parent_id, false)
        .map_err(|_| TaskError::ParentNotFound(new_parent_id.to_string()))?;
    if !parent.is_root() {
        return Err(TaskError::ThirdLevelForbidden);
    }

    let children = list_subtasks_tx(&tx, id)?;
    let target_tab = ensure_task_tab(&tx, parent.tab_id.clone())?;
    let now = now_rfc3339();

    let base = if place_at_bottom {
        next_sort_order_at_bottom(&tx, Some(new_parent_id))?
    } else {
        next_sort_order(&tx, Some(new_parent_id), Some(&target_tab))?
    };

    tx.execute(
        "UPDATE tasks SET parent_id = ?1, tab_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
        rusqlite::params![new_parent_id, target_tab, base, now, id],
    )?;
    // Children are promoted to siblings, keeping their order right after the
    // task that used to own them.
    for (offset, child) in children.iter().enumerate() {
        tx.execute(
            "UPDATE tasks SET parent_id = ?1, tab_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
            rusqlite::params![
                new_parent_id,
                target_tab,
                base + offset as i64 + 1,
                now,
                child.id
            ],
        )?;
    }
    tx.commit()?;

    Ok(ReparentReceipt {
        task_id: id.to_string(),
        previous_parent_id: task.parent_id.clone(),
        previous_tab_id: task.tab_id.clone(),
        previous_sort_order: task.sort_order,
        flattened: children
            .iter()
            .map(|child| ChildPlacement {
                id: child.id.clone(),
                sort_order: child.sort_order,
            })
            .collect(),
    })
}

/// Restores the structure produced by `nest_and_flatten`.
pub fn undo_nest(conn: &mut Connection, receipt: &ReparentReceipt) -> Result<(), TaskError> {
    let tx = conn.transaction()?;
    get_task_row(&tx, &receipt.task_id, false)?;
    let now = now_rfc3339();
    let tab_id = ensure_task_tab(&tx, receipt.previous_tab_id.clone())?;
    tx.execute(
        "UPDATE tasks SET parent_id = ?1, tab_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
        rusqlite::params![
            receipt.previous_parent_id,
            tab_id,
            receipt.previous_sort_order,
            now,
            receipt.task_id
        ],
    )?;
    for child in &receipt.flattened {
        tx.execute(
            "UPDATE tasks SET parent_id = ?1, tab_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
            rusqlite::params![receipt.task_id, tab_id, child.sort_order, now, child.id],
        )?;
    }
    tx.commit()?;
    Ok(())
}

fn collect_all_descendants(tx: &Connection, root_id: &str) -> Result<Vec<Task>, TaskError> {
    let mut collected: Vec<Task> = Vec::new();
    let mut queue = vec![root_id.to_string()];
    while let Some(current) = queue.pop() {
        let children = {
            let mut stmt = tx.prepare(&format!(
                "SELECT {TASK_COLUMNS} FROM tasks WHERE parent_id = ?1"
            ))?;
            let rows = stmt
                .query_map([&current], row_to_task)?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        for child in children {
            queue.push(child.id.clone());
            collected.push(child);
        }
    }
    Ok(collected)
}

/// Permanently removes a soft-deleted task (and all of its subtasks).
/// This cannot be undone, so the UI must confirm first.
///
/// Refuses to touch a task that is still active: the command is reachable from
/// the webview, and an active task must go through soft delete first so it
/// stays recoverable.
pub fn purge_task(conn: &mut Connection, id: &str) -> Result<Vec<Task>, TaskError> {
    let tx = conn.transaction()?;
    let target = get_task_row(&tx, id, true)?;
    if target.deleted_at.is_none() {
        return Err(TaskError::NotDeleted(id.to_string()));
    }
    let mut doomed = vec![target.clone()];
    doomed.extend(collect_all_descendants(&tx, id)?);

    // Children first so the foreign key never dangles mid-transaction.
    let mut ids: Vec<String> = doomed.iter().map(|t| t.id.clone()).collect();
    ids.sort_by_key(|child_id| {
        doomed
            .iter()
            .position(|t| &t.id == child_id)
            .unwrap_or(usize::MAX)
    });
    ids.reverse();
    for task_id in ids {
        tx.execute("DELETE FROM tasks WHERE id = ?1", [&task_id])?;
    }
    purge_emptied_tabs(&tx)?;
    tx.commit()?;
    Ok(doomed)
}

/// Removes what has sat in the trash longer than `days`, for good. Runs when the
/// UI loads, like the completed-task sweep. Returns the tasks removed.
///
/// The unit is the tree, and the countdown starts at the *last* moment it
/// entered the trash. Deleting a parent stamps the whole tree at once, so a
/// subtask that arrived afterwards would otherwise be removed the same second
/// it was thrown away — nobody had a chance to change their mind about it.
///
/// A subtree deleted on its own is its own unit: its parent is still in the
/// list, and there is no tree to speak of.
pub fn purge_trash_by_age(conn: &mut Connection, days: i64) -> Result<Vec<Task>, TaskError> {
    if days <= 0 {
        return Ok(Vec::new());
    }
    let tx = conn.transaction()?;

    // One query for the same reason as `purge_completed_tasks`: a tree that
    // entered the trash between two lookups must not be swept by the very
    // check written to protect it.
    let expired: Vec<Task> = {
        let mut stmt = tx.prepare(&format!(
            "SELECT {TASK_COLUMNS}
             FROM tasks AS root
             WHERE root.deleted_at IS NOT NULL
               AND (
                   root.parent_id IS NULL
                   OR NOT EXISTS (
                       SELECT 1 FROM tasks AS alive
                       WHERE alive.id = root.parent_id
                         AND alive.deleted_at IS NOT NULL
                   )
               )
               AND datetime((
                   SELECT MAX(gone.deleted_at) FROM tasks AS gone
                   WHERE gone.id = root.id OR gone.parent_id = root.id
               )) < datetime('now', ?1)"
        ))?;
        let rows = stmt
            .query_map([format!("-{days} days")], row_to_task)?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let mut removed: Vec<Task> = Vec::new();
    for root in expired {
        let mut doomed = vec![root];
        let root_id = doomed[0].id.clone();
        doomed.extend(collect_all_descendants(&tx, &root_id)?);

        // Children first so the foreign key never dangles mid-transaction.
        let mut ids: Vec<String> = doomed.iter().map(|t| t.id.clone()).collect();
        ids.sort_by_key(|child_id| {
            doomed
                .iter()
                .position(|t| &t.id == child_id)
                .unwrap_or(usize::MAX)
        });
        ids.reverse();
        for task_id in ids {
            tx.execute("DELETE FROM tasks WHERE id = ?1", [&task_id])?;
        }
        removed.extend(doomed);
    }
    purge_emptied_tabs(&tx)?;
    tx.commit()?;
    Ok(removed)
}

/// Evicts the oldest tasks once the trash is full, and reports what went.
///
/// The oldest goes because it is the one the user has had longest to recover.
/// A task leaves together with its subtasks, the same rule the per-task
/// permanent delete already follows: a parent without its children would leave
/// them in the trash belonging to nothing, where no list would show them.
///
/// The count may end up below the limit rather than exactly at it, and that is
/// correct: keeping a parent and half its subtasks is worse than keeping fewer.
pub fn trim_trash(conn: &mut Connection, limit: i64) -> Result<Vec<Task>, TaskError> {
    // Zero is a choice, not a small number: the rule is off and the trash
    // grows without a bound.
    if limit <= TRASH_LIMIT_OFF {
        return Ok(Vec::new());
    }
    let limit = limit.min(MAX_TRASH_LIMIT);
    let tx = conn.transaction()?;

    let overflowing: Vec<String> = {
        let mut stmt = tx.prepare(
            "SELECT id FROM tasks WHERE deleted_at IS NOT NULL
             ORDER BY deleted_at DESC, sort_order ASC
             LIMIT -1 OFFSET ?1",
        )?;
        let rows = stmt
            .query_map([limit], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    if overflowing.is_empty() {
        return Ok(Vec::new());
    }

    let mut evicted: Vec<Task> = Vec::new();
    for id in overflowing {
        // A parent may already have gone with its own descendants.
        let gone = tx
            .query_row("SELECT 1 FROM tasks WHERE id = ?1", [&id], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
            .is_none();
        if gone {
            continue;
        }
        let doomed = {
            let root = get_task_row(&tx, &id, true)?;
            let mut all = vec![root];
            all.extend(collect_all_descendants(&tx, &id)?);
            all
        };
        let mut ids: Vec<String> = doomed.iter().map(|task| task.id.clone()).collect();
        // Children first, so the foreign key never dangles mid-transaction.
        ids.reverse();
        for task_id in &ids {
            tx.execute("DELETE FROM tasks WHERE id = ?1", [task_id])?;
        }
        evicted.extend(doomed);
    }

    purge_emptied_tabs(&tx)?;
    tx.commit()?;
    Ok(evicted)
}

/// Permanently removes every soft-deleted task (used by "delete all").
///
/// A row whose parent is deleted belongs to the deleted tree even when its
/// own `deleted_at` was never set — old data, or a subtask that was moved in
/// before the cascade reached it. Such orphans are collected too, and
/// children are deleted before parents so the foreign key never dangles
/// (SQLite checks immediate constraints per statement, not per transaction).
pub fn purge_all_deleted(conn: &mut Connection) -> Result<usize, TaskError> {
    let tx = conn.transaction()?;
    let mut removed = 0usize;

    let children = {
        let mut stmt = tx.prepare(
            "SELECT id FROM tasks
             WHERE parent_id IN (SELECT id FROM tasks WHERE deleted_at IS NOT NULL)",
        )?;
        let ids = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        ids
    };
    for id in &children {
        removed += tx.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
    }

    removed += tx.execute("DELETE FROM tasks WHERE deleted_at IS NOT NULL", [])?;
    purge_emptied_tabs(&tx)?;
    tx.commit()?;
    Ok(removed)
}

/// Resolves the tab a task must belong to, never returning `None`: a task
/// without a tab is filtered out of every list and becomes invisible.
fn ensure_task_tab(conn: &Connection, candidate: Option<String>) -> Result<String, TaskError> {
    if let Some(tab_id) = candidate {
        return Ok(tab_id);
    }
    let first: Option<String> = conn
        .query_row(
            "SELECT id FROM tabs ORDER BY sort_order ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    match first {
        Some(id) => Ok(id),
        None => Ok(ensure_default_tab(conn)?.id),
    }
}

/// True when `ancestor_id` is `node_id` itself or one of its subtasks.
/// Used to reject moves that would create a cycle.
fn is_descendant_of(
    conn: &Connection,
    node_id: &str,
    ancestor_id: &str,
) -> Result<bool, TaskError> {
    let mut current = Some(node_id.to_string());
    // Nesting is capped at one level, so the walk is two hops at most. The
    // bound is a safety net: corrupt data must not spin while holding the
    // connection mutex.
    for _ in 0..64 {
        let Some(id) = current else { return Ok(false) };
        if id == ancestor_id {
            return Ok(true);
        }
        current = conn
            .query_row("SELECT parent_id FROM tasks WHERE id = ?1", [&id], |row| {
                row.get::<_, Option<String>>(0)
            })
            .optional()?
            .flatten();
    }
    Err(TaskError::Internal(
        "task hierarchy is corrupted".to_string(),
    ))
}

/// Drops deleted tabs that no task refers to any more.
///
/// A tab deletion keeps the row so its tasks keep a valid reference. Once the
/// trash is emptied those references are gone, and the row would otherwise sit
/// in the database forever.
fn purge_emptied_tabs(conn: &Connection) -> Result<usize, TaskError> {
    let removed = conn.execute(
        "DELETE FROM tabs
         WHERE deleted_at IS NOT NULL
           AND id NOT IN (SELECT tab_id FROM tasks WHERE tab_id IS NOT NULL)",
        [],
    )?;
    Ok(removed)
}

/// Places a task at the end of its sibling group.
fn next_sort_order_at_bottom(conn: &Connection, parent_id: Option<&str>) -> Result<i64, TaskError> {
    let max: Option<i64> = match parent_id {
        None => conn.query_row(
            "SELECT MAX(sort_order) FROM tasks WHERE deleted_at IS NULL AND parent_id IS NULL",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )?,
        Some(pid) => conn.query_row(
            "SELECT MAX(sort_order) FROM tasks WHERE deleted_at IS NULL AND parent_id = ?1",
            [pid],
            |row| row.get::<_, Option<i64>>(0),
        )?,
    };
    Ok(max.unwrap_or(0).saturating_add(1))
}

/// Moves a task to another parent (`None` makes it a root task) — the backend
/// of drag & drop between levels. Enforces every subtask rule: one level only,
/// no cycles, no completed parent, and subtasks follow the parent's tab.
pub fn reparent_task(
    conn: &mut Connection,
    id: &str,
    new_parent_id: Option<&str>,
    place_at_top: bool,
) -> Result<Task, TaskError> {
    let tx = conn.transaction()?;
    let task = get_task_row(&tx, id, false)?;

    let parent_tab = match new_parent_id {
        None => None,
        Some(pid) => {
            if pid == id || is_descendant_of(&tx, pid, id)? {
                return Err(TaskError::WouldCreateCycle);
            }
            let parent = get_task_row(&tx, pid, false)
                .map_err(|_| TaskError::ParentNotFound(pid.to_string()))?;
            // A completed parent can adopt subtasks — it is then "partially
            // done" rather than plain completed.
            // Only one subtask level: the new parent must itself be a root task.
            if !parent.is_root() {
                return Err(TaskError::ThirdLevelForbidden);
            }
            parent.tab_id.clone()
        }
    };

    let target_tab = ensure_task_tab(&tx, parent_tab.or_else(|| task.tab_id.clone()))?;
    let target_tab = Some(target_tab);
    let sort_order = if place_at_top {
        next_sort_order(&tx, new_parent_id, target_tab.as_deref())?
    } else {
        next_sort_order_at_bottom(&tx, new_parent_id)?
    };

    let now = now_rfc3339();
    tx.execute(
        "UPDATE tasks SET parent_id = ?1, tab_id = ?2, sort_order = ?3, updated_at = ?4 WHERE id = ?5",
        rusqlite::params![new_parent_id, target_tab, sort_order, now, id],
    )?;
    // Descendants follow the tab of their root.
    tx.execute(
        "UPDATE tasks SET tab_id = ?1, updated_at = ?2 WHERE parent_id = ?3",
        rusqlite::params![target_tab, now, id],
    )?;
    tx.commit()?;
    get_task_row(conn, id, false)
}

/// Moves a task to another tab (drag & drop onto a tab). A parent moves its
/// whole subtask tree along with it.
///
/// A subtask cannot leave its parent behind: `list_roots` filters by `tab_id`
/// while subtasks are resolved through their parent, so a subtask with a
/// foreign `tab_id` would show up in the wrong tab and leak into search
/// results. Dropping a subtask on a tab therefore moves its parent, keeping
/// the "a tree lives in exactly one tab" invariant.
pub fn move_task_to_tab(conn: &mut Connection, id: &str, tab_id: &str) -> Result<Task, TaskError> {
    let tx = conn.transaction()?;
    let task = get_task_row(&tx, id, false)?;
    let exists: Option<String> = tx
        .query_row("SELECT id FROM tabs WHERE id = ?1", [tab_id], |row| {
            row.get(0)
        })
        .optional()?;
    if exists.is_none() {
        return Err(TaskError::TabNotFound(tab_id.to_string()));
    }

    // Resolve the tree root: a subtask travels with its parent.
    let target_root_id = match task.parent_id.as_deref() {
        Some(parent_id) => parent_id.to_string(),
        None => id.to_string(),
    };

    let now = now_rfc3339();
    let next = next_sort_order(&tx, None, Some(tab_id))?;
    tx.execute(
        "UPDATE tasks SET tab_id = ?1, sort_order = ?2, updated_at = ?3 WHERE id = ?4",
        rusqlite::params![tab_id, next, now, target_root_id],
    )?;
    tx.execute(
        "UPDATE tasks SET tab_id = ?1, updated_at = ?2 WHERE parent_id = ?3",
        rusqlite::params![tab_id, now, target_root_id],
    )?;
    tx.commit()?;
    get_task_row(conn, id, false)
}

/// Assigns a new explicit order to the given tasks (drag & drop reordering).
/// `ordered_ids` are siblings in the desired top-to-bottom order.
pub fn reorder_tasks(conn: &mut Connection, ordered_ids: &[String]) -> Result<(), TaskError> {
    let tx = conn.transaction()?;
    let now = now_rfc3339();
    for (index, id) in ordered_ids.iter().enumerate() {
        let changed = tx.execute(
            "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
            rusqlite::params![(index as i64) * 10, now, id],
        )?;
        if changed == 0 {
            return Err(TaskError::NotFound(id.clone()));
        }
    }
    tx.commit()?;
    Ok(())
}

/// Assigns a new explicit order to the given tabs (drag & drop on the strip).
pub fn reorder_tabs(conn: &mut Connection, ordered_ids: &[String]) -> Result<(), TaskError> {
    let tx = conn.transaction()?;
    let now = now_rfc3339();
    for (index, id) in ordered_ids.iter().enumerate() {
        let changed = tx.execute(
            "UPDATE tabs SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![(index as i64) * 10, now, id],
        )?;
        if changed == 0 {
            return Err(TaskError::NotFound(id.clone()));
        }
    }
    tx.commit()?;
    Ok(())
}

/// Soft-deleted root tasks (newest first), for the restore section in settings.
pub fn list_deleted_roots(conn: &Connection, limit: i64) -> Result<Vec<Task>, TaskError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks WHERE deleted_at IS NOT NULL AND parent_id IS NULL ORDER BY deleted_at DESC LIMIT ?1"
    ))?;
    let tasks = stmt
        .query_map([limit], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

/// Builds a LIKE pattern with escaped wildcards.
fn like_pattern(query: &str) -> String {
    let mut out = String::with_capacity(query.len() + 2);
    out.push('%');
    for c in query.chars() {
        if matches!(c, '%' | '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('%');
    out
}

/// Local search over titles. Soft-deleted and completed tasks are excluded by
/// default. Subtask matches are returned with their parent's title so the UI
/// can show context and expand the parent.
///
/// Both sides of the comparison are folded in Rust: the stored `search_text`
/// column and the pattern built here. Folding only the stored side would trade
/// one bug for another, since SQLite compares non-ASCII byte for byte.
pub fn search_tasks(
    conn: &Connection,
    query: &str,
    tab_id: Option<&str>,
) -> Result<SearchResult, TaskError> {
    let pattern = like_pattern(&crate::db::migrations::fold(query));

    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks
         WHERE parent_id IS NULL AND deleted_at IS NULL
            AND completed_at IS NULL AND not_relevant_at IS NULL
           AND (?2 IS NULL OR tab_id = ?2)
           AND search_text LIKE ?1 ESCAPE '\\'
         ORDER BY sort_order ASC"
    ))?;
    let roots = stmt
        .query_map(rusqlite::params![pattern, tab_id], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT t.id AS t_id, t.parent_id AS t_parent_id, t.tab_id AS t_tab_id, t.title AS t_title,
                    t.created_at AS t_created_at, t.updated_at AS t_updated_at,
                    t.completed_at AS t_completed_at, t.deleted_at AS t_deleted_at,
                    t.due_date AS t_due_date, t.comment AS t_comment, t.sort_order AS t_sort_order,
                    t.completion_event_id AS t_completion_event_id,
                    t.auto_completed_by AS t_auto_completed_by,
                    t.not_relevant_at AS t_not_relevant_at,
                    t.not_relevant_event_id AS t_not_relevant_event_id,
                    t.auto_not_relevant_by AS t_auto_not_relevant_by,
                    p.id AS p_id, p.parent_id AS p_parent_id, p.title AS p_title,
                    p.created_at AS p_created_at, p.updated_at AS p_updated_at,
                    p.completed_at AS p_completed_at, p.deleted_at AS p_deleted_at,
                    p.due_date AS p_due_date, p.comment AS p_comment, p.sort_order AS p_sort_order,
                    p.completion_event_id AS p_completion_event_id,
                    p.auto_completed_by AS p_auto_completed_by,
                    p.not_relevant_at AS p_not_relevant_at,
                    p.not_relevant_event_id AS p_not_relevant_event_id,
                    p.auto_not_relevant_by AS p_auto_not_relevant_by
             FROM tasks t
             JOIN tasks p ON p.id = t.parent_id
             WHERE t.parent_id IS NOT NULL AND t.deleted_at IS NULL
               AND t.completed_at IS NULL AND t.not_relevant_at IS NULL
               AND (?2 IS NULL OR t.tab_id = ?2)
               AND t.search_text LIKE ?1 ESCAPE '\\'
             ORDER BY t.sort_order ASC",
    )?;
    let matched_subtasks = stmt
        .query_map(rusqlite::params![pattern, tab_id], |row| {
            let task = Task {
                id: row.get("t_id")?,
                parent_id: row.get("t_parent_id")?,
                tab_id: row.get("t_tab_id")?,
                title: row.get("t_title")?,
                created_at: row.get("t_created_at")?,
                updated_at: row.get("t_updated_at")?,
                completed_at: row.get("t_completed_at")?,
                deleted_at: row.get("t_deleted_at")?,
                due_date: row.get("t_due_date")?,
                comment: row.get("t_comment")?,
                sort_order: row.get("t_sort_order")?,
                completion_event_id: row.get("t_completion_event_id")?,
                auto_completed_by: row.get("t_auto_completed_by")?,
                not_relevant_at: row.get("t_not_relevant_at")?,
                not_relevant_event_id: row.get("t_not_relevant_event_id")?,
                auto_not_relevant_by: row.get("t_auto_not_relevant_by")?,
            };
            let parent = Task {
                id: row.get("p_id")?,
                parent_id: row.get("p_parent_id")?,
                tab_id: row.get("t_tab_id")?,
                title: row.get("p_title")?,
                created_at: row.get("p_created_at")?,
                updated_at: row.get("p_updated_at")?,
                completed_at: row.get("p_completed_at")?,
                deleted_at: row.get("p_deleted_at")?,
                due_date: row.get("p_due_date")?,
                comment: row.get("p_comment")?,
                sort_order: row.get("p_sort_order")?,
                completion_event_id: row.get("p_completion_event_id")?,
                auto_completed_by: row.get("p_auto_completed_by")?,
                not_relevant_at: row.get("p_not_relevant_at")?,
                not_relevant_event_id: row.get("p_not_relevant_event_id")?,
                auto_not_relevant_by: row.get("p_auto_not_relevant_by")?,
            };
            Ok(MatchedSubtask { task, parent })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(SearchResult {
        roots,
        matched_subtasks,
    })
}

fn row_to_tab(row: &Row) -> rusqlite::Result<Tab> {
    Ok(Tab {
        id: row.get("id")?,
        name: row.get("name")?,
        color: row.get("color")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        deleted_at: row.get("deleted_at")?,
    })
}

/// Creates the first tab on first launch and attaches tasks that existed
/// before tabs (migration v2 leaves their `tab_id` NULL) to it, so no task
/// ever becomes invisible.
pub fn ensure_default_tab(conn: &Connection) -> Result<Tab, TaskError> {
    // Only live tabs count. A deleted one is still a row, and picking it as the
    // default would put new tasks on a tab the strip does not show.
    let existing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tabs WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    let tab = if existing > 0 {
        let mut stmt = conn.prepare(
            "SELECT * FROM tabs WHERE deleted_at IS NULL ORDER BY sort_order ASC LIMIT 1",
        )?;
        stmt.query_row([], row_to_tab)?
    } else {
        create_tab(conn, DEFAULT_TAB_NAME)?
    };

    conn.execute(
        "UPDATE tasks
         SET tab_id = ?1
         WHERE tab_id IS NULL
           AND parent_id IS NOT NULL
           AND parent_id IN (SELECT id FROM tasks WHERE tab_id = ?1)",
        [&tab.id],
    )?;
    conn.execute(
        "UPDATE tasks SET tab_id = ?1 WHERE tab_id IS NULL",
        [&tab.id],
    )?;
    Ok(tab)
}

pub fn list_tabs(conn: &Connection) -> Result<Vec<Tab>, TaskError> {
    // A deleted tab keeps its row so its tasks stay validly referenced while
    // they wait in the trash, which means it has to be filtered out here.
    let mut stmt =
        conn.prepare("SELECT * FROM tabs WHERE deleted_at IS NULL ORDER BY sort_order ASC")?;
    let tabs = stmt
        .query_map([], row_to_tab)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tabs)
}

/// Creates a tab. An empty or over-long name is rejected, exactly like
/// `rename_tab`, so the UI can show a consistent error instead of silently
/// getting a tab called "Tasks".
pub fn create_tab(conn: &Connection, name: &str) -> Result<Tab, TaskError> {
    let name = validation::normalize_title(name)
        .filter(|n| n.chars().count() <= validation::MAX_TAB_NAME_CHARS)
        .ok_or(TaskError::EmptyTitle)?;

    let used: Vec<String> = {
        let mut stmt = conn.prepare("SELECT color FROM tabs")?;
        let colors = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        colors
    };
    // Pick the first palette colour that is not in use, so tabs stay distinct.
    let color = TAB_COLORS
        .iter()
        .find(|c| !used.iter().any(|u| u.eq_ignore_ascii_case(c)))
        .copied()
        .unwrap_or(TAB_COLORS[used.len() % TAB_COLORS.len()])
        .to_string();

    let max_order: Option<i64> =
        conn.query_row("SELECT MAX(sort_order) FROM tabs", [], |row| row.get(0))?;
    let now = now_rfc3339();
    let tab = Tab {
        id: Uuid::new_v4().to_string(),
        name,
        color,
        sort_order: max_order.unwrap_or(-1) + 1,
        created_at: now.clone(),
        updated_at: now,
        deleted_at: None,
    };
    conn.execute(
        "INSERT INTO tabs (id, name, color, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            tab.id,
            tab.name,
            tab.color,
            tab.sort_order,
            tab.created_at,
            tab.updated_at
        ],
    )?;
    Ok(tab)
}

pub fn rename_tab(conn: &mut Connection, id: &str, name: &str) -> Result<Tab, TaskError> {
    let name = validation::normalize_title(name).ok_or(TaskError::EmptyTitle)?;
    let name: String = name.chars().take(validation::MAX_TAB_NAME_CHARS).collect();
    let tx = conn.transaction()?;
    let exists: Option<String> = tx
        .query_row("SELECT id FROM tabs WHERE id = ?1", [id], |row| row.get(0))
        .optional()?;
    if exists.is_none() {
        return Err(TaskError::NotFound(id.to_string()));
    }
    let now = now_rfc3339();
    tx.execute(
        "UPDATE tabs SET name = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![name, now, id],
    )?;
    tx.commit()?;
    get_tab(conn, id)
}

/// Every tab, deleted ones included.
///
/// Only the export needs this. Filtering the deleted tabs out of a snapshot
/// would orphan the tasks that still point at them, and on import there would
/// be no way to tell that apart from a task that simply arrived without a tab.
pub fn list_tabs_including_deleted(conn: &Connection) -> Result<Vec<Tab>, TaskError> {
    let mut stmt = conn.prepare("SELECT * FROM tabs ORDER BY sort_order ASC")?;
    let tabs = stmt
        .query_map([], row_to_tab)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tabs)
}

pub fn get_tab(conn: &Connection, id: &str) -> Result<Tab, TaskError> {
    conn.query_row("SELECT * FROM tabs WHERE id = ?1", [id], row_to_tab)
        .optional()?
        .ok_or_else(|| TaskError::NotFound(id.to_string()))
}

/// Hides a tab and remembers its tasks, without touching the tasks yet.
///
/// The undo window is ten seconds long, and it is only honest if the tasks are
/// still whole while it runs. Marking them deleted right away meant that closing
/// the app inside the window left a tab nobody could undo and a trash nobody had
/// emptied — the decision had been made for the user by a timer they never saw.
/// So the tab goes first and alone; `finalize_tab_deletion` puts the tasks into
/// the trash once the window has passed.
///
/// The row is kept rather than erased: the tasks still point at it, which is
/// what lets the trash group them and the undo put them back where they were.
pub fn delete_tab(conn: &mut Connection, id: &str) -> Result<TabDeletion, TaskError> {
    let tx = conn.transaction()?;

    let tab: Tab = tx
        .query_row(
            "SELECT * FROM tabs WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            row_to_tab,
        )
        .optional()?
        .ok_or_else(|| TaskError::NotFound(id.to_string()))?;

    // Only the tabs on screen count: the ones already deleted are not a reason
    // to refuse deleting the last one still standing.
    let remaining: i64 = tx.query_row(
        "SELECT COUNT(*) FROM tabs WHERE deleted_at IS NULL AND id <> ?1",
        [id],
        |row| row.get(0),
    )?;
    if remaining == 0 {
        return Err(TaskError::LastTab);
    }

    let task_ids: Vec<String> = {
        let mut stmt = tx.prepare(
            "SELECT id FROM tasks WHERE tab_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map([id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let now = now_rfc3339();
    tx.execute(
        "UPDATE tabs SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, id],
    )?;

    let deleted = TabDeletion {
        tab: Tab {
            deleted_at: Some(now.clone()),
            updated_at: now,
            ..tab
        },
        task_ids,
    };
    tx.commit()?;
    Ok(deleted)
}

/// Completes a tab deletion whose undo window has passed: the tasks the tab was
/// holding go to the trash together, all stamped at the same moment.
///
/// One timestamp for the whole tree is deliberate. If the tasks were stamped as
/// they went, a tree deleted one row at a time would land in the trash with a
/// spread of dates, and every one of them would start its own trash-retention
/// term on a different day.
pub fn finalize_tab_deletion(
    conn: &mut Connection,
    tab_id: &str,
    limit: i64,
) -> Result<TrashSweep, TaskError> {
    let mut conn_tx = conn.transaction()?;
    let tx = &mut conn_tx;

    let task_ids: Vec<String> = {
        let mut stmt = tx.prepare(
            "SELECT id FROM tasks WHERE tab_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC",
        )?;
        let rows = stmt.query_map([tab_id], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let now = now_rfc3339();
    for id in &task_ids {
        tx.execute(
            "UPDATE tasks SET deleted_at = ?1, updated_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            rusqlite::params![now, id],
        )?;
    }
    // Read back with `include_deleted`: the rows were marked a statement ago, and
    // the caller wants the tasks as they now are, not as they were.
    let moved: Vec<Task> = task_ids
        .iter()
        .map(|id| get_task_row(tx, id, true))
        .collect::<Result<Vec<_>, _>>()?;
    conn_tx.commit()?;
    // A whole tab arriving at once is the fastest way past the bound, so this
    // path enforces it for the same reason as a deletion by hand.
    let evicted = trim_trash(conn, limit)?;
    Ok(TrashSweep { moved, evicted })
}

/// Finishes tab deletions whose undo window was cut short by the app closing.
///
/// Without this a tab hidden ten seconds before quitting would stay hidden for
/// ever with its tasks still in place — a tab nobody can see and nobody asked to
/// keep. Called once at startup, before the first render.
pub fn sweep_pending_tab_deletions(
    conn: &mut Connection,
    limit: i64,
) -> Result<TrashSweep, TaskError> {
    let pending: Vec<String> = {
        let mut stmt = conn.prepare("SELECT id FROM tabs WHERE deleted_at IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    if pending.is_empty() {
        return Ok(TrashSweep::empty());
    }

    let mut moved: Vec<Task> = Vec::new();
    let mut evicted: Vec<Task> = Vec::new();
    for tab_id in pending {
        let sweep = finalize_tab_deletion(conn, &tab_id, limit)?;
        moved.extend(sweep.moved);
        evicted.extend(sweep.evicted);
    }
    Ok(TrashSweep { moved, evicted })
}

/// Takes back a tab deletion: the tab reappears with its tasks untouched.
///
/// While the undo window runs, `delete_tab` marked nothing, so there is nothing
/// to unmark either. Clearing the tasks anyway costs one statement and keeps the
/// undo correct if a caller finalized early.
pub fn restore_deleted_tab(
    conn: &mut Connection,
    tab_id: &str,
    task_ids: &[String],
) -> Result<TabDeletion, TaskError> {
    let tx = conn.transaction()?;
    // Read before the update so a tab id that does not exist fails loudly
    // instead of silently doing nothing.
    tx.query_row("SELECT id FROM tabs WHERE id = ?1", [tab_id], |row| {
        row.get::<_, String>(0)
    })
    .optional()?
    .ok_or_else(|| TaskError::NotFound(tab_id.to_string()))?;

    let now = now_rfc3339();
    // No task is touched here, and that is the whole point of the split: while
    // the undo window runs, `delete_tab` marked nothing, so nothing needs
    // unmarking. A task the user deleted by hand inside the window stays in the
    // trash — the undo must not dig out what it did not put there.
    let _ = task_ids;

    tx.execute(
        "UPDATE tabs SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, tab_id],
    )?;

    let tab = tx.query_row("SELECT * FROM tabs WHERE id = ?1", [tab_id], row_to_tab)?;
    tx.commit()?;
    Ok(TabDeletion {
        tab,
        task_ids: task_ids.to_vec(),
    })
}

/// All non-deleted tasks (including completed) for JSON export.
/// Every task, deleted ones included: an export has to reproduce the state
/// exactly as it is, otherwise the trash could not come back with it.
pub fn export_tasks(conn: &Connection) -> Result<Vec<Task>, TaskError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {TASK_COLUMNS} FROM tasks ORDER BY tab_id ASC, sort_order ASC"
    ))?;
    let tasks = stmt
        .query_map([], row_to_task)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tasks)
}

/// Accepts an imported colour only if it is one of the palette hex values we
/// can actually render; anything else falls back to the first palette entry.
fn sanitize_tab_color(raw: &str) -> String {
    let trimmed = raw.trim();
    let known = TAB_COLORS.iter().any(|c| c.eq_ignore_ascii_case(trimmed));
    if known && trimmed.starts_with('#') && trimmed.len() == 7 {
        trimmed.to_ascii_lowercase()
    } else {
        TAB_COLORS[0].to_string()
    }
}

/// Merges exported tabs into the database by id: a tab that is already there is
/// updated, a new one is inserted. Tabs are restored before the tasks that
/// reference them.
pub fn import_tabs(conn: &mut Connection, tabs: &[Tab]) -> Result<(usize, usize), TaskError> {
    if tabs.is_empty() {
        return Ok((0, 0));
    }
    let tx = conn.transaction()?;
    let (mut added, mut updated) = (0usize, 0usize);

    for tab in tabs {
        let name = validation::normalize_title(&tab.name)
            .filter(|n| n.chars().count() <= validation::MAX_TAB_NAME_CHARS)
            .unwrap_or_else(|| DEFAULT_TAB_NAME.to_string());
        let color = sanitize_tab_color(&tab.color);

        let exists: Option<String> = tx
            .query_row("SELECT id FROM tabs WHERE id = ?1", [&tab.id], |row| {
                row.get(0)
            })
            .optional()?
            .flatten();

        if exists.is_some() {
            tx.execute(
                "UPDATE tabs SET name = ?1, color = ?2, sort_order = ?3, updated_at = ?4,
                        deleted_at = ?5
                 WHERE id = ?6",
                rusqlite::params![
                    name,
                    color,
                    tab.sort_order,
                    now_rfc3339(),
                    tab.deleted_at,
                    tab.id
                ],
            )?;
            updated += 1;
        } else {
            tx.execute(
                "INSERT INTO tabs (id, name, color, sort_order, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    tab.id,
                    name,
                    color,
                    tab.sort_order,
                    tab.created_at,
                    now_rfc3339(),
                    tab.deleted_at
                ],
            )?;
            added += 1;
        }
    }
    tx.commit()?;
    Ok((added, updated))
}

/// Applies exported settings, so a restore really returns the application to
/// the state it was exported from.
pub fn import_settings(conn: &mut Connection, settings: &AppSettings) -> Result<(), TaskError> {
    update_settings(conn, settings).map(|_| ())
}

pub fn get_settings(conn: &Connection) -> Result<AppSettings, TaskError> {
    let theme: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'theme'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let show_completed: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'show_completed'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let cascade: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'cascade_complete'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let trash_limit: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'trash_limit'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let auto_delete_days: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'auto_delete_days'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let trash_retention_days: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'trash_retention_days'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let language: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'language'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let expand_subtasks: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'expand_subtasks'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let show_tab_counts: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'show_tab_counts'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let newest_first: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'newest_first'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(AppSettings {
        theme: theme.unwrap_or_else(|| "system".to_string()),
        show_completed: show_completed.as_deref() == Some("1"),
        // A missing key means the default: completing a parent cascades.
        cascade_complete: cascade.as_deref() != Some("0"),
        trash_limit: trash_limit
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(DEFAULT_TRASH_LIMIT)
            .clamp(TRASH_LIMIT_OFF, MAX_TRASH_LIMIT),
        auto_delete_days: auto_delete_days.and_then(|v| v.parse::<i64>().ok()),
        trash_retention_days: trash_retention_days.and_then(|v| v.parse::<i64>().ok()),
        language: match language.as_deref() {
            Some("en") => "en".to_string(),
            _ => "ru".to_string(),
        },
        // Subtasks are expanded by default.
        expand_subtasks: expand_subtasks.as_deref() != Some("0"),
        show_tab_counts: show_tab_counts.as_deref() != Some("0"),
        // New tasks land on top unless the user reverses it.
        newest_first: newest_first.as_deref() != Some("0"),
    })
}

pub fn update_settings(
    conn: &mut Connection,
    settings: &AppSettings,
) -> Result<AppSettings, TaskError> {
    let theme = AppSettings::validate_theme(&settings.theme).map_err(TaskError::Internal)?;
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('theme', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [theme],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('show_completed', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [flag(settings.show_completed)],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('cascade_complete', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [flag(settings.cascade_complete)],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('trash_limit', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [settings
            .trash_limit
            .clamp(TRASH_LIMIT_OFF, MAX_TRASH_LIMIT)
            .to_string()],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('auto_delete_days', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [settings
            .auto_delete_days
            .map(|d| d.to_string())
            .unwrap_or_default()],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('trash_retention_days', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [settings
            .trash_retention_days
            .map(|d| d.to_string())
            .unwrap_or_default()],
    )?;
    let language = match settings.language.as_str() {
        "en" => "en",
        _ => "ru",
    };
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('language', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [language],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('expand_subtasks', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [flag(settings.expand_subtasks)],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('show_tab_counts', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [flag(settings.show_tab_counts)],
    )?;
    tx.execute(
        "INSERT INTO settings (key, value) VALUES ('newest_first', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [flag(settings.newest_first)],
    )?;
    tx.commit()?;
    get_settings(conn)
}

/// Merges tasks from an export file.
///
/// Rows are matched by id: a task that already exists is updated, a new one is
/// inserted. Nothing is deleted, so importing an old backup can never lose
/// work that happened after it was taken. Tasks referring to tabs that no
/// longer exist are placed into the first tab instead of vanishing.
pub fn import_tasks(
    conn: &mut Connection,
    tasks: &[Task],
) -> Result<(usize, usize, usize), TaskError> {
    if tasks.is_empty() {
        return Ok((0, 0, 0));
    }
    let tx = conn.transaction()?;
    let fallback_tab = ensure_default_tab(&tx)?.id;
    let now = now_rfc3339();
    let (mut added, mut updated, mut skipped) = (0usize, 0usize, 0usize);

    // Parents first, so a subtask never arrives before its parent.
    let mut ordered: Vec<&Task> = tasks.iter().collect();
    ordered.sort_by_key(|t| t.parent_id.is_some());

    for task in ordered {
        // An imported title is untrusted input: it must obey the same limit as
        // a typed one, or a hostile file could put megabytes into a single row
        // and blow the layout apart.
        let title = validation::normalize_title(&task.title);
        let Some(title) = title.filter(|s| s.chars().count() <= validation::MAX_TITLE_CHARS) else {
            skipped += 1;
            continue;
        };

        // The same argument applies to the comment, and the limit was missing
        // here: it was enforced on the typed path but not on this one, so a
        // file could hand the UI a field as long as the whole upload limit.
        // A task whose comment is too long loses the comment rather than the
        // task — dropping the task would make an export look like data loss.
        // The file chooses the order, but not beyond what the arithmetic above
        // can step away from: a sort_order at the very edge of i64 would make
        // the next new task overflow.
        let sort_order = task.sort_order.clamp(-SORT_ORDER_LIMIT, SORT_ORDER_LIMIT);
        let comment = task.comment.as_deref().and_then(|raw| {
            let cleaned = raw.trim();
            (!cleaned.is_empty() && cleaned.chars().count() <= validation::MAX_COMMENT_CHARS)
                .then(|| cleaned.to_string())
        });
        let parent_ok = match &task.parent_id {
            // A task may not be its own parent. The database refuses to store
            // that, but refusing here is what keeps the import from dying on a
            // trigger abort halfway through a batch: a task whose parent is
            // missing is imported as a root instead.
            Some(pid) if pid == &task.id => false,
            Some(pid) => tx
                .query_row(
                    "SELECT 1 FROM tasks WHERE id = ?1 AND parent_id IS NULL",
                    [pid],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some(),
            None => true,
        };
        // A subtask whose parent is missing would be invisible, so it is
        // imported as a root task instead of being dropped.
        let parent_id = if parent_ok {
            task.parent_id.clone()
        } else {
            None
        };
        if !parent_ok {
            skipped += 1;
        }
        let tab_id = ensure_task_tab(
            &tx,
            task.tab_id
                .clone()
                .filter(|tab| {
                    tx.query_row("SELECT 1 FROM tabs WHERE id = ?1", [tab], |row| {
                        row.get::<_, i64>(0)
                    })
                    .optional()
                    .ok()
                    .flatten()
                    .is_some()
                })
                .or(Some(fallback_tab.clone())),
        )?;

        let exists: Option<String> = tx
            .query_row("SELECT id FROM tasks WHERE id = ?1", [&task.id], |row| {
                row.get(0)
            })
            .optional()?
            .flatten();

        if exists.is_some() {
            tx.execute(
                "UPDATE tasks SET parent_id = ?1, tab_id = ?2, title = ?3, search_text = ?4,
                        updated_at = ?5, completed_at = ?6, deleted_at = ?7, due_date = ?8,
                        comment = ?9, sort_order = ?10, completion_event_id = ?11,
                        auto_completed_by = ?12, not_relevant_at = ?13,
                        not_relevant_event_id = ?14, auto_not_relevant_by = ?15
                 WHERE id = ?16",
                rusqlite::params![
                    parent_id,
                    tab_id,
                    title,
                    crate::db::migrations::fold(&title),
                    now,
                    task.completed_at,
                    task.deleted_at,
                    task.due_date,
                    comment,
                    sort_order,
                    task.completion_event_id,
                    task.auto_completed_by,
                    task.not_relevant_at,
                    task.not_relevant_event_id,
                    task.auto_not_relevant_by,
                    task.id
                ],
            )?;
            updated += 1;
        } else {
            tx.execute(
                "INSERT INTO tasks (id, parent_id, tab_id, title, search_text, created_at,
                        updated_at, completed_at, deleted_at, due_date, comment, sort_order,
                        completion_event_id, auto_completed_by, not_relevant_at,
                        not_relevant_event_id, auto_not_relevant_by)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                rusqlite::params![
                    task.id,
                    parent_id,
                    tab_id,
                    title,
                    crate::db::migrations::fold(&title),
                    task.created_at,
                    now,
                    task.completed_at,
                    task.deleted_at,
                    task.due_date,
                    comment,
                    sort_order,
                    task.completion_event_id,
                    task.auto_completed_by,
                    task.not_relevant_at,
                    task.not_relevant_event_id,
                    task.auto_not_relevant_by
                ],
            )?;
            added += 1;
        }
    }
    tx.commit()?;
    Ok((added, updated, skipped))
}

/// Disk and row statistics of the database, for the settings screen.
pub fn db_stats(conn: &Connection) -> Result<DbStats, TaskError> {
    // `PRAGMA` always returns exactly one row, so no `.optional()` here.
    let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
    let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
    let freelist_count: i64 = conn.query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
    let task_count: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
    let deleted_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE deleted_at IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    // The label says "Tabs", and the number next to it is what the strip
    // shows. A tab waiting in the trash with its tasks is neither.
    let tab_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tabs WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;

    // WAL keeps recent writes in the `-wal` file until a checkpoint, so the
    // real disk footprint is the sum of all three files.
    let mut size_bytes = 0u64;
    if let Some(path) = conn.path() {
        for suffix in ["", "-wal", "-shm"] {
            let candidate = format!("{path}{suffix}");
            if let Ok(meta) = std::fs::metadata(&candidate) {
                size_bytes += meta.len();
            }
        }
    }

    Ok(DbStats {
        size_bytes,
        page_size: page_size.max(0) as u64,
        page_count: page_count.max(0) as u64,
        freelist_count: freelist_count.max(0) as u64,
        task_count,
        active_count: task_count - deleted_count,
        deleted_count,
        tab_count,
    })
}

/// Rewrites the database file, returning the free pages to the file system.
///
/// SQLite never shrinks the file on its own: deleted rows leave reusable
/// pages behind. That is the right default, and for a task list this only
/// becomes noticeable at a scale of roughly a million tasks — the button is
/// therefore offered with an explanation rather than as routine maintenance.
pub fn vacuum(conn: &Connection) -> Result<DbStats, TaskError> {
    conn.execute_batch("VACUUM").map_err(TaskError::from)?;
    db_stats(conn)
}

fn flag(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

/// Reverses the order of root tasks inside every tab.
///
/// Used when the "newest first" preference flips: existing tasks must follow
/// the same rule as new ones, so the list visibly turns over. Soft-deleted
/// roots are included and keep their place, so restoring one later does not
/// break the order. Subtasks are never touched.
pub fn reverse_root_order(conn: &mut Connection) -> Result<(), TaskError> {
    let tx = conn.transaction()?;
    let now = now_rfc3339();
    let tabs: Vec<String> = {
        let mut stmt = tx.prepare("SELECT id FROM tabs ORDER BY sort_order ASC")?;
        let list = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        list
    };
    for tab_id in tabs {
        let mut ids: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT id FROM tasks WHERE parent_id IS NULL AND tab_id = ?1 ORDER BY sort_order ASC",
            )?;
            let list = stmt
                .query_map([&tab_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            list
        };
        ids.reverse();
        for (index, id) in ids.iter().enumerate() {
            tx.execute(
                "UPDATE tasks SET sort_order = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![(index as i64) * 10, now, id],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Per-tab task counters shown on the tab bar.
///
/// A task with three subtasks counts as four: subtasks are tasks too. Deleted
/// rows are excluded, completed ones are **included** so the progress stays
/// visible regardless of the "show completed" filter.
pub fn tab_stats(conn: &Connection) -> Result<Vec<TabStat>, TaskError> {
    let mut stmt = conn.prepare(
        "SELECT t.id AS tab_id,
                COUNT(tasks.id) AS total,
                SUM(CASE WHEN tasks.completed_at IS NOT NULL OR tasks.not_relevant_at IS NOT NULL THEN 1 ELSE 0 END) AS done
         FROM tabs t
         LEFT JOIN tasks ON tasks.tab_id = t.id AND tasks.deleted_at IS NULL
         GROUP BY t.id
         ORDER BY t.sort_order ASC",
    )?;
    let stats = stmt
        .query_map([], |row| {
            let total: i64 = row.get(1)?;
            let done: Option<i64> = row.get(2)?;
            Ok(TabStat {
                tab_id: row.get(0)?,
                total,
                done: done.unwrap_or(0),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(stats)
}

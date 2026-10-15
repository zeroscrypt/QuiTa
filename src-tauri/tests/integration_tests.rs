//! Integration tests against a real (temporary) SQLite database, exercising
//! the same repository functions the Tauri commands use.

use quita_lib::db::migrations;
use quita_lib::db::repository;
use quita_lib::domain::rules::compute_toggle_plan;
use quita_lib::domain::task::{AppSettings, Task};
use quita_lib::domain::transfer;
use quita_lib::domain::validation;
use quita_lib::error::TaskError;
use rusqlite::Connection;

fn open_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    // Mirrors app startup: the first tab exists and adopts pre-tab tasks.
    repository::ensure_default_tab(&conn).unwrap();
    conn
}

fn active_task(conn: &Connection, id: &str) -> Task {
    conn.query_row(
        "SELECT id, parent_id, tab_id, title, created_at, updated_at, completed_at, deleted_at, due_date, comment, sort_order, completion_event_id, auto_completed_by
         FROM tasks WHERE id = ?1 AND deleted_at IS NULL",
        [id],
        |row| {
            Ok(Task {
                id: row.get("id")?,
                parent_id: row.get("parent_id")?,
                tab_id: row.get("tab_id")?,
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
            })
        },
    )
    .unwrap()
}

/// Reads a task straight from the table, deleted or not. `active_task` filters
/// the trash out by design, which is exactly what these assertions must look at.
fn deleted_at_of(conn: &Connection, id: &str) -> Option<String> {
    conn.query_row("SELECT deleted_at FROM tasks WHERE id = ?1", [id], |row| {
        row.get(0)
    })
    .unwrap()
}

fn subtask_titles(conn: &Connection, parent_id: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare("SELECT title FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY sort_order ASC")
        .unwrap();
    stmt.query_map([parent_id], |row| row.get(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

// ---------- Creation and nesting ----------

#[test]
fn create_task_and_subtask() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&parent.id), None, true).unwrap();

    assert!(parent.parent_id.is_none());
    assert_eq!(child.parent_id.as_deref(), Some(parent.id.as_str()));
    // Root tasks are added at the top, subtasks at the bottom.
    assert!(parent.sort_order < child.sort_order);

    let roots = repository::list_roots(&conn, true, None).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, parent.id);
    assert_eq!(subtask_titles(&conn, &parent.id), vec!["Child"]);
}

#[test]
fn new_subtasks_are_appended_under_their_parent() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let first = repository::create_task(&mut conn, "Первый", Some(&parent.id), None, true).unwrap();
    let second =
        repository::create_task(&mut conn, "Второй", Some(&parent.id), None, true).unwrap();
    let third = repository::create_task(&mut conn, "Третий", Some(&parent.id), None, true).unwrap();

    assert!(
        first.sort_order < second.sort_order && second.sort_order < third.sort_order,
        "each new subtask must land under the previous ones"
    );
    assert_eq!(
        subtask_titles(&conn, &parent.id),
        vec!["Первый", "Второй", "Третий"]
    );
}

#[test]
fn third_level_is_rejected_by_domain_logic() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Root", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&root.id), None, true).unwrap();

    let result = repository::create_task(&mut conn, "Grandchild", Some(&child.id), None, true);
    assert!(matches!(result, Err(TaskError::ThirdLevelForbidden)));
}

#[test]
fn empty_and_whitespace_titles_are_rejected() {
    let mut conn = open_test_db();
    assert!(matches!(
        repository::create_task(&mut conn, "   ", None, None, true),
        Err(TaskError::EmptyTitle)
    ));
    assert!(matches!(
        repository::create_task(&mut conn, "", None, None, true),
        Err(TaskError::EmptyTitle)
    ));
    assert_eq!(repository::list_roots(&conn, true, None).unwrap().len(), 0);
}

#[test]
fn titles_are_trimmed() {
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "  spaced  ", None, None, true).unwrap();
    assert_eq!(task.title, "spaced");
}

// ---------- Completion rules ----------

#[test]
fn completing_parent_completes_all_children_under_one_event() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    repository::create_task(&mut conn, "A", Some(&parent.id), None, true).unwrap();
    repository::create_task(&mut conn, "B", Some(&parent.id), None, true).unwrap();
    repository::create_task(&mut conn, "C", Some(&parent.id), None, true).unwrap();

    let outcome = repository::toggle_completion(&mut conn, &parent.id, true).unwrap();
    assert_eq!(outcome.updated.len(), 4);

    let parent_row = active_task(&conn, &parent.id);
    assert!(parent_row.is_completed());
    let event = parent_row.completion_event_id.clone().unwrap();

    for title in ["A", "B", "C"] {
        let child = conn
            .query_row("SELECT * FROM tasks WHERE title = ?1", [title], |row| {
                Ok((
                    row.get::<_, Option<String>>("completed_at")?,
                    row.get::<_, Option<String>>("auto_completed_by")?,
                ))
            })
            .unwrap();
        assert!(child.0.is_some(), "child {title} should be completed");
        assert_eq!(
            child.1.as_deref(),
            Some(event.as_str()),
            "child {title} should carry the parent event id"
        );
    }
}

#[test]
fn reopening_parent_reopens_only_children_completed_by_that_event() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let manual =
        repository::create_task(&mut conn, "Manual", Some(&parent.id), None, true).unwrap();
    repository::create_task(&mut conn, "Auto", Some(&parent.id), None, true).unwrap();

    // User completes "Manual" by hand first.
    repository::toggle_completion(&mut conn, &manual.id, true).unwrap();
    // Now complete the parent: "Manual" must stay completed.
    repository::toggle_completion(&mut conn, &parent.id, true).unwrap();
    // Reopen the parent: only the auto-completed child returns.
    repository::toggle_completion(&mut conn, &parent.id, true).unwrap();

    let manual_row = active_task(&conn, &manual.id);
    assert!(
        manual_row.is_completed(),
        "manually completed child stays completed"
    );
    assert!(manual_row.auto_completed_by.is_none());

    let children = subtask_titles(&conn, &parent.id);
    assert_eq!(children.len(), 2);
    let auto = conn
        .query_row(
            "SELECT completed_at FROM tasks WHERE title = 'Auto'",
            [],
            |row| row.get::<_, Option<String>>(0),
        )
        .unwrap();
    assert!(auto.is_none(), "auto-completed child is reopened");
}

#[test]
fn reopening_a_child_leaves_the_completed_parent_closed() {
    // All subtasks closed, then the parent closed (cascade closed them).
    // Taking one subtask back out must NOT reopen the parent: the result is
    // the "partially done" state, the same one the user sees when the parent
    // is closed manually while subtasks are still open.
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&parent.id), None, true).unwrap();
    let child2 =
        repository::create_task(&mut conn, "Child2", Some(&parent.id), None, true).unwrap();

    repository::toggle_completion(&mut conn, &parent.id, true).unwrap();
    assert!(active_task(&conn, &parent.id).is_completed());
    assert!(active_task(&conn, &child.id).is_completed());
    assert!(active_task(&conn, &child2.id).is_completed());

    repository::toggle_completion(&mut conn, &child.id, true).unwrap();

    assert!(!active_task(&conn, &child.id).is_completed());
    assert!(
        active_task(&conn, &parent.id).is_completed(),
        "unchecking a subtask must not reopen the parent the user closed"
    );
    assert!(active_task(&conn, &child2.id).is_completed());
}

#[test]
fn reopening_a_parent_after_a_child_was_taken_back_out_respects_that_child() {
    // The child detached from the cascade event, so reopening the parent must
    // not silently close it again.
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&parent.id), None, true).unwrap();
    let other = repository::create_task(&mut conn, "Other", Some(&parent.id), None, true).unwrap();

    repository::toggle_completion(&mut conn, &parent.id, true).unwrap();
    repository::toggle_completion(&mut conn, &child.id, true).unwrap();
    assert!(active_task(&conn, &child.id).auto_completed_by.is_none());

    repository::toggle_completion(&mut conn, &parent.id, true).unwrap();
    assert!(!active_task(&conn, &parent.id).is_completed());
    assert!(
        !active_task(&conn, &child.id).is_completed(),
        "the child the user opened must stay open"
    );
    assert!(
        !active_task(&conn, &other.id).is_completed(),
        "the child closed by the cascade is restored"
    );
}

#[test]
fn completing_child_keeps_parent_open_and_is_recorded_as_manual() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&parent.id), None, true).unwrap();

    repository::toggle_completion(&mut conn, &child.id, true).unwrap();

    let parent_row = active_task(&conn, &parent.id);
    assert!(!parent_row.is_completed(), "parent stays open");
    let child_row = active_task(&conn, &child.id);
    assert!(child_row.is_completed());
    // Manually completed children must never be attributed to a parent event,
    // otherwise reopening the parent would wrongly reopen them.
    assert!(child_row.auto_completed_by.is_none());
}

#[test]
fn completed_parent_can_receive_new_subtasks() {
    // "Поднять ВМ" is done, but a new step can still appear under it: the row
    // then reads as partially done instead of plain completed.
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    repository::toggle_completion(&mut conn, &parent.id, true).unwrap();

    let child =
        repository::create_task(&mut conn, "Late child", Some(&parent.id), None, true).unwrap();
    assert!(child.completed_at.is_none(), "the new subtask starts open");
    assert!(child.parent_id.is_some());

    // The parent stays completed: opening the child does not reopen it.
    assert!(active_task(&conn, &parent.id).completed_at.is_some());
    assert!(active_task(&conn, &child.id).completed_at.is_none());
}

// ---------- Delete / restore ----------

#[test]
fn soft_delete_cascades_atomically() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    repository::create_task(&mut conn, "A", Some(&parent.id), None, true).unwrap();
    repository::create_task(&mut conn, "B", Some(&parent.id), None, true).unwrap();
    let other = repository::create_task(&mut conn, "Other", None, None, true).unwrap();

    let deleted = repository::soft_delete_task(&mut conn, &parent.id).unwrap();
    assert_eq!(deleted.len(), 3);

    let roots = repository::list_roots(&conn, true, None).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, other.id);
    assert!(subtask_titles(&conn, &parent.id).is_empty());

    // Restoring the parent brings back the whole tree that went with it.
    let restored = repository::restore_task(&mut conn, &parent.id).unwrap();
    assert!(!restored.is_empty());
    assert_eq!(restored[0].title, "Parent");
    assert!(restored.iter().all(|t| t.deleted_at.is_none()));
}

#[test]
fn restoring_a_parent_brings_back_children_deleted_before_it() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Gone", Some(&parent.id), None, true).unwrap();

    // Delete the subtask first (independently), then the parent.
    repository::soft_delete_task(&mut conn, &child.id).unwrap();
    repository::soft_delete_task(&mut conn, &parent.id).unwrap();

    // Everything that is in the trash under this parent comes back with it —
    // otherwise those rows would sit in the trash forever, invisible.
    let restored = repository::restore_task(&mut conn, &parent.id).unwrap();
    assert_eq!(restored.len(), 2, "the parent and the child");
    let children = repository::list_subtasks(&conn, &parent.id).unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, child.id);
    assert_eq!(children[0].title, "Gone");
    assert!(repository::list_deleted_all(&conn, 200).unwrap().is_empty());
}

#[test]
fn restore_of_subtask_with_deleted_parent_promotes_it() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&parent.id), None, true).unwrap();

    repository::soft_delete_task(&mut conn, &parent.id).unwrap();
    // The parent is still in the trash, so the subtask cannot go back under it
    // and is restored as a root task instead of staying unreachable.
    let restored = repository::restore_task(&mut conn, &child.id).unwrap();
    assert_eq!(restored.len(), 1);
    assert!(restored[0].parent_id.is_none());
    assert!(restored[0].deleted_at.is_none());
    assert_eq!(repository::list_roots(&conn, true, None).unwrap().len(), 1);
}

// ---------- Due dates ----------

#[test]
fn due_date_roundtrips_as_calendar_date_without_tz_shift() {
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "Task", None, None, true).unwrap();

    // 2026-12-31 stored as a calendar date must be read back exactly,
    // regardless of the machine's timezone.
    let updated = repository::set_due_date(&mut conn, &task.id, Some("2026-12-31")).unwrap();
    assert_eq!(updated.due_date.as_deref(), Some("2026-12-31"));

    let stored: String = conn
        .query_row(
            "SELECT due_date FROM tasks WHERE id = ?1",
            [&task.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, "2026-12-31");

    let cleared = repository::set_due_date(&mut conn, &task.id, None).unwrap();
    assert!(cleared.due_date.is_none());
}

#[test]
fn changing_parent_due_date_does_not_touch_children() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&parent.id), None, true).unwrap();

    repository::set_due_date(&mut conn, &parent.id, Some("2026-11-15")).unwrap();
    let child_row = active_task(&conn, &child.id);
    assert!(child_row.due_date.is_none());
}

#[test]
fn invalid_due_dates_are_rejected() {
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "Task", None, None, true).unwrap();
    for bad in [
        "2025-02-30",
        "31-12-2026",
        "2026-1-1",
        "завтра",
        "2026-12-31T00:00:00Z",
    ] {
        assert!(
            matches!(
                repository::set_due_date(&mut conn, &task.id, Some(bad)),
                Err(TaskError::InvalidDueDate(_))
            ),
            "expected rejection of {bad}"
        );
    }
}

// ---------- Sorting ----------

#[test]
fn new_roots_go_to_the_top() {
    let mut conn = open_test_db();
    repository::create_task(&mut conn, "First", None, None, true).unwrap();
    repository::create_task(&mut conn, "Second", None, None, true).unwrap();
    repository::create_task(&mut conn, "Third", None, None, true).unwrap();

    let titles: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, vec!["Third", "Second", "First"]);
}

#[test]
fn order_survives_connection_restart() {
    let dir = std::env::temp_dir().join(format!("quita-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let db_path = dir.join("quita.db");
    {
        let mut conn = Connection::open(&db_path).unwrap();
        migrations::run(&conn).unwrap();
        repository::create_task(&mut conn, "First", None, None, true).unwrap();
        repository::create_task(&mut conn, "Second", None, None, true).unwrap();
    }
    {
        let conn = Connection::open(&db_path).unwrap();
        let titles: Vec<String> = repository::list_roots(&conn, true, None)
            .unwrap()
            .into_iter()
            .map(|t| t.title)
            .collect();
        assert_eq!(titles, vec!["Second", "First"]);
    }
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn move_task_swaps_sort_order_with_neighbor() {
    let mut conn = open_test_db();
    repository::create_task(&mut conn, "A", None, None, true).unwrap();
    repository::create_task(&mut conn, "B", None, None, true).unwrap();
    repository::create_task(&mut conn, "C", None, None, true).unwrap();
    // Order top->bottom: C, B, A (newest first).
    let c = repository::list_roots(&conn, true, None).unwrap()[0]
        .id
        .clone();

    repository::move_task(&mut conn, &c, false).unwrap(); // C goes down
    let titles: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, vec!["B", "C", "A"]);

    // Moving up at the very top is a no-op.
    let b = repository::list_roots(&conn, true, None).unwrap()[0]
        .id
        .clone();
    repository::move_task(&mut conn, &b, true).unwrap();
    let titles: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert_eq!(titles, vec!["B", "C", "A"]);
}

// ---------- Search ----------

#[test]
fn search_excludes_deleted_and_completed_tasks_by_default() {
    let mut conn = open_test_db();
    let visible = repository::create_task(&mut conn, "Купить молоко", None, None, true).unwrap();
    let completed = repository::create_task(&mut conn, "Купить хлеб", None, None, true).unwrap();
    repository::toggle_completion(&mut conn, &completed.id, true).unwrap();
    let deleted = repository::create_task(&mut conn, "Купить масло", None, None, true).unwrap();
    repository::soft_delete_task(&mut conn, &deleted.id).unwrap();

    let result = repository::search_tasks(&conn, "Купить", None).unwrap();
    assert_eq!(result.roots.len(), 1);
    assert_eq!(result.roots[0].id, visible.id);

    // Empty query returns nothing (UI treats it as "no search").
    let empty = repository::search_tasks(&conn, "   ", None).unwrap();
    assert!(empty.roots.is_empty() && empty.matched_subtasks.is_empty());
}

#[test]
fn search_finds_cyrillic_whatever_the_case() {
    // SQLite folds case for ASCII only. Without the search_text column a
    // Cyrillic title was findable only by its exact spelling, which in a
    // Russian interface means typing the title over again.
    let mut conn = open_test_db();
    repository::create_task(&mut conn, "Купить Молоко", None, None, true).unwrap();

    for query in ["молоко", "МОЛОКО", "Молоко", "МоЛоКо"] {
        let found = repository::search_tasks(&conn, query, None).unwrap();
        assert_eq!(found.roots.len(), 1, "запрос {query} ничего не нашёл");
    }
}

#[test]
fn search_finds_subtasks_in_any_case_too() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Продукты", None, None, true).unwrap();
    repository::create_task(&mut conn, "Одна кофе", Some(&parent.id), None, true).unwrap();

    let found = repository::search_tasks(&conn, "КОФЕ", None).unwrap();
    assert_eq!(found.matched_subtasks.len(), 1);
    assert_eq!(found.matched_subtasks[0].parent.title, "Продукты");
}

#[test]
fn renaming_a_task_moves_it_to_the_new_folded_text() {
    // A stale search_text would keep the task findable under its old
    // title, which reads as the rename not having happened.
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "Старое", None, None, true).unwrap();
    repository::rename_task(&mut conn, &task.id, "Новое Имя").unwrap();

    assert_eq!(
        repository::search_tasks(&conn, "новое имя", None)
            .unwrap()
            .roots
            .len(),
        1
    );
    assert_eq!(
        repository::search_tasks(&conn, "старое", None)
            .unwrap()
            .roots
            .len(),
        0,
        "старое название продолжает находиться"
    );
}

#[test]
fn tasks_written_before_the_search_column_are_still_findable() {
    // A database created by an older build has no folded copy of the title.
    // The backfill is what makes those rows searchable in any case.
    let mut conn = open_test_db();
    repository::create_task(&mut conn, "Старая задача", None, None, true).unwrap();
    // Wipe the column the way a pre-migration row would look.
    conn.execute("UPDATE tasks SET search_text = ''", [])
        .unwrap();
    migrations::run(&conn).unwrap();

    assert_eq!(
        repository::search_tasks(&conn, "СТАРАЯ", None)
            .unwrap()
            .roots
            .len(),
        1
    );
}

#[test]
fn search_matches_subtasks_with_parent_context() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Проект", None, None, true).unwrap();
    repository::create_task(&mut conn, "Сверстать макет", Some(&parent.id), None, true).unwrap();
    repository::create_task(&mut conn, "Купить продукты", None, None, true).unwrap();

    let result = repository::search_tasks(&conn, "макет", None).unwrap();
    assert!(result.roots.is_empty());
    assert_eq!(result.matched_subtasks.len(), 1);
    assert_eq!(result.matched_subtasks[0].task.title, "Сверстать макет");
    assert_eq!(result.matched_subtasks[0].parent.title, "Проект");
}

#[test]
fn search_escapes_like_wildcards() {
    let mut conn = open_test_db();
    repository::create_task(&mut conn, "50% done", None, None, true).unwrap();
    repository::create_task(&mut conn, "500 dollars", None, None, true).unwrap();

    let result = repository::search_tasks(&conn, "50%", None).unwrap();
    assert_eq!(result.roots.len(), 1);
    assert_eq!(result.roots[0].title, "50% done");

    let result = repository::search_tasks(&conn, "%", None).unwrap();
    assert_eq!(result.roots.len(), 1);
}

// ---------- Export ----------

#[test]
fn export_contains_all_fields_and_preserves_links() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let child = repository::create_task(&mut conn, "Child", Some(&parent.id), None, true).unwrap();
    repository::set_due_date(&mut conn, &parent.id, Some("2026-10-13")).unwrap();

    let exported = repository::export_tasks(&conn).unwrap();
    assert_eq!(exported.len(), 2);
    let parent_row = exported.iter().find(|t| t.id == parent.id).unwrap();
    assert_eq!(parent_row.due_date.as_deref(), Some("2026-10-13"));
    assert_eq!(parent_row.deleted_at, None);
    let child_row = exported.iter().find(|t| t.id == child.id).unwrap();
    assert_eq!(child_row.parent_id.as_deref(), Some(parent.id.as_str()));
}

#[test]
fn the_export_keeps_a_deleted_tab_and_its_tasks() {
    // A tab deletion keeps the row so its tasks keep a valid reference. An
    // export that left the deleted tab out would carry task ids that no tab
    // in the file could hold, and the import would file them under the
    // first tab instead — losing which tab they came from.
    let mut conn = open_test_db();
    let _home = repository::ensure_default_tab(&conn).unwrap();
    let work = repository::create_tab(&conn, "Работа").unwrap();
    let kept = repository::create_task(&mut conn, "Остаётся", None, Some(&work.id), true).unwrap();
    let gone = repository::create_task(&mut conn, "Уходит", None, Some(&work.id), true).unwrap();
    repository::delete_tab(&mut conn, &work.id).unwrap();

    let exported = repository::list_tabs_including_deleted(&conn).unwrap();
    assert!(
        exported
            .iter()
            .any(|t| t.id == work.id && t.deleted_at.is_some()),
        "the deleted tab must be in the snapshot, trash included"
    );
    assert!(exported
        .iter()
        .any(|t| t.id == kept.tab_id.clone().unwrap() || t.id == _home.id));

    let tasks = repository::export_tasks(&conn).unwrap();
    assert!(tasks
        .iter()
        .any(|t| t.id == gone.id && t.deleted_at.is_some()));

    // Re-importing into an empty database must reproduce the same picture,
    // not pile the trashed tasks onto the first tab.
    let mut other = open_test_db();
    repository::import_tabs(&mut other, &exported).unwrap();
    repository::import_tasks(&mut other, &tasks).unwrap();
    let tabs_after = repository::list_tabs_including_deleted(&other).unwrap();
    assert!(
        tabs_after
            .iter()
            .any(|t| t.id == work.id && t.deleted_at.is_some()),
        "the tab came back already deleted, so its tasks stay in the trash"
    );
    let owner: String = other
        .query_row("SELECT tab_id FROM tasks WHERE id = ?1", [&gone.id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(owner, work.id, "the task was filed under the wrong tab");
}

// ---------- Validation unit mirrors ----------

#[test]
fn cascade_disabled_completes_only_the_parent() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let a = repository::create_task(&mut conn, "A", Some(&parent.id), None, true).unwrap();
    let b = repository::create_task(&mut conn, "B", Some(&parent.id), None, true).unwrap();

    // Cascade off: only the parent changes.
    repository::toggle_completion(&mut conn, &parent.id, false).unwrap();

    let parent_row = active_task(&conn, &parent.id);
    assert!(parent_row.is_completed());
    assert!(parent_row.completion_event_id.is_none());
    assert!(!active_task(&conn, &a.id).is_completed());
    assert!(!active_task(&conn, &b.id).is_completed());

    // Reopening restores nothing (subtasks were never touched).
    repository::toggle_completion(&mut conn, &parent.id, false).unwrap();
    assert!(!active_task(&conn, &parent.id).is_completed());
    assert!(!active_task(&conn, &a.id).is_completed());
}

#[test]
fn cascade_setting_round_trips_through_settings() {
    let mut conn = open_test_db();
    assert!(repository::get_settings(&conn).unwrap().cascade_complete);

    let settings = repository::get_settings(&conn).unwrap();
    let updated = repository::update_settings(
        &mut conn,
        &AppSettings {
            cascade_complete: false,
            ..settings
        },
    )
    .unwrap();
    assert!(!updated.cascade_complete);
    assert!(!repository::get_settings(&conn).unwrap().cascade_complete);
}

#[test]
fn purge_removes_completed_tasks_older_than_retention_with_children() {
    let mut conn = open_test_db();
    let old = repository::create_task(&mut conn, "Старая", None, None, true).unwrap();
    let old_child =
        repository::create_task(&mut conn, "Её подзадача", Some(&old.id), None, true).unwrap();
    let fresh = repository::create_task(&mut conn, "Свежая", None, None, true).unwrap();
    let open = repository::create_task(&mut conn, "Открытая", None, None, true).unwrap();

    repository::toggle_completion(&mut conn, &old.id, true).unwrap();
    repository::toggle_completion(&mut conn, &fresh.id, true).unwrap();

    // Age both completed tasks by different amounts.
    conn.execute(
        "UPDATE tasks SET completed_at = datetime('now', '-40 days') WHERE id = ?1",
        [&old.id],
    )
    .unwrap();
    conn.execute(
        "UPDATE tasks SET completed_at = datetime('now', '-2 days') WHERE id = ?1",
        [&fresh.id],
    )
    .unwrap();

    let purged = repository::purge_completed_tasks(&mut conn, 30).unwrap();
    assert_eq!(purged.len(), 2, "the old task and its subtask are purged");

    let remaining: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .into_iter()
        .map(|t| t.title)
        .collect();
    assert!(remaining.contains(&"Свежая".to_string()));
    assert!(remaining.contains(&"Открытая".to_string()));
    assert!(!remaining.contains(&"Старая".to_string()));
    assert!(repository::list_subtasks(&conn, &old.id)
        .unwrap()
        .is_empty());

    // A non-positive retention disables the cleanup entirely.
    assert!(repository::purge_completed_tasks(&mut conn, 0)
        .unwrap()
        .is_empty());
    let _ = (open, old_child);
}

#[test]
fn tasks_created_before_tabs_are_adopted_by_the_first_tab() {
    // Simulate a database upgraded from migration v1: tasks with NULL tab_id.
    let conn = Connection::open_in_memory().unwrap();
    migrations::run(&conn).unwrap();
    conn.execute(
        "INSERT INTO tasks (id, parent_id, title, created_at, updated_at, sort_order)
         VALUES ('legacy', NULL, 'Старая задача', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', -1)",
        [],
    )
    .unwrap();

    let tab = repository::ensure_default_tab(&conn).unwrap();
    assert_eq!(tab.name, "Новая вкладка");

    let tasks = repository::list_task_nodes(&conn, true, Some(&tab.id)).unwrap();
    assert_eq!(
        tasks.len(),
        1,
        "legacy task must be visible in the first tab"
    );
    assert_eq!(tasks[0].task.title, "Старая задача");

    // Running again must not duplicate or detach anything.
    repository::ensure_default_tab(&conn).unwrap();
    assert_eq!(repository::list_tabs(&conn).unwrap().len(), 1);
    assert_eq!(
        repository::list_task_nodes(&conn, true, Some(&tab.id))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn tabs_split_tasks_and_get_distinct_colors() {
    let mut conn = open_test_db();
    let first = repository::ensure_default_tab(&conn).unwrap();
    assert_eq!(first.name, "Новая вкладка");

    let second = repository::create_tab(&conn, "Работа").unwrap();
    let third = repository::create_tab(&conn, "Дом").unwrap();

    let tabs = repository::list_tabs(&conn).unwrap();
    assert_eq!(tabs.len(), 3);
    let mut colors: Vec<&str> = tabs.iter().map(|t| t.color.as_str()).collect();
    colors.sort_unstable();
    let before = colors.len();
    colors.dedup();
    assert_eq!(colors.len(), before, "tabs must have distinct colours");

    let work_task =
        repository::create_task(&mut conn, "Отчёт", None, Some(&second.id), true).unwrap();
    let home_task =
        repository::create_task(&mut conn, "Продукты", None, Some(&third.id), true).unwrap();
    repository::create_task(&mut conn, "Старая", None, Some(&first.id), true).unwrap();

    // Listing is scoped to the tab.
    let work = repository::list_task_nodes(&conn, false, Some(&second.id)).unwrap();
    assert_eq!(work.len(), 1);
    assert_eq!(work[0].task.id, work_task.id);
    let home = repository::list_task_nodes(&conn, false, Some(&third.id)).unwrap();
    assert_eq!(home.len(), 1);
    assert_eq!(home[0].task.id, home_task.id);

    // Search is scoped too.
    let search = repository::search_tasks(&conn, "Отчёт", Some(&second.id)).unwrap();
    assert_eq!(search.roots.len(), 1);
    let search_other = repository::search_tasks(&conn, "Отчёт", Some(&first.id)).unwrap();
    assert!(search_other.roots.is_empty());

    // Subtasks inherit the parent's tab.
    let child = repository::create_task(&mut conn, "Под", Some(&work_task.id), None, true).unwrap();
    assert_eq!(child.tab_id.as_deref(), Some(second.id.as_str()));
}

#[test]
fn deleting_a_tab_takes_its_tasks_to_the_trash_and_keeps_the_last_one() {
    let mut conn = open_test_db();
    let first = repository::ensure_default_tab(&conn).unwrap();
    let second = repository::create_tab(&conn, "Работа").unwrap();
    let task = repository::create_task(&mut conn, "Важная", None, Some(&second.id), true).unwrap();

    let deleted = repository::delete_tab(&mut conn, &second.id).unwrap();
    assert_eq!(
        deleted.task_ids,
        vec![task.id.clone()],
        "the undo needs the list"
    );

    let remaining = repository::list_tabs(&conn).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, first.id);

    // The task goes to the trash and does not appear under the tab on the left.
    // It used to be moved there, which read as the notes quietly leaking across.
    assert!(
        deleted_at_of(&conn, &task.id).is_some(),
        "the task did not reach the trash"
    );
    assert!(
        repository::list_task_nodes(&conn, false, Some(&first.id))
            .unwrap()
            .is_empty(),
        "the neighbouring tab stayed empty"
    );
    assert_eq!(
        repository::list_deleted_all(&conn, 50).unwrap().len(),
        1,
        "the task is recoverable from the trash"
    );

    // The last tab cannot be removed.
    assert!(matches!(
        repository::delete_tab(&mut conn, &first.id),
        Err(TaskError::LastTab)
    ));
}

#[test]
fn undoing_a_tab_deletion_brings_the_tab_and_its_tasks_back() {
    let mut conn = open_test_db();
    let _first = repository::ensure_default_tab(&conn).unwrap();
    let work = repository::create_tab(&conn, "Работа").unwrap();
    let keep = repository::create_task(&mut conn, "Остаётся", None, Some(&work.id), true).unwrap();
    let gone = repository::create_task(&mut conn, "Уходит", None, Some(&work.id), true).unwrap();
    // A task that was already in the trash must stay there after the undo.
    let earlier = repository::create_task(&mut conn, "Старая", None, Some(&work.id), true).unwrap();
    repository::soft_delete_task(&mut conn, &earlier.id).unwrap();

    let deleted = repository::delete_tab(&mut conn, &work.id).unwrap();
    let restored = repository::restore_deleted_tab(&mut conn, &work.id, &deleted.task_ids).unwrap();

    assert_eq!(restored.tab.id, work.id);
    assert!(restored.tab.deleted_at.is_none());
    assert!(
        repository::list_tabs(&conn)
            .unwrap()
            .iter()
            .any(|t| t.id == work.id),
        "the tab is back in the strip"
    );
    assert!(deleted_at_of(&conn, &keep.id).is_none());
    assert!(deleted_at_of(&conn, &gone.id).is_none());
    assert_eq!(
        repository::list_task_nodes(&conn, false, Some(&work.id))
            .unwrap()
            .len(),
        2
    );
    assert!(
        deleted_at_of(&conn, &earlier.id).is_some(),
        "the undo must not dig out of the trash what it did not put there"
    );
}

#[test]
fn the_last_visible_tab_survives_deleted_ones() {
    // A tab deleted earlier is still a row, but it must not make the final
    // live tab undeletable.
    let mut conn = open_test_db();
    let first = repository::ensure_default_tab(&conn).unwrap();
    let second = repository::create_tab(&conn, "Втор��я").unwrap();
    repository::delete_tab(&mut conn, &second.id).unwrap();

    assert!(matches!(
        repository::delete_tab(&mut conn, &first.id),
        Err(TaskError::LastTab)
    ));
    assert_eq!(repository::list_tabs(&conn).unwrap().len(), 1);
}

#[test]
fn deleting_an_empty_tab_is_allowed_and_undoable() {
    let mut conn = open_test_db();
    let _first = repository::ensure_default_tab(&conn).unwrap();
    let empty = repository::create_tab(&conn, "Пустая").unwrap();

    let deleted = repository::delete_tab(&mut conn, &empty.id).unwrap();
    assert!(deleted.task_ids.is_empty());

    let restored =
        repository::restore_deleted_tab(&mut conn, &empty.id, &deleted.task_ids).unwrap();
    assert_eq!(restored.tab.name, "Пустая");
    assert!(restored.tab.deleted_at.is_none());
}

#[test]
fn restoring_a_task_from_a_deleted_tab_brings_the_tab_back() {
    // The tab row is hidden but alive, so a task restored on its own would
    // land in a tab that no list shows. Restoring has to bring the tab too.
    let mut conn = open_test_db();
    let _first = repository::ensure_default_tab(&conn).unwrap();
    let work = repository::create_tab(&conn, "Работа").unwrap();
    let task = repository::create_task(&mut conn, "Задача", None, Some(&work.id), true).unwrap();

    repository::delete_tab(&mut conn, &work.id).unwrap();
    assert_eq!(repository::list_tabs(&conn).unwrap().len(), 1);

    repository::restore_task(&mut conn, &task.id).unwrap();
    assert!(
        repository::list_tabs(&conn)
            .unwrap()
            .iter()
            .any(|t| t.id == work.id),
        "the tab stayed hidden, so the restored task is invisible"
    );
    assert_eq!(
        repository::list_task_nodes(&conn, false, Some(&work.id))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn emptying_the_trash_drops_tabs_nothing_points_at() {
    // A deleted tab keeps its row only for as long as something refers to it.
    let mut conn = open_test_db();
    let _first = repository::ensure_default_tab(&conn).unwrap();
    let work = repository::create_tab(&conn, "Работа").unwrap();
    repository::create_task(&mut conn, "Задача", None, Some(&work.id), true).unwrap();

    repository::delete_tab(&mut conn, &work.id).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM tabs WHERE id = ?1", [&work.id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        rows, 1,
        "the row survives while the task waits in the trash"
    );

    repository::purge_all_deleted(&mut conn).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM tabs WHERE id = ?1", [&work.id], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(rows, 0, "nothing refers to it any more");
}

#[test]
fn renaming_a_tab_trims_and_rejects_empty_names() {
    let mut conn = open_test_db();
    let tab = repository::ensure_default_tab(&conn).unwrap();
    let renamed = repository::rename_tab(&mut conn, &tab.id, "  Покупки  ").unwrap();
    assert_eq!(renamed.name, "Покупки");
    assert!(matches!(
        repository::rename_tab(&mut conn, &tab.id, "   "),
        Err(TaskError::EmptyTitle)
    ));
}

#[test]
fn purge_task_removes_the_task_and_every_subtask() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Parent", None, None, true).unwrap();
    let a = repository::create_task(&mut conn, "A", Some(&parent.id), None, true).unwrap();
    let b = repository::create_task(&mut conn, "B", Some(&parent.id), None, true).unwrap();

    // Delete a subtask first, then the parent: both become soft-deleted.
    repository::soft_delete_task(&mut conn, &a.id).unwrap();
    repository::soft_delete_task(&mut conn, &parent.id).unwrap();

    let removed = repository::purge_task(&mut conn, &parent.id).unwrap();
    assert_eq!(removed.len(), 3, "parent plus both subtasks");

    let left: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tasks WHERE id IN (?1, ?2, ?3)",
            rusqlite::params![parent.id, a.id, b.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(left, 0, "nothing may survive a permanent delete");
}

#[test]
fn purge_task_reports_missing_ids() {
    let mut conn = open_test_db();
    assert!(matches!(
        repository::purge_task(&mut conn, "does-not-exist"),
        Err(TaskError::NotFound(_))
    ));
}

#[test]
fn reparent_moves_tasks_between_levels_and_tabs() {
    let mut conn = open_test_db();
    let tab_a = repository::ensure_default_tab(&conn).unwrap();
    let tab_b = repository::create_tab(&conn, "Архив").unwrap();

    let root = repository::create_task(&mut conn, "Корень", None, Some(&tab_a.id), true).unwrap();
    let sub = repository::create_task(&mut conn, "Под", Some(&root.id), None, true).unwrap();
    let free =
        repository::create_task(&mut conn, "Свободная", None, Some(&tab_a.id), true).unwrap();

    // Root -> child of another root (at the top of its subtasks).
    let moved = repository::reparent_task(&mut conn, &free.id, Some(&root.id), true).unwrap();
    assert_eq!(moved.parent_id.as_deref(), Some(root.id.as_str()));
    let children = repository::list_subtasks(&conn, &root.id).unwrap();
    assert_eq!(children.first().unwrap().id, free.id, "placed at the top");

    // Child -> root again.
    let promoted = repository::reparent_task(&mut conn, &free.id, None, true).unwrap();
    assert!(promoted.parent_id.is_none());
    assert_eq!(promoted.tab_id.as_deref(), Some(tab_a.id.as_str()));

    // Move the whole tree to another tab.
    repository::move_task_to_tab(&mut conn, &root.id, &tab_b.id).unwrap();
    assert_eq!(
        active_task(&conn, &root.id).tab_id.as_deref(),
        Some(tab_b.id.as_str())
    );
    assert_eq!(
        active_task(&conn, &sub.id).tab_id.as_deref(),
        Some(tab_b.id.as_str()),
        "subtasks follow their parent"
    );
}

#[test]
fn reparent_rejects_cycles_and_third_level() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Root", None, None, true).unwrap();
    let sub = repository::create_task(&mut conn, "Sub", Some(&root.id), None, true).unwrap();
    let sub2 = repository::create_task(&mut conn, "Sub2", Some(&root.id), None, true).unwrap();

    // A task cannot become its own subtask...
    assert!(matches!(
        repository::reparent_task(&mut conn, &root.id, Some(&root.id), true),
        Err(TaskError::WouldCreateCycle)
    ));
    // ...nor a subtask of its own child.
    assert!(matches!(
        repository::reparent_task(&mut conn, &root.id, Some(&sub.id), true),
        Err(TaskError::WouldCreateCycle)
    ));
    // A subtask cannot gain subtasks of its own (would be level three).
    assert!(matches!(
        repository::reparent_task(&mut conn, &sub2.id, Some(&sub.id), true),
        Err(TaskError::ThirdLevelForbidden)
    ));

    // A completed parent still adopts subtasks (partially-done state), and
    // only the level rule applies.
    repository::toggle_completion(&mut conn, &root.id, false).unwrap();
    let free = repository::create_task(&mut conn, "Свободная", None, None, true).unwrap();
    let adopted = repository::reparent_task(&mut conn, &free.id, Some(&root.id), true).unwrap();
    assert_eq!(adopted.parent_id.as_deref(), Some(root.id.as_str()));
    assert!(
        active_task(&conn, &root.id).completed_at.is_some(),
        "adopting must not reopen the parent"
    );
}

#[test]
fn move_task_to_tab_rejects_unknown_tab() {
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "Task", None, None, true).unwrap();
    assert!(matches!(
        repository::move_task_to_tab(&mut conn, &task.id, "nope"),
        Err(TaskError::TabNotFound(_))
    ));
}

#[test]
fn reorder_tabs_persists_the_new_order() {
    let mut conn = open_test_db();
    // The first tab is created automatically on startup.
    let first = repository::list_tabs(&conn).unwrap()[0].clone();
    let a = repository::create_tab(&conn, "A").unwrap();
    let b = repository::create_tab(&conn, "B").unwrap();

    // Every tab must be part of the submitted order.
    let ids = vec![b.id.clone(), first.id.clone(), a.id.clone()];
    repository::reorder_tabs(&mut conn, &ids).unwrap();

    let order: Vec<String> = repository::list_tabs(&conn)
        .unwrap()
        .into_iter()
        .map(|tab| tab.name)
        .collect();
    assert_eq!(order, vec!["B", "Новая вкладка", "A"]);

    // Unknown ids are rejected instead of silently reordering.
    assert!(matches!(
        repository::reorder_tabs(&mut conn, &["nope".to_string()]),
        Err(TaskError::NotFound(_))
    ));
}

#[test]
fn validation_helpers_match_their_specs() {
    assert_eq!(validation::normalize_title("  x  "), Some("x".to_string()));
    assert_eq!(validation::normalize_title(" "), None);
    assert!(validation::is_valid_due_date("2026-10-13"));
    assert!(!validation::is_valid_due_date("2025-02-29"));
}

#[test]
fn toggle_plan_children_are_filtered_correctly() {
    let make = |id: &str, completed: bool| Task {
        id: id.to_string(),
        parent_id: None,
        tab_id: None,
        title: id.to_string(),
        created_at: String::new(),
        updated_at: String::new(),
        completed_at: if completed {
            Some("2026-01-01T00:00:00Z".to_string())
        } else {
            None
        },
        deleted_at: None,
        due_date: None,
        comment: None,
        sort_order: 0,
        completion_event_id: None,
        auto_completed_by: None,
    };
    let parent = make("p", false);
    let open_child = make("c1", false);
    let done_child = make("c2", true);

    let plan = compute_toggle_plan(&parent, &[open_child, done_child], true);
    assert_eq!(plan.children_to_complete, vec!["c1".to_string()]);
    assert!(plan.completion_event_id.is_some());
}

// ---------- Invariants guarded after the security review ----------

#[test]
fn error_messages_serialize_as_strings() {
    // The UI renders whatever a rejected command returns. An externally
    // tagged enum would show up as `[object Object]`.
    assert_eq!(
        serde_json::to_string(&TaskError::LastTab).unwrap(),
        "\"cannot delete the last tab\""
    );
    assert!(serde_json::to_string(&TaskError::NotFound("x".into()))
        .unwrap()
        .starts_with('"'));
}

#[test]
fn moving_a_subtask_to_another_tab_moves_its_parent() {
    let mut conn = open_test_db();
    let tab_a = repository::ensure_default_tab(&conn).unwrap();
    let tab_b = repository::create_tab(&conn, "Архив").unwrap();

    let root = repository::create_task(&mut conn, "Корень", None, Some(&tab_a.id), true).unwrap();
    let sub = repository::create_task(&mut conn, "Под", Some(&root.id), None, true).unwrap();

    repository::move_task_to_tab(&mut conn, &sub.id, &tab_b.id).unwrap();

    // A tree never spans tabs, so the parent travels with the subtask.
    let moved_root = conn
        .query_row("SELECT tab_id FROM tasks WHERE id = ?1", [&root.id], |r| {
            r.get::<_, Option<String>>(0)
        })
        .unwrap();
    let moved_sub = conn
        .query_row("SELECT tab_id FROM tasks WHERE id = ?1", [&sub.id], |r| {
            r.get::<_, Option<String>>(0)
        })
        .unwrap();
    assert_eq!(moved_root.as_deref(), Some(tab_b.id.as_str()));
    assert_eq!(moved_sub.as_deref(), Some(tab_b.id.as_str()));
}

#[test]
fn purge_refuses_a_task_that_is_not_deleted() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Живая", None, None, true).unwrap();
    assert!(matches!(
        repository::purge_task(&mut conn, &root.id),
        Err(TaskError::NotDeleted(_))
    ));
    // Still there, still visible.
    assert!(!active_task(&conn, &root.id).title.is_empty());
    assert_eq!(repository::list_roots(&conn, true, None).unwrap().len(), 1);
}

#[test]
fn a_deleted_tabs_tasks_keep_pointing_at_it() {
    // The tab row survives a deletion so the foreign key holds and the undo
    // can put the tasks back into the same tab. Reassigning them to another
    // tab is exactly the leak this behaviour replaced.
    let mut conn = open_test_db();
    let tab_a = repository::ensure_default_tab(&conn).unwrap();
    let tab_b = repository::create_tab(&conn, "Вторая").unwrap();
    let task = repository::create_task(&mut conn, "Задача", None, Some(&tab_a.id), true).unwrap();

    repository::delete_tab(&mut conn, &tab_a.id).unwrap();

    let owner = conn
        .query_row("SELECT tab_id FROM tasks WHERE id = ?1", [&task.id], |r| {
            r.get::<_, Option<String>>(0)
        })
        .unwrap();
    assert_eq!(owner.as_deref(), Some(tab_a.id.as_str()));
    assert_ne!(owner.as_deref(), Some(tab_b.id.as_str()));
}

#[test]
fn a_task_never_ends_up_without_a_tab() {
    let mut conn = open_test_db();
    let sub = repository::create_task(&mut conn, "Сирота", None, None, true).unwrap();
    assert!(
        sub.tab_id.is_some(),
        "a task without a tab would be filtered out of every list"
    );
    assert_eq!(repository::list_roots(&conn, true, None).unwrap().len(), 1);
}

#[test]
fn empty_tab_name_is_rejected_instead_of_silently_defaulted() {
    let conn = open_test_db();
    assert!(matches!(
        repository::create_tab(&conn, "   "),
        Err(TaskError::EmptyTitle)
    ));
}

#[test]
fn count_subtasks_includes_completed_and_hidden_children() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Корень", None, None, true).unwrap();
    repository::create_task(&mut conn, "Открытая", Some(&root.id), None, true).unwrap();
    let done = repository::create_task(&mut conn, "Закрытая", Some(&root.id), None, true).unwrap();
    repository::toggle_completion(&mut conn, &done.id, false).unwrap();

    // The list hides completed rows when the preference is off, but the
    // delete confirmation must still quote the real number.
    assert_eq!(repository::count_subtasks(&conn, &root.id).unwrap(), 2);
    assert_eq!(subtask_titles(&conn, &root.id).len(), 2);

    repository::soft_delete_task(&mut conn, &done.id).unwrap();
    assert_eq!(repository::count_subtasks(&conn, &root.id).unwrap(), 1);
}

#[test]
fn overlong_titles_are_rejected_at_the_boundary() {
    let mut conn = open_test_db();
    let long = "x".repeat(validation::MAX_TITLE_CHARS + 1);
    assert!(matches!(
        repository::create_task(&mut conn, &long, None, None, true),
        Err(TaskError::EmptyTitle)
    ));
    let ok = "x".repeat(validation::MAX_TITLE_CHARS);
    assert!(repository::create_task(&mut conn, &ok, None, None, true).is_ok());
}

#[test]
fn comment_round_trips_and_blank_is_cleared() {
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "С комментарием", None, None, true).unwrap();
    assert!(task.comment.is_none());

    let saved =
        repository::set_comment(&mut conn, &task.id, Some("  Позвонить в 15:00  ")).unwrap();
    assert_eq!(saved.comment.as_deref(), Some("Позвонить в 15:00"));

    // Whitespace-only text is the same as no comment at all.
    let cleared = repository::set_comment(&mut conn, &task.id, Some("   ")).unwrap();
    assert!(cleared.comment.is_none());

    // The value survives a reload and an export round trip.
    repository::set_comment(&mut conn, &task.id, Some("живой текст")).unwrap();
    assert_eq!(
        active_task(&conn, &task.id).comment.as_deref(),
        Some("живой текст")
    );
    let exported = repository::export_tasks(&conn).unwrap();
    assert_eq!(
        exported
            .iter()
            .find(|t| t.id == task.id)
            .and_then(|t| t.comment.as_deref()),
        Some("живой текст")
    );
}

#[test]
fn overlong_comment_is_rejected() {
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "Задача", None, None, true).unwrap();
    let huge = "x".repeat(validation::MAX_COMMENT_CHARS + 1);
    assert!(matches!(
        repository::set_comment(&mut conn, &task.id, Some(&huge)),
        Err(TaskError::CommentTooLong(_))
    ));
    assert!(repository::set_comment(&mut conn, &task.id, Some(&huge)).is_err());
    assert!(active_task(&conn, &task.id).comment.is_none());
}

#[test]
fn deleted_tasks_keep_their_tab_for_grouping() {
    let mut conn = open_test_db();
    let tab_a = repository::ensure_default_tab(&conn).unwrap();
    let tab_b = repository::create_tab(&conn, "Архив").unwrap();
    let from_a =
        repository::create_task(&mut conn, "Из первой", None, Some(&tab_a.id), true).unwrap();
    let from_b =
        repository::create_task(&mut conn, "Из второй", None, Some(&tab_b.id), true).unwrap();

    repository::soft_delete_task(&mut conn, &from_a.id).unwrap();
    repository::soft_delete_task(&mut conn, &from_b.id).unwrap();

    let deleted = repository::list_deleted_roots(&conn, 50).unwrap();
    assert_eq!(deleted.len(), 2);
    for task in &deleted {
        assert!(
            task.tab_id.is_some(),
            "the recycle bin groups deleted tasks by tab, so tab_id must survive"
        );
    }
}

#[test]
fn db_stats_reports_rows_and_pages() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Живая", None, None, true).unwrap();
    repository::soft_delete_task(&mut conn, &root.id).unwrap();

    let stats = repository::db_stats(&conn).unwrap();
    assert_eq!(stats.task_count, 1);
    assert_eq!(stats.deleted_count, 1);
    assert_eq!(stats.active_count, 0);
    assert_eq!(stats.tab_count, 1);
    assert!(stats.page_size >= 512, "SQLite uses 512..65536 byte pages");
    assert!(stats.page_count >= 1);
    // An in-memory database has no file, so the reported size is zero.
    assert_eq!(stats.size_bytes, 0);
}

#[test]
fn vacuum_returns_free_pages_and_reports_new_stats() {
    let mut conn = open_test_db();
    let mut ids = Vec::new();
    for i in 0..200 {
        ids.push(
            repository::create_task(&mut conn, &format!("Задача {i}"), None, None, true)
                .unwrap()
                .id,
        );
    }
    for id in &ids {
        // Purge only accepts soft-deleted tasks, so the task is deleted first.
        repository::soft_delete_task(&mut conn, id).unwrap();
        repository::purge_task(&mut conn, id).unwrap();
    }

    let before = repository::db_stats(&conn).unwrap();
    assert_eq!(before.task_count, 0);
    assert!(
        before.freelist_count > 0,
        "deleted rows must leave reusable pages behind"
    );

    let after = repository::vacuum(&conn).unwrap();
    assert_eq!(
        after.freelist_count, 0,
        "VACUUM returns the free pages to the file system"
    );
    assert!(after.page_count <= before.page_count);
}

#[test]
fn purge_all_also_removes_children_of_deleted_parents() {
    // Reproduces the state found in a real database: a subtask that is still
    // active under an already deleted parent. Purging must not fail on the
    // foreign key, and the orphan must go with its parent.
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Родитель", None, None, true).unwrap();
    let child =
        repository::create_task(&mut conn, "Подзадача", Some(&parent.id), None, true).unwrap();
    repository::soft_delete_task(&mut conn, &parent.id).unwrap();

    // Reproduce the inconsistent row on purpose.
    conn.execute(
        "UPDATE tasks SET deleted_at = NULL WHERE id = ?1",
        [&child.id],
    )
    .unwrap();

    let removed = repository::purge_all_deleted(&mut conn).unwrap();
    assert_eq!(removed, 2, "both the parent and its orphan child are gone");
    assert_eq!(repository::count_subtasks(&conn, &parent.id).unwrap(), 0);
}

#[test]
fn tab_stats_counts_the_parent_as_a_task() {
    let mut conn = open_test_db();
    let tab_a = repository::ensure_default_tab(&conn).unwrap();
    let tab_b = repository::create_tab(&conn, "Второй").unwrap();

    let root = repository::create_task(&mut conn, "Корень", None, Some(&tab_a.id), true).unwrap();
    let s1 = repository::create_task(&mut conn, "Под 1", Some(&root.id), None, true).unwrap();
    let s2 = repository::create_task(&mut conn, "Под 2", Some(&root.id), None, true).unwrap();
    let s3 = repository::create_task(&mut conn, "Под 3", Some(&root.id), None, true).unwrap();
    repository::create_task(&mut conn, "Чужая", None, Some(&tab_b.id), true).unwrap();

    let stats = repository::tab_stats(&conn).unwrap();
    let a = stats.iter().find(|s| s.tab_id == tab_a.id).unwrap();
    // The root counts as one task: 1 root + 3 subtasks = 4.
    assert_eq!(a.total, 4);
    assert_eq!(a.done, 0);

    repository::toggle_completion(&mut conn, &root.id, false).unwrap();
    repository::toggle_completion(&mut conn, &s1.id, false).unwrap();
    let stats = repository::tab_stats(&conn).unwrap();
    let a = stats.iter().find(|s| s.tab_id == tab_a.id).unwrap();
    assert_eq!(
        (a.done, a.total),
        (2, 4),
        "2/4 — the parent is part of the count"
    );

    // Completed-but-hidden tasks stay in the total; deleted ones leave it.
    let s4 = repository::create_task(&mut conn, "Под 4", Some(&root.id), None, true).unwrap();
    repository::soft_delete_task(&mut conn, &s4.id).unwrap();
    let stats = repository::tab_stats(&conn).unwrap();
    let a = stats.iter().find(|s| s.tab_id == tab_a.id).unwrap();
    assert_eq!(
        (a.done, a.total),
        (2, 4),
        "a soft-deleted task is not counted"
    );

    let _ = s2;
    let _ = s3;
    let b = stats.iter().find(|s| s.tab_id == tab_b.id).unwrap();
    assert_eq!((b.done, b.total), (0, 1));
}

#[test]
fn new_root_task_lands_on_top_or_bottom() {
    let mut conn = open_test_db();
    let first = repository::create_task(&mut conn, "Первая", None, None, true).unwrap();
    let second = repository::create_task(&mut conn, "Вторая", None, None, true).unwrap();
    let third = repository::create_task(&mut conn, "Третья", None, None, true).unwrap();
    // "Newest first": every new root gets a smaller sort_order.
    assert!(third.sort_order < second.sort_order);
    assert!(second.sort_order < first.sort_order);
    let titles: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .iter()
        .map(|t| t.title.clone())
        .collect();
    assert_eq!(titles, vec!["Третья", "Вторая", "Первая"]);

    // Reversed preference: new roots are appended.
    let bottom = repository::create_task(&mut conn, "Внизу", None, None, false).unwrap();
    assert!(bottom.sort_order > third.sort_order);
    let titles: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .iter()
        .map(|t| t.title.clone())
        .collect();
    assert_eq!(titles.last().unwrap(), "Внизу");
}

#[test]
fn reversing_the_order_turns_the_list_over() {
    let mut conn = open_test_db();
    for name in ["Первая", "Вторая", "Третья"] {
        repository::create_task(&mut conn, name, None, None, true).unwrap();
    }
    // A subtask must stay exactly where it is: only roots are reversed.
    let parent = repository::list_roots(&conn, true, None).unwrap()[0].clone();
    let child = repository::create_task(&mut conn, "Под", Some(&parent.id), None, true).unwrap();

    let titles = |conn: &Connection| -> Vec<String> {
        repository::list_roots(conn, true, None)
            .unwrap()
            .iter()
            .map(|t| t.title.clone())
            .collect()
    };
    assert_eq!(titles(&conn), vec!["Третья", "Вторая", "Первая"]);

    repository::reverse_root_order(&mut conn).unwrap();
    assert_eq!(titles(&conn), vec!["Первая", "Вторая", "Третья"]);

    let children = repository::list_subtasks(&conn, &parent.id).unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, child.id);

    // Toggling back returns to the original order.
    repository::reverse_root_order(&mut conn).unwrap();
    assert_eq!(titles(&conn), vec!["Третья", "Вторая", "Первая"]);
}

#[test]
fn reversing_keeps_every_tab_independent() {
    let mut conn = open_test_db();
    let tab_a = repository::ensure_default_tab(&conn).unwrap();
    let tab_b = repository::create_tab(&conn, "Второй").unwrap();
    repository::create_task(&mut conn, "A1", None, Some(&tab_a.id), true).unwrap();
    repository::create_task(&mut conn, "A2", None, Some(&tab_a.id), true).unwrap();
    repository::create_task(&mut conn, "B1", None, Some(&tab_b.id), true).unwrap();

    repository::reverse_root_order(&mut conn).unwrap();

    let names = |conn: &Connection, tab: &str| -> Vec<String> {
        repository::list_roots(conn, true, Some(tab))
            .unwrap()
            .iter()
            .map(|t| t.title.clone())
            .collect()
    };
    // A1 then A2 were added with "newest first", so A2 sat on top; reversing
    // puts A1 back on top.
    assert_eq!(names(&conn, &tab_a.id), vec!["A1", "A2"]);
    assert_eq!(
        names(&conn, &tab_b.id),
        vec!["B1"],
        "a single task stays put"
    );
}

#[test]
fn reparent_can_land_at_the_end_of_the_subtask_list() {
    let mut conn = open_test_db();
    let target = repository::create_task(&mut conn, "Цель", None, None, true).unwrap();
    let first = repository::create_task(&mut conn, "Первая", Some(&target.id), None, true).unwrap();
    let second =
        repository::create_task(&mut conn, "Вторая", Some(&target.id), None, true).unwrap();
    let free = repository::create_task(&mut conn, "Свободная", None, None, true).unwrap();

    // Drag & drop passes place_at_top = false.
    let moved = repository::reparent_task(&mut conn, &free.id, Some(&target.id), false).unwrap();
    assert_eq!(moved.parent_id.as_deref(), Some(target.id.as_str()));

    let order = |conn: &Connection| -> Vec<String> {
        repository::list_subtasks(conn, &target.id)
            .unwrap()
            .iter()
            .map(|t| t.title.clone())
            .collect()
    };
    assert_eq!(order(&conn), vec!["Первая", "Вторая", "Свободная"]);
    assert!(moved.sort_order > second.sort_order);

    // The same task dragged to the top goes first.
    repository::reparent_task(&mut conn, &free.id, Some(&target.id), true).unwrap();
    assert_eq!(order(&conn), vec!["Свободная", "Первая", "Вторая"]);

    let _ = first;
}

#[test]
fn reorder_tasks_persists_subtask_order() {
    let mut conn = open_test_db();
    let parent = repository::create_task(&mut conn, "Родитель", None, None, true).unwrap();
    let a = repository::create_task(&mut conn, "A", Some(&parent.id), None, true).unwrap();
    let b = repository::create_task(&mut conn, "B", Some(&parent.id), None, true).unwrap();
    let c = repository::create_task(&mut conn, "C", Some(&parent.id), None, true).unwrap();

    repository::reorder_tasks(&mut conn, &[c.id.clone(), a.id.clone(), b.id.clone()]).unwrap();

    let titles: Vec<String> = repository::list_subtasks(&conn, &parent.id)
        .unwrap()
        .iter()
        .map(|t| t.title.clone())
        .collect();
    assert_eq!(
        titles,
        vec!["C", "A", "B"],
        "subtask order is stored, not only rendered"
    );
}

#[test]
fn nesting_a_task_with_subtasks_keeps_its_children() {
    let mut conn = open_test_db();
    let target = repository::create_task(&mut conn, "Цель", None, None, true).unwrap();
    let moved = repository::create_task(&mut conn, "Переносимая", None, None, true).unwrap();
    let child_a = repository::create_task(&mut conn, "Под A", Some(&moved.id), None, true).unwrap();
    let child_b = repository::create_task(&mut conn, "Под B", Some(&moved.id), None, true).unwrap();

    // Dropped onto a subtask row of another task: it becomes a subtask of it.
    repository::reparent_task(&mut conn, &moved.id, Some(&target.id), false).unwrap();

    // The task moved...
    assert_eq!(
        active_task(&conn, &moved.id).parent_id.as_deref(),
        Some(target.id.as_str())
    );
    // ...and its children are still attached to it, not deleted and not lost.
    let kept = repository::list_subtasks(&conn, &moved.id).unwrap();
    assert_eq!(kept.len(), 2);
    assert_eq!(kept[0].id, child_a.id);
    assert_eq!(kept[1].id, child_b.id);
    // Nothing landed in the recycle bin.
    assert!(repository::list_deleted_roots(&conn, 50)
        .unwrap()
        .is_empty());
    // Descendants followed the tab of their root.
    let tab = repository::ensure_default_tab(&conn).unwrap();
    assert_eq!(
        active_task(&conn, &child_a.id).tab_id.as_deref(),
        Some(tab.id.as_str())
    );
}

#[test]
fn nesting_a_task_with_subtasks_flattens_them_and_can_be_undone() {
    let mut conn = open_test_db();
    let target = repository::create_task(&mut conn, "Цель", None, None, true).unwrap();
    let a = repository::create_task(&mut conn, "Первая", None, None, true).unwrap();
    let b = repository::create_task(&mut conn, "Вторая", None, None, true).unwrap();
    let existing =
        repository::create_task(&mut conn, "Уже там", Some(&target.id), None, true).unwrap();
    repository::create_task(&mut conn, "Под A", Some(&a.id), None, true).unwrap();
    repository::create_task(&mut conn, "Под B", Some(&a.id), None, true).unwrap();

    let receipt = repository::nest_and_flatten(&mut conn, &a.id, &target.id, true).unwrap();

    // Only two levels exist, so the children become siblings of the moved
    // task, keeping their order, after it and after the pre-existing ones.
    let titles = |conn: &Connection| -> Vec<String> {
        repository::list_subtasks(conn, &target.id)
            .unwrap()
            .iter()
            .map(|t| t.title.clone())
            .collect()
    };
    assert_eq!(titles(&conn), vec!["Уже там", "Первая", "Под A", "Под B"]);
    assert!(repository::list_subtasks(&conn, &a.id).unwrap().is_empty());
    assert_eq!(receipt.flattened.len(), 2);

    // Nothing was deleted or completed by the move.
    assert_eq!(repository::list_deleted_roots(&conn, 50).unwrap().len(), 0);
    assert!(active_task(&conn, &a.id).deleted_at.is_none());

    // Undo restores the original shape, positions included.
    repository::undo_nest(&mut conn, &receipt).unwrap();
    assert_eq!(titles(&conn), vec!["Уже там"]);
    let restored: Vec<String> = repository::list_subtasks(&conn, &a.id)
        .unwrap()
        .iter()
        .map(|t| t.title.clone())
        .collect();
    assert_eq!(restored, vec!["Под A", "Под B"]);
    assert_eq!(active_task(&conn, &b.id).parent_id, None);
    let _ = existing;
}

#[test]
fn nesting_refuses_cycles_and_third_level() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Корень", None, None, true).unwrap();
    let sub = repository::create_task(&mut conn, "Под", Some(&root.id), None, true).unwrap();
    let other = repository::create_task(&mut conn, "Другая", None, None, true).unwrap();

    // Into its own subtask: rejected as a cycle (checked first).
    assert!(matches!(
        repository::nest_and_flatten(&mut conn, &root.id, &sub.id, true),
        Err(TaskError::WouldCreateCycle)
    ));
    // Into the subtask of a different task: that would make a third level.
    let other_root = repository::create_task(&mut conn, "Другой корень", None, None, true).unwrap();
    let other_sub =
        repository::create_task(&mut conn, "Чужой под", Some(&other_root.id), None, true).unwrap();
    assert!(matches!(
        repository::nest_and_flatten(&mut conn, &root.id, &other_sub.id, true),
        Err(TaskError::ThirdLevelForbidden)
    ));
    assert!(matches!(
        repository::nest_and_flatten(&mut conn, &root.id, &root.id, true),
        Err(TaskError::WouldCreateCycle)
    ));
    // A leaf task has nothing to flatten: the receipt is empty.
    let receipt = repository::nest_and_flatten(&mut conn, &other.id, &root.id, false).unwrap();
    assert!(receipt.flattened.is_empty());
    repository::undo_nest(&mut conn, &receipt).unwrap();
    assert!(active_task(&conn, &other.id).parent_id.is_none());
}

#[test]
fn transfer_round_trips_through_every_format() {
    let mut conn = open_test_db();
    let task = repository::create_task(&mut conn, "Проверка «кавычек» & <теги>", None, None, true)
        .unwrap();
    let child =
        repository::create_task(&mut conn, "Подзадача", Some(&task.id), None, true).unwrap();
    repository::set_comment(&mut conn, &task.id, Some("строка с & и <")).unwrap();
    repository::set_due_date(&mut conn, &child.id, Some("2026-12-31")).unwrap();

    let payload = transfer::ExportPayload {
        version: 2,
        exported_at: "2026-10-14T12:00:00Z".to_string(),
        tasks: repository::export_tasks(&conn).unwrap(),
        tabs: Some(repository::list_tabs(&conn).unwrap()),
        settings: Some(repository::get_settings(&conn).unwrap()),
    };

    let text = transfer::render(&payload).unwrap();
    assert!(!text.is_empty(), "render produced nothing");
    let parsed = transfer::parse(&text).unwrap();

    assert_eq!(parsed.tasks.len(), payload.tasks.len(), "tasks lost");
    let restored = parsed
        .tasks
        .iter()
        .find(|t| t.id == task.id)
        .expect("task survives the round trip");
    assert_eq!(
        restored.title, task.title,
        "special characters must survive"
    );
    assert_eq!(restored.comment.as_deref(), Some("строка с & и <"));
    let restored_child = parsed.tasks.iter().find(|t| t.id == child.id).unwrap();
    assert_eq!(restored_child.due_date.as_deref(), Some("2026-12-31"));
}

#[test]
fn import_merges_by_id_and_never_deletes() {
    let mut conn = open_test_db();
    let keep = repository::create_task(&mut conn, "Останется тут", None, None, true).unwrap();
    let original = repository::create_task(&mut conn, "Оригинал", None, None, true).unwrap();

    // A backup taken earlier: one new task and an older version of one we have.
    let mut changed = original.clone();
    changed.title = "Обновлённое название".to_string();
    let incoming = vec![
        Task {
            id: original.id.clone(),
            ..changed
        },
        Task {
            id: "imported-1".to_string(),
            ..original.clone()
        },
    ];
    let (added, updated, skipped) = repository::import_tasks(&mut conn, &incoming).unwrap();
    assert_eq!((added, updated, skipped), (1, 1, 0));

    let titles: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .iter()
        .map(|t| t.title.clone())
        .collect();
    assert!(titles.contains(&"Обновлённое название".to_string()));
    assert!(
        titles.contains(&"Останется тут".to_string()),
        "import never deletes"
    );
    assert_eq!(repository::list_roots(&conn, true, None).unwrap().len(), 3);
    let _ = keep;
}

#[test]
fn import_repairs_broken_hierarchy_instead_of_dropping_tasks() {
    let mut conn = open_test_db();
    let orphan = Task {
        id: "orphan".to_string(),
        parent_id: Some("missing-parent".to_string()),
        tab_id: None,
        title: "Сирота".to_string(),
        created_at: "2026-10-14T12:00:00Z".to_string(),
        updated_at: "2026-10-14T12:00:00Z".to_string(),
        completed_at: None,
        deleted_at: None,
        due_date: None,
        comment: None,
        sort_order: 0,
        completion_event_id: None,
        auto_completed_by: None,
    };

    let (added, _, skipped) = repository::import_tasks(&mut conn, &[orphan]).unwrap();
    assert_eq!(
        (added, skipped),
        (1, 1),
        "counted as skipped, but still imported"
    );

    // It became a root task instead of vanishing from every list.
    let roots = repository::list_roots(&conn, true, None).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].title, "Сирота");
}

#[test]
fn trash_lists_subtasks_and_restore_brings_them_back() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Родитель", None, None, true).unwrap();
    let a = repository::create_task(&mut conn, "Под A", Some(&root.id), None, true).unwrap();
    let b = repository::create_task(&mut conn, "Под B", Some(&root.id), None, true).unwrap();

    repository::soft_delete_task(&mut conn, &root.id).unwrap();

    // The trash shows what is really in it: the parent *and* its subtasks.
    let deleted = repository::list_deleted_all(&conn, 200).unwrap();
    assert_eq!(deleted.len(), 3, "1 parent + 2 subtasks");
    assert!(deleted.iter().any(|t| t.id == root.id));
    assert!(deleted.iter().any(|t| t.id == a.id));
    assert!(deleted.iter().any(|t| t.id == b.id));

    // Restoring the parent brings the whole tree back.
    let restored = repository::restore_task(&mut conn, &root.id).unwrap();
    assert_eq!(restored.len(), 3, "the parent and both subtasks");
    assert!(repository::list_deleted_all(&conn, 200).unwrap().is_empty());
    assert_eq!(repository::list_subtasks(&conn, &root.id).unwrap().len(), 2);
    let _ = b;
}

#[test]
fn restoring_a_subtask_on_its_own_promotes_it_to_a_root_task() {
    let mut conn = open_test_db();
    let root = repository::create_task(&mut conn, "Родитель", None, None, true).unwrap();
    let child =
        repository::create_task(&mut conn, "Одинокая под", Some(&root.id), None, true).unwrap();
    repository::soft_delete_task(&mut conn, &root.id).unwrap();

    // The parent stays in the trash; only the subtask is restored.
    let restored = repository::restore_task(&mut conn, &child.id).unwrap();
    assert_eq!(restored.len(), 1);
    assert!(
        restored[0].parent_id.is_none(),
        "a restored subtask becomes a root task instead of staying unreachable"
    );
    assert!(active_task(&conn, &child.id).deleted_at.is_none());
    assert_eq!(repository::list_roots(&conn, true, None).unwrap().len(), 1);
    // Only the parent is left in the trash.
    assert_eq!(repository::list_deleted_all(&conn, 200).unwrap().len(), 1);
    let _ = root;
}

#[test]
fn purge_all_handles_the_real_trash_shapes() {
    let mut conn = open_test_db();
    // Shape 1: a parent deleted together with its subtasks.
    let a = repository::create_task(&mut conn, "A", None, None, true).unwrap();
    let a1 = repository::create_task(&mut conn, "A1", Some(&a.id), None, true).unwrap();
    repository::create_task(&mut conn, "A2", Some(&a.id), None, true).unwrap();
    repository::soft_delete_task(&mut conn, &a.id).unwrap();

    // Shape 2: a subtask still alive under a deleted parent (legacy data).
    let b = repository::create_task(&mut conn, "B", None, None, true).unwrap();
    let b1 = repository::create_task(&mut conn, "B1", Some(&b.id), None, true).unwrap();
    repository::soft_delete_task(&mut conn, &b.id).unwrap();
    conn.execute("UPDATE tasks SET deleted_at = NULL WHERE id = ?1", [&b1.id])
        .unwrap();

    // Shape 3: a subtask deleted on its own.
    let c = repository::create_task(&mut conn, "C", None, None, true).unwrap();
    let c1 = repository::create_task(&mut conn, "C1", Some(&c.id), None, true).unwrap();
    repository::soft_delete_task(&mut conn, &c1.id).unwrap();

    let before = repository::list_deleted_all(&conn, 200).unwrap().len();
    // B1 is not in the trash any more — that is exactly why it is invisible.
    assert_eq!(before, 5, "A, A1, A2, B and C1");

    let removed = repository::purge_all_deleted(&mut conn).unwrap();
    assert_eq!(
        removed, 6,
        "the five visible rows plus the invisible orphan"
    );
    assert!(repository::list_deleted_all(&conn, 200).unwrap().is_empty());
    // Only C survives: A and B were in the trash, B1 went with its parent.
    let survivors: Vec<String> = repository::list_roots(&conn, true, None)
        .unwrap()
        .iter()
        .map(|t| t.title.clone())
        .collect();
    assert_eq!(survivors, vec!["C"]);
    let _ = a1;
}

#[test]
fn export_carries_the_whole_state_and_import_reproduces_it() {
    let mut conn = open_test_db();
    // Two tabs, a hierarchy, a completed task, a comment and a deleted task.
    let home = repository::create_tab(&conn, "Дом").unwrap();
    let root = repository::create_task(&mut conn, "Корень", None, Some(&home.id), true).unwrap();
    let child = repository::create_task(&mut conn, "Под", Some(&root.id), None, true).unwrap();
    let done = repository::create_task(&mut conn, "Готово", None, None, true).unwrap();
    let gone = repository::create_task(&mut conn, "В корзине", None, None, true).unwrap();
    repository::set_comment(&mut conn, &child.id, Some("заметка")).unwrap();
    repository::soft_delete_task(&mut conn, &gone.id).unwrap();
    let settings = AppSettings {
        theme: "dark".to_string(),
        show_completed: true,
        cascade_complete: false,
        expand_subtasks: false,
        show_tab_counts: true,
        newest_first: false,
        auto_delete_days: Some(14),
        language: "en".to_string(),
    };
    repository::update_settings(&mut conn, &settings).unwrap();
    let _ = done;

    let payload = transfer::ExportPayload {
        version: 2,
        exported_at: "2026-10-14T12:00:00Z".to_string(),
        tasks: repository::export_tasks(&conn).unwrap(),
        tabs: Some(repository::list_tabs(&conn).unwrap()),
        settings: Some(repository::get_settings(&conn).unwrap()),
    };
    assert_eq!(payload.tabs.as_ref().map(Vec::len), Some(2));
    assert!(payload
        .tasks
        .iter()
        .any(|x| x.id == gone.id && x.deleted_at.is_some()));

    // The snapshot must carry all of it, not only the visible tasks.
    let text = transfer::render(&payload).unwrap();
    let parsed = transfer::parse(&text).unwrap();
    assert_eq!(parsed.tasks.len(), payload.tasks.len(), "tasks lost");
    assert_eq!(parsed.tabs.map(|v| v.len()), Some(2), "tabs lost");
    let s = parsed.settings.as_ref().expect("settings lost");
    assert_eq!(s.theme, "dark");
    assert_eq!(s.auto_delete_days, Some(14));
    assert!(!s.cascade_complete);
    assert_eq!(s.language, "en");
    let ids: Vec<String> = parsed.tasks.iter().map(|x| x.id.clone()).collect();
    for id in [root.id.clone(), child.id.clone(), gone.id.clone()] {
        assert!(ids.contains(&id), "lost {id}");
    }
}

#[test]
fn importing_a_state_file_restores_tabs_hierarchy_and_settings() {
    let mut src = open_test_db();
    let home = repository::create_tab(&src, "Дом").unwrap();
    let root = repository::create_task(&mut src, "Корень", None, Some(&home.id), true).unwrap();
    let child = repository::create_task(&mut src, "Под", Some(&root.id), None, true).unwrap();
    repository::update_settings(
        &mut src,
        &AppSettings {
            theme: "light".to_string(),
            show_completed: true,
            cascade_complete: false,
            expand_subtasks: true,
            show_tab_counts: false,
            newest_first: false,
            auto_delete_days: Some(7),
            language: "en".to_string(),
        },
    )
    .unwrap();

    let payload = transfer::ExportPayload {
        version: 2,
        exported_at: "2026-10-14T12:00:00Z".to_string(),
        tasks: repository::export_tasks(&src).unwrap(),
        tabs: Some(repository::list_tabs(&src).unwrap()),
        settings: Some(repository::get_settings(&src).unwrap()),
    };

    // A completely different, empty database.
    let mut dst = open_test_db();
    let dst_default = repository::list_tabs(&dst).unwrap()[0].id.clone();
    let (tabs_added, tabs_updated) =
        repository::import_tabs(&mut dst, payload.tabs.as_ref().unwrap()).unwrap();
    assert_eq!((tabs_added, tabs_updated), (2, 0));
    let (added, updated, skipped) = repository::import_tasks(&mut dst, &payload.tasks).unwrap();
    assert_eq!((added, updated, skipped), (2, 0, 0));
    repository::import_settings(&mut dst, payload.settings.as_ref().unwrap()).unwrap();

    let restored = repository::get_settings(&dst).unwrap();
    assert_eq!(restored.theme, "light");
    assert_eq!(restored.auto_delete_days, Some(7));
    assert_eq!(restored.language, "en");
    assert!(!restored.show_tab_counts);
    // The hierarchy is intact: the child is still under the parent, not a root.
    let subtasks = repository::list_subtasks(&dst, &root.id).unwrap();
    assert_eq!(subtasks.len(), 1);
    assert_eq!(subtasks[0].id, child.id);
    // The task kept its tab instead of falling back to the default one.
    let moved = active_task(&dst, &root.id);
    assert_eq!(moved.tab_id.as_deref(), Some(home.id.as_str()));
    assert_ne!(moved.tab_id.as_deref(), Some(dst_default.as_str()));
}

#[test]
fn a_version_1_file_without_tabs_or_settings_still_imports() {
    let conn = open_test_db();
    let legacy = r#"{"version":1,"exportedAt":"2026-01-01T00:00:00Z","tasks":[
        {"id":"t1","parentId":null,"tabId":null,"title":"Старая задача","createdAt":"2026-01-01T00:00:00Z",
         "updatedAt":"2026-01-01T00:00:00Z","completedAt":null,"deletedAt":null,"dueDate":null,
         "comment":null,"sortOrder":0,"completionEventId":null,"autoCompletedBy":null}]}"#;
    let payload = transfer::parse(legacy).unwrap();
    assert!(payload.tabs.is_none());
    assert!(payload.settings.is_none());
    assert_eq!(payload.tasks.len(), 1);

    let mut db = open_test_db();
    let (added, _, _) = repository::import_tasks(&mut db, &payload.tasks).unwrap();
    assert_eq!(added, 1);
    // With no tab in the file the task lands in the only existing tab.
    assert!(active_task(&db, "t1").tab_id.is_some());
    drop(conn);
}

#[test]
fn importing_tabs_twice_updates_instead_of_duplicating() {
    let mut conn = open_test_db();
    let tab = repository::list_tabs(&conn).unwrap()[0].clone();
    let mut renamed = tab.clone();
    renamed.name = "Переименованная".to_string();

    let (added, updated) = repository::import_tabs(&mut conn, &[tab.clone(), renamed]).unwrap();
    assert_eq!((added, updated), (0, 2));
    assert_eq!(repository::list_tabs(&conn).unwrap().len(), 1);
    assert_eq!(
        repository::list_tabs(&conn).unwrap()[0].name,
        "Переименованная"
    );
}

#[test]
fn import_refuses_a_hostile_title_instead_of_storing_it() {
    // A file from anywhere is untrusted input. The UI caps a typed title at 500
    // characters; an imported one must obey the same rule or one row could
    // hold megabytes and wreck the layout.
    let mut conn = open_test_db();
    let huge = "x".repeat(50_000);
    let payload = transfer::ExportPayload {
        version: 2,
        exported_at: "2026-10-14T00:00:00Z".to_string(),
        tasks: vec![
            task_json("huge", &huge),
            task_json("ok", "Нормальная задача"),
        ],
        tabs: None,
        settings: None,
    };

    let (added, _, skipped) = repository::import_tasks(&mut conn, &payload.tasks).unwrap();
    assert_eq!(
        (added, skipped),
        (1, 1),
        "the oversized title must be skipped"
    );

    let stored = repository::list_roots(&conn, true, None).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].title, "Нормальная задача");
    assert!(
        stored[0].title.len() < 1_000,
        "nothing oversized reached the database"
    );
}

#[test]
fn import_trims_and_normalises_a_title_like_typing_does() {
    let mut conn = open_test_db();
    let payload = transfer::ExportPayload {
        version: 2,
        exported_at: "2026-10-14T00:00:00Z".to_string(),
        tasks: vec![task_json("padded", "   Задача с обрезкой   ")],
        tabs: None,
        settings: None,
    };
    repository::import_tasks(&mut conn, &payload.tasks).unwrap();
    assert_eq!(
        repository::list_roots(&conn, true, None).unwrap()[0].title,
        "Задача с обрезкой"
    );
}

fn task_json(id: &str, title: &str) -> Task {
    Task {
        id: id.to_string(),
        parent_id: None,
        tab_id: None,
        title: title.to_string(),
        created_at: "2026-10-14T00:00:00Z".to_string(),
        updated_at: "2026-10-14T00:00:00Z".to_string(),
        completed_at: None,
        deleted_at: None,
        due_date: None,
        comment: None,
        sort_order: 0,
        completion_event_id: None,
        auto_completed_by: None,
    }
}

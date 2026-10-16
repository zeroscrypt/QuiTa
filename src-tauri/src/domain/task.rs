use serde::{Deserialize, Serialize};

pub type TaskId = String;

/// Core task entity. Dates without time-of-day are calendar dates (YYYY-MM-DD);
/// timestamps are RFC 3339 UTC strings.
///
/// `rename_all = "camelCase"` keeps the JSON contract identical to the
/// TypeScript model in `src/lib/types.ts` (completedAt, parentId, sortOrder...).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: TaskId,
    pub parent_id: Option<TaskId>,
    /// Tab (topic/project) this task belongs to. NULL for legacy rows before
    /// tabs existed — those are shown under the first tab.
    pub tab_id: Option<TaskId>,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub deleted_at: Option<String>,
    /// Calendar date YYYY-MM-DD, never a UTC timestamp.
    pub due_date: Option<String>,
    /// Free-form note. `None` when the task has no comment.
    pub comment: Option<String>,
    pub sort_order: i64,
    /// Set on a root task when its children were auto-completed by a parent
    /// completion event. Identifies the event so it can be precisely undone.
    pub completion_event_id: Option<String>,
    /// For subtasks only: completion event id of the parent that
    /// auto-completed this subtask. NULL means the user completed it manually.
    pub auto_completed_by: Option<String>,
}

impl Task {
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    pub fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

/// A root task with its (one level of) subtasks, as delivered to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNode {
    pub task: Task,
    pub children: Vec<Task>,
}

/// A subtask matched by search together with its parent for context.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchedSubtask {
    pub task: Task,
    pub parent: Task,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// Active root tasks whose title matches.
    pub roots: Vec<Task>,
    /// Active subtasks whose title matches, with parent context.
    pub matched_subtasks: Vec<MatchedSubtask>,
}

/// A child's position, captured before a flattening move so it can be undone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildPlacement {
    pub id: TaskId,
    pub sort_order: i64,
}

/// Everything needed to undo `nest_and_flatten`, and to describe the move to
/// the user ("3 subtasks were moved as well").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReparentReceipt {
    pub task_id: TaskId,
    pub previous_parent_id: Option<TaskId>,
    pub previous_tab_id: Option<TaskId>,
    pub previous_sort_order: i64,
    /// The children that became siblings, with their original positions.
    pub flattened: Vec<ChildPlacement>,
}

/// Counter shown on a tab: done/total over every non-deleted task it holds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabStat {
    pub tab_id: TaskId,
    pub total: i64,
    pub done: i64,
}

/// Disk and row statistics of the local database, shown on the "Database"
/// settings tab.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStats {
    /// On-disk size of the database file plus its `-wal`/`-shm` companions.
    pub size_bytes: u64,
    pub page_size: u64,
    pub page_count: u64,
    /// Pages freed by deletions. SQLite reuses them for new rows and never
    /// returns them to the file system without `VACUUM`.
    pub freelist_count: u64,
    pub task_count: i64,
    pub active_count: i64,
    pub deleted_count: i64,
    pub tab_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// "system" | "light" | "dark"
    pub theme: String,
    pub show_completed: bool,
    /// When true, completing a parent also completes its open subtasks
    /// (and reopening it reopens exactly those). When false, the parent's
    /// checkbox affects only the parent itself.
    pub cascade_complete: bool,
    /// When set, completed tasks are soft-deleted after this many days.
    /// `None` disables the automatic cleanup.
    pub auto_delete_days: Option<i64>,
    /// How many soft-deleted tasks the trash keeps. Once it is full, deleting
    /// anything pushes the oldest task out for good.
    pub trash_limit: i64,
    /// When set, trees that have sat in the trash this many days are removed for
    /// good on the next launch. `None` keeps them until the user says otherwise.
    pub trash_retention_days: Option<i64>,
    /// Interface language: "ru" | "en".
    pub language: String,
    /// When true, every task with subtasks shows them without being expanded
    /// by hand. Individual tasks can still be collapsed.
    pub expand_subtasks: bool,
    /// When true, each tab shows a done/total counter.
    pub show_tab_counts: bool,
    /// New root tasks are placed at the top of their tab when true, at the
    /// bottom when false.
    pub newest_first: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            theme: "system".to_string(),
            show_completed: false,
            cascade_complete: true,
            auto_delete_days: None,
            trash_limit: crate::db::repository::DEFAULT_TRASH_LIMIT,
            trash_retention_days: None,
            language: "ru".to_string(),
            expand_subtasks: true,
            show_tab_counts: true,
            newest_first: true,
        }
    }
}

impl AppSettings {
    pub fn validate_theme(theme: &str) -> Result<String, String> {
        match theme {
            "system" | "light" | "dark" => Ok(theme.to_string()),
            other => Err(format!("unknown theme: {other}")),
        }
    }
}

/// Tasks touched by a completion toggle, returned to the UI for a precise update.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToggleOutcome {
    pub updated: Vec<Task>,
}

/// A tab groups tasks by topic or project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    pub id: TaskId,
    pub name: String,
    /// Hex colour (`#rrggbb`) assigned automatically.
    pub color: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
    /// Set when the tab was deleted. The row survives so that its tasks keep a
    /// valid tab reference while they sit in the trash.
    pub deleted_at: Option<String>,
}

/// What a tab deletion did, and everything needed to take it back.
///
/// The UI holds this for the length of the undo window; nothing else can work
/// out which tasks were involved, because afterwards they are indistinguishable
/// from any other deleted task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabDeletion {
    pub tab: Tab,
    pub task_ids: Vec<TaskId>,
}

/// What one move into the trash did: what arrived, and what it had to push out
/// to keep the trash within its bound.
///
/// Both halves travel together because the second one is a silent loss. Every
/// path that puts tasks in the trash reports through this type — a deletion by
/// hand, the completion timer, a deleted tab — so that a trash pushed over its
/// bound cannot happen without the caller learning which task it cost.
#[derive(Debug, Clone)]
pub struct TrashSweep {
    pub moved: Vec<Task>,
    pub evicted: Vec<Task>,
}

impl TrashSweep {
    /// Nothing moved and nothing left.
    pub fn empty() -> Self {
        TrashSweep {
            moved: Vec::new(),
            evicted: Vec::new(),
        }
    }
}

/// Palette used to colour tabs automatically.
pub const TAB_COLORS: [&str; 10] = [
    "#4c6fff", "#0ea5e9", "#10b981", "#84cc16", "#eab308", "#f97316", "#ef4444", "#ec4899",
    "#a855f7", "#14b8a6",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> Task {
        Task {
            id: "id-1".to_string(),
            parent_id: Some("id-0".to_string()),
            tab_id: Some("tab-1".to_string()),
            title: "Задача".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            completed_at: None,
            deleted_at: None,
            due_date: Some("2026-10-13".to_string()),
            comment: None,
            sort_order: -1,
            completion_event_id: None,
            auto_completed_by: None,
        }
    }

    /// The UI reads camelCase; snake_case silently yields `undefined` in
    /// TypeScript (which made every task look completed). Keep this locked.
    #[test]
    fn task_serializes_to_camel_case() {
        let json = serde_json::to_string(&sample_task()).unwrap();
        for key in [
            "parentId",
            "createdAt",
            "updatedAt",
            "completedAt",
            "deletedAt",
            "dueDate",
            "sortOrder",
            "completionEventId",
            "autoCompletedBy",
        ] {
            assert!(
                json.contains(&format!("\"{key}\"")),
                "missing {key} in {json}"
            );
        }
        assert!(!json.contains("parent_id"));
        assert!(!json.contains("sort_order"));
        // completed_at must be present as JSON null, not missing.
        assert!(json.contains("\"completedAt\":null"), "{json}");
    }

    #[test]
    fn task_deserializes_from_camel_case() {
        let json = r#"{
            "id": "x",
            "parentId": null,
            "title": "t",
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z",
            "completedAt": null,
            "deletedAt": null,
            "dueDate": null,
            "sortOrder": 3,
            "completionEventId": null,
            "autoCompletedBy": null
        }"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.sort_order, 3);
        assert!(task.parent_id.is_none());
        assert!(!task.is_completed());
    }

    #[test]
    fn settings_use_camel_case() {
        let json = serde_json::to_string(&AppSettings {
            theme: "dark".to_string(),
            show_completed: true,
            cascade_complete: false,
            auto_delete_days: Some(30),
            trash_limit: 50,
            trash_retention_days: Some(90),
            language: "en".to_string(),
            expand_subtasks: true,
            show_tab_counts: true,
            newest_first: true,
        })
        .unwrap();
        assert!(json.contains("\"showCompleted\":true"), "{json}");
        assert!(json.contains("\"cascadeComplete\":false"), "{json}");
        assert!(json.contains("\"autoDeleteDays\":30"), "{json}");
        assert!(
            json.contains("\"trashLimit\":50"),
            "the trash size rule crosses the bridge under this name: {json}"
        );
        assert!(json.contains("\"language\":\"en\""), "{json}");
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert!(parsed.show_completed);
        assert!(!parsed.cascade_complete);
        assert_eq!(parsed.auto_delete_days, Some(30));
        assert_eq!(parsed.trash_limit, 50);
        assert_eq!(parsed.language, "en");
    }

    #[test]
    fn search_result_uses_camel_case() {
        let result = SearchResult {
            roots: vec![sample_task()],
            matched_subtasks: Vec::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"matchedSubtasks\""), "{json}");
    }
}

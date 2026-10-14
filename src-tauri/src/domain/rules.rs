use crate::domain::task::Task;

/// Pure decision used by both the repository (to apply) and tests (to verify).
/// Computing the plan without touching the database keeps the rules explicit.
#[derive(Debug, Clone, PartialEq)]
pub struct TogglePlan {
    pub target_id: String,
    pub complete: bool,
    /// Generated event id when completing a parent auto-completes its children.
    pub completion_event_id: Option<String>,
    pub children_to_complete: Vec<String>,
    pub children_to_reopen: Vec<String>,
}

/// Computes what must happen to `target`, its children and its parent when the
/// user toggles `target`. Rules (see docs/TASK_RULES.md):
///
/// - Completing a parent auto-completes all incomplete subtasks under a fresh
///   completion event id; that id is stored on the parent (`completion_event_id`)
///   and on each auto-completed child (`auto_completed_by`).
/// - Reopening a parent reopens only subtasks auto-completed by that exact event.
///   Manually completed subtasks stay completed.
/// - Toggling a *subtask* never touches its parent, in either direction. A
///   completed parent with open subtasks is the ordinary "partially done"
///   state: the parent is closed, work below it is not. Opening one subtask
///   must not silently reopen the task the user deliberately closed.
/// - With `cascade == false` the parent's checkbox only affects the parent:
///   subtasks keep their state and no completion event is recorded.
pub fn compute_toggle_plan(target: &Task, children: &[Task], cascade: bool) -> TogglePlan {
    if !target.is_completed() {
        // Completing the target.
        let children_to_complete: Vec<String> = if cascade {
            children
                .iter()
                .filter(|c| !c.is_completed())
                .map(|c| c.id.clone())
                .collect()
        } else {
            Vec::new()
        };
        // Only generate an event id when there is something to auto-complete.
        let completion_event_id = if children_to_complete.is_empty() {
            None
        } else {
            Some(uuid::Uuid::new_v4().to_string())
        };
        TogglePlan {
            target_id: target.id.clone(),
            complete: true,
            completion_event_id,
            children_to_complete,
            children_to_reopen: Vec::new(),
        }
    } else {
        // Reopening the target. With cascade off the parent's checkbox never
        // touches subtasks, in either direction.
        let children_to_reopen: Vec<String> = if cascade {
            match &target.completion_event_id {
                Some(event_id) => children
                    .iter()
                    .filter(|c| {
                        c.is_completed()
                            && c.auto_completed_by.as_deref() == Some(event_id.as_str())
                    })
                    .map(|c| c.id.clone())
                    .collect(),
                None => Vec::new(),
            }
        } else {
            Vec::new()
        };
        TogglePlan {
            target_id: target.id.clone(),
            complete: false,
            completion_event_id: None,
            children_to_complete: Vec::new(),
            children_to_reopen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::Task;

    fn task(
        id: &str,
        parent_id: Option<&str>,
        completed_at: Option<&str>,
        completion_event_id: Option<&str>,
        auto_completed_by: Option<&str>,
    ) -> Task {
        Task {
            id: id.to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            tab_id: None,
            title: id.to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            completed_at: completed_at.map(|s| s.to_string()),
            deleted_at: None,
            due_date: None,
            comment: None,
            sort_order: 0,
            completion_event_id: completion_event_id.map(|s| s.to_string()),
            auto_completed_by: auto_completed_by.map(|s| s.to_string()),
        }
    }

    #[test]
    fn completing_parent_auto_completes_children_under_one_event() {
        let parent = task("p", None, None, None, None);
        let c1 = task("c1", Some("p"), None, None, None);
        let c2 = task("c2", Some("p"), None, None, None);
        let plan = compute_toggle_plan(&parent, &[c1, c2], true);
        assert!(plan.complete);
        assert!(plan.completion_event_id.is_some());
        assert_eq!(plan.children_to_complete.len(), 2);
    }

    #[test]
    fn completing_parent_keeps_manually_completed_children_untouched() {
        let parent = task("p", None, None, None, None);
        let manual = task("c1", Some("p"), Some("2026-01-02T00:00:00Z"), None, None);
        let open = task("c2", Some("p"), None, None, None);
        let plan = compute_toggle_plan(&parent, &[manual, open], true);
        assert_eq!(plan.children_to_complete, vec!["c2"]);
    }

    #[test]
    fn reopening_parent_reopens_only_auto_completed_children() {
        let event = "event-1";
        let parent = task("p", None, Some("2026-01-02T00:00:00Z"), Some(event), None);
        let auto = task(
            "c1",
            Some("p"),
            Some("2026-01-02T00:00:00Z"),
            Some(event),
            Some(event),
        );
        let manual = task("c2", Some("p"), Some("2026-01-01T00:00:00Z"), None, None);
        let plan = compute_toggle_plan(&parent, &[auto, manual], true);
        assert!(!plan.complete);
        assert_eq!(plan.children_to_reopen, vec!["c1"]);
    }

    #[test]
    fn reopening_subtask_leaves_the_completed_parent_closed() {
        // The parent was closed on purpose; taking one subtask back out of it
        // must produce "partially done", not silently reopen the parent.
        let child = task(
            "c",
            Some("p"),
            Some("2026-01-02T00:00:00Z"),
            None,
            Some("e"),
        );
        let plan = compute_toggle_plan(&child, &[], true);
        assert!(!plan.complete);
        // Nothing beyond the subtask itself changes.
        assert!(plan.children_to_reopen.is_empty());
    }

    #[test]
    fn toggling_a_subtask_never_touches_the_parent_in_either_direction() {
        // "Поднять ВМ" is done. Finishing the last step, or taking one back
        // out, must both leave the parent alone — it reads as partially done
        // instead of silently reopening behind the user's back.
        let open_child = task("c", Some("p"), None, None, None);
        let done_child = task("c", Some("p"), Some("2026-01-02T00:00:00Z"), None, None);
        let completing = compute_toggle_plan(&open_child, &[], true);
        assert!(completing.complete);
        let reopening = compute_toggle_plan(&done_child, &[], true);
        assert!(!reopening.complete);
        // A subtask has no children of its own, so nothing else may move.
        assert!(completing.children_to_complete.is_empty());
        assert!(reopening.children_to_reopen.is_empty());
    }

    #[test]
    fn cascade_disabled_leaves_children_untouched() {
        let parent = task("p", None, None, None, None);
        let open = task("c1", Some("p"), None, None, None);
        let done = task("c2", Some("p"), Some("2026-01-01T00:00:00Z"), None, None);

        let plan = compute_toggle_plan(&parent, &[open, done], false);
        assert!(plan.complete);
        assert!(plan.children_to_complete.is_empty(), "subtasks stay open");
        assert!(
            plan.completion_event_id.is_none(),
            "no event is recorded when nothing was auto-completed"
        );
    }

    #[test]
    fn cascade_disabled_means_reopening_restores_nothing() {
        let event = "event-1";
        let parent = task("p", None, Some("2026-01-02T00:00:00Z"), Some(event), None);
        let auto = task(
            "c1",
            Some("p"),
            Some("2026-01-02T00:00:00Z"),
            Some(event),
            Some(event),
        );

        let plan = compute_toggle_plan(&parent, &[auto], false);
        assert!(!plan.complete);
        assert!(
            plan.children_to_reopen.is_empty(),
            "reopening with cascade off must not touch subtasks"
        );
    }
}

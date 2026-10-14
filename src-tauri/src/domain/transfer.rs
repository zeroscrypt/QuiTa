use serde::{Deserialize, Serialize};

use crate::domain::task::{AppSettings, Tab, Task};
use crate::error::TaskError;

/// Export envelope: a snapshot of the whole application state.
///
/// Version 1 contained tasks only. Version 2 adds the tabs and the settings,
/// which makes an import reproduce the application instead of just the task
/// list. Both versions import cleanly: the newer fields are optional.
///
/// Only JSON is produced. YAML and XML were dropped deliberately: their parsers
/// carried unresolved advisories (`serde_yml` — unsound and unmaintained,
/// `quick-xml` — quadratic runtime and unbounded allocation on hostile input),
/// and an import accepts a file from anywhere, so a parser nobody can audit is
/// a risk taken for no benefit. One format means one code path to trust.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPayload {
    pub version: u32,
    pub exported_at: String,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tabs: Option<Vec<Tab>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<AppSettings>,
}

/// Name offered in the save dialog.
pub const EXPORT_FILE_NAME: &str = "quita-export.json";

/// Formats the payload. `serde_json` only — see the note on [`ExportPayload`].
pub fn render(payload: &ExportPayload) -> Result<String, TaskError> {
    serde_json::to_string_pretty(payload).map_err(|e| TaskError::Internal(e.to_string()))
}

/// Reads a payload back.
///
/// A YAML or XML file is rejected with an explanation rather than a raw parser
/// error: those formats existed in earlier builds, so a user may still have one
/// on disk and deserves to know why it will not load.
pub fn parse(contents: &str) -> Result<ExportPayload, TaskError> {
    if let Some(kind) = foreign_format(contents) {
        return Err(TaskError::ImportParse(format!(
            "only JSON is supported now; this file looks like {kind}. \
             Export it as JSON from an older build, or convert it with any JSON tool."
        )));
    }
    serde_json::from_str(contents).map_err(|e| TaskError::ImportParse(e.to_string()))
}

/// Recognises the two formats that are no longer accepted, for a clear message.
fn foreign_format(contents: &str) -> Option<&'static str> {
    let trimmed = contents.trim_start();
    if trimmed.starts_with('<') {
        return Some("XML");
    }
    // YAML has no reliable marker; a leading `- ` or `key: ` is the giveaway.
    let first = trimmed.lines().next().unwrap_or_default();
    if first.starts_with("- ") || first.contains(": ") || first.starts_with("---") {
        return Some("YAML");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> ExportPayload {
        ExportPayload {
            version: 2,
            exported_at: "2026-10-14T12:00:00Z".to_string(),
            tasks: vec![Task {
                id: "t1".to_string(),
                parent_id: None,
                tab_id: Some("tab1".to_string()),
                title: "Кавычки \" и & <теги>".to_string(),
                created_at: "2026-10-14T10:00:00Z".to_string(),
                updated_at: "2026-10-14T10:00:00Z".to_string(),
                completed_at: None,
                deleted_at: None,
                due_date: None,
                comment: None,
                sort_order: -1,
                completion_event_id: None,
                auto_completed_by: None,
            }],
            tabs: Some(vec![Tab {
                id: "tab1".to_string(),
                name: "Работа".to_string(),
                color: "#4c6fff".to_string(),
                sort_order: 0,
                created_at: "2026-10-14T10:00:00Z".to_string(),
                updated_at: "2026-10-14T10:00:00Z".to_string(),
            }]),
            settings: Some(AppSettings::default()),
        }
    }

    #[test]
    fn round_trips_the_whole_state() {
        let original = payload();
        let text = render(&original).unwrap();
        let parsed = parse(&text).unwrap();

        assert_eq!(parsed.version, 2);
        assert_eq!(parsed.tasks.len(), 1);
        assert_eq!(parsed.tasks[0].title, original.tasks[0].title);
        assert_eq!(parsed.tabs.as_ref().map(Vec::len), Some(1));
        assert!(parsed.settings.is_some());
    }

    #[test]
    fn survives_special_characters_in_titles_and_comments() {
        let mut original = payload();
        original.tasks[0].comment = Some("строка с \" и & и \\ и <теги>".to_string());
        let parsed = parse(&render(&original).unwrap()).unwrap();
        assert_eq!(
            parsed.tasks[0].comment.as_deref(),
            Some("строка с \" и & и \\ и <теги>")
        );
    }

    #[test]
    fn a_version_1_file_without_tabs_or_settings_still_imports() {
        let legacy = r#"{"version":1,"exportedAt":"2026-01-01T00:00:00Z","tasks":[
            {"id":"t1","parentId":null,"tabId":null,"title":"Старая","createdAt":"c","updatedAt":"u",
             "completedAt":null,"deletedAt":null,"dueDate":null,"comment":null,"sortOrder":0,
             "completionEventId":null,"autoCompletedBy":null}]}"#;
        let parsed = parse(legacy).unwrap();
        assert!(parsed.tabs.is_none());
        assert!(parsed.settings.is_none());
        assert_eq!(parsed.tasks.len(), 1);
    }

    #[test]
    fn yaml_and_xml_are_rejected_with_an_explanation() {
        let xml = parse("<quita version=\"2\"><tasks/></quita>").unwrap_err();
        assert!(xml.to_string().contains("XML"), "{xml}");

        let yaml = parse("version: 2\ntasks: []\n").unwrap_err();
        assert!(yaml.to_string().contains("YAML"), "{yaml}");

        // A plain JSON file must not be mistaken for YAML.
        assert!(parse(r#"{"version":2,"exportedAt":"x","tasks":[]}"#).is_ok());
    }
}

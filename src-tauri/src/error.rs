use serde::{Serialize, Serializer};
use thiserror::Error;

/// Typed application errors. Messages are user-facing and must be honest.
///
/// The UI renders whatever a rejected command produces, so this enum is
/// serialized as its plain message string. Deriving `Serialize` would emit an
/// externally tagged value instead (`"LastTab"` for unit variants,
/// `{"NotFound":"…"}` for the rest), which the frontend would show as
/// `[object Object]`.
#[derive(Debug, Error)]
pub enum TaskError {
    #[error("task title must not be empty")]
    EmptyTitle,

    #[error("task not found: {0}")]
    NotFound(String),

    #[error("parent task not found: {0}")]
    ParentNotFound(String),

    #[error("cannot add subtasks to a completed task")]
    ParentCompleted,

    #[error("only one subtask level is allowed: subtasks cannot have their own subtasks")]
    ThirdLevelForbidden,

    #[error("cannot move a task into itself or one of its subtasks")]
    WouldCreateCycle,

    #[error("tab not found: {0}")]
    TabNotFound(String),

    #[error("cannot restore a subtask whose parent is deleted")]
    ParentDeleted,

    #[error("only a deleted task can be removed permanently: {0}")]
    NotDeleted(String),

    #[error("invalid due date {0}: expected a YYYY-MM-DD calendar date")]
    InvalidDueDate(String),

    #[error("comment is too long: the limit is {0} characters")]
    CommentTooLong(String),

    #[error("unknown export format: {0}")]
    UnknownFormat(String),

    #[error("cannot read the file: {0}")]
    ImportRead(String),

    #[error("cannot parse the file: {0}")]
    ImportParse(String),

    #[error("the file is too large to import: the limit is {0} bytes")]
    ImportTooLarge(u64),

    #[error("title is too long: the limit is {0} characters")]
    TitleTooLong(String),

    #[error("invalid move direction: {0}")]
    InvalidDirection(String),

    #[error("cannot delete the last tab")]
    LastTab,

    #[error("database error: {0}")]
    Database(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl Serialize for TaskError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rusqlite::Error> for TaskError {
    fn from(error: rusqlite::Error) -> Self {
        TaskError::Database(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::TaskError;

    /// The frontend shows this value verbatim, so it must never become an
    /// object that renders as `[object Object]`.
    #[test]
    fn serializes_as_a_plain_message_string() {
        assert_eq!(
            serde_json::to_string(&TaskError::LastTab).unwrap(),
            "\"cannot delete the last tab\""
        );
        assert_eq!(
            serde_json::to_string(&TaskError::NotFound("abc".into())).unwrap(),
            "\"task not found: abc\""
        );
    }
}

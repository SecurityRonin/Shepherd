//! Session replay — structured audit log for agent sessions.
//!
//! Parses terminal output into structured timeline events that can
//! be queried, searched, and replayed in the frontend.

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Type of session event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Agent started working.
    SessionStart,
    /// Agent finished or was killed.
    SessionEnd,
    /// User or system input to agent.
    Input,
    /// Agent terminal output.
    Output,
    /// Agent invoked a tool (file write, bash, etc.).
    ToolCall,
    /// Tool returned a result.
    ToolResult,
    /// Agent made an LLM API call.
    LlmCall,
    /// An error occurred.
    Error,
    /// Permission was requested.
    PermissionRequest,
    /// Permission was resolved (approved/denied).
    PermissionResolve,
    /// File was modified.
    FileChange,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{self:?}"));
        write!(f, "{s}")
    }
}

/// A single structured event in a session timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: i64,
    pub task_id: i64,
    pub session_id: i64,
    pub event_type: EventType,
    /// Human-readable summary of the event.
    pub summary: String,
    /// Full content/payload (may be large for output events).
    pub content: String,
    /// Optional metadata as JSON.
    pub metadata: Option<String>,
    pub timestamp: String,
}

/// Create the session_events table.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS session_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_id INTEGER NOT NULL,
            session_id INTEGER NOT NULL DEFAULT 0,
            event_type TEXT NOT NULL,
            summary TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL DEFAULT '',
            metadata TEXT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_session_events_task
            ON session_events(task_id);
        CREATE INDEX IF NOT EXISTS idx_session_events_type
            ON session_events(event_type);
        CREATE INDEX IF NOT EXISTS idx_session_events_time
            ON session_events(timestamp);
        ",
    )?;
    Ok(())
}

/// Record a new event in the timeline.
pub fn record_event(
    conn: &Connection,
    task_id: i64,
    session_id: i64,
    event_type: &EventType,
    summary: &str,
    content: &str,
    metadata: Option<&str>,
) -> Result<i64> {
    let type_str = event_type.to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO session_events (task_id, session_id, event_type, summary, content, metadata, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![task_id, session_id, type_str, summary, content, metadata, now],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Get all events for a task, ordered by timestamp.
pub fn get_timeline(conn: &Connection, task_id: i64) -> Result<Vec<TimelineEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, session_id, event_type, summary, content, metadata, timestamp
         FROM session_events WHERE task_id = ?1 ORDER BY timestamp ASC, id ASC",
    )?;

    let events = stmt
        .query_map(params![task_id], row_to_event)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(events)
}

/// Get events of a specific type for a task.
pub fn get_events_by_type(
    conn: &Connection,
    task_id: i64,
    event_type: &EventType,
) -> Result<Vec<TimelineEvent>> {
    let type_str = event_type.to_string();
    let mut stmt = conn.prepare(
        "SELECT id, task_id, session_id, event_type, summary, content, metadata, timestamp
         FROM session_events WHERE task_id = ?1 AND event_type = ?2 ORDER BY timestamp ASC",
    )?;

    let events = stmt
        .query_map(params![task_id, type_str], row_to_event)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(events)
}

/// Search events by content substring.
pub fn search_events(
    conn: &Connection,
    task_id: i64,
    query: &str,
) -> Result<Vec<TimelineEvent>> {
    let pattern = format!("%{query}%");
    let mut stmt = conn.prepare(
        "SELECT id, task_id, session_id, event_type, summary, content, metadata, timestamp
         FROM session_events WHERE task_id = ?1 AND (content LIKE ?2 OR summary LIKE ?2)
         ORDER BY timestamp ASC",
    )?;

    let events = stmt
        .query_map(params![task_id, pattern], row_to_event)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(events)
}

/// Get event count for a task.
pub fn event_count(conn: &Connection, task_id: i64) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM session_events WHERE task_id = ?1",
        params![task_id],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Get the duration of a session (time between first and last event).
pub fn session_duration(conn: &Connection, task_id: i64) -> Result<Option<f64>> {
    let result: std::result::Result<(String, String), _> = conn.query_row(
        "SELECT MIN(timestamp), MAX(timestamp) FROM session_events WHERE task_id = ?1",
        params![task_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );

    match result {
        Ok((start, end)) => {
            let start_dt = chrono::DateTime::parse_from_rfc3339(&start).ok();
            let end_dt = chrono::DateTime::parse_from_rfc3339(&end).ok();
            match (start_dt, end_dt) {
                (Some(s), Some(e)) => {
                    let duration = (e - s).num_milliseconds() as f64 / 1000.0;
                    Ok(Some(duration))
                }
                _ => Ok(None),
            }
        }
        Err(_) => Ok(None),
    }
}

/// Parse raw terminal output into a classified event type and summary.
pub fn classify_output(raw: &str) -> (EventType, String) {
    let trimmed = raw.trim();

    // Error patterns
    if trimmed.contains("error[E") || trimmed.contains("Error:") || trimmed.contains("FAILED") {
        return (EventType::Error, truncate_summary(trimmed));
    }

    // Permission patterns
    if trimmed.contains("Allow") && (trimmed.contains("?") || trimmed.contains("(y/n)")) {
        return (
            EventType::PermissionRequest,
            truncate_summary(trimmed),
        );
    }

    // Tool call patterns
    if trimmed.starts_with("$ ") || trimmed.starts_with("Running: ") {
        return (EventType::ToolCall, truncate_summary(trimmed));
    }

    // File change patterns
    if trimmed.contains("Created file") || trimmed.contains("Modified file") || trimmed.contains("Wrote to") {
        return (EventType::FileChange, truncate_summary(trimmed));
    }

    // Default: generic output
    (EventType::Output, truncate_summary(trimmed))
}

fn truncate_summary(s: &str) -> String {
    if s.len() <= 120 {
        s.to_string()
    } else {
        format!("{}...", &s[..117])
    }
}

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<TimelineEvent> {
    let type_str: String = row.get(3)?;
    let event_type = serde_json::from_value(serde_json::Value::String(type_str.clone()))
        .unwrap_or(EventType::Output);

    Ok(TimelineEvent {
        id: row.get(0)?,
        task_id: row.get(1)?,
        session_id: row.get(2)?,
        event_type,
        summary: row.get(4)?,
        content: row.get(5)?,
        metadata: row.get(6)?,
        timestamp: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate(&conn).unwrap();
        conn
    }

    // ── Migration ─────────────────────────────────────────────────

    #[test]
    fn migrate_creates_table() {
        let conn = setup_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='session_events'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn migrate_idempotent() {
        let conn = setup_db();
        migrate(&conn).unwrap();
    }

    // ── Recording events ──────────────────────────────────────────

    #[test]
    fn record_and_get_event() {
        let conn = setup_db();
        let id = record_event(
            &conn, 1, 1,
            &EventType::SessionStart,
            "Session started",
            "Agent claude-code started",
            None,
        ).unwrap();
        assert!(id > 0);

        let timeline = get_timeline(&conn, 1).unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].event_type, EventType::SessionStart);
        assert_eq!(timeline[0].summary, "Session started");
    }

    #[test]
    fn multiple_events_ordered() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::SessionStart, "Start", "", None).unwrap();
        record_event(&conn, 1, 1, &EventType::ToolCall, "Running tests", "cargo test", None).unwrap();
        record_event(&conn, 1, 1, &EventType::Output, "Tests passed", "test result: ok", None).unwrap();
        record_event(&conn, 1, 1, &EventType::SessionEnd, "Done", "", None).unwrap();

        let timeline = get_timeline(&conn, 1).unwrap();
        assert_eq!(timeline.len(), 4);
        assert_eq!(timeline[0].event_type, EventType::SessionStart);
        assert_eq!(timeline[3].event_type, EventType::SessionEnd);
    }

    #[test]
    fn events_isolated_by_task() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::Output, "Task 1", "", None).unwrap();
        record_event(&conn, 2, 1, &EventType::Output, "Task 2", "", None).unwrap();

        let t1 = get_timeline(&conn, 1).unwrap();
        let t2 = get_timeline(&conn, 2).unwrap();
        assert_eq!(t1.len(), 1);
        assert_eq!(t2.len(), 1);
        assert_eq!(t1[0].summary, "Task 1");
        assert_eq!(t2[0].summary, "Task 2");
    }

    // ── Querying ──────────────────────────────────────────────────

    #[test]
    fn get_events_by_type_filters() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::ToolCall, "Tool", "cargo test", None).unwrap();
        record_event(&conn, 1, 1, &EventType::Output, "Output", "ok", None).unwrap();
        record_event(&conn, 1, 1, &EventType::ToolCall, "Tool2", "cargo build", None).unwrap();

        let tools = get_events_by_type(&conn, 1, &EventType::ToolCall).unwrap();
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn search_events_finds_content() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::Output, "Test run", "test result: ok. 42 passed", None).unwrap();
        record_event(&conn, 1, 1, &EventType::Error, "Build error", "error[E0308]: mismatched types", None).unwrap();

        let results = search_events(&conn, 1, "error").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_type, EventType::Error);
    }

    #[test]
    fn search_events_finds_summary() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::FileChange, "Modified auth.rs", "", None).unwrap();
        record_event(&conn, 1, 1, &EventType::Output, "Done", "", None).unwrap();

        let results = search_events(&conn, 1, "auth").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn event_count_tracks() {
        let conn = setup_db();
        assert_eq!(event_count(&conn, 1).unwrap(), 0);
        record_event(&conn, 1, 1, &EventType::Output, "A", "", None).unwrap();
        record_event(&conn, 1, 1, &EventType::Output, "B", "", None).unwrap();
        assert_eq!(event_count(&conn, 1).unwrap(), 2);
    }

    // ── Output classification ─────────────────────────────────────

    #[test]
    fn classify_error_output() {
        let (t, _) = classify_output("error[E0308]: mismatched types");
        assert_eq!(t, EventType::Error);
    }

    #[test]
    fn classify_permission_request() {
        let (t, _) = classify_output("Allow bash tool? (y/n)");
        assert_eq!(t, EventType::PermissionRequest);
    }

    #[test]
    fn classify_tool_call() {
        let (t, _) = classify_output("$ cargo test --lib");
        assert_eq!(t, EventType::ToolCall);
    }

    #[test]
    fn classify_file_change() {
        let (t, _) = classify_output("Created file src/auth.rs");
        assert_eq!(t, EventType::FileChange);
    }

    #[test]
    fn classify_generic_output() {
        let (t, _) = classify_output("All tests passed successfully");
        assert_eq!(t, EventType::Output);
    }

    #[test]
    fn truncate_long_summary() {
        let long = "a".repeat(200);
        let (_, summary) = classify_output(&long);
        assert!(summary.len() <= 120);
        assert!(summary.ends_with("..."));
    }

    // ── Event type display ────────────────────────────────────────

    #[test]
    fn event_type_display() {
        assert_eq!(EventType::SessionStart.to_string(), "session_start");
        assert_eq!(EventType::ToolCall.to_string(), "tool_call");
        assert_eq!(EventType::Error.to_string(), "error");
    }

    #[test]
    fn event_type_serde_roundtrip() {
        let types = vec![
            EventType::SessionStart, EventType::SessionEnd,
            EventType::Input, EventType::Output,
            EventType::ToolCall, EventType::ToolResult,
            EventType::LlmCall, EventType::Error,
            EventType::PermissionRequest, EventType::PermissionResolve,
            EventType::FileChange,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let parsed: EventType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, t);
        }
    }

    #[test]
    fn metadata_stored_and_retrieved() {
        let conn = setup_db();
        let meta = r#"{"tool":"bash","args":"cargo test"}"#;
        record_event(&conn, 1, 1, &EventType::ToolCall, "Test", "cargo test", Some(meta)).unwrap();

        let timeline = get_timeline(&conn, 1).unwrap();
        assert_eq!(timeline[0].metadata.as_deref(), Some(meta));
    }

    // ── Session duration ─────────────────────────────────────────

    #[test]
    fn session_duration_no_events() {
        let conn = setup_db();
        let duration = session_duration(&conn, 999).unwrap();
        assert!(duration.is_none() || duration == Some(0.0));
    }

    #[test]
    fn session_duration_single_event() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::SessionStart, "Start", "", None).unwrap();
        let duration = session_duration(&conn, 1).unwrap();
        // With one event, min == max, so duration = 0
        assert!(duration.is_some());
        assert!((duration.unwrap() - 0.0).abs() < 1.0);
    }

    // ── Output classification edge cases ─────────────────────────

    #[test]
    fn classify_error_with_capital_error() {
        let (t, _) = classify_output("Error: something went wrong");
        assert_eq!(t, EventType::Error);
    }

    #[test]
    fn classify_error_with_failed() {
        let (t, _) = classify_output("test result: FAILED. 3 passed; 1 failed");
        assert_eq!(t, EventType::Error);
    }

    #[test]
    fn classify_running_prefix() {
        let (t, _) = classify_output("Running: npm test");
        assert_eq!(t, EventType::ToolCall);
    }

    #[test]
    fn classify_modified_file() {
        let (t, _) = classify_output("Modified file src/lib.rs");
        assert_eq!(t, EventType::FileChange);
    }

    #[test]
    fn classify_wrote_to() {
        let (t, _) = classify_output("Wrote to src/new.rs");
        assert_eq!(t, EventType::FileChange);
    }

    #[test]
    fn classify_whitespace_only() {
        let (t, summary) = classify_output("   \n\t  ");
        assert_eq!(t, EventType::Output);
        assert!(summary.is_empty());
    }

    #[test]
    fn classify_empty_string() {
        let (t, summary) = classify_output("");
        assert_eq!(t, EventType::Output);
        assert!(summary.is_empty());
    }

    #[test]
    fn truncate_exactly_120_chars() {
        let s = "a".repeat(120);
        let (_, summary) = classify_output(&s);
        assert_eq!(summary.len(), 120);
        assert!(!summary.ends_with("..."));
    }

    #[test]
    fn truncate_121_chars() {
        let s = "b".repeat(121);
        let (_, summary) = classify_output(&s);
        assert!(summary.len() <= 120);
        assert!(summary.ends_with("..."));
    }

    // ── Event type display all variants ──────────────────────────

    #[test]
    fn event_type_display_all() {
        assert_eq!(EventType::SessionEnd.to_string(), "session_end");
        assert_eq!(EventType::Input.to_string(), "input");
        assert_eq!(EventType::Output.to_string(), "output");
        assert_eq!(EventType::ToolResult.to_string(), "tool_result");
        assert_eq!(EventType::LlmCall.to_string(), "llm_call");
        assert_eq!(EventType::PermissionRequest.to_string(), "permission_request");
        assert_eq!(EventType::PermissionResolve.to_string(), "permission_resolve");
        assert_eq!(EventType::FileChange.to_string(), "file_change");
    }

    // ── row_to_event unknown type ────────────────────────────────

    #[test]
    fn unknown_event_type_defaults_to_output() {
        let conn = setup_db();
        conn.execute(
            "INSERT INTO session_events (task_id, session_id, event_type, summary, content, timestamp)
             VALUES (1, 1, 'unknown_type', 'test', '', datetime('now'))",
            [],
        )
        .unwrap();
        let timeline = get_timeline(&conn, 1).unwrap();
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline[0].event_type, EventType::Output);
    }

    // ── No metadata ──────────────────────────────────────────────

    #[test]
    fn metadata_none_retrieved_as_none() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::Output, "Test", "", None).unwrap();
        let timeline = get_timeline(&conn, 1).unwrap();
        assert!(timeline[0].metadata.is_none());
    }

    // ── Search finds nothing ─────────────────────────────────────

    #[test]
    fn search_events_no_match() {
        let conn = setup_db();
        record_event(&conn, 1, 1, &EventType::Output, "Hello", "World", None).unwrap();
        let results = search_events(&conn, 1, "zzzznotfound").unwrap();
        assert!(results.is_empty());
    }
}

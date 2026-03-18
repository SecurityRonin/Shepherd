use anyhow::Result;
use rusqlite::{params, Connection};
use super::package::{ContextFeedback, ContextPackage};

/// Create context tracking tables if they don't exist.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS context_packages (
            id TEXT PRIMARY KEY,
            task_id INTEGER,
            items_json TEXT NOT NULL,
            mcp_queries_json TEXT NOT NULL DEFAULT '[]',
            summary TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS context_feedback (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            package_id TEXT NOT NULL,
            task_id INTEGER NOT NULL,
            task_succeeded INTEGER NOT NULL DEFAULT 0,
            items_used_json TEXT NOT NULL DEFAULT '[]',
            agent_duration_secs REAL,
            notes TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_context_feedback_package
            ON context_feedback(package_id);
        CREATE INDEX IF NOT EXISTS idx_context_feedback_task
            ON context_feedback(task_id);
        ",
    )?;
    Ok(())
}

/// Save a context package to the database.
pub fn save_package(conn: &Connection, pkg: &ContextPackage) -> Result<()> {
    let items_json = serde_json::to_string(&pkg.items)?;
    let queries_json = serde_json::to_string(&pkg.mcp_queries)?;
    conn.execute(
        "INSERT INTO context_packages (id, task_id, items_json, mcp_queries_json, summary, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            pkg.id,
            pkg.task_id,
            items_json,
            queries_json,
            pkg.summary,
            pkg.created_at,
        ],
    )?;
    Ok(())
}

/// Load a context package by ID.
pub fn load_package(conn: &Connection, id: &str) -> Result<Option<ContextPackage>> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, items_json, mcp_queries_json, summary, created_at
         FROM context_packages WHERE id = ?1",
    )?;

    let result = stmt.query_row(params![id], |row| {
        let items_json: String = row.get(2)?;
        let queries_json: String = row.get(3)?;
        Ok(ContextPackage {
            id: row.get(0)?,
            task_id: row.get(1)?,
            items: serde_json::from_str(&items_json).unwrap_or_default(),
            mcp_queries: serde_json::from_str(&queries_json).unwrap_or_default(),
            summary: row.get(4)?,
            created_at: row.get(5)?,
        })
    });

    match result {
        Ok(pkg) => Ok(Some(pkg)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        // tarpaulin-start-ignore
        Err(e) => Err(e.into()),
        // tarpaulin-stop-ignore
    }
}

/// Record feedback on how effective a context package was.
pub fn record_feedback(conn: &Connection, feedback: &ContextFeedback) -> Result<i64> {
    let items_used_json = serde_json::to_string(&feedback.items_used)?;
    conn.execute(
        "INSERT INTO context_feedback (package_id, task_id, task_succeeded, items_used_json, agent_duration_secs, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            feedback.package_id,
            feedback.task_id,
            feedback.task_succeeded as i32,
            items_used_json,
            feedback.agent_duration_secs,
            feedback.notes,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Get the success rate for context items (files) across all feedback.
///
/// Returns (file_path, times_suggested, times_used, success_rate) tuples
/// for files that have been suggested at least `min_suggestions` times.
pub fn get_effectiveness(
    conn: &Connection,
    min_suggestions: usize,
) -> Result<Vec<FileEffectiveness>> {
    // Get all packages and their feedback
    let mut stmt = conn.prepare(
        "SELECT cp.items_json, cf.items_used_json, cf.task_succeeded
         FROM context_packages cp
         JOIN context_feedback cf ON cf.package_id = cp.id",
    )?;

    let mut file_stats: std::collections::HashMap<String, (u32, u32, u32)> =
        std::collections::HashMap::new();

    let rows = stmt.query_map([], |row| {
        let items_json: String = row.get(0)?;
        let used_json: String = row.get(1)?;
        let succeeded: bool = row.get::<_, i32>(2)? != 0;
        Ok((items_json, used_json, succeeded))
    })?;

    for row in rows {
        let (items_json, used_json, succeeded) = row?;
        let items: Vec<super::package::ContextItem> =
            serde_json::from_str(&items_json).unwrap_or_default();
        let used: Vec<String> = serde_json::from_str(&used_json).unwrap_or_default();

        for item in &items {
            let path_str = item.file_path.to_string_lossy().to_string();
            let entry = file_stats.entry(path_str.clone()).or_insert((0, 0, 0));
            entry.0 += 1; // times suggested
            if used.contains(&path_str) {
                entry.1 += 1; // times used
                if succeeded {
                    entry.2 += 1; // times used in successful tasks
                }
            }
        }
    }

    let mut results: Vec<FileEffectiveness> = file_stats
        .into_iter()
        .filter(|(_, (suggested, _, _))| *suggested >= min_suggestions as u32)
        .map(|(path, (suggested, used, succeeded))| {
            let usage_rate = if suggested > 0 {
                used as f64 / suggested as f64
            } else {
                0.0 // tarpaulin-start-ignore
            }; // tarpaulin-stop-ignore
            let success_rate = if used > 0 {
                succeeded as f64 / used as f64
            } else {
                0.0
            };
            FileEffectiveness {
                file_path: path,
                times_suggested: suggested,
                times_used: used,
                times_succeeded: succeeded,
                usage_rate,
                success_rate,
            }
        })
        .collect();

    results.sort_by(|a, b| b.success_rate.partial_cmp(&a.success_rate).unwrap());
    Ok(results)
}

/// Effectiveness statistics for a single file across context packages.
#[derive(Debug, Clone)]
pub struct FileEffectiveness {
    pub file_path: String,
    pub times_suggested: u32,
    pub times_used: u32,
    pub times_succeeded: u32,
    pub usage_rate: f64,
    pub success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::package::*;
    use std::path::PathBuf;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn sample_package() -> ContextPackage {
        ContextPackage {
            id: "pkg-test-001".into(),
            task_id: Some(1),
            items: vec![
                ContextItem {
                    source: ContextSource::FileReference,
                    file_path: PathBuf::from("src/auth.rs"),
                    relevance_score: 1.0,
                    reason: "Directly mentioned".into(),
                },
                ContextItem {
                    source: ContextSource::Structural,
                    file_path: PathBuf::from("src/db.rs"),
                    relevance_score: 0.7,
                    reason: "Imported by auth.rs".into(),
                },
            ],
            mcp_queries: vec![McpQuery {
                server: "serena".into(),
                tool: "find_symbol".into(),
                params: serde_json::json!({"name": "AuthService"}),
                reason: "Find AuthService".into(),
            }],
            summary: "Context for auth fix".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
        }
    }

    // ── Migration tests ──────────────────────────────────────────

    #[test]
    fn migrate_creates_tables() {
        let conn = setup_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('context_packages', 'context_feedback')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn migrate_idempotent() {
        let conn = setup_db();
        // Second migration should not fail
        migrate(&conn).unwrap();
    }

    // ── Package CRUD tests ───────────────────────────────────────

    #[test]
    fn save_and_load_package() {
        let conn = setup_db();
        let pkg = sample_package();
        save_package(&conn, &pkg).unwrap();

        let loaded = load_package(&conn, "pkg-test-001").unwrap().unwrap();
        assert_eq!(loaded.id, "pkg-test-001");
        assert_eq!(loaded.task_id, Some(1));
        assert_eq!(loaded.items.len(), 2);
        assert_eq!(loaded.mcp_queries.len(), 1);
        assert_eq!(loaded.summary, "Context for auth fix");
    }

    #[test]
    fn load_nonexistent_package_returns_none() {
        let conn = setup_db();
        let result = load_package(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn save_duplicate_package_fails() {
        let conn = setup_db();
        let pkg = sample_package();
        save_package(&conn, &pkg).unwrap();
        assert!(save_package(&conn, &pkg).is_err());
    }

    // ── Feedback tests ───────────────────────────────────────────

    #[test]
    fn record_and_query_feedback() {
        let conn = setup_db();
        let pkg = sample_package();
        save_package(&conn, &pkg).unwrap();

        let feedback = ContextFeedback {
            package_id: "pkg-test-001".into(),
            task_id: 1,
            task_succeeded: true,
            items_used: vec!["src/auth.rs".into()],
            agent_duration_secs: Some(45.0),
            notes: Some("Fixed quickly".into()),
        };
        let id = record_feedback(&conn, &feedback).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn multiple_feedback_entries() {
        let conn = setup_db();
        let pkg = sample_package();
        save_package(&conn, &pkg).unwrap();

        let fb1 = ContextFeedback {
            package_id: "pkg-test-001".into(),
            task_id: 1,
            task_succeeded: true,
            items_used: vec!["src/auth.rs".into()],
            agent_duration_secs: Some(30.0),
            notes: None,
        };
        let fb2 = ContextFeedback {
            package_id: "pkg-test-001".into(),
            task_id: 1,
            task_succeeded: false,
            items_used: vec!["src/db.rs".into()],
            agent_duration_secs: Some(120.0),
            notes: Some("Agent got stuck".into()),
        };
        record_feedback(&conn, &fb1).unwrap();
        record_feedback(&conn, &fb2).unwrap();
    }

    // ── Effectiveness tests ──────────────────────────────────────

    #[test]
    fn effectiveness_calculates_rates() {
        let conn = setup_db();

        // Create two packages with different items
        let pkg1 = ContextPackage {
            id: "pkg-eff-001".into(),
            task_id: Some(1),
            items: vec![
                ContextItem {
                    source: ContextSource::FileReference,
                    file_path: PathBuf::from("src/auth.rs"),
                    relevance_score: 1.0,
                    reason: "test".into(),
                },
                ContextItem {
                    source: ContextSource::Semantic,
                    file_path: PathBuf::from("src/config.rs"),
                    relevance_score: 0.5,
                    reason: "test".into(),
                },
            ],
            mcp_queries: vec![],
            summary: "test".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
        };

        let pkg2 = ContextPackage {
            id: "pkg-eff-002".into(),
            task_id: Some(2),
            items: vec![ContextItem {
                source: ContextSource::FileReference,
                file_path: PathBuf::from("src/auth.rs"),
                relevance_score: 1.0,
                reason: "test".into(),
            }],
            mcp_queries: vec![],
            summary: "test".into(),
            created_at: "2026-03-13T01:00:00Z".into(),
        };

        save_package(&conn, &pkg1).unwrap();
        save_package(&conn, &pkg2).unwrap();

        // Feedback: auth.rs used both times, succeeded once
        record_feedback(&conn, &ContextFeedback {
            package_id: "pkg-eff-001".into(),
            task_id: 1,
            task_succeeded: true,
            items_used: vec!["src/auth.rs".into()],
            agent_duration_secs: None,
            notes: None,
        }).unwrap();

        record_feedback(&conn, &ContextFeedback {
            package_id: "pkg-eff-002".into(),
            task_id: 2,
            task_succeeded: false,
            items_used: vec!["src/auth.rs".into()],
            agent_duration_secs: None,
            notes: None,
        }).unwrap();

        let results = get_effectiveness(&conn, 1).unwrap();

        // auth.rs: suggested 2x, used 2x, succeeded 1x
        let auth = results.iter().find(|r| r.file_path == "src/auth.rs").unwrap();
        assert_eq!(auth.times_suggested, 2);
        assert_eq!(auth.times_used, 2);
        assert_eq!(auth.times_succeeded, 1);
        assert!((auth.usage_rate - 1.0).abs() < f64::EPSILON);
        assert!((auth.success_rate - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn effectiveness_min_suggestions_filter() {
        let conn = setup_db();

        let pkg = ContextPackage {
            id: "pkg-min-001".into(),
            task_id: Some(1),
            items: vec![ContextItem {
                source: ContextSource::Semantic,
                file_path: PathBuf::from("src/rare.rs"),
                relevance_score: 0.3,
                reason: "test".into(),
            }],
            mcp_queries: vec![],
            summary: "test".into(),
            created_at: "2026-03-13T00:00:00Z".into(),
        };
        save_package(&conn, &pkg).unwrap();
        record_feedback(&conn, &ContextFeedback {
            package_id: "pkg-min-001".into(),
            task_id: 1,
            task_succeeded: true,
            items_used: vec!["src/rare.rs".into()],
            agent_duration_secs: None,
            notes: None,
        }).unwrap();

        // With min_suggestions=1, should appear
        let results = get_effectiveness(&conn, 1).unwrap();
        assert!(results.iter().any(|r| r.file_path == "src/rare.rs"));

        // With min_suggestions=5, should be filtered out
        let results = get_effectiveness(&conn, 5).unwrap();
        assert!(!results.iter().any(|r| r.file_path == "src/rare.rs"));
    }

    #[test]
    fn effectiveness_empty_db() {
        let conn = setup_db();
        let results = get_effectiveness(&conn, 1).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn save_and_load_package_with_no_task_id() {
        let conn = setup_db();
        let pkg = ContextPackage {
            id: "pkg-no-task".into(),
            task_id: None,
            items: vec![],
            mcp_queries: vec![],
            summary: "No task".into(),
            created_at: "2026-03-14T00:00:00Z".into(),
        };
        save_package(&conn, &pkg).unwrap();
        let loaded = load_package(&conn, "pkg-no-task").unwrap().unwrap();
        assert!(loaded.task_id.is_none());
        assert_eq!(loaded.summary, "No task");
    }

    #[test]
    fn record_feedback_with_empty_items_used() {
        let conn = setup_db();
        let pkg = sample_package();
        save_package(&conn, &pkg).unwrap();

        let feedback = ContextFeedback {
            package_id: "pkg-test-001".into(),
            task_id: 1,
            task_succeeded: false,
            items_used: vec![], // no items used
            agent_duration_secs: None,
            notes: None,
        };
        let id = record_feedback(&conn, &feedback).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn file_effectiveness_clone_and_debug() {
        let eff = FileEffectiveness {
            file_path: "src/auth.rs".into(),
            times_suggested: 3,
            times_used: 2,
            times_succeeded: 1,
            usage_rate: 0.667,
            success_rate: 0.5,
        };
        let cloned = eff.clone();
        assert_eq!(cloned.file_path, "src/auth.rs");
        // Debug formatting should not panic
        let _ = format!("{:?}", cloned);
    }

    #[test]
    fn load_package_preserves_items_and_queries() {
        let conn = setup_db();
        let pkg = sample_package();
        save_package(&conn, &pkg).unwrap();

        let loaded = load_package(&conn, "pkg-test-001").unwrap().unwrap();
        assert_eq!(loaded.items.len(), 2);
        assert_eq!(loaded.mcp_queries.len(), 1);
        assert_eq!(loaded.mcp_queries[0].server, "serena");
    }
}

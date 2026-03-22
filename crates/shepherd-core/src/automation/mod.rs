use anyhow::Result;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    pub id: i64,
    pub name: String,
    pub rule_type: String,
    pub pattern: String,
    pub scope: Option<String>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AutomationDecision {
    Approve,
    Reject,
}

pub struct AutomationEngine;

impl AutomationEngine {
    /// Evaluate a permission request against all enabled rules.
    /// Canonicalizes path to prevent traversal attacks (e.g., `src/../../etc/passwd`).
    /// Returns the first matching decision, or None if no rule matches.
    pub fn evaluate(
        conn: &Connection,
        tool: &str,
        path: &str,
        project_dir: &str,
    ) -> Result<Option<AutomationDecision>> {
        // Reject absolute paths — this engine only handles project-relative paths
        use std::path::Path;
        if Path::new(path).is_absolute() {
            return Ok(None);
        }

        // Canonicalize path to prevent traversal attacks (e.g., src/../../etc/passwd)
        let clean_path = Path::new(path);
        let mut components = Vec::new();
        for component in clean_path.components() {
            match component {
                std::path::Component::ParentDir => {
                    // Path traversal detected — strip the `..` and the preceding component
                    components.pop();
                }
                std::path::Component::Normal(c) => {
                    components.push(c.to_string_lossy().to_string());
                }
                _ => {}
            }
        }
        let canonical_path = components.join("/");

        // Filter by scope: rules with NULL scope match all projects,
        // rules with a scope only match that specific project_dir
        let mut stmt = conn.prepare(
            "SELECT rule_type, pattern FROM automation_rules WHERE enabled = 1 AND (scope IS NULL OR scope = ?1)",
        )?;

        let rules: Vec<(String, String)> = stmt
            .query_map(params![project_dir], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let request_str = format!("{}:{}", tool, canonical_path);

        for (rule_type, pattern) in &rules {
            if glob_match::glob_match(pattern, &request_str) {
                return Ok(Some(match rule_type.as_str() {
                    "auto_approve" => AutomationDecision::Approve,
                    "auto_reject" => AutomationDecision::Reject,
                    _ => continue,
                }));
            }
        }

        Ok(None)
    }

    /// List all automation rules.
    pub fn list_rules(conn: &Connection) -> Result<Vec<AutomationRule>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, rule_type, pattern, scope, enabled, created_at FROM automation_rules ORDER BY id",
        )?;
        let rules = stmt
            .query_map([], |row| {
                Ok(AutomationRule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    rule_type: row.get(2)?,
                    pattern: row.get(3)?,
                    scope: row.get(4)?,
                    enabled: row.get::<_, i64>(5)? == 1,
                    created_at: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rules)
    }

    /// Create a new automation rule. Returns the created rule.
    pub fn create_rule(
        conn: &Connection,
        name: &str,
        rule_type: &str,
        pattern: &str,
        scope: Option<&str>,
    ) -> Result<AutomationRule> {
        // Validate rule_type
        if rule_type != "auto_approve" && rule_type != "auto_reject" {
            anyhow::bail!(
                "Invalid rule_type '{}': must be 'auto_approve' or 'auto_reject'",
                rule_type
            );
        }

        conn.execute(
            "INSERT INTO automation_rules (name, rule_type, pattern, scope) VALUES (?1, ?2, ?3, ?4)",
            params![name, rule_type, pattern, scope],
        )?;
        let id = conn.last_insert_rowid();
        let rule = conn.query_row(
            "SELECT id, name, rule_type, pattern, scope, enabled, created_at FROM automation_rules WHERE id = ?1",
            params![id],
            |row| {
                Ok(AutomationRule {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    rule_type: row.get(2)?,
                    pattern: row.get(3)?,
                    scope: row.get(4)?,
                    enabled: row.get::<_, i64>(5)? == 1,
                    created_at: row.get(6)?,
                })
            },
        )?;
        Ok(rule)
    }

    /// Delete a rule by ID. Returns true if a row was deleted.
    pub fn delete_rule(conn: &Connection, id: i64) -> Result<bool> {
        let affected = conn.execute("DELETE FROM automation_rules WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_memory;

    #[test]
    fn create_rule_and_list() {
        let conn = open_memory().unwrap();
        let rule = AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();
        assert_eq!(rule.name, "Allow src reads");
        assert_eq!(rule.rule_type, "auto_approve");
        assert!(rule.enabled);

        let rules = AutomationEngine::list_rules(&conn).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, rule.id);
    }

    #[test]
    fn auto_approve_matches_pattern() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();

        let decision =
            AutomationEngine::evaluate(&conn, "read_file", "src/main.rs", "/project").unwrap();
        assert_eq!(decision, Some(AutomationDecision::Approve));
    }

    #[test]
    fn auto_reject_matches() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(&conn, "Block bash", "auto_reject", "bash:**", None).unwrap();

        let decision = AutomationEngine::evaluate(&conn, "bash", "rm -rf /", "/project").unwrap();
        assert_eq!(decision, Some(AutomationDecision::Reject));
    }

    #[test]
    fn no_rule_matches_returns_none() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();

        let decision = AutomationEngine::evaluate(&conn, "bash", "ls", "/project").unwrap();
        assert_eq!(decision, None);
    }

    #[test]
    fn delete_rule_removes_it() {
        let conn = open_memory().unwrap();
        let rule =
            AutomationEngine::create_rule(&conn, "Temp rule", "auto_approve", "read_file:**", None)
                .unwrap();

        assert!(AutomationEngine::delete_rule(&conn, rule.id).unwrap());
        assert!(AutomationEngine::list_rules(&conn).unwrap().is_empty());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let conn = open_memory().unwrap();
        assert!(!AutomationEngine::delete_rule(&conn, 99999).unwrap());
    }

    #[test]
    fn auto_approve_rejects_traversal() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads",
            "auto_approve",
            "read_file:src/**",
            None,
        )
        .unwrap();

        // Path traversal: src/../../etc/passwd should NOT match src/**
        let decision =
            AutomationEngine::evaluate(&conn, "read_file", "src/../../etc/passwd", "/project")
                .unwrap();
        assert_eq!(decision, None);
    }

    #[test]
    fn absolute_path_returns_none() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow etc reads",
            "auto_approve",
            "read_file:etc/**",
            None,
        )
        .unwrap();

        // Absolute path /etc/passwd should NOT match even though etc/** would match etc/passwd
        let decision =
            AutomationEngine::evaluate(&conn, "read_file", "/etc/passwd", "/project").unwrap();
        assert_eq!(decision, None);
    }

    #[test]
    fn create_rule_rejects_invalid_type() {
        let conn = open_memory().unwrap();
        let result = AutomationEngine::create_rule(
            &conn,
            "Bad rule",
            "allow", // invalid — should be "auto_approve"
            "read_file:src/**",
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn scope_restricts_rule_to_project() {
        let conn = open_memory().unwrap();
        AutomationEngine::create_rule(
            &conn,
            "Allow src reads for project-a",
            "auto_approve",
            "read_file:src/**",
            Some("/projects/a"),
        )
        .unwrap();

        // Should match when project_dir matches scope
        let decision =
            AutomationEngine::evaluate(&conn, "read_file", "src/main.rs", "/projects/a").unwrap();
        assert_eq!(decision, Some(AutomationDecision::Approve));

        // Should NOT match when project_dir differs from scope
        let decision =
            AutomationEngine::evaluate(&conn, "read_file", "src/main.rs", "/projects/b").unwrap();
        assert_eq!(decision, None);
    }
}

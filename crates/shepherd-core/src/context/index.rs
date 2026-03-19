//! Persistent file index for fast codebase context lookups.
//!
//! Stores file metadata (path, language, content hash, timestamps)
//! in SQLite so the context orchestrator can skip re-scanning
//! unchanged files on every request.

use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// A single indexed file's metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexedFile {
    pub file_path: String,
    pub language: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub last_modified: String,
    pub indexed_at: String,
}

/// Create the file_index table if it doesn't exist.
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS file_index (
            file_path TEXT PRIMARY KEY,
            language TEXT NOT NULL DEFAULT '',
            content_hash TEXT NOT NULL DEFAULT '',
            size_bytes INTEGER NOT NULL DEFAULT 0,
            last_modified TEXT NOT NULL DEFAULT '',
            indexed_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_file_index_language
            ON file_index(language);
        ",
    )?;
    Ok(())
}

/// Detect language from file extension.
pub fn detect_language(path: &str) -> &str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "rb" => "ruby",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" | "cxx" => "cpp",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "md" => "markdown",
        "sql" => "sql",
        "html" | "htm" => "html",
        "css" | "scss" | "sass" => "css",
        "vue" => "vue",
        "svelte" => "svelte",
        "sh" | "bash" | "zsh" => "shell",
        _ => "unknown",
    }
}

/// Compute a simple hash of file content for change detection.
/// Uses a fast non-cryptographic approach (FNV-style).
pub fn content_hash(content: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in content {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Upsert a file into the index.
pub fn upsert_file(conn: &Connection, file: &IndexedFile) -> Result<()> {
    conn.execute(
        "INSERT INTO file_index (file_path, language, content_hash, size_bytes, last_modified, indexed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(file_path) DO UPDATE SET
            language = excluded.language,
            content_hash = excluded.content_hash,
            size_bytes = excluded.size_bytes,
            last_modified = excluded.last_modified,
            indexed_at = excluded.indexed_at",
        params![
            file.file_path,
            file.language,
            file.content_hash,
            file.size_bytes as i64,
            file.last_modified,
            file.indexed_at,
        ],
    )?;
    Ok(())
}

/// Get a file's index entry by path.
pub fn get_file(conn: &Connection, file_path: &str) -> Result<Option<IndexedFile>> {
    let mut stmt = conn.prepare(
        "SELECT file_path, language, content_hash, size_bytes, last_modified, indexed_at
         FROM file_index WHERE file_path = ?1",
    )?;

    let result = stmt.query_row(params![file_path], |row| {
        Ok(IndexedFile {
            file_path: row.get(0)?,
            language: row.get(1)?,
            content_hash: row.get(2)?,
            size_bytes: row.get::<_, i64>(3)? as u64,
            last_modified: row.get(4)?,
            indexed_at: row.get(5)?,
        })
    });

    match result {
        Ok(f) => Ok(Some(f)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        // tarpaulin-start-ignore
        Err(e) => Err(e.into()),
        // tarpaulin-stop-ignore
    }
}

/// Get all indexed files for a given language.
pub fn get_files_by_language(conn: &Connection, language: &str) -> Result<Vec<IndexedFile>> {
    let mut stmt = conn.prepare(
        "SELECT file_path, language, content_hash, size_bytes, last_modified, indexed_at
         FROM file_index WHERE language = ?1 ORDER BY file_path",
    )?;

    let files = stmt
        .query_map(params![language], |row| {
            Ok(IndexedFile {
                file_path: row.get(0)?,
                language: row.get(1)?,
                content_hash: row.get(2)?,
                size_bytes: row.get::<_, i64>(3)? as u64,
                last_modified: row.get(4)?,
                indexed_at: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(files)
}

/// Get total number of indexed files.
pub fn file_count(conn: &Connection) -> Result<u64> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM file_index", [], |row| row.get(0))?;
    Ok(count as u64)
}

/// Remove a file from the index.
pub fn remove_file(conn: &Connection, file_path: &str) -> Result<bool> {
    let affected = conn.execute(
        "DELETE FROM file_index WHERE file_path = ?1",
        params![file_path],
    )?;
    Ok(affected > 0)
}

/// Scan a directory and update the index with any new or changed files.
/// Returns the number of files that were added or updated.
pub fn scan_and_index(conn: &Connection, repo_path: &Path) -> Result<u64> {
    let mut updated = 0u64;

    let walker = walkdir(repo_path);
    for entry in walker {
        let abs_path = entry.as_path();
        let rel_path = abs_path
            .strip_prefix(repo_path)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .to_string();

        // Skip non-code files
        let lang = detect_language(&rel_path).to_string();
        if lang == "unknown" {
            continue;
        }

        // Read file content for hashing
        let content = match std::fs::read(abs_path) {
            Ok(c) => c,
            Err(_) => continue, // tarpaulin-start-ignore
        }; // tarpaulin-stop-ignore

        let hash = content_hash(&content);

        // Check if already indexed with same hash
        if let Ok(Some(existing)) = get_file(conn, &rel_path) {
            if existing.content_hash == hash {
                continue; // No change
            }
        }

        let metadata = std::fs::metadata(abs_path).ok();
        let last_modified = metadata
            .and_then(|m| m.modified().ok())
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_default();

        let file = IndexedFile {
            file_path: rel_path,
            language: lang,
            content_hash: hash,
            size_bytes: content.len() as u64,
            last_modified,
            indexed_at: chrono::Utc::now().to_rfc3339(),
        };

        upsert_file(conn, &file)?;
        updated += 1;
    }

    Ok(updated)
}

/// Simple recursive directory walk that skips hidden dirs and common non-code dirs.
fn walkdir(root: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    walk_recursive(root, &mut result);
    result
}

fn walk_recursive(dir: &Path, result: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // tarpaulin-start-ignore
    }; // tarpaulin-stop-ignore

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden dirs, node_modules, target, .git, etc.
        if name_str.starts_with('.')
            || name_str == "node_modules"
            || name_str == "target"
            || name_str == "__pycache__"
            || name_str == "vendor"
            || name_str == "dist"
            || name_str == "build"
        {
            continue;
        }

        if path.is_dir() {
            walk_recursive(&path, result);
        } else if path.is_file() {
            result.push(path);
        }
    }
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

    fn sample_file(path: &str, lang: &str) -> IndexedFile {
        IndexedFile {
            file_path: path.to_string(),
            language: lang.to_string(),
            content_hash: content_hash(b"sample content"),
            size_bytes: 14,
            last_modified: chrono::Utc::now().to_rfc3339(),
            indexed_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    // ── Migration ─────────────────────────────────────────────────

    #[test]
    fn migrate_creates_table() {
        let conn = setup_db();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='file_index'",
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

    // ── Language detection ─────────────────────────────────────────

    #[test]
    fn detect_rust() {
        assert_eq!(detect_language("src/main.rs"), "rust");
    }

    #[test]
    fn detect_typescript() {
        assert_eq!(detect_language("app/page.tsx"), "typescript");
        assert_eq!(detect_language("lib/utils.ts"), "typescript");
    }

    #[test]
    fn detect_python() {
        assert_eq!(detect_language("script.py"), "python");
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(detect_language("README"), "unknown");
        assert_eq!(detect_language("Makefile"), "unknown");
    }

    // ── Content hashing ───────────────────────────────────────────

    #[test]
    fn hash_deterministic() {
        let h1 = content_hash(b"hello world");
        let h2 = content_hash(b"hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_different_content() {
        let h1 = content_hash(b"hello");
        let h2 = content_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn hash_format() {
        let h = content_hash(b"test");
        assert_eq!(h.len(), 16); // 16 hex chars = 64-bit
    }

    // ── CRUD operations ───────────────────────────────────────────

    #[test]
    fn upsert_and_get() {
        let conn = setup_db();
        let file = sample_file("src/main.rs", "rust");
        upsert_file(&conn, &file).unwrap();

        let loaded = get_file(&conn, "src/main.rs").unwrap().unwrap();
        assert_eq!(loaded.file_path, "src/main.rs");
        assert_eq!(loaded.language, "rust");
        assert_eq!(loaded.content_hash, file.content_hash);
    }

    #[test]
    fn upsert_updates_existing() {
        let conn = setup_db();
        let file = sample_file("src/main.rs", "rust");
        upsert_file(&conn, &file).unwrap();

        let updated = IndexedFile {
            content_hash: content_hash(b"new content"),
            size_bytes: 11,
            ..file
        };
        upsert_file(&conn, &updated).unwrap();

        let loaded = get_file(&conn, "src/main.rs").unwrap().unwrap();
        assert_eq!(loaded.content_hash, content_hash(b"new content"));
        assert_eq!(loaded.size_bytes, 11);
    }

    #[test]
    fn get_nonexistent() {
        let conn = setup_db();
        let result = get_file(&conn, "nonexistent.rs").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn remove_existing() {
        let conn = setup_db();
        upsert_file(&conn, &sample_file("src/main.rs", "rust")).unwrap();
        assert!(remove_file(&conn, "src/main.rs").unwrap());
        assert!(get_file(&conn, "src/main.rs").unwrap().is_none());
    }

    #[test]
    fn remove_nonexistent() {
        let conn = setup_db();
        assert!(!remove_file(&conn, "nonexistent.rs").unwrap());
    }

    // ── Query operations ──────────────────────────────────────────

    #[test]
    fn get_files_by_language_filters() {
        let conn = setup_db();
        upsert_file(&conn, &sample_file("src/main.rs", "rust")).unwrap();
        upsert_file(&conn, &sample_file("src/lib.rs", "rust")).unwrap();
        upsert_file(&conn, &sample_file("app/page.tsx", "typescript")).unwrap();

        let rust_files = get_files_by_language(&conn, "rust").unwrap();
        assert_eq!(rust_files.len(), 2);

        let ts_files = get_files_by_language(&conn, "typescript").unwrap();
        assert_eq!(ts_files.len(), 1);
    }

    #[test]
    fn file_count_tracks_entries() {
        let conn = setup_db();
        assert_eq!(file_count(&conn).unwrap(), 0);

        upsert_file(&conn, &sample_file("a.rs", "rust")).unwrap();
        upsert_file(&conn, &sample_file("b.rs", "rust")).unwrap();
        assert_eq!(file_count(&conn).unwrap(), 2);
    }

    // ── Scan and index ────────────────────────────────────────────

    #[test]
    fn scan_indexes_code_files() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(src.join("lib.rs"), "pub mod foo;").unwrap();
        std::fs::write(tmp.path().join("README"), "Hello").unwrap(); // unknown ext

        let conn = setup_db();
        let count = scan_and_index(&conn, tmp.path()).unwrap();
        assert_eq!(count, 2); // only .rs files

        assert_eq!(file_count(&conn).unwrap(), 2);
    }

    #[test]
    fn scan_skips_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();

        let conn = setup_db();
        let first = scan_and_index(&conn, tmp.path()).unwrap();
        assert_eq!(first, 1);

        let second = scan_and_index(&conn, tmp.path()).unwrap();
        assert_eq!(second, 0); // Already indexed, no changes
    }

    #[test]
    fn scan_detects_changes() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();

        let conn = setup_db();
        scan_and_index(&conn, tmp.path()).unwrap();

        // Modify file
        std::fs::write(
            tmp.path().join("main.rs"),
            "fn main() { println!(\"hi\"); }",
        )
        .unwrap();
        let updated = scan_and_index(&conn, tmp.path()).unwrap();
        assert_eq!(updated, 1);
    }

    #[test]
    fn scan_skips_hidden_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let hidden = tmp.path().join(".git");
        std::fs::create_dir_all(&hidden).unwrap();
        std::fs::write(hidden.join("config.rs"), "// git internal").unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();

        let conn = setup_db();
        let count = scan_and_index(&conn, tmp.path()).unwrap();
        assert_eq!(count, 1); // Only main.rs, not .git/config.rs
    }

    #[test]
    fn scan_skips_node_modules() {
        let tmp = tempfile::tempdir().unwrap();
        let nm = tmp.path().join("node_modules").join("react");
        std::fs::create_dir_all(&nm).unwrap();
        std::fs::write(nm.join("index.js"), "module.exports = {}").unwrap();
        std::fs::write(tmp.path().join("app.ts"), "const x = 1;").unwrap();

        let conn = setup_db();
        let count = scan_and_index(&conn, tmp.path()).unwrap();
        assert_eq!(count, 1); // Only app.ts
    }

    // ── Additional language detection ────────────────────────────

    #[test]
    fn detect_javascript() {
        assert_eq!(detect_language("src/index.js"), "javascript");
        assert_eq!(detect_language("app/component.jsx"), "javascript");
    }

    #[test]
    fn detect_go() {
        assert_eq!(detect_language("main.go"), "go");
    }

    #[test]
    fn detect_json() {
        assert_eq!(detect_language("package.json"), "json");
        assert_eq!(detect_language("tsconfig.json"), "json");
    }

    #[test]
    fn detect_sql() {
        assert_eq!(detect_language("migrations.sql"), "sql");
    }

    #[test]
    fn detect_yaml() {
        assert_eq!(detect_language("config.yaml"), "yaml");
        assert_eq!(detect_language("docker-compose.yml"), "yaml");
    }

    #[test]
    fn detect_ruby() {
        assert_eq!(detect_language("app/models/user.rb"), "ruby");
    }

    #[test]
    fn detect_shell() {
        assert_eq!(detect_language("scripts/deploy.sh"), "shell");
        assert_eq!(detect_language("setup.bash"), "shell");
        assert_eq!(detect_language("init.zsh"), "shell");
    }

    #[test]
    fn content_hash_empty_slice() {
        let h = content_hash(b"");
        // Should return a valid 16-char hex string (the FNV offset basis)
        assert_eq!(h.len(), 16);
    }

    #[test]
    fn file_count_after_remove() {
        let conn = setup_db();
        upsert_file(&conn, &sample_file("a.rs", "rust")).unwrap();
        upsert_file(&conn, &sample_file("b.rs", "rust")).unwrap();
        assert_eq!(file_count(&conn).unwrap(), 2);
        remove_file(&conn, "a.rs").unwrap();
        assert_eq!(file_count(&conn).unwrap(), 1);
    }

    #[test]
    fn get_files_by_language_empty_result() {
        let conn = setup_db();
        upsert_file(&conn, &sample_file("a.rs", "rust")).unwrap();
        let py_files = get_files_by_language(&conn, "python").unwrap();
        assert!(py_files.is_empty());
    }
}

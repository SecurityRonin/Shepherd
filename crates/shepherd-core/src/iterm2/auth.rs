use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Iterm2Auth {
    pub cookie: String,
    pub key: String,
}

/// Read auth credentials from the JSON file written by the AutoLaunch bridge script.
pub fn load_auth(path: &Path) -> anyhow::Result<Iterm2Auth> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("reading iTerm2 auth from {}", path.display()))?;
    serde_json::from_str(&contents).with_context(|| "parsing iTerm2 auth JSON")
}

/// Default path where the AutoLaunch bridge script writes credentials.
pub fn default_auth_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join(".shepherd")
        .join("iterm2-auth.json")
}

/// Spawn a background thread that watches the auth file for changes and sends
/// the new credentials over `tx`. Uses std::sync::mpsc to bridge the blocking
/// notify callback into the async world.
pub fn watch_auth(path: PathBuf, tx: mpsc::Sender<Iterm2Auth>) {
    std::thread::spawn(move || {
        use notify::{Event, RecursiveMode, Watcher};

        let (sync_tx, sync_rx) = std::sync::mpsc::channel::<notify::Result<Event>>();
        let mut watcher = match notify::RecommendedWatcher::new(
            move |res| {
                let _ = sync_tx.send(res);
            },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                tracing::warn!("iterm2 auth watcher init failed: {e}");
                return;
            }
        };
        if let Err(e) = watcher.watch(&path, RecursiveMode::NonRecursive) {
            tracing::warn!("iterm2 auth watcher watch failed: {e}");
            return;
        }
        for event in sync_rx {
            match event {
                Ok(_) => match load_auth(&path) {
                    Ok(auth) => {
                        // blocking_send is fine — we're on a dedicated std thread
                        let _ = tx.blocking_send(auth);
                    }
                    Err(e) => tracing::warn!("iterm2 auth reload failed: {e}"),
                },
                Err(e) => tracing::warn!("iterm2 auth watch event error: {e}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_auth_ok() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"cookie":"cook1","key":"key1"}}"#).unwrap();
        let auth = load_auth(f.path()).unwrap();
        assert_eq!(auth.cookie, "cook1");
        assert_eq!(auth.key, "key1");
    }

    #[test]
    fn test_load_auth_missing_file() {
        let result = load_auth(std::path::Path::new("/nonexistent/iterm2-auth.json"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_auth_invalid_json() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "not json").unwrap();
        let result = load_auth(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_default_auth_path_is_in_shepherd_dir() {
        let path = default_auth_path();
        let s = path.to_str().unwrap();
        assert!(
            s.contains(".shepherd"),
            "path should be under ~/.shepherd: {s}"
        );
        assert!(
            s.ends_with("iterm2-auth.json"),
            "path should end with iterm2-auth.json: {s}"
        );
    }

    #[test]
    fn test_auth_serde_roundtrip() {
        let auth = Iterm2Auth {
            cookie: "mycookie123".to_string(),
            key: "mykey456".to_string(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        let parsed: Iterm2Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.cookie, "mycookie123");
        assert_eq!(parsed.key, "mykey456");
    }

    #[test]
    fn test_auth_debug() {
        let auth = Iterm2Auth {
            cookie: "c".to_string(),
            key: "k".to_string(),
        };
        let debug = format!("{:?}", auth);
        assert!(debug.contains("Iterm2Auth"));
    }

    #[test]
    fn test_auth_clone() {
        let auth = Iterm2Auth {
            cookie: "c".to_string(),
            key: "k".to_string(),
        };
        let cloned = auth.clone();
        assert_eq!(cloned.cookie, auth.cookie);
        assert_eq!(cloned.key, auth.key);
    }

    #[test]
    fn test_load_auth_missing_fields() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"cookie":"only_cookie"}}"#).unwrap();
        let result = load_auth(f.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_load_auth_extra_fields_ignored() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, r#"{{"cookie":"c","key":"k","extra":"ignored"}}"#).unwrap();
        let auth = load_auth(f.path()).unwrap();
        assert_eq!(auth.cookie, "c");
        assert_eq!(auth.key, "k");
    }

    #[test]
    fn test_watch_auth_does_not_panic_on_bad_path() {
        // watch_auth spawns a background thread; verify it starts without panic
        // even when the watched path doesn't exist.
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        watch_auth(
            std::path::PathBuf::from("/nonexistent/iterm2-auth.json"),
            tx,
        );
        // Give the thread a moment to start and exit gracefully
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

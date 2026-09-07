pub mod migrations;

use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::settings::Settings;

/// Thin SQLite wrapper with migration support. For Phase 1 a single
/// Mutex-guarded connection is sufficient; Phase 2 can move to a pool if needed.
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
    path: String,
}

impl Storage {
    /// Open (or create) a SQLite database at `path` and run migrations.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        migrations::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: path.to_string_lossy().to_string(),
        })
    }

    /// Open an in-memory database (useful for tests).
    pub fn open_in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        migrations::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            path: ":memory:".into(),
        })
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    // ── Settings (key/value JSON) ────────────────────────────────

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        settings.validate().map_err(|e| e.to_string())?;
        let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings(key, value, updated_at) VALUES(?1, ?2, strftime('%Y-%m-%dT%H:%M:%SZ','now')) ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
            params!["app_settings", json],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<Settings, String> {
        let conn = self.conn.lock().unwrap();
        let result: Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key='app_settings'",
            [],
            |r| r.get(0),
        );
        match result {
            Ok(json) => serde_json::from_str(&json).map_err(|e| e.to_string()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Settings::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    // ── History (Phase 1: schema only, minimal helper) ──────────

    pub fn push_history(&self, text: &str, model_id: &str) -> Result<i64, String> {
        if text.trim().is_empty() {
            return Err("history text must not be empty".into());
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO transcription_history(text, model_id) VALUES(?1, ?2)",
            params![text, model_id],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn history_count(&self) -> Result<i64, String> {
        let conn = self.conn.lock().unwrap();
        let c: i64 = conn
            .query_row("SELECT count(*) FROM transcription_history", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        Ok(c)
    }

    // ── Dictionary placeholder ───────────────────────────────────

    pub fn upsert_dictionary(&self, phrase: &str, replacement: &str) -> Result<(), String> {
        if phrase.trim().is_empty() || replacement.trim().is_empty() {
            return Err("phrase/replacement must not be empty".into());
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO dictionary_entries(phrase, replacement) VALUES(?1, ?2) ON CONFLICT(phrase) DO UPDATE SET replacement=excluded.replacement",
            params![phrase, replacement],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_persistence_roundtrip() {
        let s = Storage::open_in_memory().unwrap();
        let original = Settings {
            global_shortcut: "ctrl+shift+space".into(),
            ..Default::default()
        };
        s.save_settings(&original).unwrap();
        let loaded = s.load_settings().unwrap();
        assert_eq!(original, loaded);
    }

    #[test]
    fn settings_default_when_missing() {
        let s = Storage::open_in_memory().unwrap();
        let loaded = s.load_settings().unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn history_push_and_count() {
        let s = Storage::open_in_memory().unwrap();
        assert_eq!(s.history_count().unwrap(), 0);
        s.push_history("hello world", "whisper-tiny").unwrap();
        assert_eq!(s.history_count().unwrap(), 1);
    }

    #[test]
    fn dictionary_upsert() {
        let s = Storage::open_in_memory().unwrap();
        s.upsert_dictionary("wisp er", "Wispr").unwrap();
        s.upsert_dictionary("wisp er", "Wispr Flow").unwrap();
        // Verify replacement updated
        let conn = s.conn.lock().unwrap();
        let val: String = conn
            .query_row(
                "SELECT replacement FROM dictionary_entries WHERE phrase='wisp er'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(val, "Wispr Flow");
    }

    #[test]
    fn file_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        {
            let s = Storage::open(&path).unwrap();
            s.save_settings(&Settings {
                global_shortcut: "fn".into(),
                selected_model_id: "whisper-base".into(),
                ..Default::default()
            })
            .unwrap();
        }
        // Reopen
        let s2 = Storage::open(&path).unwrap();
        let loaded = s2.load_settings().unwrap();
        assert_eq!(loaded.selected_model_id, "whisper-base");
    }
}

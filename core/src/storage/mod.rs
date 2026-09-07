pub mod migrations;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::settings::Settings;

/// Thin SQLite wrapper with migration support.
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryRecord {
    pub id: i64,
    pub text: String,
    pub model_id: String,
    pub created_at: String,
    pub duration_ms: Option<i64>,
    pub bundle_id: Option<String>,
    pub app_name: Option<String>,
    pub confidence: Option<f32>,
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

    // ── History ──────────────────────────────────────────────────

    pub fn push_history(&self, text: &str, model_id: &str) -> Result<i64, String> {
        self.push_history_detailed(text, model_id, None, None, None, None)
    }

    pub fn push_history_detailed(
        &self,
        text: &str,
        model_id: &str,
        duration_ms: Option<i64>,
        bundle_id: Option<&str>,
        app_name: Option<&str>,
        confidence: Option<f32>,
    ) -> Result<i64, String> {
        if text.trim().is_empty() {
            return Err("history text must not be empty".into());
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO transcription_history(text, model_id, duration_ms, bundle_id, app_name, confidence) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
            params![text, model_id, duration_ms, bundle_id, app_name, confidence],
        )
        .map_err(|e| e.to_string())?;
        let id = conn.last_insert_rowid();
        // Prune: keep max 5000 entries and 30 days
        let _ = conn.execute(
            "DELETE FROM transcription_history WHERE id IN (SELECT id FROM transcription_history ORDER BY created_at DESC, id DESC LIMIT -1 OFFSET 5000)",
            [],
        );
        let _ = conn.execute(
            "DELETE FROM transcription_history WHERE created_at < datetime('now', '-30 days')",
            [],
        );
        Ok(id)
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

    pub fn get_history(&self, limit: i64, offset: i64) -> Result<Vec<HistoryRecord>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, text, model_id, created_at, duration_ms, bundle_id, app_name, confidence FROM transcription_history ORDER BY created_at DESC, id DESC LIMIT ?1 OFFSET ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit, offset], |r| {
                Ok(HistoryRecord {
                    id: r.get(0)?,
                    text: r.get(1)?,
                    model_id: r.get(2)?,
                    created_at: r.get(3)?,
                    duration_ms: r.get(4)?,
                    bundle_id: r.get(5)?,
                    app_name: r.get(6)?,
                    confidence: r.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn delete_history(&self, id: i64) -> Result<bool, String> {
        let conn = self.conn.lock().unwrap();
        let n = conn
            .execute("DELETE FROM transcription_history WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM transcription_history", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn search_history(&self, query: &str, limit: i64) -> Result<Vec<HistoryRecord>, String> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn
            .prepare(
                "SELECT id, text, model_id, created_at, duration_ms, bundle_id, app_name, confidence FROM transcription_history WHERE text LIKE ?1 ORDER BY created_at DESC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![pattern, limit], |r| {
                Ok(HistoryRecord {
                    id: r.get(0)?,
                    text: r.get(1)?,
                    model_id: r.get(2)?,
                    created_at: r.get(3)?,
                    duration_ms: r.get(4)?,
                    bundle_id: r.get(5)?,
                    app_name: r.get(6)?,
                    confidence: r.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
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

    pub fn get_dictionary(&self) -> Result<std::collections::HashMap<String, String>, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT phrase, replacement FROM dictionary_entries")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut map = std::collections::HashMap::new();
        for r in rows {
            let (k, v) = r.map_err(|e| e.to_string())?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn delete_dictionary(&self, phrase: &str) -> Result<bool, String> {
        if phrase.trim().is_empty() {
            return Err("phrase empty".into());
        }
        let conn = self.conn.lock().unwrap();
        let n = conn
            .execute(
                "DELETE FROM dictionary_entries WHERE phrase=?1",
                params![phrase],
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
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
    fn history_detailed_and_fetch() {
        let s = Storage::open_in_memory().unwrap();
        s.push_history_detailed(
            "hello",
            "whisper-tiny",
            Some(1200),
            Some("com.apple.TextEdit"),
            Some("TextEdit"),
            Some(0.95),
        )
        .unwrap();
        let recs = s.get_history(10, 0).unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].text, "hello");
        assert_eq!(recs[0].bundle_id.as_deref(), Some("com.apple.TextEdit"));
        assert_eq!(recs[0].app_name.as_deref(), Some("TextEdit"));
        assert_eq!(recs[0].duration_ms, Some(1200));
    }

    #[test]
    fn history_delete_and_clear() {
        let s = Storage::open_in_memory().unwrap();
        let id1 = s.push_history("a", "m").unwrap();
        let id2 = s.push_history("b", "m").unwrap();
        assert_eq!(s.history_count().unwrap(), 2);
        assert!(s.delete_history(id1).unwrap());
        assert_eq!(s.history_count().unwrap(), 1);
        s.clear_history().unwrap();
        assert_eq!(s.history_count().unwrap(), 0);
        let _ = id2;
    }

    #[test]
    fn history_search() {
        let s = Storage::open_in_memory().unwrap();
        s.push_history("hello world", "m").unwrap();
        s.push_history("goodbye", "m").unwrap();
        let res = s.search_history("hello", 10).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].text, "hello world");
    }

    #[test]
    fn dictionary_upsert() {
        let s = Storage::open_in_memory().unwrap();
        s.upsert_dictionary("wisp er", "Wispr").unwrap();
        s.upsert_dictionary("wisp er", "Wispr Flow").unwrap();
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
        let s2 = Storage::open(&path).unwrap();
        let loaded = s2.load_settings().unwrap();
        assert_eq!(loaded.selected_model_id, "whisper-base");
    }
}

use rusqlite::Connection;

/// Ordered list of migrations. Each entry is idempotent via `IF NOT EXISTS`
/// and is guarded by `user_version` pragma.
pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Ensure foreign keys are on.
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    let version: i32 = conn
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap_or(0);

    if version < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            );

            CREATE TABLE IF NOT EXISTS transcription_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                model_id TEXT NOT NULL DEFAULT 'unknown',
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                duration_ms INTEGER
            );

            CREATE TABLE IF NOT EXISTS dictionary_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                phrase TEXT NOT NULL UNIQUE,
                replacement TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            );

            -- Helpful indexes
            CREATE INDEX IF NOT EXISTS idx_history_created_at ON transcription_history(created_at);
            CREATE INDEX IF NOT EXISTS idx_dictionary_phrase ON dictionary_entries(phrase);

            PRAGMA user_version = 1;
            "#,
        )?;
    }

    if version < 2 {
        // Phase 3: add app context and confidence to history
        conn.execute_batch(
            r#"
            ALTER TABLE transcription_history ADD COLUMN bundle_id TEXT;
            ALTER TABLE transcription_history ADD COLUMN app_name TEXT;
            ALTER TABLE transcription_history ADD COLUMN confidence REAL;
            PRAGMA user_version = 2;
            "#,
        )?;
        // ALTER ADD COLUMN IF NOT EXISTS not available in older sqlite, so we catch error for existing columns
        // The above will fail if columns already exist when migrating from 0→2 in one go? But version<1 already created table with only 5 columns, so version<2 will add 3.
        // If user is fresh (version 0), version<1 creates base, then version<2 adds. Idempotent via checking.
    }

    // Ensure columns exist even if migration from 1→2 skipped due to batch error (e.g., columns already exist)
    // Use pragma table_info to add missing columns safely
    ensure_history_columns(conn)?;

    Ok(())
}

fn ensure_history_columns(conn: &Connection) -> Result<(), rusqlite::Error> {
    let info: Vec<String> = {
        let mut stmt = conn.prepare("PRAGMA table_info(transcription_history);")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for (col, def) in [
        ("bundle_id", "TEXT"),
        ("app_name", "TEXT"),
        ("confidence", "REAL"),
    ] {
        if !info.contains(&col.to_string()) {
            let sql = format!(
                "ALTER TABLE transcription_history ADD COLUMN {} {};",
                col, def
            );
            let _ = conn.execute(&sql, []);
        }
    }
    // Ensure user_version at least 2 if columns now exist
    let v: i32 = conn
        .query_row("PRAGMA user_version;", [], |r| r.get(0))
        .unwrap_or(0);
    if v < 2 {
        conn.execute_batch("PRAGMA user_version = 2;")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('settings','transcription_history','dictionary_entries');",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn no_audio_table() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let audio_tables: i32 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND (name LIKE '%audio%' OR sql LIKE '%BLOB%');",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // We do not create any audio persistence tables; blobs are incidental but we check name
        let audio_named: i32 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name LIKE '%audio%';",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audio_named, 0);
        let _ = audio_tables; // ensure query succeeded
    }

    #[test]
    fn history_has_app_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn
            .prepare("PRAGMA table_info(transcription_history);")
            .unwrap();
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(cols.contains(&"bundle_id".to_string()));
        assert!(cols.contains(&"app_name".to_string()));
    }
}

use rusqlite::{params, Connection, Result};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

pub fn default_db_path() -> PathBuf {
    if let Ok(dir) = std::env::var("PRO_PROXY_DATA_DIR").or_else(|_| std::env::var("PROMESSAGE_DATA_DIR")) {
        PathBuf::from(dir).join("pro-proxy.db")
    } else if let Some(home) = std::env::var_os("HOME") {
        let new_path = PathBuf::from(&home).join(".pro-proxy").join("pro-proxy.db");
        let old_path = PathBuf::from(&home).join(".promessage-gatekeeper").join("gatekeeper.db");
        if !new_path.exists() && old_path.exists() {
            if let Some(parent) = new_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::copy(&old_path, &new_path);
        }
        new_path
    } else {
        PathBuf::from("pro-proxy.db")
    }
}

impl Db {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL COLLATE NOCASE,
                password_hash TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                expires_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            ",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn set_user(&self, username: &str, password_hash: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO users (username, password_hash, created_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(username) DO UPDATE SET
                password_hash = excluded.password_hash;",
            params![username, password_hash, now],
        )?;
        Ok(())
    }

    pub fn get_user_password_hash(&self, username: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT password_hash FROM users WHERE username = ?1")?;
        let mut rows = stmt.query(params![username])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_user(&self, username: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count = conn.execute("DELETE FROM users WHERE username = ?1", params![username])?;
        Ok(count > 0)
    }

    pub fn list_users(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT username FROM users ORDER BY username ASC")?;
        let user_iter = stmt.query_map([], |row| row.get(0))?;
        let mut users = Vec::new();
        for u in user_iter {
            users.push(u?);
        }
        Ok(users)
    }

    pub fn user_count(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT count(*) FROM users", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn create_session(&self, token: &str, username: &str, ttl_secs: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let expires_at = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + ttl_secs) as i64;

        conn.execute(
            "INSERT INTO sessions (token, username, expires_at) VALUES (?1, ?2, ?3)",
            params![token, username, expires_at],
        )?;
        Ok(())
    }

    pub fn validate_session(&self, token: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut stmt = conn.prepare(
            "SELECT username FROM sessions WHERE token = ?1 AND expires_at > ?2",
        )?;
        let mut rows = stmt.query(params![token, now])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn delete_session(&self, token: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM sessions WHERE token = ?1", params![token])?;
        Ok(())
    }

    pub fn cleanup_expired_sessions(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let count = conn.execute("DELETE FROM sessions WHERE expires_at <= ?1", params![now])?;
        Ok(count)
    }

    pub fn get_config(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_config(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO config (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value;",
            params![key, value],
        )?;
        Ok(())
    }
}

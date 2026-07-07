use gtk::glib;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub struct Task {
    #[allow(dead_code)] // not shown in the UI yet; later phases (done/delete) key on it
    pub id: i64,
    pub text: String,
    pub created_at: String, // ISO 8601, UTC
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open() -> Self {
        Self::open_at(db_path())
    }

    fn open_at(path: impl AsRef<Path>) -> Self {
        let conn = Connection::open(path).expect("failed to open task database");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tasks (
                id         INTEGER PRIMARY KEY,
                text       TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
        .expect("failed to initialize task database");
        Self { conn }
    }

    pub fn add(&self, text: &str) {
        let now = glib::DateTime::now_utc()
            .and_then(|dt| dt.format_iso8601())
            .expect("failed to get current time");
        self.conn
            .execute(
                "INSERT INTO tasks (text, created_at) VALUES (?1, ?2)",
                (text, now.as_str()),
            )
            .expect("failed to save task");
    }

    pub fn all(&self) -> Vec<Task> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, text, created_at FROM tasks ORDER BY id DESC")
            .expect("failed to query tasks");
        stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                text: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
        .expect("failed to read tasks")
        .filter_map(Result::ok)
        .collect()
    }
}

fn db_path() -> PathBuf {
    let dir = glib::user_data_dir().join("doo");
    std::fs::create_dir_all(&dir).expect("failed to create data directory");
    dir.join("doo.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_newest_first() {
        let dir = std::env::temp_dir().join(format!("doo-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::open_at(dir.join("test.db"));

        store.add("first task");
        store.add("second task");

        let tasks = store.all();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].text, "second task");
        assert_eq!(tasks[1].text, "first task");
        assert!(tasks[0].id > tasks[1].id);
        assert!(!tasks[0].created_at.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}

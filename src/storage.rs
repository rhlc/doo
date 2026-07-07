use gtk::glib;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub struct Task {
    pub id: i64,
    pub text: String,
    pub created_at: String,         // ISO 8601, UTC
    pub image_path: Option<String>, // absolute path to a pasted screenshot, if any
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
                created_at TEXT NOT NULL,
                image_path TEXT
            );",
        )
        .expect("failed to initialize task database");
        migrate_image_path(&conn);
        Self { conn }
    }

    pub fn add(&self, text: &str, image_path: Option<&str>) {
        let now = glib::DateTime::now_utc()
            .and_then(|dt| dt.format_iso8601())
            .expect("failed to get current time");
        self.conn
            .execute(
                "INSERT INTO tasks (text, created_at, image_path) VALUES (?1, ?2, ?3)",
                (text, now.as_str(), image_path),
            )
            .expect("failed to save task");
    }

    pub fn all(&self) -> Vec<Task> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, text, created_at, image_path FROM tasks ORDER BY id DESC")
            .expect("failed to query tasks");
        stmt.query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                text: row.get(1)?,
                created_at: row.get(2)?,
                image_path: row.get(3)?,
            })
        })
        .expect("failed to read tasks")
        .filter_map(Result::ok)
        .collect()
    }

    pub fn delete(&self, id: i64) {
        // Remove the attached screenshot too, so deleting a task doesn't leave
        // an orphaned file behind.
        if let Some(path) = self
            .conn
            .query_row(
                "SELECT image_path FROM tasks WHERE id = ?1",
                [id],
                |row| row.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
        {
            let _ = std::fs::remove_file(path);
        }
        self.conn
            .execute("DELETE FROM tasks WHERE id = ?1", [id])
            .expect("failed to delete task");
    }
}

/// Add the `image_path` column to databases created before image support.
fn migrate_image_path(conn: &Connection) {
    let has_column = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('tasks') WHERE name = 'image_path'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        > 0;
    if !has_column {
        conn.execute("ALTER TABLE tasks ADD COLUMN image_path TEXT", [])
            .expect("failed to migrate task database");
    }
}

/// Directory where pasted screenshots are stored.
pub fn images_dir() -> PathBuf {
    let dir = glib::user_data_dir().join("doo").join("images");
    std::fs::create_dir_all(&dir).expect("failed to create images directory");
    dir
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

        store.add("first task", None);
        store.add("second task", Some("/tmp/shot.png"));

        let tasks = store.all();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].text, "second task");
        assert_eq!(tasks[0].image_path.as_deref(), Some("/tmp/shot.png"));
        assert_eq!(tasks[1].text, "first task");
        assert_eq!(tasks[1].image_path, None);
        assert!(tasks[0].id > tasks[1].id);
        assert!(!tasks[0].created_at.is_empty());

        store.delete(tasks[0].id);
        let tasks = store.all();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].text, "first task");

        std::fs::remove_dir_all(&dir).ok();
    }
}

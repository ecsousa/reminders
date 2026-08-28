use crate::models::Reminder;
use chrono::NaiveDateTime;
use std::path::PathBuf;
use tokio_rusqlite::Connection;

pub async fn init_db(db_folder: &str) -> Connection {
    let mut db_path = PathBuf::from(db_folder);
    db_path.push("reminders.db3");

    tracing::info!("Using database file at: {}", db_path.display());

    let conn = Connection::open(db_path).await.expect("Failed to open db");

    conn.call(|conn| {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS reminders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username VARCHAR(32) NOT NULL,
                creation_time DATETIME NOT NULL,
                reminder_message TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_reminders_username_creation_time ON reminders(username, creation_time)",
            [],
        )?;
        Ok(())
    }).await.expect("Failed to initialize database schema");

    conn
}

pub async fn save_reminder(
    conn: &Connection,
    mut reminder: Reminder,
) -> Result<Reminder, tokio_rusqlite::Error> {
    let now = chrono::Local::now().naive_local();
    let formatted_date = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let username = reminder.username.clone().unwrap_or_default();
    let message = reminder.reminder_message.clone();

    conn.call(move |conn| {
        conn.execute(
            "INSERT INTO reminders (username, creation_time, reminder_message) VALUES (?1, ?2, ?3)",
            (&username, &formatted_date, &message),
        )?;
        let id = conn.last_insert_rowid();
        reminder.id = Some(id);
        reminder.created_time = Some(now);
        Ok(reminder)
    })
    .await
}

pub async fn find_all_by_username(
    conn: &Connection,
    username: String,
) -> Result<Vec<Reminder>, tokio_rusqlite::Error> {
    conn.call(move |conn| {
        let mut stmt = conn.prepare("SELECT id, username, creation_time, reminder_message FROM reminders WHERE username = ?1 ORDER BY creation_time ASC")?;
        let reminder_iter = stmt.query_map([&username], |row| {
            let creation_time_str: String = row.get(2)?;
            let created_time = NaiveDateTime::parse_from_str(&creation_time_str, "%Y-%m-%d %H:%M:%S").ok();
            Ok(Reminder {
                id: row.get(0)?,
                username: row.get(1)?,
                created_time,
                reminder_message: row.get(3)?,
            })
        })?;

        let mut reminders = Vec::new();
        for r in reminder_iter {
            reminders.push(r?);
        }
        Ok(reminders)
    }).await
}

pub async fn find_all(conn: &Connection) -> Result<Vec<Reminder>, tokio_rusqlite::Error> {
    conn.call(|conn| {
        let mut stmt =
            conn.prepare("SELECT id, username, creation_time, reminder_message FROM reminders")?;
        let reminder_iter = stmt.query_map([], |row| {
            let creation_time_str: String = row.get(2)?;
            let created_time =
                NaiveDateTime::parse_from_str(&creation_time_str, "%Y-%m-%d %H:%M:%S").ok();
            Ok(Reminder {
                id: row.get(0)?,
                username: row.get(1)?,
                created_time,
                reminder_message: row.get(3)?,
            })
        })?;

        let mut reminders = Vec::new();
        for r in reminder_iter {
            reminders.push(r?);
        }
        Ok(reminders)
    })
    .await
}

pub async fn delete_by_id_and_username(
    conn: &Connection,
    id: i64,
    username: String,
) -> Result<usize, tokio_rusqlite::Error> {
    conn.call(move |conn| {
        let deleted = conn.execute(
            "DELETE FROM reminders WHERE id = ?1 AND username = ?2",
            (id, &username),
        )?;
        Ok(deleted)
    })
    .await
}

pub async fn delete_by_id(conn: &Connection, id: i64) -> Result<usize, tokio_rusqlite::Error> {
    conn.call(move |conn| {
        let deleted = conn.execute("DELETE FROM reminders WHERE id = ?1", [id])?;
        Ok(deleted)
    })
    .await
}

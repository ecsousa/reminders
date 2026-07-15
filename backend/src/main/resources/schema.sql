CREATE TABLE IF NOT EXISTS reminders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username VARCHAR(32) NOT NULL,
    creation_time DATETIME NOT NULL,
    reminder_message TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_reminders_username_creation_time ON reminders(username, creation_time);

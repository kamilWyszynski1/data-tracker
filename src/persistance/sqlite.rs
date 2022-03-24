use super::interface::Persistance;
use rusqlite::{params, Connection};
use std::error::Error;
use std::result::Result;
use uuid::Uuid;

/// SqliteClient is a sqlite implementation of Persistance trait.
pub struct SqliteClient {
    /// sqlite connection.
    conn: Connection,
}

impl SqliteClient {
    pub fn new(conn: Connection) -> Self {
        SqliteClient { conn }
    }
}

impl Persistance for SqliteClient {
    fn write(&mut self, key: Uuid, value: u32) -> Result<(), &'static str> {
        match self.conn.execute(
            "INSERT INTO tasks VALUES (?1, ?2)",
            params![key.to_string(), value],
        ) {
            Err(_) => Err("could not write to db"),
            Ok(_) => Ok(()),
        }
    }

    fn read(&self, key: &Uuid) -> Option<u32> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM tasks WHERE id = ?")
            .unwrap();

        let value: Result<u32, rusqlite::Error> =
            stmt.query_row([key.to_string()], |row| Ok(row.get(0).unwrap()));
        value.ok()
    }
}

/// initializes sqlite connection along with db table init.
pub fn init(path: &str) -> Result<Connection, Box<dyn Error>> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
        id string primary key,
        value integer not null
    )",
        [],
    )?;
    Ok(conn)
}

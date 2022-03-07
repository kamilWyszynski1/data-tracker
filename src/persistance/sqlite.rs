// use super::persistance::Persistance;
// use rusqlite::{params, Connection};
// use std::error::Error;
// use std::result::Result;
// use uuid::Uuid;

// /// SqliteClient is a sqlite implementation of Persistance trait.
// pub struct SqliteClient {
//     /// sqlite connection.
//     conn: Connection,
// }

// impl SqliteClient {
//     fn new(conn: Connection) -> Self {
//         SqliteClient { conn }
//     }
// }

// impl Persistance for SqliteClient {
//     fn write(&mut self, key: Uuid, value: u32) -> Result<(), String> {
//         match self.conn.execute(
//             "INSERT INTO tasks VALUES (?1, ?2)",
//             params![key.to_string(), value],
//         ) {
//             Err(e) => Err("could not write to db".to_string()),
//             Ok(_) => Ok(()),
//         }
//     }

//     fn read(&self, key: &Uuid) -> Option<u32> {
//         let mut stmt = self
//             .conn
//             .prepare("SELECT value FROM tasks WHERE id = ?")
//             .unwrap();

//         match stmt.query_row([key.to_string()], |row| {
//             Ok(row.get::<i32, rusqlite::Result<u32>>(0).unwrap())
//         }) {
//             Err(e) => None,
//             Ok(r) => Some(1),
//         }
//     }
// }

// /// initializes sqlite connection along with db table init.
// pub fn init(path: &str) -> Result<Connection, Box<dyn Error>> {
//     let conn = Connection::open(path)?;

//     conn.execute(
//         "CREATE TABLE IF NOT EXISTS tasks (
//         id string primary key,
//         value integer not null
//     )",
//         [],
//     )?;
//     Ok(conn)
// }

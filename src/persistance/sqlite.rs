use super::interface::Persistance;
use crate::core::task::TrackingTask;
use crate::diesel::OptionalExtension;
use crate::diesel::RunQueryDsl;
use crate::models::location::Location;
use crate::schema::*;
use diesel::{insert_into, ExpressionMethods, QueryDsl};
use diesel::{Connection, SqliteConnection};
use std::env;
use std::result::Result;
use uuid::Uuid;

pub fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

/// SqliteClient is a sqlite implementation of Persistance trait.
pub struct SqliteClient {
    /// sqlite connection.
    conn: SqliteConnection,
}

impl SqliteClient {
    pub fn new(conn: SqliteConnection) -> Self {
        SqliteClient { conn }
    }
}

impl Persistance for SqliteClient {
    fn save_location(&mut self, key: Uuid, value: u32) -> Result<(), &'static str> {
        let l = Location {
            key: key.to_string(),
            value: value.try_into().unwrap(),
        };
        insert_into(location::table)
            .values(&l)
            .execute(&self.conn)
            .expect("Error saving location");
        Ok(())
    }

    fn read_location(&self, k: &Uuid) -> Option<u32> {
        use crate::schema::location::dsl::*;

        location
            .filter(key.eq(k.to_string()))
            .first(&self.conn)
            .optional()
            .ok()?
            .map(|l: Location| l.value as u32)
    }

    fn save_task(&mut self, _t: &TrackingTask) -> Result<(), String> {
       
        Ok(())
    }

    fn read_task(&mut self, _id: Uuid) -> Result<TrackingTask, String> {
        todo!()
    }
}

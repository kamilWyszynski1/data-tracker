use super::interface::PResult;
use super::interface::Persistance;
use crate::core::task::TrackingTask;
use crate::diesel::OptionalExtension;
use crate::diesel::RunQueryDsl;
use crate::error::types::Error;
use crate::models::location::Location;
use crate::models::task::TaskModel;
use crate::schema::*;
use diesel::{insert_into, ExpressionMethods, QueryDsl};
use diesel::{Connection, SqliteConnection};
use std::env;
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
    fn save_location(&mut self, key: Uuid, value: u32) -> PResult<()> {
        let l = Location {
            key: key.to_string(),
            value: value as i32,
        };
        insert_into(location::table)
            .values(&l)
            .execute(&self.conn)
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("failed to execute save_location query"),
                    err.to_string(),
                )
            })?;
        Ok(())
    }

    fn read_location(&self, k: &Uuid) -> PResult<u32> {
        use crate::schema::location::dsl::*;

        Ok(location
            .filter(key.eq(k.to_string()))
            .first::<Location>(&self.conn)
            .optional()
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("failed to execute save_location query"),
                    err.to_string(),
                )
            })?
            .ok_or_else(|| {
                Error::new_persistance_internal(
                    String::from("empty Option from query read_location"),
                    "".to_string(),
                )
            })?
            .value as u32)
    }

    fn save_task(&mut self, t: &TrackingTask) -> PResult<()> {
        let tm = TaskModel::from_tracking_task(t);
        insert_into(tasks::table)
            .values(&tm)
            .execute(&self.conn)
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("failed to execute save_location query"),
                    err.to_string(),
                )
            })?;
        Ok(())
    }

    fn read_task(&mut self, id: Uuid) -> PResult<TrackingTask> {
        use crate::schema::tasks::dsl::*;

        let t: TaskModel = tasks
            .filter(uuid.eq(id.to_string()))
            .first::<TaskModel>(&self.conn)
            .optional()
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("failed to execute save_location query"),
                    err.to_string(),
                )
            })?
            .ok_or_else(|| {
                Error::new_persistance_internal(
                    String::from("empty Option from query read_location"),
                    "".to_string(),
                )
            })?;
        Ok(TrackingTask::from_task_model(&t).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::direction::Direction;
    use crate::core::intype::InputType;
    use crate::core::task::{InputData, TrackingTask};
    use crate::core::timestamp::TimestampPosition;
    use crate::error::types::Result;
    use crate::lang::engine::Definition;
    use crate::lang::lexer::EvalForest;
    use crate::persistance::interface::Persistance;
    use diesel::{Connection, SqliteConnection};
    use diesel_migrations::embed_migrations;
    use std::fs::{self, File};
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    use super::SqliteClient;

    embed_migrations!("migrations");

    async fn test_get_data_fn() -> Result<InputData> {
        Ok(InputData::String(String::from("test")))
    }

    #[test]
    fn test_save_read_task() {
        let file_name = "test.sqlite3";
        File::create(file_name).unwrap();

        let connection = SqliteConnection::establish("file:test.sqlite3")
            .unwrap_or_else(|_| panic!("Error connecting to {}", file_name));

        // This will run the necessary migrations.
        embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).unwrap();

        let eval_forest = EvalForest::from_definition(&Definition::new(vec![
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
            String::from("DEFINE(var3, EXTRACT(GET(var), use))"),
            String::from("DEFINE(var4, EXTRACT(GET(var), n))"),
        ]));

        let id = Uuid::parse_str("a54a0fb9-25c9-4f73-ad82-0b7f30ca1ab6").unwrap();
        let tt = TrackingTask {
            id,
            name: Some(String::from("name")),
            description: Some(String::from("description")),
            data_fn: Arc::new(Box::new(move || Box::pin(test_get_data_fn()))),
            spreadsheet_id: String::from("spreadsheet_id"),
            starting_position: String::from("starting_position"),
            sheet: String::from("sheet"),
            direction: Direction::Vertical,
            interval: Duration::from_secs(1),
            with_timestamp: true,
            timestamp_position: TimestampPosition::Before,
            invocations: None,
            eval_forest: eval_forest,
            url: String::from("url"),
            input_type: InputType::String,
            callbacks: None,
        };

        let mut client = SqliteClient::new(connection);
        client.save_task(&tt).unwrap();

        let tt_db = client.read_task(id).unwrap();
        assert_eq!(tt, tt_db);

        fs::remove_file(file_name).unwrap();
    }
}

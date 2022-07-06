use super::interface::PResult;
use super::interface::Persistance;
use crate::core::handler::Report;
use crate::core::task::TrackingTask;
use crate::core::types::State;
use crate::diesel::OptionalExtension;
use crate::diesel::RunQueryDsl;
use crate::error::types::Error;
use crate::models::location::Location;
use crate::models::report::ReportModel;
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
    fn save_location(&mut self, k: Uuid, v: u32) -> PResult<()> {
        use crate::schema::location::dsl::*;

        let l = Location {
            key: k.to_string(),
            value: v as i32,
        };
        insert_into(location)
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
                    String::from("failed to execute read_location query"),
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
                    String::from("failed to execute save_task query"),
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
                    String::from("failed to execute read_task query"),
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

    fn update_task_status(&mut self, id: Uuid, s: State) -> PResult<()> {
        use crate::schema::tasks::dsl::*;

        let target = tasks.filter(uuid.eq(id.to_string()));
        diesel::update(target)
            .set(status.eq(s))
            .execute(&self.conn)
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("empty Option from query update_task_status"),
                    err.to_string(),
                )
            })?;
        Ok(())
    }

    fn delete_task(&mut self, id: Uuid) -> PResult<()> {
        use crate::schema::tasks::dsl::*;

        let target = tasks.filter(uuid.eq(id.to_string()));
        diesel::delete(target).execute(&self.conn).map_err(|err| {
            Error::new_persistance_internal(String::from("could not delete task"), err.to_string())
        })?;
        Ok(())
    }

    fn get_tasks_by_status(&mut self, statuses: &[State]) -> PResult<Vec<TrackingTask>> {
        use crate::schema::tasks::dsl::*;

        let t: Vec<TaskModel> = tasks
            .filter(status.eq_any(statuses))
            .load(&self.conn)
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("failed to execute get_tasks_by_status query"),
                    err.to_string(),
                )
            })?;
        t.into_iter()
            .map(|tm| TrackingTask::from_task_model(&tm))
            .collect::<PResult<Vec<TrackingTask>>>()
    }

    fn save_report(&mut self, report: &Report) -> PResult<i32> {
        use crate::schema::reports::dsl::*;

        let model = ReportModel::from_report(report);
        insert_into(reports)
            .values(&model)
            .execute(&self.conn)
            .and_then(|_| -> Result<i32, diesel::result::Error> {
                reports.select(id).order(id.desc()).first(&self.conn)
            })
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("empty Option from query update_task_status"),
                    err.to_string(),
                )
            })
    }

    fn read_reports(&mut self, uuid: Uuid) -> PResult<Option<Vec<ReportModel>>> {
        use crate::schema::reports::dsl::*;

        let report_models: Vec<ReportModel> = reports
            .select((task_id, phases, failed, start))
            .filter(task_id.eq(uuid.to_string()))
            .load(&self.conn)
            .map_err(|err| {
                Error::new_persistance_internal(
                    String::from("failed to execute read_reports query"),
                    err.to_string(),
                )
            })?;
        if report_models.len() == 0 {
            return Ok(None);
        }
        Ok(Some(report_models))
    }
}

#[cfg(test)]
mod tests {
    use crate::core::task::{InputData, TaskInput, TrackingTask};
    use crate::core::types::*;
    use crate::lang::engine::Definition;
    use crate::lang::eval::EvalForest;
    use crate::persistance::interface::Persistance;
    use crate::server::task::TaskKindRequest;
    use diesel::{Connection, SqliteConnection};
    use diesel_migrations::embed_migrations;
    use std::fs::{self, File};
    use uuid::Uuid;

    use super::SqliteClient;

    embed_migrations!("migrations");

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
            data_fn: None,
            spreadsheet_id: String::from("spreadsheet_id"),
            starting_position: String::from("starting_position"),
            sheet: String::from("sheet"),
            direction: Direction::Vertical,
            with_timestamp: true,
            timestamp_position: TimestampPosition::Before,
            invocations: None,
            eval_forest,
            callbacks: None,
            status: State::Created,
            input: Some(TaskInput::String {
                value: String::from("test"),
            }),
            kind: None,
            kind_request: TaskKindRequest::Ticker { interval_secs: 1 },
        };

        let mut client = SqliteClient::new(connection);
        client.save_task(&tt).unwrap();

        let tt_db = client.read_task(id).unwrap();
        assert_eq!(tt, tt_db);

        client.update_task_status(id, State::Quit).unwrap();

        let tt_db = client.read_task(id).unwrap();
        assert_eq!(tt_db.status, State::Quit);

        client.delete_task(id).unwrap();
        assert!(client.read_task(id).is_err());
        fs::remove_file(file_name).unwrap();
    }

    fn test_save_read_by_status() {
        let file_name = "test.sqlite3";
        File::create(file_name).unwrap();

        let connection = SqliteConnection::establish("file:test.sqlite3")
            .unwrap_or_else(|_| panic!("Error connecting to {}", file_name));

        // This will run the necessary migrations.
        embedded_migrations::run_with_output(&connection, &mut std::io::stdout()).unwrap();

        let eval_forest = EvalForest::default();

        let id = Uuid::parse_str("a54a0fb9-25c9-4f73-ad82-0b7f30ca1ab6").unwrap();
        let kind_request = TaskKindRequest::Ticker { interval_secs: 1 };
        let tt = TrackingTask {
            id,
            name: Some(String::from("name")),
            description: Some(String::from("description")),
            data_fn: None,
            spreadsheet_id: String::from("spreadsheet_id"),
            starting_position: String::from("starting_position"),
            sheet: String::from("sheet"),
            direction: Direction::Vertical,
            with_timestamp: true,
            timestamp_position: TimestampPosition::Before,
            invocations: None,
            eval_forest: eval_forest.clone(),
            input: Some(TaskInput::String {
                value: String::from("test"),
            }),
            callbacks: None,
            status: State::Created,
            kind: None,
            kind_request: kind_request.clone(),
        };
        let tt2 = TrackingTask {
            id,
            name: Some(String::from("name")),
            description: Some(String::from("description")),
            data_fn: None,
            spreadsheet_id: String::from("spreadsheet_id"),
            starting_position: String::from("starting_position"),
            sheet: String::from("sheet"),
            direction: Direction::Vertical,
            with_timestamp: true,
            timestamp_position: TimestampPosition::Before,
            invocations: None,
            eval_forest: eval_forest.clone(),
            input: Some(TaskInput::String {
                value: String::from("test"),
            }),
            callbacks: None,
            status: State::Running,
            kind: None,
            kind_request: kind_request.clone(),
        };
        let tt3 = TrackingTask {
            id,
            name: Some(String::from("name")),
            description: Some(String::from("description")),
            data_fn: None,
            spreadsheet_id: String::from("spreadsheet_id"),
            starting_position: String::from("starting_position"),
            sheet: String::from("sheet"),
            direction: Direction::Vertical,
            with_timestamp: true,
            timestamp_position: TimestampPosition::Before,
            invocations: None,
            eval_forest,
            input: Some(TaskInput::String {
                value: String::from("test"),
            }),
            callbacks: None,
            status: State::Quit,
            kind: None,
            kind_request,
        };

        let mut client = SqliteClient::new(connection);
        client.save_task(&tt).unwrap();
        client.save_task(&tt2).unwrap();
        client.save_task(&tt3).unwrap();

        let tasks = client
            .get_tasks_by_status(&[State::Running, State::Quit])
            .unwrap();

        assert_eq!(vec![tt2, tt3], tasks);

        fs::remove_file(file_name).unwrap();
    }
}

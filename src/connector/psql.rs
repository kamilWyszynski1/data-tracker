use crate::core::task::{BoxFnThatReturnsAFuture, InputData};
use crate::error::types::{Error, Result};
use postgres::{Client, NoTls};

#[derive(Clone)]
pub struct PSQLConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
    db: String,
}

impl PSQLConfig {
    pub fn new(host: String, port: u16, user: String, password: String, db: String) -> Self {
        Self {
            host,
            port,
            user,
            password,
            db,
        }
    }

    pub fn to_conn_str(&self) -> String {
        format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.user, self.password, self.db,
        )
    }
}

/// Function wraps psql config and query into async function that will be run
/// in order to retrieve data from postgresql.
async fn psql_wrap(cfg: PSQLConfig, query: String) -> Result<InputData> {
    tokio::task::spawn_blocking(move || -> Result<InputData> {
        let mut client = Client::connect(cfg.to_conn_str().as_str(), NoTls).map_err(|err| {
            Error::new_internal(
                String::from("psql_wrap"),
                String::from("failed to create psql client"),
                err.to_string(),
            )
        })?;
        let rows = client.query(&query, &[]).map_err(|err| {
            Error::new_internal(
                String::from("psql_wrap"),
                String::from("failed to create query "),
                err.to_string(),
            )
        })?;
        if rows.len() == 1 {
            return Ok(InputData::String(rows[0].get(0)));
        }
        Ok(InputData::Vector(
            rows.iter().map(|r| InputData::String(r.get(0))).collect(),
        ))
    })
    .await
    .map_err(|err| {
        Error::new_internal(
            String::from("psql_wrap"),
            String::from("failed to await spawned blocked "),
            err.to_string(),
        )
    })?
}

/// Creates getter for data from psql.
pub fn getter_from_psql(cfg: PSQLConfig, query: String) -> BoxFnThatReturnsAFuture {
    Box::new(move || Box::pin(psql_wrap(cfg.clone(), query.clone())))
}

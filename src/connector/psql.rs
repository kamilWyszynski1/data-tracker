use crate::core::task::{BoxFnThatReturnsAFuture, InputData};
use crate::error::types::Result;
use postgres::{Client, NoTls};

#[derive(Clone)]
pub struct PSQLConfig {
    host: String,
    username: String,
    password: String,
    db: String,
}

impl PSQLConfig {
    pub fn new(host: String, username: String, password: String, db: String) -> Self {
        Self {
            host,
            username,
            password,
            db,
        }
    }

    pub fn to_conn_str(&self) -> String {
        format!(
            "host={} user={} password={} dbname={}",
            self.host, self.username, self.password, self.db,
        )
    }
}

pub fn new(cfg: PSQLConfig) -> Client {
    Client::connect(cfg.to_conn_str().as_str(), NoTls).unwrap()
}

/// Function wraps psql config and query into async function that will be run
/// in order to retrieve data from postgresql.
async fn psql_wrap(cfg: PSQLConfig, query: String) -> Result<InputData> {
    let s = tokio::task::spawn_blocking(move || {
        let mut client = new(cfg);
        let row = client.query_one(&query, &[]).unwrap();
        row.get(0)
    })
    .await
    .unwrap();

    Ok(InputData::String(s))
}

/// Creates getter for data from psql.
pub fn getter_from_psql(cfg: PSQLConfig, query: String) -> BoxFnThatReturnsAFuture {
    Box::new(move || Box::pin(psql_wrap(cfg.clone(), query.clone())))
}

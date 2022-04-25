use crate::core::task::{BoxFnThatReturnsAFuture, InputData};
use crate::error::types::Result;
use postgres::{Client, NoTls};

#[derive(Clone)]
pub struct PSQLConfig {
    host: String,
    username: String,
    password: String,
}

impl PSQLConfig {
    pub fn new(host: String, username: String, password: String) -> Self {
        Self {
            host,
            username,
            password,
        }
    }

    pub fn to_conn_str(&self) -> String {
        format!(
            "host={} user={} password={}",
            self.host, self.username, self.password
        )
    }
}

pub struct PSQLConnector {
    client: Client,
    query: String,
}


impl PSQLConnector {
    pub fn new(cfg: PSQLConfig, query: String) -> Self {
        let client = Client::connect(cfg.to_conn_str().as_str(), NoTls).unwrap();
        Self { client, query }
    }

    /// Queries new data, returns only String for now.
    fn query(&mut self) -> String {
        let row = self.client.query_one(&self.query, &[]).unwrap();
        row.get(0)
    }
}

/// Function wraps psql config and query into async function that will be run
/// in order to retrieve data from postgresql.
async fn psql_wrap(cfg: PSQLConfig, query: String) -> Result<InputData> {
    let mut psql = PSQLConnector::new(cfg, query);
    Ok(InputData::String(psql.query()))
}

/// Creates getter for data from psql.
pub fn getter_from_psql(cfg: PSQLConfig, query: String) -> BoxFnThatReturnsAFuture {
    Box::new(move || Box::pin(psql_wrap(cfg.clone(), query.clone())))
}

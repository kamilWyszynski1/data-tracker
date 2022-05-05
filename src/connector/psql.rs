use std::sync::Arc;

use crate::core::task::{BoxFnThatReturnsAFuture, InputData};
use crate::error::types::{Error, Result};
use futures::{stream, StreamExt};
use postgres::{Client, NoTls};
use tokio::sync::mpsc::Sender;
use tokio_postgres::AsyncMessage;

#[derive(Clone)]
pub struct PSQLConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub db: String,

    // fields for changes monitor.
    pub channel_name: Option<String>,
}

impl PSQLConfig {
    pub fn new(
        host: String,
        port: u16,
        user: String,
        password: String,
        db: String,
        channel_name: Option<String>,
    ) -> Self {
        Self {
            host,
            port,
            user,
            password,
            db,
            channel_name,
        }
    }

    pub fn to_conn_str(&self) -> String {
        format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.user, self.password, self.db,
        )
    }
}

fn new_psql_client(cfg: &PSQLConfig) -> Result<Client> {
    Client::connect(cfg.to_conn_str().as_str(), NoTls).map_err(|err| {
        Error::new_internal(
            String::from("psql_wrap"),
            String::from("failed to create psql client"),
            err.to_string(),
        )
    })
}

/// Function wraps psql config and query into async function that will be run
/// in order to retrieve data from postgresql.
async fn psql_wrap(cfg: PSQLConfig, query: String) -> Result<InputData> {
    tokio::task::spawn_blocking(move || -> Result<InputData> {
        let mut client = new_psql_client(&cfg)?;
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

/// Function wraps psql NOTIFY/LISTEN functionality.
pub async fn monitor_changes(cfg: PSQLConfig, sender: Sender<InputData>) {
    debug!("1!");
    let (client, mut connection) = tokio_postgres::connect(cfg.to_conn_str().as_str(), NoTls)
        .await
        .unwrap();
    let client = Arc::new(client);
    let client2 = client.clone();
    tokio::spawn(async move {
        let mut stream =
            stream::poll_fn(move |cx| connection.poll_message(cx).map_err(|e| panic!("{}", e)));

        debug!("2!");

        while let Some(n) = stream.next().await {
            debug!("message!");
            let msg = n.unwrap();
            if let AsyncMessage::Notification(notification) = msg {
                debug!("notification: {:?}", notification);
                sender
                    .send(InputData::String(notification.payload().to_string()))
                    .await
                    .unwrap();
            } else {
                return;
            }
        }
        drop(client);
    });
    client2.query("LISTEN test_channel", &[]).await.unwrap();

    debug!("3!");
}

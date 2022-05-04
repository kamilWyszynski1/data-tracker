use diesel::backend::Backend;
use diesel::deserialize;
use diesel::serialize::{self, Output};
use diesel::sql_types::Text;
use diesel::types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::sync::Mutex;

use super::manager::Command;
use super::task::InputData;

/// Supported types for task's input data.
/// Should match with InputData.
#[derive(FromSqlRow, AsExpression, Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sql_type = "Text"]
pub enum InputType {
    String,
    Json,
}

impl Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = match self {
            Self::String => "string",
            Self::Json => "json",
        };
        write!(f, "{:?}", v)
    }
}

impl<DB: Backend> ToSql<Text, DB> for InputType
where
    String: ToSql<Text, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        let v = String::from(match *self {
            Self::String => "string",
            Self::Json => "json",
        });
        v.to_sql(out)
    }
}

impl<DB: Backend> FromSql<Text, DB> for InputType
where
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let v = String::from_sql(bytes)?;
        Ok(match v.as_str() {
            "string" => Self::String,
            "json" => Self::Json,
            _ => return Err("replace me with a real error".into()),
        })
    }
}

#[derive(FromSqlRow, AsExpression, Debug, Clone, Copy, Deserialize, PartialEq)]
#[sql_type = "Text"]
/// State is state of currently handled task.
pub enum State {
    Created,
    Running,
    Stopped,
    Quit,
}

impl State {
    pub fn from_cmd(cmd: Command) -> Self {
        match cmd {
            Command::Resume => State::Running,
            Command::Stop => State::Stopped,
            Command::Delete => State::Quit,
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = match self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Quit => "quit",
        };
        write!(f, "{:?}", v)
    }
}

impl Default for State {
    fn default() -> Self {
        Self::Created
    }
}

impl<DB: Backend> ToSql<Text, DB> for State
where
    String: ToSql<Text, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        let v = String::from(match *self {
            Self::Created => "created",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Quit => "quit",
        });
        v.to_sql(out)
    }
}

impl<DB: Backend> FromSql<Text, DB> for State
where
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let v = String::from_sql(bytes)?;
        Ok(match v.as_str() {
            "created" => Self::Created,
            "running" => Self::Running,
            "stopped" => Self::Stopped,
            "quit" => Self::Quit,
            _ => return Err("replace me with a real error".into()),
        })
    }
}

// Direction indicates direction of written data.
#[derive(AsExpression, FromSqlRow, Clone, Debug, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[sql_type = "Text"]
pub enum Direction {
    Vertical,   // data will be written in columns.
    Horizontal, // data will be written in rows.
}

impl Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = match self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        };
        write!(f, "{:?}", v)
    }
}

impl<DB: Backend> ToSql<Text, DB> for Direction
where
    String: ToSql<Text, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        let v = String::from(match *self {
            Self::Vertical => "vertical",
            Self::Horizontal => "horizontal",
        });
        v.to_sql(out)
    }
}

impl<DB: Backend> FromSql<Text, DB> for Direction
where
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let v = String::from_sql(bytes)?;
        Ok(match v.as_str() {
            "vertical" => Self::Vertical,
            "horizontal" => Self::Horizontal,
            _ => return Err("replace me with a real error".into()),
        })
    }
}

// TimestampPosition indicates position of timestamp in the data.
#[derive(AsExpression, FromSqlRow, Clone, Debug, Copy, PartialEq)]
#[sql_type = "Text"]
pub enum TimestampPosition {
    None, // timestamp will not be written.
    Before,
    After,
}

impl Display for TimestampPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v = match self {
            Self::None => "none",
            Self::Before => "before",
            Self::After => "after",
        };
        write!(f, "{:?}", v)
    }
}

impl<DB: Backend> ToSql<Text, DB> for TimestampPosition
where
    String: ToSql<Text, DB>,
{
    fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
    where
        W: io::Write,
    {
        let v = String::from(match *self {
            Self::None => "none",
            Self::Before => "before",
            Self::After => "after",
        });
        v.to_sql(out)
    }
}

impl<DB: Backend> FromSql<Text, DB> for TimestampPosition
where
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
        let v = String::from_sql(bytes)?;
        Ok(match v.as_str() {
            "none" => Self::None,
            "before" => Self::Before,
            "after" => Self::After,
            _ => return Err("replace me with a real error".into()),
        })
    }
}

#[derive(Debug, Derivative, Clone)]
/// Describes how TrackingTask is being run.
pub enum TaskKind {
    Ticker { interval: Duration },
    Triggered { ch: Arc<Mutex<Receiver<InputData>>> },
}

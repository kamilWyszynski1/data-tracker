use crate::schema::tasks;
use diesel::backend::Backend;
use diesel::deserialize;
use diesel::serialize::{self, Output};
use diesel::sql_types::Text;
use diesel::types::{FromSql, ToSql};
use serde::Deserialize;
use std::fmt::{self, Display};
use std::io;

// #[derive(SqlType)]
// #[diesel(sqlite_type(name = "My_Type"))]
// pub struct MyType;

/// Supported types for task's input data.
/// Should match with InputData.
#[derive(FromSqlRow, AsExpression, Debug, Clone, Copy, Deserialize, PartialEq)]
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

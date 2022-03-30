use diesel::backend::Backend;
use diesel::deserialize;
use diesel::serialize::{self, Output};
use diesel::sql_types::Text;
use diesel::types::{FromSql, ToSql};
use serde::Deserialize;
use std::fmt::{self, Display};
use std::io;

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

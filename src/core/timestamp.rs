use diesel::backend::Backend;
use diesel::deserialize;
use diesel::serialize::{self, Output};
use diesel::sql_types::Text;
use diesel::types::{FromSql, ToSql};
use std::fmt::{self, Display};
use std::io;

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

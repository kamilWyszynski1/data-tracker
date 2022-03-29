use crate::schema::location;
use diesel::{Insertable, Queryable};

#[derive(Queryable, Insertable)]
#[table_name = "location"]
pub struct Location {
    pub key: String,
    pub value: i32,
}

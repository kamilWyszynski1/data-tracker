use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use serde::{de, ser, Serialize};
use std::error::Error as StdError;
use std::fmt::Result as FmtResult;
use std::fmt::{Display, Formatter};
use std::result::Result as StdResult;

/// Wrapper for Result from standard library to be used across application.
pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum EvalError {
    InvalidType {
        operation: String,
        t: String,
        wanted: String,
    },
    Internal {
        operation: String,
        msg: String,
    },
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", *self)
    }
}

impl EvalError {
    pub fn new_invalid_type(operation: String, t: String, wanted: String) -> Self {
        Self::InvalidType {
            operation,
            t,
            wanted,
        }
    }

    pub fn new_internal<S: ToString>(operation: S, msg: S) -> Self {
        Self::Internal {
            operation: operation.to_string(),
            msg: msg.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PersistanceError {
    /// Indicates on internal errors like db connection, invalid query.
    Internal { msg: String, err: String },
    /// Indicates on errors related to moving from models to structs.
    Parsing {
        msg: String,
        err: String,
        field: String,
    },
}

impl PersistanceError {
    pub fn new_internal(msg: String, err: String) -> Self {
        Self::Internal { msg, err }
    }

    pub fn new_parsing(msg: String, err: String, field: String) -> Self {
        Self::Parsing { msg, err, field }
    }
}

impl Display for PersistanceError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", *self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Enum for handling errors for whole application.
pub enum Error {
    Eval(EvalError),
    Persistance(PersistanceError),
    Internal {
        place: String, // where error occurred
        msg: String,
        err: String,
    },
    Validation {
        entity: String,
        msg: String,
        field: String,
    },
}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal {
            place: String::from("from anyhow"),
            msg: e.to_string(),
            err: e.source().map_or(String::new(), |s| s.to_string()),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Internal {
            place: String::from("from anyhow"),
            msg: e.to_string(),
            err: e.source().map_or(String::new(), |s| s.to_string()),
        }
    }
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Error {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
        let value = serde_json::json!(self);
        Response::build_from(value.respond_to(req).unwrap())
            .status(Status::InternalServerError)
            .header(ContentType::JSON)
            .ok()
    }
}

impl Error {
    pub fn new_internal(place: String, msg: String, err: String) -> Self {
        Self::Internal { place, msg, err }
    }
    pub fn new_eval(err: EvalError) -> Self {
        Self::Eval(err)
    }

    pub fn new_persistance(err: PersistanceError) -> Self {
        Self::Persistance(err)
    }

    pub fn new_eval_invalid_type<S: ToString>(operation: S, t: S, wanted: S) -> Self {
        Self::new_eval(EvalError::new_invalid_type(
            operation.to_string(),
            t.to_string(),
            wanted.to_string(),
        ))
    }

    pub fn new_eval_internal<S: ToString>(operation: S, msg: S) -> Self {
        Self::new_eval(EvalError::new_internal(operation, msg))
    }

    pub fn new_persistance_internal(msg: String, err: String) -> Self {
        Self::new_persistance(PersistanceError::new_internal(msg, err))
    }

    pub fn new_persistance_parsing(msg: String, err: String, field: String) -> Self {
        Self::new_persistance(PersistanceError::new_parsing(msg, err, field))
    }

    pub fn new_validation<S: ToString>(entity: S, msg: S, field: S) -> Self {
        Self::Validation {
            entity: entity.to_string(),
            msg: msg.to_string(),
            field: field.to_string(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", *self)
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Internal {
            place: String::from("from serde"),
            msg: msg.to_string(),
            err: msg.to_string(),
        }
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Internal {
            place: String::from("from serde"),
            msg: msg.to_string(),
            err: msg.to_string(),
        }
    }
}

impl StdError for Error {}

pub trait LogExt {
    fn log(self) -> Self;
}

impl<T> LogExt for Result<T> {
    fn log(self) -> Self {
        if let Err(e) = &self {
            error!("An error happened: {}", e);
        }
        self
    }
}

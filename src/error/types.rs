use std::error::Error as StdError;
use std::fmt::Result as FmtResult;
use std::fmt::{Display, Formatter};
use std::result::Result as StdResult;

/// Wrapper for Result from standard library to be used across application.
pub type Result<T> = StdResult<T, Error>;

#[derive(Debug, Clone)]
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

    pub fn new_internal(operation: String, msg: String) -> Self {
        Self::Internal { operation, msg }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
/// Enum for handling errors for whole application.
pub enum Error {
    Eval(EvalError),
    Persistance(PersistanceError),
    Internal {
        place: String, // where error occurred
        msg: String,
        err: String,
    },
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

    pub fn new_eval_invalid_type(operation: String, t: String, wanted: String) -> Self {
        Self::new_eval(EvalError::new_invalid_type(operation, t, wanted))
    }

    pub fn new_eval_internal(operation: String, msg: String) -> Self {
        Self::new_eval(EvalError::new_internal(operation, msg))
    }

    pub fn new_persistance_internal(msg: String, err: String) -> Self {
        Self::new_persistance(PersistanceError::new_internal(msg, err))
    }
    pub fn new_persistance_parsing(msg: String, err: String, field: String) -> Self {
        Self::new_persistance(PersistanceError::new_parsing(msg, err, field))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", *self)
    }
}

impl StdError for Error {}

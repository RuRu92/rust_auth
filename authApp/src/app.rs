use std::result;
use mysql::Error as DBError;

#[derive(thiserror::Error, Debug)]
pub enum APIError {
    #[error("Failed to hash user password")]
    PasswordHashing,

    #[error("Incorrect password")]
    IncorrectPassword,

    #[error("Bad Request")]
    DBException(DBError),
}

pub type APIResult<T, E = APIError> = Result<T, E>;

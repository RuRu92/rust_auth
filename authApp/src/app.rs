use std::result;
use mysql::Error as DBError;

#[derive(thiserror::Error, Debug)]
pub enum APIError {
    #[error("Failed to hash user password")]
    PasswordHashing,

    #[error("Incorrect password")]
    IncorrectPassword,

    #[error("{0}")]
    DBException(#[from] DBError),

    #[error("User creation failed")]
    UserCreationFailed(),

    #[error("Missing realm header")]
    MissingRealmHeader,
}

pub type APIResult<T, E = APIError> = Result<T, E>;

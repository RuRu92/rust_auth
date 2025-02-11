
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to hash user password")]
    PasswordHashing,

    #[error("Incorrect password")]
    IncorrectPassword,
}
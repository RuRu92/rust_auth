use lazy_static::lazy_static;
use maplit::hashmap;

use std::{collections::HashMap, result, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{db::ExecutionContext, domain::customer::Role, adapter::auth::PathGuard, repository::realm::RealmSettingProvider};

#[derive(Deserialize, Serialize, Debug)]
pub struct Principal {
    id: String,
    role: String,
    name: String,
}

pub struct AppState {
    pub realm_settings_provider: Arc<RealmSettingProvider>,
    pub execution_context: ExecutionContext,
}


lazy_static! {
    pub static ref ROUTING_TABLE: HashMap<&'static str, PathGuard> = {
        hashmap! {
            // Open paths (accessible to anyone)
            "/api" => PathGuard::Open,
            "/api/customer" => PathGuard::Open,
            "/api/customer/{user_id}" => PathGuard::Open,
            "/api/realm/login" => PathGuard::Open,

            // Authorized paths (requires a valid token)
            "/api/realm" => PathGuard::Authorized,
            "/api/realm/{realm}" => PathGuard::Authorized,

            // Guarded paths (requires specific roles)
            "/api/admin" => PathGuard::Guarded(Role::ADMIN),
        }
    };
}

pub mod error {
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
}



use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, convert::TryFrom, time::Duration};
use strum_macros::Display;

type ID = String;
pub type RealmName = String;

pub struct Realm {
    internal_id: ID,
    pub name: RealmName,
}

// Realm specific settings
pub trait RealmSettings {
    fn is_confirmation_required(&self) -> bool;

    fn realm_salt_itr(&self) -> u32;

    fn is_guest_allowed(&self) -> bool;

    fn get_authentication_token_duration(&self) -> Duration;

    fn get_refresh_token_duration(&self) -> Duration;

    fn get_password_reset_token_duration(&self) -> Duration;

    fn get_secret(&self) -> String;
}
#[derive(Debug, Serialize, Deserialize)]
pub enum Theme {
    Dark, 
    Dracuala,
    Default
}

// pub type Metadata: HashMap<String, Value>;

#[derive(Debug, Serialize, Deserialize)]
pub enum RealmMetaData {
    GenericMap(HashMap<String, Value>),
    
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserRealmSettings {
    pub theme: Theme, 
    #[serde(rename(serialize="metadata"))]
    pub metadata: RealmMetaData,
    pub is_confirmation_required: bool,
}

impl UserRealmSettings {
    pub fn from_realm_settings(value: &dyn RealmSettings) -> Self {
        UserRealmSettings {
            theme: Theme::Default,
            metadata: RealmMetaData::GenericMap(HashMap::default()),
            is_confirmation_required: value.is_confirmation_required(),
        }
    }
}

impl Default for UserRealmSettings {
    fn default() -> Self {
        Self { theme: Theme::Default, metadata: RealmMetaData::GenericMap(HashMap::default()), is_confirmation_required: false }
    }
}


pub struct InternalRealmSettings {
    pub is_confirmation_required: bool,
    pub is_guest_allowed: bool,
    pub realm_salt_itr: u32,
    pub authentication_token_duration: Duration,
    pub refresh_token_duration: Duration,
    pub password_reset_token_duration: Duration,
    pub realm_secret: String,
}

impl RealmSettings for InternalRealmSettings {
    fn is_confirmation_required(&self) -> bool {
        self.is_confirmation_required.clone()
    }

    fn realm_salt_itr(&self) -> u32 {
        return self.realm_salt_itr;
    }

    fn is_guest_allowed(&self) -> bool {
        self.is_guest_allowed.clone()
    }

    fn get_authentication_token_duration(&self) -> Duration {
        self.authentication_token_duration.clone()
    }

    fn get_refresh_token_duration(&self) -> Duration {
        self.refresh_token_duration.clone()
    }

    fn get_password_reset_token_duration(&self) -> Duration {
        self.password_reset_token_duration.clone()
    }

    fn get_secret(&self) -> String {
        self.realm_secret.clone()
    }
}
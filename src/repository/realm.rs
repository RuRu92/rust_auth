use std::time::Duration;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use rand::Rng;
use mysql::AccessMode;

use crate::db::{ExecutionContext, DB};
use crate::domain::realm::{Realm, RealmName, RealmSettings, InternalRealmSettings};

pub struct RealmSettingProvider {
    settings: Arc<HashMap<RealmName, RwLock<InternalRealmSettings>>>,
    db: Arc<DB>,
}

impl RealmSettingProvider {
    pub fn init(db: Arc<DB>) -> RealmSettingProvider {
        let realms = vec!("rj.fg", "rj.wire", "rj.fa");

        let mut realm_settings = HashMap::new();
        for r in realms {
            &realm_settings.insert(r.to_string(), RwLock::new(InternalRealmSettings {
                is_confirmation_required: false,
                is_guest_allowed: false,
                authentication_token_duration: std::time::Duration::new(120, 0),
                refresh_token_duration: std::time::Duration::new(60, 0),
                password_reset_token_duration: std::time::Duration::new(30, 0),
            }));
        }

        RealmSettingProvider { settings: Arc::new(realm_settings), db}
    }

    pub fn is_confirmation_required(&self, realm: &RealmName) -> bool {
        self.settings.get(realm)
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .is_confirmation_required()
    }

    pub fn is_guest_allowed(&self, realm: RealmName) -> bool {
        self.settings.get(&realm)
            .expect("Failed to get realm settings").read()
            .unwrap()
            .is_guest_allowed()
    }

    pub fn get_authentication_token_duration(&self, realm: RealmName) -> Duration {
        self.settings.get(&realm)
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .get_authentication_token_duration()
    }

    pub fn get_refresh_token_duration(&self, realm: RealmName) -> Duration {
       self.settings.get(&realm)
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
           .get_refresh_token_duration()
    }

    pub fn get_password_reset_token_duration(&self, realm: RealmName) -> Duration {
        self.settings.get(&realm).expect("Failed to get realm settings")
            .read()
            .unwrap()
            .get_password_reset_token_duration()
    }

    pub fn reload(&self) -> &Self {
        let mut settings = &self.settings.clone();
        let mut rng = rand::thread_rng();

        for (realm, value) in settings.as_ref().iter() {
            let mut lock = value.write().unwrap();
            *lock = InternalRealmSettings {
                is_confirmation_required: false,
                is_guest_allowed: false,
                authentication_token_duration: Duration::new(rng.gen_range(0..180), 0),
                refresh_token_duration: Duration::new(rng.gen_range(0..90), 0),
                password_reset_token_duration: Duration::new(rng.gen_range(0..50), 0),
            };
            println!("updated realm settings for {}", realm);
        }

        self
    }
}

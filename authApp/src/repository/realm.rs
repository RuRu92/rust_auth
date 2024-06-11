use mysql::AccessMode;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::db::{ExecutionContext, DB};
use crate::domain::realm::{InternalRealmSettings, Realm, RealmName, RealmSettings};

pub struct RealmSettingProvider {
    settings: Arc<HashMap<RealmName, RwLock<InternalRealmSettings>>>,
    db: Arc<DB>,
}

impl RealmSettingProvider {
    pub fn init(db: Arc<DB>) -> RealmSettingProvider {
        let realms = vec!["rj.fg", "rj.wire", "rj.fa"];

        let mut realm_settings = HashMap::new();
        for r in realms {
            let _ = &realm_settings.insert(
                r.to_string(),
                RwLock::new(InternalRealmSettings {
                    is_confirmation_required: false,
                    is_guest_allowed: false,
                    realm_salt_itr: 10000,
                    authentication_token_duration: std::time::Duration::new(120, 0),
                    refresh_token_duration: std::time::Duration::new(60, 0),
                    password_reset_token_duration: std::time::Duration::new(30, 0),
                }),
            );
        }

        RealmSettingProvider {
            settings: Arc::new(realm_settings),
            db,
        }
    }

    pub fn is_confirmation_required(&self, realm: &str) -> bool {
        self.settings
            .get(realm)
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .is_confirmation_required()
    }

    pub fn is_guest_allowed(&self, realm: &str) -> bool {
        self.settings
            .get(&realm.to_string())
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .is_guest_allowed()
    }

    pub fn get_authentication_token_duration(&self, realm: &str) -> Duration {
        self.settings
            .get(&realm.to_string())
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .get_authentication_token_duration()
    }

    pub fn get_refresh_token_duration(&self, realm: &str) -> Duration {
        self.settings
            .get(&realm.to_string())
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .get_refresh_token_duration()
    }

    pub fn get_password_reset_token_duration(&self, realm: &str) -> Duration {
        self.settings
            .get(&realm.to_string())
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .get_password_reset_token_duration()
    }

    pub fn get_realm_salt_itr(&self, realm: &str) -> u32 {
        self.settings
            .get(&realm.to_string())
            .expect("Failed to get realm settings")
            .read()
            .unwrap()
            .realm_salt_itr()
    }

    pub fn reload(&self) -> &Self {
        let mut settings = &self.settings.clone();
        let mut rng = rand::thread_rng();

        for (realm, value) in settings.as_ref().iter() {
            let mut lock = value.write().unwrap();
            *lock = InternalRealmSettings {
                is_confirmation_required: false,
                is_guest_allowed: false,
                realm_salt_itr: rng.gen_range(4000..20000),
                authentication_token_duration: Duration::new(rng.gen_range(0..180), 0),
                refresh_token_duration: Duration::new(rng.gen_range(0..90), 0),
                password_reset_token_duration: Duration::new(rng.gen_range(0..50), 0),
            };
            println!("updated realm settings for {}", realm);
        }

        self
    }
}

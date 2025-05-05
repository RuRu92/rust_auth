pub mod token;

pub mod customer_service {
    use crate::db::DB;
    use crate::domain::customer::{dto::CreateUser, Role, User, UserWithAddress};
    use crate::domain::realm::RealmName;
    use crate::repository::realm::RealmSettingProvider;
    use crate::repository::{AddressStorage, Repository, UserStorage};
    use crate::{AppState, Principal};
    use actix_web::http::header::{HeaderMap, HeaderValue};
    use mysql::{AccessMode, Error, Result, Transaction};
    use std::str::FromStr;
    use std::sync::Arc;
    use std::{clone::Clone, option::Option};

    pub struct AuthenticatorService {}

    impl AuthenticatorService {
        pub fn initialise_token(data: &User) -> String {
            "a".parse().unwrap()
        }
    }

    pub struct CustomerService {}

    impl CustomerService {
        // type Handler = fn(&mut Transaction) -> Result<_>;

        pub fn fetch_users(db_context: &DB) -> std::result::Result<Vec<UserWithAddress>, Error> {
            let principal_vec: Vec<UserWithAddress> = db_context
                .in_transaction(AccessMode::ReadWrite, CustomerService::handle_fetch_users());

            Ok(principal_vec)
        }

        pub fn fetch_user(
            user_id: &String,
            db_context: &DB,
        ) -> std::result::Result<Option<User>, Error> {
            Ok(
                db_context.in_transaction(AccessMode::ReadWrite, |tx: &mut Transaction| {
                    UserStorage::get_user(user_id, tx)
                }),
            )
        }

        pub fn fetch_user_by_name(
            username: &String,
            realm: &RealmName,
            db_context: &DB,
        ) -> Option<User> {
            db_context.in_transaction(AccessMode::ReadWrite, |tx: &mut Transaction| {
                UserStorage::get_user_by_name(username, realm, tx)
            })
        }

        pub fn create(
            user_data: CreateUser,
            realm: RealmName,
            app: &AppState,
        ) -> std::result::Result<String, Error> {
            let realm_settings_provider = &app.realm_settings_provider;
            let db_context = &app.execution_context.db;

            let result = db_context.in_transaction(
                AccessMode::ReadWrite,
                CustomerService::handle_create_user(
                    user_data,
                    &realm,
                    realm_settings_provider.clone(),
                ),
            );
            let user_id = result.0;
            println!("user_id {}", &user_id);

            return Ok(user_id);
        }

        fn handle_create_user(
            mut user_data: CreateUser,
            realm: &RealmName,
            arc: Arc<RealmSettingProvider>,
        ) -> impl FnOnce(&mut Transaction) -> Result<(String, String)> + '_ {
            return move |tx: &mut Transaction| {
                let address = user_data.address.clone();
                user_data.hash_password(realm);
                let user_id =
                    UserStorage::create_from(user_data, realm, tx).expect("Failed to create user");
                let address_id = AddressStorage::create_from((address, user_id.to_owned()), realm, tx)
                    .expect("Failed to create user");
                Ok((user_id, address_id))
            };
        }

        fn handle_fetch_users() -> fn(&mut Transaction) -> Result<Vec<UserWithAddress>> {
            return |tx: &mut Transaction| UserStorage::get_users(tx);
        }
    }
}

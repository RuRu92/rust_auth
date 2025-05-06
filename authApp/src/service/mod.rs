pub mod token;

pub mod customer_service {
    use crate::db::{DB};
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
    use crate::app::{APIError, APIResult};

    pub struct AuthenticatorService {}

    impl AuthenticatorService {
        pub fn initialise_token(data: &User) -> String {
            "a".parse().unwrap()
        }
    }

    pub struct CustomerService {}

    impl CustomerService {
        pub fn fetch_users(realm: &RealmName, db_context: &DB) -> APIResult<Vec<UserWithAddress>, APIError> {
            db_context.in_transaction(AccessMode::ReadWrite,
                 |tx| UserStorage::get_users(realm, tx))
        }

        pub fn fetch_user(
            user_id: &String,
            db_context: &DB,
        ) -> APIResult<Option<User>> {
            db_context.in_transaction(AccessMode::ReadWrite, |tx: &mut Transaction| {
                UserStorage::get_user(user_id, tx)
            })
        }

        pub fn fetch_user_by_name(
            username: &String,
            realm: &RealmName,
            db_context: &DB,
        ) -> APIResult<Option<User>> {
            db_context.in_transaction(AccessMode::ReadWrite, |tx: &mut Transaction| {
                UserStorage::get_user_by_name(username, realm, tx)
            })
        }

        pub fn create(
            mut user_data: CreateUser,
            realm: RealmName,
            app: &AppState,
        ) -> APIResult<String> {
            let db_context = &app.execution_context.db;

            let create_result = user_data.hash_password(&realm).and_then(|_| {
                db_context.in_transaction(
                    AccessMode::ReadWrite,
                    CustomerService::handle_create_user(user_data),
                )
            });

            match create_result {
                Ok((user_id, _)) => Ok(user_id.to_string()),
                Err(err) => Err(err),
            }
        }

        fn handle_create_user(user_data: CreateUser) -> impl FnMut(&mut Transaction) -> APIResult<(String, String), mysql::Error> {
            move |tx: &mut Transaction| {
                let address = user_data.address.clone();
                let user_id =
                    UserStorage::create_from(user_data.clone(), tx).expect("Failed to create user");
                let address_id = AddressStorage::create_from((user_data.realm.clone(), user_id.to_owned(), address), tx)
                    .expect("Failed to create user");
                Ok((user_id, address_id))
            }
        }
    }

    // fn handle_fetch_users() -> fn(&mut Transaction) -> APIResult<Vec<UserWithAddress>> {
    //     |tx: &mut Transaction| UserStorage::get_users(tx).map_err(|err| APIError::DBException(err))
    // }
}


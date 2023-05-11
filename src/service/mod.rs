pub mod customer_service {
    use std::str::FromStr;
    use crate::domain::customer::{User, Role, dto::CreateUser, UserWithAddress};
    use crate::repository::{UserStorage, AddressStorage, Repository};
    use crate::db::DB;
    use mysql::{AccessMode, Transaction, Result, Error};
    use std::{
        option::Option,
        clone::Clone,
    };
    use crate::Principal;

    // type Handler = fn(&mut Transaction) -> Result<_>;

    pub fn fetch_users(db_context: &DB) -> std::result::Result<Vec<UserWithAddress>, Error> {
        let principal_vec: Vec<UserWithAddress> =
            db_context.in_transaction(
                AccessMode::ReadWrite,
                handle_fetch_users());

        Ok(principal_vec)
    }

    pub fn fetch_user(user_id: &String, db_context: &DB) -> std::result::Result<Option<User>, Error> {
        Ok(
            db_context.in_transaction(
            AccessMode::ReadWrite,
            |tx: &mut Transaction| {
                UserStorage::get_user(user_id, tx)
            })
        )
    }

    pub fn create(user_data: CreateUser, db_context: &DB) -> std::result::Result<String, Error> {
        let result = db_context.in_transaction(AccessMode::ReadWrite,
                                               handle_create_user(user_data));
        let user_id = result.0;
        println!("user_id {}", &user_id);

        return Ok(user_id);
    }

    fn handle_create_user(user_data: CreateUser) -> impl FnOnce(&mut Transaction) -> Result<(String, String)> {
        return |tx: &mut Transaction| {
            let address = user_data.address.clone();
            let user_id = UserStorage::create_from(user_data, tx).expect("Failed to create user");
            let address_id = AddressStorage::create_from((address, user_id.to_owned()), tx).expect("Failed to create user");
            Ok((user_id, address_id))
        };
    }

    fn handle_fetch_users() -> fn(&mut Transaction) -> Result<Vec<UserWithAddress>> {
        return |tx: &mut Transaction| {
            UserStorage::get_users(tx)
        };
    }
}
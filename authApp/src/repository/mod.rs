use crate::domain::customer::dto::{CreateUser, UserMetadata};
use crate::domain::customer::{Address, Role, User, UserWithAddress};
use crate::domain::realm::RealmName;
use crate::Principal;
use mysql::error::Result;
use mysql::prelude::Queryable;
use mysql::{params, Transaction};
use uuid::Uuid;

pub mod realm;

type CountryCode = String;

pub trait Repository<T> {
    type CreationData;
    type UpdateMetaData;
    type ID;

    fn create_from(data: Self::CreationData, realm: &RealmName, tx: &mut Transaction) -> Result<Self::ID>;

    fn update(data: Self::UpdateMetaData, tx: &mut Transaction) -> bool;

    fn delete(id: Self::ID, tx: &mut Transaction) -> ();
}

pub struct UserStorage {}

pub struct AddressStorage {}

impl UserStorage {
    pub fn get_user(user_id: &String, tx: &mut Transaction) -> Result<Option<User>> {
        tx.exec_first(
            "SELECT \
                    user_id, \
                    name,\
                    password, \
                    role \
                    FROM realm_user \
                    WHERE user_id = :user_id",
            params! { "user_id" => user_id },
        )
        .map(|row| {
            //Unpack Option
            row.map(|(user_id, name, password, role)| User {
                user_id,
                username: name,
                hashed_pass: password,
                role,
            })
        })
    }

    pub fn get_user_by_name(
        username: &String,
        realm: &RealmName,
        tx: &mut Transaction,
    ) -> Result<Option<User>> {
        tx.exec_first(
            "SELECT \
                    user_id, \
                    username,\
                    password, \
                    role \
                    FROM ream_user \
                    WHERE username = :username\
                    AND realm = :realm",
            params! {
                "username" => username,
                "realm" => realm
            },
        )
        .map(|row| {
            //Unpack Option
            row.map(|(user_id, name, password, role)| User {
                user_id,
                username: name,
                hashed_pass: password,
                role,
            })
        })
    }

    pub fn get_users(tx: &mut Transaction) -> Result<Vec<UserWithAddress>> {
        tx.query_map(
            "SELECT \
            u.user_id, \
            u.name,\
            u.role, \
            a.street, \
            a.city, \
            a.post_code, \
            a.country \
            FROM realm_user u \
            INNER JOIN address a on u.user_id = a.user_id ",
            |(id, name, role, street, city, post_code, country)| UserWithAddress {
                user_id: id,
                role,
                username: name,
                address: Address {
                    street,
                    country,
                    city,
                    post_code,
                },
            },
        )
    }
}

impl Repository<User> for UserStorage {
    type CreationData = CreateUser;
    type UpdateMetaData = UserMetadata;
    type ID = String;

    fn create_from(data: Self::CreationData, realm: &RealmName, tx: &mut Transaction) -> Result<Self::ID> {
        let user_id = Uuid::new_v4().to_string();

        tx.exec_drop(
            "INSERT INTO realm_user (realm_name, user_id, username, role, name, password, email) \
                      VALUES (:realm, :user_id, :username, :role, :name, :password, :email)",
            params! {
            "realm" => realm,
            "user_id" => &user_id,
            "username" => &data.username,
            "role" => Role::CUSTOMER.to_string(),
            "name" => &data.name,
            "password" => &data.password,
            "email" => &data.email,
            "expired_at" => chrono::DateTime(),
            },
        )
        .expect("Failed to create user");

        return Ok(user_id);
    }

    fn update(data: Self::UpdateMetaData, tx: &mut Transaction) -> bool {
        AddressStorage::update((data.address, data.user_id), tx)
    }

    fn delete(id: Self::ID, tx: &mut Transaction) -> () {
        tx.exec_drop(
            "DELETE FROM USER WHERE user_id = :user_id",
            params! { "user_id" => id },
        )
        .expect("Failed to delete user");
        // match to print logs
    }
}

impl AddressStorage {
    // fn map_to_country_code(country: String) -> CountryCode {
    //     match country.as_ref().map(String::as_ref) {
    //         String::from("United Kingdom") => "UK".to_string(),
    //         String::from("Great Britain") => "UK".to_string(),
    //         String::from("Denmark") => "DK".to_string()
    //         // "Russia" => "RU",
    //         // "America" => "USA",
    //     }
    // }
}

impl Repository<Address> for AddressStorage {
    type CreationData = (Address, String);
    type UpdateMetaData = (Option<Address>, String);
    type ID = String;

    fn create_from(data: Self::CreationData, _: &RealmName, tx: &mut Transaction) -> Result<Self::ID> {
        let address = &data.0;
        let user_id: String = data.1;
        let address_id = Uuid::new_v4().to_string();

        tx.exec_drop(
            "INSERT INTO address (\
            address_id, \
            user_id, \
            street, \
            post_code, \
            country, \
            city, \
            country_code) \
            VALUES (\
             :address_id, \
             :user_id, \
             :street, \
             :city, \
             :post_code, \
             :country, \
             :country_code)",
            params! {
            "address_id" => &address_id,
            "user_id" => &user_id,
            "street" => &address.street,
            "city" => &address.city,
            "post_code" => &address.post_code,
            "country" => &address.country,
            "country_code" => "UK".to_string() },
        )
        .expect("Failed to insert user address");

        return Ok(address_id);
    }

    fn update(data: Self::UpdateMetaData, tx: &mut Transaction) -> bool {
        let maybe_address = data.0;
        let user_id: String = data.1;

        match maybe_address {
            Some(address) => {
                let result = tx.exec_drop(
                    "UPDATE Address
                SET street = :street,
                    post_code = :post_code,
                    country = :country,
                WHERE user_id = :user_id",
                    (
                        "street",
                        &address.street,
                        "post_code",
                        &address.post_code,
                        "country",
                        &address.country,
                        "user_id",
                        &user_id,
                    ),
                );

                match result {
                    Ok(_) => true,
                    Err(x) => {
                        println!("error encountered for user {}, \n {}", user_id, x);
                        return false;
                    }
                }
            }
            None => true,
        }
    }

    fn delete(id: Self::ID, tx: &mut Transaction) -> () {
        tx.exec_drop(
            "DELETE FROM ADDRESS WHERE address_id = :address_id",
            ("address_id", id),
        )
        .expect("Failed to delete address");
    }
}

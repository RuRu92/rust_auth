use log::log;
use crate::domain::customer::dto::{CreateUser, UserMetadata};
use crate::domain::customer::{Address, Role, User, UserWithAddress};
use crate::domain::realm::{Realm, RealmName};
use crate::Principal;
use mysql::error::Result;
use mysql::prelude::Queryable;
use mysql::{params, Transaction};
use mysql_common::prelude::FromRow;
use uuid::Uuid;
use crate::app::{APIError, APIResult};

pub mod realm;

type CountryCode = String;

pub trait WithRealm {
    fn realm_name(&self) -> &RealmName;
}

pub trait Repository<T> {
    type CreationData: WithRealm;
    type UpdateMetaData;
    type ID;

    fn create_from(data: Self::CreationData, tx: &mut Transaction) -> APIResult<Self::ID, mysql::Error>;

    fn update(data: Self::UpdateMetaData, tx: &mut Transaction) -> APIResult<bool, mysql::Error>;

    fn delete(id: Self::ID, tx: &mut Transaction) -> APIResult<(), mysql::Error>;
}

pub struct UserStorage {}

pub struct AddressStorage {}

impl UserStorage {
    pub fn get_user(user_id: &String, tx: &mut Transaction) -> APIResult<Option<User>, mysql::Error> {
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
            .map_err(|err| err)
    }

    pub fn get_user_by_name(
        username: &String,
        realm: &RealmName,
        tx: &mut Transaction,
    ) -> APIResult<Option<User>, mysql::Error> {
        tx.exec_first(
            "SELECT \
                    user_id, \
                    username,\
                    password, \
                    role \
                    FROM realm_user \
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

    pub fn get_users(realm: &RealmName, tx: &mut Transaction) -> APIResult<Vec<UserWithAddress>, mysql::Error> {
        tx.exec_map(
            "SELECT \
            u.user_id, \
            u.name,\
            u.role, \
            a.street, \
            a.city, \
            a.post_code, \
            a.country \
            FROM realm_user u \
            INNER JOIN address a on u.user_id = a.user_id
            WHERE realm = :realm",
            params! {
                "realm" => realm
            },
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

    fn create_from(data: Self::CreationData, tx: &mut Transaction) -> APIResult<Self::ID, mysql::Error> {
        let user_id = Uuid::new_v4().to_string();
        println!("Generated UUID: {}", user_id);
        tx.exec_drop(
            "INSERT INTO realm_user (user_id, realm_name, username, role, name, password, email) \
                      VALUES (:user_id, :realm, :username, :role, :name, :password, :email)",
            params! {
            "user_id" => &user_id,
            "realm" => &data.realm,
            "username" => &data.username,
            "role" => Role::CUSTOMER.to_string(),
            "name" => &data.name,
            "password" => &data.password,
            "email" => &data.email,
            },
        )
            .expect("Failed to create user");

        Ok(user_id)
    }

    fn update(data: Self::UpdateMetaData, tx: &mut Transaction) -> APIResult<bool, mysql::Error> {
        AddressStorage::update((data.address, data.user_id), tx)
    }

    fn delete(id: Self::ID, tx: &mut Transaction) -> APIResult<(), mysql::Error> {
        tx.exec_drop(
            "DELETE FROM USER WHERE user_id = :user_id",
            params! { "user_id" => id },
        )
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

impl WithRealm for (RealmName, String, Address) {
    fn realm_name(&self) -> &RealmName {
        &self.0
    }
}

impl Repository<Address> for AddressStorage {
    type CreationData = (RealmName, String, Address);
    type UpdateMetaData = (Option<Address>, String);
    type ID = String;

   

    fn create_from(data: Self::CreationData, tx: &mut Transaction) -> APIResult<Self::ID, mysql::Error> {
        let realm = &data.0;
        let user_id: String = data.1;
        let address = data.2;
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
        ).expect("Failed to create address");

        Ok(address_id)
    }

    fn update(data: Self::UpdateMetaData, tx: &mut Transaction) -> APIResult<bool, mysql::Error> {
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
                    Ok(_) => Ok(true),
                    Err(x) => {
                        println!("error encountered for user {}, \n {}", user_id, x);
                        Err(x)
                    }
                }
            }
            None => Ok(true),
        }
    }

    fn delete(id: Self::ID, tx: &mut Transaction) -> APIResult<(), mysql::Error> {
        tx.exec_drop(
            "DELETE FROM ADDRESS WHERE address_id = :address_id",
            ("address_id", id),
        )
    }
}

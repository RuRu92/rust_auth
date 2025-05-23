pub mod infra;
pub mod realm;

pub mod customer {
    use actix_web::body::MessageBody;
    use mysql::{FromValueError, Value};
    use mysql_common;
    use serde::{Deserialize, Serialize};
    use core::panic;
    use std::fmt::{Display, Formatter as FMT_Formatter};
    use std::str::FromStr;
    use strum_macros::EnumString;
    use mysql::prelude::FromValue;    

    
    #[derive(Serialize, Deserialize, FromValue, EnumString, Clone, Debug)]
    #[mysql(is_string)]
    pub enum Role {
        ADMIN,
        CUSTOMER,
    }

    impl Display for Role {
        fn fmt(&self, f: &mut FMT_Formatter<'_>) -> std::fmt::Result {
            match *self {
                Role::CUSTOMER => write!(f, "CUSTOMER"),
                Role::ADMIN => write!(f, "ADMIN"),
            }
        }
    }


    #[derive(Serialize, Deserialize, Clone, Debug)]
     pub struct LoginRequest {
        pub username: String,
        pub password: String,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct LoginRequestArguments {
        pub login_request: LoginRequest,
        pub user: User,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct User {
        pub user_id: String,
        pub username: String,
        pub hashed_pass: String,
        pub role: Role,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct UserWithAddress {
        pub user_id: String,
        pub username: String,
        pub role: Role,
        pub address: Address,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct Address {
        pub street: String,
        pub country: String,
        pub city: String,
        pub post_code: String,
    }

    pub mod dto {
        use crate::domain::customer::Address;
        use crate::domain::realm::{Realm, RealmName};
        use mysql::prelude::FromValue;
        use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
        use pbkdf2::Pbkdf2;
        use rand::rngs::OsRng;
        use serde::{Deserialize, Serialize};
        use crate::app::Error;

        #[derive(Serialize, Deserialize, Clone, Debug)]
        pub struct CreateUser {
            pub username: String,
            pub password: String,
            pub name: String,
            pub age: i32,
            pub email: String,
            pub address: Address,
        }

        impl CreateUser {
            pub fn hash_password(&mut self, realm: &RealmName) {
                let salt_format = format!("{}|{}", &self.username, realm).into_bytes();
                let salt = SaltString::encode_b64(salt_format.as_slice()).unwrap();
                match Pbkdf2.hash_password(self.password.as_bytes(), &salt) {
                    Ok(x) => {
                        println!("Hashed pwd to: {}", &self.password);
                        self.password = String::from(x.to_string().as_str());
                    }
                    Err(_) => {
                        println!("Failed to hash password");
                    }
                };
            }
        }

        #[derive(Serialize, Deserialize, Clone, Debug)]
        pub struct UserMetadata {
            pub user_id: String,
            pub address: Option<Address>,
        }
    }
}

mod test {
    use crate::domain::customer::dto::CreateUser;
    use crate::domain::customer::Address;
    use crate::domain::realm::RealmName;
    use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
    use pbkdf2::Pbkdf2;
    use rand::rngs::OsRng;
    use std::num::NonZeroU32;

    #[test]
    fn test_hashing_password() {
        let mut user = CreateUser {
            username: "ruru".to_string(),
            password: "password".to_string(),
            name: "RuRu".to_string(),
            age: 21,
            email: "ruru@nitro.com".to_string(),
            address: Address {
                street: "The Street".to_string(),
                country: "UK".to_string(),
                city: "London".to_string(),
                post_code: "W1 2DE".to_string(),
            },
        };
        let iter = NonZeroU32::new(4026).unwrap();
        let realm = &"rj.nitro".to_string();
        let current_pass = user.password.clone();

        println!("Current Pass: {}", &current_pass);
        let salt = format!("{}|{}", &user.username, realm).into_bytes();
        let password_hash  =  user.hash_password(&realm).unwrap();

        println!("Hashed Pass: {}", &password_hash);

        // Verify password against PHC string
        let parsed_hash = PasswordHash::new(&password_hash).unwrap();
        assert!(Pbkdf2.verify_password(&current_pass.as_bytes(), &parsed_hash).is_ok());

    }
}

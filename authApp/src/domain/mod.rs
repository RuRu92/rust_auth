pub mod infra;
pub mod realm;

pub mod customer {
    use mysql::Value;
    use serde::{Deserialize, Serialize};
    use std::fmt::{Display, Formatter as FMT_Formatter};
    use std::str::FromStr;
    use actix_web::http::header::ByteRangeSpec::From;
    use mysql_common::prelude::{FromRow, FromValue};
    use mysql_common::{FromRowError, FromValueError, Row};
    use mysql_common::misc::raw::bytes;
    use strum_macros::EnumString;
    use mysql_common::serde_json;

    #[derive(Serialize, Deserialize, EnumString, Clone, Debug)]
    pub enum Role {
        ADMIN,
        CUSTOMER,
    }

    impl Role {
        fn from_value(v: Value) -> Result<Self, FromValueError> {
            match v {
                Value::Bytes(y) => {
                    let str = String::from_utf8(y).unwrap();
                    Ok(Role::from_str(&str).unwrap())
                }
                x => Err(FromValueError(x))
            }
        }
    }

    impl std::convert::From<String> for Role {
        fn from(s: String) -> Self {
            Role::from_str(&s).unwrap()
        }
    }

    impl FromValue for Role {
        type Intermediate = String;

        fn from_value(v: Value) -> Self {
            if let Value::Bytes(raw) = v {
                let str = String::from_utf8(raw).unwrap();
                return Role::from_str(&str).unwrap();
            };
            panic!("Failed to get value for Role");
        }

        fn from_value_opt(v: Value) -> Result<Self, FromValueError> {
            Role::from_value(v)
        }


        fn get_intermediate(v: Value) -> Result<Self::Intermediate, FromValueError> {
            match v {
                Value::Bytes(raw) => {
                    let s = String::from_utf8(raw).unwrap();
                    Ok(s)
                }
                x => Err(FromValueError(x))
            }
        }
    }

    pub fn role_des(val: Value) -> Result<Role, FromValueError> {
        match val {
            Value::Bytes(raw) => {
                let s = String::from_utf8(raw).unwrap();
                let role = Role::from_str(s.as_str());
                Ok(role.unwrap())
            }
            x => Err(FromValueError(x))
        }
    }

    pub fn role_ser(role: Role) -> Value {
        Value::Bytes(role.to_string().into_bytes())
    }

    impl Display for Role {
        fn fmt(&self, f: &mut FMT_Formatter<'_>) -> std::fmt::Result {
            match *self {
                Role::CUSTOMER => write!(f, "CUSTOMER"),
                Role::ADMIN => write!(f, "ADMIN"),
            }
        }
    }

    #[derive(Debug)]
    pub struct EnumIr {
        string: String,
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
        use pbkdf2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
        use pbkdf2::Pbkdf2;
        use rand::rngs::OsRng;
        use serde::{Deserialize, Serialize};
        use crate::app::{APIError, APIResult};

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
            pub fn hash_password(&mut self, realm: &RealmName) -> APIResult<String> {
                let secret_format = format!("{}|{}", &self.username, realm).into_bytes();
                let secret = SaltString::encode_b64(secret_format.as_slice()).unwrap();
                match Pbkdf2.hash_password(self.password.as_bytes(), &secret) {
                    Ok(hash) => {
                        self.password = hash.to_string();
                        println!("Hashed pwd to: {}", &self.password);
                        Ok(hash.to_string())
                    }
                    Err(_) => Err(APIError::PasswordHashing)
                }
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
        let password_hash = user.hash_password(&realm).unwrap();

        println!("Hashed Pass: {}", &password_hash);

        // Verify password against PHC string
        let parsed_hash = PasswordHash::new(&password_hash).unwrap();
        assert!(Pbkdf2.verify_password(&current_pass.as_bytes(), &parsed_hash).is_ok());
    }
}

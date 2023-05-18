pub mod realm;
pub mod infra;

pub mod customer {
    use serde::{Serialize, Deserialize};
    use mysql::prelude::{FromValue, ConvIr};
    use mysql::{Value};
    use std::str::FromStr;
    use strum_macros::EnumString;
    use std::fmt::{Formatter as FMT_Fromatter, Display};

    #[derive(Serialize, Deserialize, EnumString, Clone, Debug)]
    pub enum Role {
        ADMIN,
        CUSTOMER,
    }

    impl Display for Role {
        fn fmt(&self, f: &mut FMT_Fromatter<'_>) -> std::fmt::Result {
            match *self {
                Role::CUSTOMER => write!(f, "CUSTOMER"),
                Role::ADMIN => write!(f, "ADMIN")
            }
        }
    }

    #[derive(Debug)]
    pub struct EnumIr {
        string: String,
    }

    impl ConvIr<Role> for EnumIr {
        fn new(v: Value) -> std::result::Result<EnumIr, mysql::FromValueError> {
            match v {
                Value::Bytes(bytes) => match String::from_utf8(bytes) {
                    Ok(string) => Ok(EnumIr { string }),
                    Err(e) => Err(mysql::FromValueError(Value::Bytes(e.into_bytes()))),
                },
                v => Err(mysql::FromValueError(v)),
            }
        }

        fn commit(self) -> Role {
            Role::from_str(&self.string).unwrap()
        }

        fn rollback(self) -> Value {
            Value::Bytes(self.string.into_bytes())
        }
    }

    impl FromValue for Role {
        type Intermediate = EnumIr;
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub struct User {
        pub user_id: String,
        pub username: String,
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
        use serde::{Serialize, Deserialize};
        use crate::domain::customer::Address;

        #[derive(Serialize, Deserialize, Clone, Debug)]
        pub struct CreateUser {
            pub username: String,
            pub name: String,
            pub age: i32,
            pub email: String,
            pub address: Address,
        }

        #[derive(Serialize, Deserialize, Clone, Debug)]
        pub struct UserMetadata {
            pub user_id: String,
            pub address: Option<Address>,
        }
    }
}



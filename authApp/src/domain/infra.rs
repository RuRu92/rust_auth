use crate::domain::realm::{Realm, RealmName, RealmSettings, UserRealmSettings};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::http::header::{ContentType, HeaderName, HeaderValue, CONTENT_TYPE};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mysql::serde_json;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use strum_macros::Display;

pub mod web {
    use crate::domain::realm::{Realm, RealmName};
    use actix_web::body::BoxBody;
    use actix_web::http::header::{ContentType, HeaderMap, HeaderValue};
    use actix_web::http::StatusCode;
    use actix_web::{HttpResponse, ResponseError};
    use mysql::serde_json;
    use serde::Serialize;
    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};

    pub trait RealmFinder {
        type Realm;
        fn get_realm(&self) -> Option<Self::Realm>;
    }

    impl RealmFinder for HeaderMap {
        type Realm = RealmName;

        fn get_realm(&self) -> Option<Self::Realm> {
            self.get("Realm")
                .map(|realm| realm.to_str().unwrap_or_else(|_e| "|").to_string())
        }
    }

    pub mod auth {
        use crate::domain::customer::{LoginRequest, LoginRequestArguments, User};
        use crate::domain::realm::{RealmName, UserRealmSettings};
        use data_encoding::HEXUPPER;
        use ring::digest::SHA256;
        use ring::pbkdf2 as pbk;
        use ring::rand::SystemRandom;
        use std::num::NonZeroU32;

        type Token = String;

        struct AppToken {
            username: String,
            password: String,
            realm_settings: UserRealmSettings,
            realm: RealmName,
        }

        pub fn verify_login(args: &LoginRequestArguments, realm: RealmName, iter: u32) -> bool {
            let login_request = &args.login_request;
            let salt = format!("{}|{}", &login_request.username, realm).into_bytes();

            let decoded_pass = HEXUPPER.decode(args.user.hashed_pass.as_bytes()).unwrap();

            let verified = pbk::verify(
                pbk::PBKDF2_HMAC_SHA256,
                NonZeroU32::new(iter).unwrap(),
                &salt,
                login_request.password.as_bytes(),
                &decoded_pass,
            );

            verified.is_ok()
        }

        struct AppAuthorizer {}

        // impl Authorizer for AppAuthorizer {
        //     fn generate_user_auth_token() -> Token {
        //
        //     }
        //
        //     fn mark_header(&self) {
        //         todo!()
        //     }
        //
        //     fn to_user_realm_settings(&self) -> Option<UserRealmSettings> {
        //         None
        //     }
        // }

        trait Authorizer {
            fn generate_user_auth_token();

            fn mark_header(&self);

            fn to_user_realm_settings(&self) -> UserRealmSettings;
        }
    }

    struct JsonRequest<T> {
        body: T,
        realm_name: Realm,
    }

    trait Identifiable<T> {
        fn get_id(&self) -> T;
    }

    pub struct JsonErrorResponse<T> {
        body: Option<T>,
        message: String,
        status_code: StatusCode,
    }

    impl<T> JsonErrorResponse<T> {
        pub fn new(
            body: Option<T>,
            message: String,
            status_code: StatusCode,
        ) -> JsonErrorResponse<T> {
            JsonErrorResponse {
                body,
                message,
                status_code,
            }
        }

        pub fn build_error(message: String, status_code: StatusCode) -> JsonErrorResponse<T> {
            JsonErrorResponse {
                body: None,
                message,
                status_code: Default::default(),
            }
        }
    }

    impl<T: Serialize> Display for JsonErrorResponse<T> {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            let serialized_body = serde_json::to_string(&self.body).unwrap_or_default();
            write!(
                f,
                "{{\"status\": {}, \"message\": \"{}\", \"body\": {}}}",
                self.status_code.as_u16(),
                self.message,
                serialized_body
            )
        }
    }

    impl<T: Debug> Debug for JsonErrorResponse<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("JsonErrorResponse")
                .field("body", &self.body)
                .field("message", &self.message)
                .field("status_code", &self.status_code)
                .finish()
        }
    }

    impl<T: Debug + Serialize> ResponseError for JsonErrorResponse<T> {
        fn status_code(&self) -> StatusCode {
            self.status_code
        }
        fn error_response(&self) -> HttpResponse<BoxBody> {
            HttpResponse::build(self.status_code)
                .insert_header(ContentType::json())
                .body(self.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mysql::chrono;
    use mysql::chrono::Utc;
    use std::fmt::Error;

    #[test]
    fn test_generate_token() {
        // let token = AppToken {
        //     password: "password".to_string(),
        //     username: "RuRu".to_string(),
        //     realm: "rj.wire".to_string(),
        //     realm_settings: UserRealmSettings {
        //         is_confirmation_required: true
        //     },
        // };
        //
        // let expiration = Utc::now()
        //     .checked_add_signed(chrono::Duration::seconds(60))
        //     .expect("valid timestamp")
        //     .timestamp();
        //
        // let header = Header::new(Algorithm::HS512);
        // let t = encode(&header, &token, &EncodingKey::from_secret(b"secret"))
        //     .map_err(|_| Error::JWTTokenCreationError);
        //
        // match t {
        //     Ok(val) => println!("val: {}", val),
        //     Err(_) => println!("fail")
        // }
    }
}

use crate::domain::realm::{Realm, RealmName, RealmSettings, UserRealmSettings};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::http::header::{ContentType, HeaderName, HeaderValue, CONTENT_TYPE};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use strum_macros::Display;

pub mod web {
    use crate::domain::customer::{LoginRequest, LoginRequestArguments, User};
    use crate::domain::realm::{Realm, RealmName, UserRealmSettings};
    use actix_web::body::BoxBody;
    use actix_web::http::header::{ContentType, HeaderMap, HeaderValue};
    use actix_web::http::StatusCode;
    use actix_web::web::Json;
    use actix_web::{HttpResponse, ResponseError};
    use base64::{engine::general_purpose, Engine as _};
    use chrono::{DateTime, Utc};
    use data_encoding::HEXUPPER;
    use jsonwebtoken::errors::Error;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use log::info;
    use mysql_common::serde_json;
    use rand::distr::Open01;
    use ring::digest::SHA256;
    use ring::pbkdf2 as pbk;
    use ring::rand::SystemRandom;
    use serde::{Deserialize, Serialize};
    use std::convert::Infallible;
    use std::f32::consts::E;
    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};
    use std::num::NonZeroU32;

    use crate::app::error::APIError;

    pub trait RealmFinder {
        type Realm;
        fn get_realm(&self) -> Option<Self::Realm>;
    }

    pub trait TokenFinder {
        type Token;
        fn get_token(&self) -> Option<Self::Token>;
    }

    impl RealmFinder for HeaderMap {
        type Realm = RealmName;

        fn get_realm(&self) -> Option<Self::Realm> {
            self.get("Realm")
                .map(|realm: &HeaderValue| realm.to_str().unwrap_or_else(|_e| "|").to_string())
        }
    }

    impl TokenFinder for HeaderMap {
        type Token = String;

        fn get_token(&self) -> Option<Self::Token> {
            self.get("Authorization")
                .and_then(|auth: &HeaderValue| auth.to_str().ok()) // Convert HeaderValue to &str
                .and_then(|auth_str| auth_str.strip_prefix("Bearer ")) // Strip "Bearer " prefix
                .map(|token| token.to_string()) // Convert to String
        }
    }

    pub mod auth {
        use crate::app::error::APIError;
        use crate::domain::customer::LoginRequestArguments;
        use crate::domain::realm::{RealmName, UserRealmSettings};
        use base64::engine::general_purpose;
        use base64::Engine;
        use data_encoding::HEXUPPER;
        use jsonwebtoken::{
            decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
        };
        use log::{error, info};
        use mysql_common::serde_json;
        use ring::digest::SHA256;
        use ring::pbkdf2 as pbk;
        use ring::rand::SecureRandom;
        use serde::{Deserialize, Serialize};
        use std::num::NonZeroU32;

        type Token = String;

        pub trait Authorizer {
            type Token;

            fn decode_token(token: &str, secret: &str) -> AppToken;

            fn verify_auth_token(incoming_token: &Self::Token, user_token: &Self::Token) -> bool;

            fn get_auth_token(header: &Header, claim: &AppToken, secret: String) -> Token;

            fn verify_login(args: &LoginRequestArguments, realm: &str, iter: u32) -> bool;
        }

        pub struct AppAuthorizer;

        #[derive(Debug, Serialize, Deserialize)]
        pub struct AppToken {
            pub username: String,
            pub password: String,
            pub realm_settings: UserRealmSettings,
            pub realm: RealmName,
            pub expiry: i64,
        }

        impl Authorizer for AppAuthorizer {
            type Token = String;

            fn decode_token(token: &str, secret: &str) -> AppToken {
                decode::<AppToken>(
                    token,
                    &DecodingKey::from_secret(secret.as_bytes()),
                    &Validation::new(Algorithm::HS256),
                )
                .unwrap()
                .claims
            }

            // type WebToken = String;
            fn get_auth_token(header: &Header, claim: &AppToken, secret: String) -> Token {
                encode(header, claim, &EncodingKey::from_secret(secret.as_bytes())).unwrap()
            }

            fn verify_auth_token(incoming_token: &Self::Token, user_token: &Self::Token) -> bool {
                incoming_token == user_token
            }

            fn verify_login(args: &LoginRequestArguments, realm: &str, iter: u32) -> bool {
                let login_request = &args.login_request;
                let salt = format!("{}|{}", &login_request.username, realm).into_bytes();
                let hashed_pass = args.user.hashed_pass.clone();
                info!("[verifyLogin] User pass = {hashed_pass}");

                let phc_parts = &args.user.hashed_pass.split('$').collect::<Vec<&str>>();

                if phc_parts.len() == 5 {
                    let hash_b64 = phc_parts[4];
                    match general_purpose::STANDARD.decode(hash_b64) {
                        Ok(decoded_pass) => {
                            let verified = pbk::verify(
                                pbk::PBKDF2_HMAC_SHA256,
                                NonZeroU32::new(iter).unwrap(),
                                &salt,
                                login_request.password.as_bytes(),
                                &decoded_pass,
                            );
                            verified.is_ok()
                        }
                        Err(err) => {
                            error!("[verifyLogin] Failed to decode password. Error = {err}");
                            return false;
                        }
                    }
                } else {
                    error!("[verifyLogin] Invalid PHC string format");
                    false
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::domain::infra::web::auth::{AppAuthorizer, AppToken, Authorizer};
        use crate::domain::realm::{Realm, RealmName, RealmSettings, UserRealmSettings};
        use actix_web::http::StatusCode;
        use chrono::{Days, Utc};
        use jsonwebtoken::{encode, EncodingKey, Header};
        use mysql_common::serde_json;
        use std::time::Duration;

        #[test]
        fn test_auth_token() {
            let mut header = Header::new(jsonwebtoken::Algorithm::ES256);
            let realm = RealmName::from("test");
            let realm_settings = UserRealmSettings::default();

            let dt = Utc::now().checked_add_days(Days::new(90));

            let claim = AppToken {
                username: "ruru".to_string(),
                password: "passw0rd".to_string(),
                realm_settings,
                realm,
                expiry: dt.unwrap().timestamp(),
            };
            let token = encode(
                &header,
                &claim,
                &EncodingKey::from_secret("test".as_bytes()),
            )
            .unwrap();
            print!("Token - {}\n", token);
            assert_eq!(token.len(), 268);
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
        pub fn empty_ok() -> JsonErrorResponse<T> {
            return JsonErrorResponse {
                body: None,
                message: "".to_string(),
                status_code: StatusCode::OK,
            };
        }

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
                status_code,
            }
        }

        pub fn set_message(&mut self, message: String) {
            self.message = message;
        }

        pub fn set_status(&mut self, status_code: StatusCode) {
            self.status_code = status_code;
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

    trait ApplicationError {
        type Error;
        fn to_json_response(self) -> JsonErrorResponse<Self::Error>;
    }

    #[derive(Debug)]
    pub enum LoginError {
        MissingAppState,
        MissingRealmHeader,
        DatabaseError(String),
        UserNotFound,
        AuthenticationFailed,
        // Other error types...
    }

    impl From<LoginError> for JsonErrorResponse<Option<String>> {
        fn from(err: LoginError) -> Self {
            match err {
                LoginError::MissingAppState => JsonErrorResponse::new(
                    None,
                    "App state not found".to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ),
                LoginError::MissingRealmHeader => JsonErrorResponse::new(
                    None,
                    "Must contain realm header".to_string(),
                    StatusCode::BAD_REQUEST,
                ),
                LoginError::DatabaseError(e) => {
                    info!("[ErrMap] Msg: {e}");
                    JsonErrorResponse::new(None, e, StatusCode::BAD_REQUEST)
                }
                LoginError::UserNotFound => JsonErrorResponse::new(
                    None,
                    "User not found".to_string(),
                    StatusCode::NOT_FOUND,
                ),
                LoginError::AuthenticationFailed => {
                    JsonErrorResponse::new(None, "Bad Auth".to_string(), StatusCode::BAD_REQUEST)
                } // Other cases...
            }
        }
    }

    // impl FromRisidual<Result<Infallible, >> {
    //
    // }
    //
}

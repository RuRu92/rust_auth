use crate::domain::realm::{Realm, RealmName, RealmSettings, UserRealmSettings};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::http::header::{ContentType, HeaderName, HeaderValue, CONTENT_TYPE};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use log::info;
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
            // info!("[get_token] Getting token from headers: {:?}", self);
            self.get("authorization")
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
        use pbkdf2::password_hash::{PasswordHash, PasswordVerifier};
        use pbkdf2::Pbkdf2;
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
            // #[serde(rename = "exp")]
            pub exp: i64,
        }

        impl Authorizer for AppAuthorizer {
            type Token = String;

            fn decode_token(token: &str, secret: &str) -> AppToken {
                decode::<AppToken>(
                    token,
                    &DecodingKey::from_secret(secret.as_bytes()),
                    &Validation::new(Algorithm::HS512),
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
                let hashed_pass = args.user.hashed_pass.clone();
                info!("[verifyLogin] User pass = {hashed_pass}");

                match PasswordHash::new(&hashed_pass) {
                    Ok(pwd) => Pbkdf2
                        .verify_password(&args.login_request.password.as_bytes(), &pwd)
                        .is_ok(),
                    Err(err) => {
                        error!("[verifyLogin] Failed to decode password. Error = {err}");
                        false
                    }
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use crate::domain::infra::web::auth::{AppAuthorizer, AppToken, Authorizer};
        use crate::domain::infra::web::TokenFinder;
        use crate::domain::realm::{Realm, RealmName, RealmSettings, UserRealmSettings};
        use crate::service::token;
        use actix_web::http::header::{HeaderMap, HeaderName, HeaderValue};
        use actix_web::http::StatusCode;
        use chrono::{Days, Utc};
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        use log::info;
        use mysql_common::serde_json;
        use std::time::Duration;

        #[test]
        fn get_bearer_token_from_header() {
            let mut header = actix_web::http::header::HeaderMap::new();
            header.insert(HeaderName::from_static("authorization"), 
            HeaderValue::from_static("Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJ1c2VybmFtZSI6InJ1cnUiLCJwYXNzd29yZCI6IiRwYmtkZjItc2hhMjU2JGk9NjAwMDAwLGw9MzIkY25WeWRYeHlhaTVvWVhabGJnJHF2QzRUeWcwOWZ3RGJVOWE4dEUwWHlmTDh4NDErblBWbnFoNDBoenhPazAiLCJyZWFsbV9zZXR0aW5ncyI6eyJ0aGVtZSI6IkRlZmF1bHQiLCJtZXRhZGF0YSI6eyJHZW5lcmljTWFwIjp7fX0sImlzX2NvbmZpcm1hdGlvbl9yZXF1aXJlZCI6ZmFsc2V9LCJyZWFsbSI6InJqLmhhdmVuIiwiZXhwaXJ5IjoxNzU1MTU0ODI1fQ.Bu-to4eyDqFXpwcbHuqFRitcmPa9YXpJNy19d7Ru8TDjEyZ0TnclC91IR7YQ8F8EH12ZhZHxr3gCvH_E8zZwqA")
        );

            let token = header.get_token();
            assert!(token.is_some());
            assert_eq!(
                token.unwrap(),
                "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJ1c2VybmFtZSI6InJ1cnUiLCJwYXNzd29yZCI6IiRwYmtkZjItc2hhMjU2JGk9NjAwMDAwLGw9MzIkY25WeWRYeHlhaTVvWVhabGJnJHF2QzRUeWcwOWZ3RGJVOWE4dEUwWHlmTDh4NDErblBWbnFoNDBoenhPazAiLCJyZWFsbV9zZXR0aW5ncyI6eyJ0aGVtZSI6IkRlZmF1bHQiLCJtZXRhZGF0YSI6eyJHZW5lcmljTWFwIjp7fX0sImlzX2NvbmZpcm1hdGlvbl9yZXF1aXJlZCI6ZmFsc2V9LCJyZWFsbSI6InJqLmhhdmVuIiwiZXhwaXJ5IjoxNzU1MTU0ODI1fQ.Bu-to4eyDqFXpwcbHuqFRitcmPa9YXpJNy19d7Ru8TDjEyZ0TnclC91IR7YQ8F8EH12ZhZHxr3gCvH_E8zZwqA"
            );
        }

        #[test]
        fn test_auth_token() {
            let mut header = Header::new(Algorithm::HS512);
            let realm = RealmName::from("test");
            let realm_settings = UserRealmSettings::default();

            let dt = Utc::now().checked_add_days(Days::new(90));

            let claim = AppToken {
                username: "ruru".to_string(),
                password: "passw0rd".to_string(),
                realm_settings,
                realm,
                exp: dt.unwrap().timestamp(),
            };
            let token = encode(
                &header,
                &claim,
                &EncodingKey::from_secret("test".as_bytes()),
            )
            .unwrap();
            print!("Token - {}\n", token);
            assert_eq!(token.len(), 354);
        }

        #[test]
        fn test_decode_auth_token() {
            let mut header = HeaderMap::with_capacity(2);
            let realm = "test";

            // Use lowercase for header name!
            header.insert(
                HeaderName::from_static("realm"),
                HeaderValue::from_str(realm).unwrap(),
            );
            header.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJ1c2VybmFtZSI6InJ1cnUiLCJwYXNzd29yZCI6InBhc3N3MHJkIiwicmVhbG1fc2V0dGluZ3MiOnsidGhlbWUiOiJEZWZhdWx0IiwibWV0YWRhdGEiOnsicmVhbG1fZGF0YSI6e319LCJpc19jb25maXJtYXRpb25fcmVxdWlyZWQiOmZhbHNlfSwicmVhbG0iOiJ0ZXN0IiwiZXhwIjoxNzU3ODQ1NjI0fQ.VGRGy6MTeCHA-Akbd-6Tsd-ttH01bLbcRyOohYFDN8PtiH56X9xoOe-nFDnHH3Q-1gITThokIsFMQUn972354w"));

            println!("[testDecodeAuthToken] Header: {:?}", header);

            let token = header.get_token();
            assert!(token.is_some());
            let token = token.unwrap();
            let data = AppAuthorizer::decode_token(&token, "test");

            assert_eq!(data.username, "ruru");
            assert_eq!(data.password, "passw0rd");
            assert_eq!(data.realm, RealmName::from("test"));
            assert!(data.exp > 0);
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

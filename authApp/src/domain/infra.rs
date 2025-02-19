use crate::domain::realm::{Realm, RealmName, RealmSettings, UserRealmSettings};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::http::header::{ContentType, HeaderName, HeaderValue, CONTENT_TYPE};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
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
    use chrono::{DateTime, Utc};
    use data_encoding::HEXUPPER;
    use jsonwebtoken::errors::Error;
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
    use ring::digest::SHA256;
    use ring::pbkdf2 as pbk;
    use ring::rand::SystemRandom;
    use serde::{Deserialize, Serialize};
    use std::convert::Infallible;
    use std::f32::consts::E;
    use std::fmt;
    use std::fmt::{Debug, Display, Formatter};
    use std::num::NonZeroU32;

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
        use crate::domain::customer::LoginRequestArguments;
        use crate::domain::realm::{RealmName, UserRealmSettings};
        use data_encoding::HEXUPPER;
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        use serde::{Deserialize, Serialize};
        use std::num::NonZeroU32;
        use ring::digest::SHA256;
        use ring::pbkdf2 as pbk;
        use ring::rand::SecureRandom;
        use mysql_common::serde_json;

        type Token = String;

        #[derive(Debug, Serialize, Deserialize)]
        struct AppToken {
            username: String,
            password: String,
            realm_settings: UserRealmSettings,
            realm: RealmName,
            expiry: i64,
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

        trait Authorizer {
            //    type WebToken;
            fn get_auth_token(claim: &AppToken) -> Token;
        }
        struct AppAuthorizer {}

        impl Authorizer for AppAuthorizer {
            // type WebToken = String;
            fn get_auth_token(claim: &AppToken) -> Token {
                let header = Header::new(Algorithm::HS512);
                return encode(
                    &header,
                    &claim,
                    &EncodingKey::from_secret("secret".as_ref()),
                )
                    .unwrap();
            }
        }

        #[cfg(test)]
        mod tests {
            use crate::domain::infra::web::auth::{AppAuthorizer, AppToken, Authorizer};
            use crate::domain::realm::{Realm, RealmName, RealmSettings, UserRealmSettings};
            use actix_web::http::StatusCode;
            use chrono::{Days, Utc};
            use jsonwebtoken::EncodingKey;
            use std::time::Duration;

            #[test]
            fn test_auth_token() {
                let realm = RealmName::from("test");
                let realm_settings = UserRealmSettings {
                    is_confirmation_required: false,
                };

                let dt = Utc::now().checked_add_days(Days::new(90));

                let claim = AppToken {
                    username: "ruru".to_string(),
                    password: "passw0rd".to_string(),
                    realm_settings,
                    realm,
                    expiry: dt.unwrap().timestamp(),
                };

                let token = AppAuthorizer::get_auth_token(&claim);
                print!("Token - {}\n", token);
                assert_eq!(token.len(), 268);
            }
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
                status_code,
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

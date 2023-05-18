use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use serde::{Serialize, Deserialize, Serializer};
use actix_web::{HttpResponse, ResponseError};
use actix_web::body::{BoxBody, MessageBody};
use actix_web::http::header::{CONTENT_TYPE, ContentType, HeaderName, HeaderValue};
use actix_web::http::StatusCode;
use mysql::serde_json;
use strum_macros::Display;
use crate::domain::realm::Realm;

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
    status_code: StatusCode
}

impl <T> JsonErrorResponse<T> {
    pub fn new(body: Option<T>, message: String, status_code: StatusCode) -> JsonErrorResponse<T> {
        JsonErrorResponse {
            body,
            message,
            status_code
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

impl <T: Debug + Serialize> ResponseError for JsonErrorResponse<T> {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }
    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code)
            .insert_header(ContentType::json())
            .body(self.to_string())
    }
}


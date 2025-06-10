use actix_web::body::MessageBody;
use actix_web::dev::{Response, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::web::{Data, Redirect};
use actix_web::{HttpMessage, HttpResponse};
use actix_web::Error;
use actix_web::http::StatusCode;
use dashmap::ReadOnlyView;
use log::info;
use rand::distr::slice::Empty;
use strum_macros::Display;
use std::collections::HashMap;
use std::future::{ready, Future, Ready};
use std::hash::Hash;
use std::sync::Arc;
use jsonwebtoken::{decode, Validation, DecodingKey};
use std::pin::Pin;
use debug_ignore::DebugIgnore;


use crate::db::DB;
use crate::domain::customer::{Role, User};
use crate::domain::infra::web::auth::{AppAuthorizer, Authorizer};
use crate::domain::infra::web::{RealmFinder, TokenFinder};
use crate::domain::realm::{RealmSettings, RealmName};
use crate::repository::realm::RealmSettingProvider;
use crate::repository::{Repository, UserStorage};
use crate::app::{AppState, ROUTING_TABLE};
use crate::repository::realm::*;

type Username = String;

enum ResponseBody {
    Empty()
}

impl MessageBody for ResponseBody {
    type Error = String;

    fn size(&self) -> actix_web::body::BodySize {
       match self {
         ResponseBody::Empty() => actix_web::body::BodySize::None
       }
    }

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<actix_web::web::Bytes, Self::Error>>> {
        match self {
            Pin => std::task::Poll::Ready(Some(Ok(actix_web::web::Bytes::new())))
        }
    }
}

#[derive(Debug, Clone, Display)]
pub enum PathGuard {
    Missing,
    Open, // Any non authorised or logged in user
    Authorized, // Any logged in and authorized entity
    Guarded(Role) // Guarded for a specified role, i.e realm user or admin 
}

impl Default for PathGuard {
    fn default() -> Self {
        PathGuard::Authorized
    }
}

pub struct AuthMiddleware {
    pub secret: Box<dyn Fn(RealmName, String) -> String>,
}

pub struct AuthMiddlewareService<S> {
    service: S, 
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where 
S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
B: 'static,

{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, ()>>;
    
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service,
        }))
    }
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S> 
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{

    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;
    
    fn poll_ready(&self, ctx: &mut core::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }
    
    async fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.request().path();
        let headers = req.headers().clone(); 
        let realm = headers.get_realm();


        let path_state: &PathGuard = ROUTING_TABLE.get(path).unwrap_or(&PathGuard::Missing);
        info!("[AuthMiddleware] Path-{path} | State-{path_state}");

        Box::pin(async move {
            match path_state {
                PathGuard::Missing => unimplemented!(), // Fallback to login
                PathGuard::Open => {
                    if path == "api/realm/login" {
                        if realm.is_none() {
                            return self.service.call(req).await;
                        }

                        if let Some(auth_token) = headers.get_token() {
                            let is_valid = verify_token(&req, realm.clone(), auth_token)?;
                            if is_valid {
                                // let response: HttpResponse<ResponseBody> = HttpResponse::build(StatusCode::FOUND) // Explicitly build the response
                                // .header("Location", "api/customer/realm")
                                // .body(ResponseBody::Empty());

                                // Wrap the HttpResponse into a ServiceResponse<B>
                                return Ok(
                                        ServiceResponse::new(req.into_parts().0, 
                                HttpResponse::with_body(StatusCode::FOUND, "hello".to_owned())));
                            }
                        }
                    }
                    return self.service.call(req).await;
                }
                PathGuard::Authorized => {
                    if realm.is_none() {
                        return Err(actix_web::error::ErrorBadRequest("Missing Realm Header"));
                    }

                    if let Some(auth_token) = headers.get_token() {
                        let is_valid = verify_token(&req, realm.clone(), auth_token)?;
                        if is_valid {
                            return self.service.call(req).await;
                        }
                    }
                    Err(actix_web::error::ErrorUnauthorized("Unauthorized access, missing or invalid token"))
                }
                PathGuard::Guarded(_role) => {
                    // Handle guarded paths (e.g., role-based access control)
                    todo!()
                }
                _ => Err(actix_web::error::ErrorNotFound("Path not found")),
            }
        })
        
    }
}

fn verify_token(req: &ServiceRequest, maybe_realm: Option<String>, auth_token: String) -> Result<bool, Error> {
    use mysql::AccessMode;
    let data: Data<AppState> = match req.app_data::<Data<AppState>>() {
        Some(data) => data.clone(),
        None => {
            return Err(actix_web::error::InternalError::new(
                "DB Timeout".to_string(), 
                StatusCode::INTERNAL_SERVER_ERROR
            )
            .into())
        }
    };
    if let None = maybe_realm {
        return Err(actix_web::error::InternalError::new(
            "No Realm provided".to_string(), 
            StatusCode::BAD_REQUEST
        )
        .into())
    }
    let realm = maybe_realm.unwrap();
    let realm_settings = data.realm_settings_provider.clone();
    let secret = &realm_settings.get_realm_secret(&realm);
    let app_token = AppAuthorizer::decode_token(&auth_token, secret);
    let db: Arc<DB> = data.execution_context.db.clone();
    let user: User = db.in_transaction(AccessMode::ReadOnly, |tx| UserStorage::get_user_by_name(&app_token.username, &realm, tx)).unwrap().unwrap();
    let is_valid = auth_token == user.auth_token;
    Ok(is_valid)
}
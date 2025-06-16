use actix_web::body::BoxBody;
use actix_web::body::MessageBody;
use actix_web::dev::{
    forward_ready, Response, Service, ServiceRequest, ServiceResponse, Transform,
};
use actix_web::http::{header::HeaderMap, header::HeaderValue, header::LOCATION, StatusCode};
use actix_web::web::{Data, Redirect};
use actix_web::Error;
use actix_web::{HttpMessage, HttpResponse};
use dashmap::ReadOnlyView;
use debug_ignore::DebugIgnore;
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use jsonwebtoken::{decode, DecodingKey, Validation};
use log::info;
use rand::distr::slice::Empty;
use std::any::type_name;
use std::collections::HashMap;
use std::future::{ready, Future, Ready};
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use strum_macros::Display;

use crate::app::{AppState, ROUTING_TABLE};
use crate::db::DB;
use crate::domain::customer::{Role, User};
use crate::domain::infra::web::auth::{AppAuthorizer, Authorizer};
use crate::domain::infra::web::{RealmFinder, TokenFinder};
use crate::domain::realm::{RealmName, RealmSettings};
use crate::repository::realm::RealmSettingProvider;
use crate::repository::realm::*;
use crate::repository::{Repository, UserStorage};

type Username = String;

enum ResponseBody {
    Empty(),
}

impl MessageBody for ResponseBody {
    type Error = String;

    fn size(&self) -> actix_web::body::BodySize {
        match self {
            ResponseBody::Empty() => actix_web::body::BodySize::None,
        }
    }

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<actix_web::web::Bytes, Self::Error>>> {
        match self {
            Pin => std::task::Poll::Ready(Some(Ok(actix_web::web::Bytes::new()))),
        }
    }
}

#[derive(Debug, Clone, Display)]
pub enum PathGuard {
    Missing,
    Open,          // Any non authorised or logged in user
    Authorized,    // Any logged in and authorized entity
    Guarded(Role), // Guarded for a specified role, i.e realm user or admin
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
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, ()>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService { service }))
    }
}

impl<S> AuthMiddlewareService<S> {
    fn to_pinned_box<B>(
        &self,
        req: ServiceRequest,
    ) -> LocalBoxFuture<'static, Result<ServiceResponse<B>, Error>>
    where
        S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    {
        let fut = self.service.call(req);
        async move { fut.await }.boxed_local()
    }
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Log the concrete type of B
        info!("Concrete body type: {}", type_name::<B>());
        let path = req.request().path();
        let headers = req.headers().clone();
        let realm_ = headers.get_realm();

        let path_state: &PathGuard = ROUTING_TABLE.get(path).unwrap_or(&PathGuard::Missing);
        info!("[AuthMiddleware] Path-{path} | State-{path_state}");

        match path_state {
            PathGuard::Missing => unimplemented!(), // Fallback to login
            PathGuard::Open => {
                log::info!("authMiddleware] - Realm: {realm_:?} | Path: {path}");
                if path == "/api/realm/login" {
                    if realm_.is_none() {
                        info!("[authMiddleware] - No relam. Passthru");
                        return self.to_pinned_box(req);
                    }

                    if let Some(auth_token) = headers.get_token() {
                        match verify_token(&req, realm_.clone(), auth_token) {
                            Ok(is_valid) => {
                                if is_valid {
                                    let fut = self.service.call(req);

                                    return Box::pin(async move {
                                        let realm = realm_.clone().unwrap();
                                        let mut res = fut.await?;
                                        log::info!(
                                            "[authMiddleware-{realm}] - Authorized. Redirecting to realm"
                                        );
                                        Ok(res.map_body(|header, body| {
                                            let new_path = format!("/api/realm/{realm}");
                                            header.headers_mut().insert(
                                                LOCATION,
                                                HeaderValue::from_str(new_path.as_str()).unwrap(),
                                            );
                                            body
                                        }))
                                    });
                                } else {
                                    return self.to_pinned_box(req);
                                }
                            }
                            Err(err) => return self.to_pinned_box(req),
                        }
                    }
                }
                let realm = realm_.unwrap();
                info!("[authMiddleware-{realm}] - No token. Login required ");
                self.to_pinned_box(req)
            }
            PathGuard::Authorized => {
                if realm_.is_none() {
                    return Box::pin(async {
                        Err(actix_web::error::ErrorBadRequest("Missing Realm Header"))
                    });
                }

                if let Some(auth_token) = headers.get_token() {
                    let realm = realm_.clone().unwrap();
                    return match verify_token(&req, realm_.clone(), auth_token) {
                        Ok(is_valid) => {
                            if is_valid {
                                info!("[authMiddleware-{realm}] - Authorized.");
                                self.to_pinned_box(req)
                            } else {
                                Box::pin(async {
                                    Err(actix_web::error::ErrorUnauthorized(
                                        "Unauthorized access, missing or invalid token",
                                    ))
                                })
                            }
                        }
                        Err(err) => Box::pin(async { Err(err) }),
                    }
                }
                Box::pin(async {
                    Err(actix_web::error::ErrorUnauthorized(
                        "Unauthorized access, missing or invalid token",
                    ))
                })
            }
            // PathGuard::Guarded(_role) => {
            //     // Handle guarded paths (e.g., role-based access control)
            //     todo!()
            // }
            _ => Box::pin(async { Err(actix_web::error::ErrorNotFound("Path not found")) }),
        }
    }
}

fn verify_token(
    req: &ServiceRequest,
    maybe_realm: Option<String>,
    auth_token: String,
) -> Result<bool, Error> {
    use mysql::AccessMode;
    let data: Data<AppState> = match req.app_data::<Data<AppState>>() {
        Some(data) => data.clone(),
        None => {
            return Err(actix_web::error::InternalError::new(
                "DB Timeout".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
            .into())
        }
    };
    if let None = maybe_realm {
        return Err(actix_web::error::InternalError::new(
            "No Realm provided".to_string(),
            StatusCode::BAD_REQUEST,
        )
        .into());
    }
    let realm = maybe_realm.unwrap();
    let realm_settings = data.realm_settings_provider.clone();
    let secret = &realm_settings.get_realm_secret(&realm);
    let app_token = AppAuthorizer::decode_token(&auth_token, secret);
    let db: Arc<DB> = data.execution_context.db.clone();
    let user: User = db
        .in_transaction(AccessMode::ReadOnly, |tx| {
            UserStorage::get_user_by_name(&app_token.username, &realm, tx)
        })
        .unwrap()
        .unwrap();
    if let Some(token) = user.auth_token {
        return Ok(auth_token == token);
    } else {
        Ok(false)
    }
}

//   Box::pin(async move {
//     match path_state {
//         PathGuard::Missing => unimplemented!(), // Fallback to login
//         PathGuard::Open => {
//             if path == "api/realm/login" {
//                 if realm.is_none() {
//                     return self.service.call(req).await;
//                 }

//                 if let Some(auth_token) = headers.get_token() {
//                     let is_valid = verify_token(&req, realm.clone(), auth_token)?;
//                     if is_valid {
//                         let response = HttpResponse::build(StatusCode::FOUND)
//                             .body("hello")
//                             .map_into_boxed_body();
//                         // Convert the boxed body to the expected type B
//                         let res = ServiceResponse::new(
//                             req.request().clone(),
//                             response
//                         ).map_into_boxed_body();

//                         return Ok(res);
//                     }
//                 }
//             }
//             return self.service.call(req).await;
//         }
//         PathGuard::Authorized => {
//             if realm.is_none() {
//                 return Err(actix_web::error::ErrorBadRequest("Missing Realm Header"));
//             }

//             if let Some(auth_token) = headers.get_token() {
//                 let is_valid = verify_token(&req, realm.clone(), auth_token)?;
//                 if is_valid {
//                     let res = self.service.call(req).await?;
//                     Ok(res)
//                 }
//             }
//             Err(actix_web::error::ErrorUnauthorized("Unauthorized access, missing or invalid token"))
//         }
//         // PathGuard::Guarded(_role) => {
//         //     // Handle guarded paths (e.g., role-based access control)
//         //     todo!()
//         // }
//         _ => Err(actix_web::error::ErrorNotFound("Path not found")),
//     }
// })

pub mod customer {
    use crate::domain::infra;
    use crate::domain::customer::{dto::CreateUser, User};
    use crate::service::customer_service::{create as create_user, fetch_users, fetch_user};
    use crate::db::{ExecutionContext};
    use actix_web::{web, web::{Path}, HttpResponse, HttpRequest, Error, Responder, http::header, HttpResponseBuilder, ResponseError};
    use actix_web::dev::Service;
    use actix_web::error::BlockingError;
    use actix_web::http::header::ContentType;
    use actix_web::http::StatusCode;
    use actix_web::web::Json;
    use mysql::prelude::TextQuery;
    use serde::{Deserialize};
    use crate::AppState;
    use crate::domain::infra::JsonErrorResponse;

    #[derive(Deserialize)]
    pub struct UserId {
        user_id: String, // must match the path param name
    }

    pub async fn get(path_param: Path<UserId>, data: web::Data<AppState>) -> Result<HttpResponse, JsonErrorResponse<Option<String>>> {
        let db_res = web::block(move || {
            let db = &data.execution_context.db;
            fetch_user(&path_param.user_id, &db)
        }).await;

        match db_res {
            Ok(user_res) => {
                user_res.map(|user| Ok(HttpResponse::Ok().json(user.unwrap()))).unwrap_or_else(|e| Err(JsonErrorResponse::<Option<String>>::new(None, "User not found".to_string(), StatusCode::NOT_FOUND)))
            }
            Err(e) => Err(JsonErrorResponse::<Option<String>>::new(None, e.to_string(), StatusCode::BAD_REQUEST)),
        }
    }

    pub async fn get_all(req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse, JsonErrorResponse<Option<String>>> {
        let db_res = web::block(move || {
            // HeaderValue::
            let provider = &data.realm_settings_provider;
            let db = &data.execution_context.db;
            let dur = provider.get_password_reset_token_duration("rj.wire".to_string());
            println!("duration: {:?}", dur);
            fetch_users(&db)
        }).await;


        match db_res {
            Ok(user_res) => {
                user_res.map(|users| Ok(HttpResponse::Ok().json(users))).unwrap_or_else(|e| Err(JsonErrorResponse::<Option<String>>::new(None, "User not found".to_string(), StatusCode::NOT_FOUND)))
            }
            Err(e) => Err(JsonErrorResponse::<Option<String>>::new(None, e.to_string(), StatusCode::BAD_REQUEST)),
        }
    }


    pub async fn update() -> impl Responder {
        HttpResponse::Ok().body("Hello world!")
    }

    pub async fn create(req_body: web::Json<CreateUser>, req: HttpRequest, data: web::Data<AppState>) -> Result<HttpResponse, impl ResponseError> {
        let future = web::block(move || {
            let db = &data.execution_context.db;

            let user_data = req_body.into_inner();
            create_user(user_data, db)
        }).await;

        match future {
            Ok(res) => {
                res.map(|user_id| {
                    // TODO ::

                    HttpResponse::Ok().body(user_id)
                })
                    .map_err(|e| JsonErrorResponse::new(None, e.to_string(), StatusCode::BAD_REQUEST))
            }
            //TODO:: Message should be logged
            Err(err) => { Err(JsonErrorResponse::<Option<String>>::new(None, "Failed to process information".to_string(), StatusCode::INTERNAL_SERVER_ERROR)) }
        }
    }

    pub async fn manual_hello() -> impl Responder {
        HttpResponse::Ok().body("Hey there!")
    }
}

pub mod admin {}


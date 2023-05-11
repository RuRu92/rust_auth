pub mod customer {
    use crate::domain::customer::{dto::CreateUser};
    use crate::service::customer_service::{create as create_user, fetch_users, fetch_user};
    use crate::db::{ExecutionContext};
    use actix_web::{web, web::{Path}, HttpResponse, HttpRequest, Error, Responder, http::header};
    use serde::{Deserialize};
    use crate::AppState;

    #[derive(Deserialize)]
    pub struct UserId {
        user_id: String, // must match the path param name
    }

    pub async fn get(path_param: Path<UserId>, data: web::Data<AppState>) -> std::result::Result<HttpResponse, Error> {
        Ok(
            web::block(move || {
                let db = &data.execution_context.db;
                fetch_user(&path_param.user_id, &db)
            })
            .await
            .map(|users| HttpResponse::Ok().json(users))
            .map_err(|_| HttpResponse::InternalServerError())?)
    }

    pub async fn get_all(req: HttpRequest, data: web::Data<AppState>) -> std::result::Result<HttpResponse, Error> {
        Ok(
            web::block(move || {
                // HeaderValue::
                let provider = &data.realm_settings_provider;
                let db = &data.execution_context.db;
                let dur = provider.get_password_reset_token_duration("rj.wire".to_string());
                println!("duration: {:?}", dur);
                fetch_users(&db)
            })
                .await
                .map(|users| {
                    HttpResponse::Ok().json(users)
                })
                .map_err(|_| HttpResponse::InternalServerError())?)
    }


    pub async fn update() -> impl Responder {
        HttpResponse::Ok().body("Hello world!")
    }

    pub async fn create(req_body: web::Json<CreateUser>, data: web::Data<AppState>) -> std::result::Result<HttpResponse, Error> {
        Ok(
            web::block(move || {
                let db = &data.execution_context.db;
                let user_data = req_body.into_inner();
                create_user(user_data, db)
            })
                .await
                .map(|user_id| HttpResponse::Ok().json(user_id))
                .map_err(|_| HttpResponse::InternalServerError())?)

    }

    pub async fn manual_hello() -> impl Responder {
        HttpResponse::Ok().body("Hey there!")
    }
}

pub mod admin {}


pub mod customer {
    use crate::domain::customer::{dto::CreateUser, LoginRequest, LoginRequestArguments};
    use crate::domain::infra::web::auth::verify_login;
    use crate::domain::infra::web::{JsonErrorResponse, RealmFinder};
    use crate::service::customer_service::CustomerService;
    use crate::AppState;
    use actix_web::body::MessageBody;
    use actix_web::dev::Service;
    use actix_web::http::StatusCode;
    use actix_web::web::{Data};
    use actix_web::{web, web::Path, HttpRequest, HttpResponse,
        Responder,
    };
    use mysql::prelude::TextQuery;
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct UserId {
        user_id: String, // must match the path param name
    }

    pub async fn login(
        path_param: Path<UserId>,
        json: web::Json<LoginRequest>,
        req: HttpRequest,
    ) -> Result<HttpResponse, JsonErrorResponse<Option<String>>> {
        let header = req.headers().get_realm();
        let login_request = json.0;

        if let Some(realm) = header {
            let data = req.app_data::<Data<AppState>>().unwrap().clone();
            let t_realm = realm.clone();
            let username = login_request.username.clone();
            let db = data.execution_context.db.clone();
            let rsp = data.realm_settings_provider.clone();
            let db_res =
                web::block(move || CustomerService::fetch_user_by_name(&username, &t_realm, &db))
                    .await;

            match db_res {
                Ok(user_res) => user_res
                    .map(|user| {
                        let itr = &rsp.get_realm_salt_itr(realm.as_str());
                        let login_arg = LoginRequestArguments {
                            login_request,
                            user: user.clone(),
                        };
                        let is_ok = verify_login(&login_arg, realm, *itr);

                        if is_ok {
                            Ok(HttpResponse::Ok().json(&user))
                        } else {
                            Err(JsonErrorResponse::<Option<String>>::new(
                                None,
                                "Bad Auth".to_string(),
                                StatusCode::BAD_REQUEST,
                            ))
                        }
                    })
                    .unwrap_or_else(|| {
                        Err(JsonErrorResponse::<Option<String>>::new(
                            None,
                            "User not found".to_string(),
                            StatusCode::NOT_FOUND,
                        ))
                    }),
                Err(e) => Err(JsonErrorResponse::<Option<String>>::new(
                    None,
                    e.to_string(),
                    StatusCode::BAD_REQUEST,
                )),
            }
        } else {
            Err(JsonErrorResponse::<Option<String>>::new(
                None,
                String::from("Must contain realm header"),
                StatusCode::BAD_REQUEST,
            ))
        }
    }

    pub async fn get(
        path_param: Path<UserId>,
        data: web::Data<AppState>,
    ) -> Result<HttpResponse, JsonErrorResponse<Option<String>>> {
        let db_res = web::block(move || {
            let db = &data.execution_context.db;
            CustomerService::fetch_user(&path_param.user_id, &db)
        })
        .await;

        match db_res {
            Ok(user_res) => user_res
                .map(|user| Ok(HttpResponse::Ok().json(user.unwrap())))
                .unwrap_or_else(|e| {
                    Err(JsonErrorResponse::<Option<String>>::new(
                        None,
                        "User not found".to_string(),
                        StatusCode::NOT_FOUND,
                    ))
                }),
            Err(e) => Err(JsonErrorResponse::<Option<String>>::new(
                None,
                e.to_string(),
                StatusCode::BAD_REQUEST,
            )),
        }
    }

    pub async fn get_all(
        req: HttpRequest,
        data: web::Data<AppState>,
    ) -> Result<HttpResponse, JsonErrorResponse<Option<String>>> {
        let db_res = web::block(move || {
            // HeaderValue::
            let provider = &data.realm_settings_provider;
            let db = &data.execution_context.db;
            let dur = provider.get_password_reset_token_duration("rj.wire");
            println!("duration: {:?}", dur);
            CustomerService::fetch_users(&db)
        })
        .await;

        match db_res {
            Ok(user_res) => user_res
                .map(|users| Ok(HttpResponse::Ok().json(users)))
                .unwrap_or_else(|e| {
                    Err(JsonErrorResponse::<Option<String>>::new(
                        None,
                        "User not found".to_string(),
                        StatusCode::NOT_FOUND,
                    ))
                }),
            Err(e) => Err(JsonErrorResponse::<Option<String>>::new(
                None,
                e.to_string(),
                StatusCode::BAD_REQUEST,
            )),
        }
    }

    pub async fn update() -> impl Responder {
        HttpResponse::Ok().body("Hello world!")
    }

    pub async fn create(
        req_body: web::Json<CreateUser>,
        req: HttpRequest,
        data: web::Data<AppState>,
    ) -> Result<HttpResponse, JsonErrorResponse<Option<String>>> {
        let realm_header = req.headers().get_realm();
        if let None = realm_header {
            Err(JsonErrorResponse::<Option<String>>::new(
                None,
                "Failed to process information".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        } else {
            let future = web::block(move || {
                let user_data = req_body.into_inner();
                CustomerService::create(user_data, realm_header.unwrap(), data.get_ref())
            })
            .await;

            match future {
                Ok(result) => result
                    .map(|user_id| HttpResponse::Ok().body(user_id))
                    .map_err(|e| {
                        JsonErrorResponse::new(None, e.to_string(), StatusCode::BAD_REQUEST)
                    }),
                //TODO:: Message should be logged
                Err(err) => Err(JsonErrorResponse::<Option<String>>::new(
                    None,
                    "Failed to process information".to_string(),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )),
            }
        }
    }

    pub async fn manual_hello() -> impl Responder {
        HttpResponse::Ok().body("Hey there!")
    }
}

pub mod admin {}

//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use actix_web::{http, test, web::Data, App};
//     use crate::AppState;
//
//
//     // Imports depending on the structure of your project
//
//     #[actix_web::test]
//     pub async fn test_login() {
//         n
//         let mut app = test::init_service(
//             App::new()
//                 .data(Data::new(AppState {
//                     realm_settings_provider:
//                 }))
//                 .route("/{id}", web::post().to(login))
//         ).await;
//
//         let req = test::TestRequest::post()
//             .uri("/test_user_id")
//             .header(http::header::CONTENT_TYPE, "application/json")
//             .header("Realm", "test_realm")
//             .set_json(&LoginRequest {
//                 // Fill with example data
//             })
//             .to_request();
//
//         let resp: Result<HttpResponse, JsonErrorResponse<Option<String>>> = test::call_service(&mut app, req).await;

// Add assertions depending on what you expect the function to return
// for the example test data
// }
// }

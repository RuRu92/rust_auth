pub mod customer {
    use crate::domain::customer::{dto::CreateUser, LoginRequest, LoginRequestArguments, User};
    use crate::domain::infra::web::auth::verify_login;
    use crate::domain::infra::web::{JsonErrorResponse, LoginError, RealmFinder};
    use crate::service::customer_service::{AuthenticatorService, CustomerService};
    use crate::AppState;

    use crate::domain::realm::RealmName;
    use crate::repository::realm::RealmSettingProvider;
    use actix_web::http::StatusCode;
    use actix_web::web::Data;
    use actix_web::{web, web::Path, HttpRequest, HttpResponse, Responder};
    use serde::Deserialize;

    #[derive(Deserialize)]
    pub struct UserId {
        user_id: String, // must match the path param name
    }

    struct LoginUserData<'a> {
        user: User,
        realm: RealmName,
        realm_settings_provider: &'a RealmSettingProvider,
    }

    type LoginErrorResponse = JsonErrorResponse<Option<String>>;

    pub async fn login(
        path_param: Path<UserId>,
        json: web::Json<LoginRequest>,
        req: HttpRequest,
    ) -> Result<HttpResponse, LoginErrorResponse> {
        let realm = req
            .headers()
            .get_realm()
            .ok_or(LoginError::MissingRealmHeader)?;
        let login_request = json.0;

        let data = match req.app_data::<Data<AppState>>() {
            Some(data) => data.clone(),
            None => {
                return Err(LoginErrorResponse::new(
                    None,
                    "Failed to find realm".to_string(),
                    StatusCode::NOT_FOUND,
                ))
            }
        };

        let login_user_data = fetch_user_data(&data, &realm, &login_request).await?;

        authenticate_user(login_user_data, login_request).await
    }

    async fn authenticate_user<'a>(
        login_user_data: LoginUserData<'a>,
        login_request: LoginRequest,
    ) -> Result<HttpResponse, LoginErrorResponse> {
        let login_arg = LoginRequestArguments {
            login_request,
            user: login_user_data.user,
        };
        let itr = login_user_data
            .realm_settings_provider
            .get_realm_salt_itr(login_user_data.realm.as_str());
        let is_ok = verify_login(&login_arg, login_user_data.realm, itr);

        if is_ok {
            let token = AuthenticatorService::initialise_token(&login_arg.user);
            Ok(HttpResponse::Ok().json(token))
        } else {
            Err(LoginErrorResponse::new(
                None,
                "Authentication failed".to_string(),
                StatusCode::UNAUTHORIZED,
            ))
        }
    }

    async fn fetch_user_data<'a>(
        data: &'a Data<AppState>,
        realm: &RealmName,
        payload: &LoginRequest,
    ) -> Result<LoginUserData<'a>, LoginError> {
        let db = data.execution_context.db.clone();
        let username = payload.username.clone();
        let rlm = realm.clone();
        let result = web::block(move || CustomerService::fetch_user_by_name(&username, &rlm, &db))
            .await
            .map_err(|e| LoginError::DatabaseError(e.to_string()))?;

        match result {
            Ok(None) => Err(LoginError::UserNotFound),
            Ok(Some(u)) => Ok(LoginUserData {
                user: u,
                realm: realm.clone(),
                realm_settings_provider: data.realm_settings_provider.as_ref(),
            }),
            Err(err) => Err(LoginError::DatabaseError(err.to_string()))
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
                .unwrap_or_else(|_| {
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
                .unwrap_or_else(|_| {
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
                Err(_err) => Err(JsonErrorResponse::<Option<String>>::new(
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use actix_web::{http, test, web::Data, App};

    // Imports depending on the structure of your project

    // #[actix_web::test]
    // pub async fn test_login() {
    //     let mut app = test::init_service(
    //         App::new()
    //             .data(Data::new(AppState {
    //                 realm_settings_provider:
    //             }))
    //             .route("/{id}", web::post().to(login))
    //     ).await;

    //     let req = test::TestRequest::post()
    //         .uri("/test_user_id")
    //         .header(http::header::CONTENT_TYPE, "application/json")
    //         .header("Realm", "test_realm")
    //         .set_json(&LoginRequest {
    //             // Fill with example data
    //         })
    //         .to_request();

    //     let resp: Result<HttpResponse, JsonErrorResponse<Option<String>>> = test::call_service(&mut app, req).await;
    // }
}

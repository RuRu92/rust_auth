use actix_web::{test, web, App, HttpResponse};
use authenticator::app::AppState;
use authenticator::domain::customer::dto::CreateUser;
use authenticator::domain::customer::Address;
use authenticator::domain::realm::RealmName;
use authenticator::repository::realm::RealmSettingProvider;
use authenticator::service::customer_service::CustomerService;
use serde_json::json;
use std::sync::Arc;

#[test]
#[actic_web::test]
async fn test_signup_and_login() {
    let db = Arc::new(rust_auth::db::DB::init(
        "mysql://root:password@localhost:3306/auth",
    )); // Or use a test DB

    let real_settings_provider = Arc::new(RealmSettingsProvider::init(db.clone()));
    let app_sate = AppState {
        realm_settings_provider,
        execution_context: rust_auth::db::ExecutionContext { db },
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(app_sate))
            .route("/signup", customer::create),
    )
    .await;

    // Prepare signup payload
    let payload = CreateUser {
        realm: "rj.wire".to_string(),
        username: "testuser".to_string(),
        password: "testpass".to_string(),
        name: "Test User".to_string(),
        age: 30,
        email: "testuser@example.com".to_string(),
        address: Address {
            street: "123 Main St".to_string(),
            country: "UK".to_string(),
            city: "London".to_string(),
            post_code: "W1 2DE".to_string(),
        },
    };

    let req = test::TestRequest::post()
        .uri("/signup")
        .set_json(&payload)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

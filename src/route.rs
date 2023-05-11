use actix_web::{web, HttpResponse, Resource, Scope};
use crate::resource::customer as customer;
use actix_web::web::{scope};

pub fn routes(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("")
            .service(web::resource("/").route(web::get().to(customer::manual_hello)))
            .service(scope("/api").configure(user_api_config))
    );
}

fn user_api_config(cfg: &mut web::ServiceConfig) {
    cfg.service(customer_resource())
        .service(admin_resource())
        .service(realm_resource());
}


fn customer_resource() -> Scope {
    return web::scope("/customer")
        .service(web::resource("/{user_id}")
            .route(web::get().to(customer::get))
            .route(web::put().to(customer::update)))
        .service(web::resource("")
            .route(web::post().to(customer::create))
            .route(web::get().to(customer::get_all)));
}

fn realm_resource() -> Scope {
    return web::scope("/realm")
        .service(web::resource("/{name}")
            .route(web::get().to(customer::get))
            .route(web::put().to(customer::update)))
        .service(web::resource("")
            .route(web::post().to(customer::create))
            .route(web::get().to(customer::get_all)));
}

fn admin_resource() -> Resource {
    return web::resource("/admin")
        .route(web::get().to(|| HttpResponse::Ok().body("test")))
        .route(web::head().to(|| HttpResponse::MethodNotAllowed()));
}



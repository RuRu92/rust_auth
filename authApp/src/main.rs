#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(warnings)]

use domain::realm::Realm;
use env_logger::{self, Env};
use actix_web::{middleware as actix_mw, rt as actix_rt, web, App, HttpServer};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map, HashMap};
use std::sync::Arc;

mod db;
mod domain;
mod repository;
mod resource;
mod route;
mod service;
mod app;
pub mod middleware;

const URL: &str = "mysql://root:password@localhost:3306/auth";

use crate::db::ExecutionContext;
use crate::repository::realm::RealmSettingProvider;
use route::routes;

#[derive(Deserialize, Serialize, Debug)]
pub struct Principal {
    id: String,
    role: String,
    name: String,
}

pub struct AppState {
    realm_settings_provider: Arc<RealmSettingProvider>,
    execution_context: ExecutionContext,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let db = Arc::new(db::DB::init(URL));
    let realm_settings_provider =
        Arc::new(repository::realm::RealmSettingProvider::init(db.clone()));

    let provider = realm_settings_provider.clone();

    actix_rt::spawn(refresh_realm_settings(provider));

    let app_data = web::Data::new(AppState {
        realm_settings_provider,
        execution_context: ExecutionContext { db },
    });

    HttpServer::new(move || {
        println!("server started");
        // realm_updater.join();
        App::new()
            .app_data(app_data.clone())
            .wrap(actix_mw::Logger::default())
            .wrap(actix_mw::Logger::new("%a - %r - %P %{User-Agent}i"))
            .wrap(middleware::auth::AuthMiddleware {
                 secret: |realm, username| {
                    format!("{realm}|{username}")
                 },
                 routing_table: initRoutingTable()
            })
            .configure(routes)
    })
    .bind("127.0.0.1:9090")?
    .run()
    .await
}

async fn refresh_realm_settings(arc: Arc<RealmSettingProvider>) {
    let mut interval = actix_rt::time::interval(std::time::Duration::from_secs(15));
    loop {
        interval.tick().await;
        let provider = arc.clone();
        actix_rt::task::spawn_blocking(move || {
            &provider.reload();
        });
    }
}

fn initRoutingTable() -> HashMap<String, middleware::auth::PathGuard> {
    
    use middleware::auth::PathGuard;
    use crate::domain::customer::Role;
    
    hashmap! {
        // Open paths (accessible to anyone)
        "/api" => PathGuard::Open,
        "/api/customer" => PathGuard::Open,
        "/api/customer/{user_id}" => PathGuard::Open,
        "/api/realm/login" => PathGuard::Open,

        // Authorized paths (requires a valid token)
        "/api/realm" => PathGuard::Authorized,
        "/api/realm/{realm}" => PathGuard::Authorized,

        // Guarded paths (requires specific roles)
        "/api/admin" => PathGuard::Guarded(Role::Admin),
    }
}
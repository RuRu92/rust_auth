#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(warnings)]

mod app;
mod db;
mod domain;
mod repository;
mod resource;
mod route;
mod service;

pub mod adapter;


use actix_web::{middleware as actix_mw, rt as actix_rt, web, App, HttpServer};
use domain::realm::Realm;
use domain::customer::Role;
use env_logger::{self, Env};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map, HashMap};
use std::pin::Pin;
use std::sync::Arc;
    
use crate::adapter::auth::PathGuard;



const URL: &str = "mysql://root:password@localhost:3306/auth";

use crate::db::ExecutionContext;
use crate::repository::realm::RealmSettingProvider;
use route::routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info");

    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let db = Arc::new(db::DB::init(URL));
    let realm_settings_provider =
        Arc::new(repository::realm::RealmSettingProvider::init(db.clone()));

    let provider = realm_settings_provider.clone();

    actix_rt::spawn(refresh_realm_settings(provider));

    let app_data = web::Data::new(app::AppState {
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
            .wrap(adapter::auth::AuthMiddleware {
                secret: Box::new(|realm, username| format!("{realm}|{username}")),
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


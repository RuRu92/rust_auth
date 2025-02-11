#![allow(unused_imports)]
#![allow(dead_code)]

use actix_web::{rt as actix_rt, web, App, HttpServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod db;
mod domain;
mod repository;
mod resource;
mod route;
mod service;
mod app;

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
        App::new().app_data(app_data.clone()).configure(routes)
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

use actix_web::{web, App, HttpServer, rt as actix_rt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod domain;
mod resource;
mod route;
mod repository;
mod service;
mod db;

const URL: &str = "mysql://ruru:77lc3lm00@localhost:3306/id";

use route::{routes};
use crate::db::ExecutionContext;
use crate::repository::realm::RealmSettingProvider;

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
    let realm_settings_provider = Arc::new(repository::realm::RealmSettingProvider::init(db.clone()));

    let provider = realm_settings_provider.clone();

    actix_rt::spawn(async {
        let interval = actix_rt::time::interval(std::time::Duration::from_secs(15));
        loop {
            interval.tick().await;
            actix_rt::task::spawn_blocking(move || {
                &provider.reload();
            }).await;
        }
    });

    let app_data = web::Data::new(AppState {
        realm_settings_provider,
        execution_context: ExecutionContext {
            db
        },
    });

    HttpServer::new(move || {
        println!("server started");
        // realm_updater.join();
        App::new()
            .app_data(app_data.clone())
            .configure(routes)
    })
        .bind("127.0.0.1:9090")?
        .run()
        .await
}
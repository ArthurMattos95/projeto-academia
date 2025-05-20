use actix_web::{web, App, HttpServer, middleware};
use actix_files::Files;
use dotenv::dotenv;
use tera::Tera;
use sqlx::PgPool;
use std::env;

mod db;
mod handlers;
mod models;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let db_pool = db::init_db().await;

    let tera = Tera::new("templates/**/*").expect("Erro ao carregar templates");

    HttpServer::new(move || {
        App::new()
            .service(Files::new("/static", "./static").show_files_listing())
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tera.clone()))
            .wrap(middleware::Logger::default())
            .configure(handlers::init_routes)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

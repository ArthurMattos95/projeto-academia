use actix_web::{web, App, HttpServer, middleware};
use actix_files::Files;
use dotenv::dotenv;
use tera::Tera;

mod db;
mod handlers;
mod models;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // Inicializa a conexão com o banco de dados
    let db_pool = db::init_db().await;

    // Carrega os templates Tera
    let tera = Tera::new("templates/**/*").expect("Erro ao carregar templates");

    // Inicializa o servidor HTTP
    HttpServer::new(move || {
        App::new()
            // Servir arquivos estáticos da pasta ./static em /static
            .service(Files::new("/static", "./static").show_files_listing())
            // Compartilha o pool do banco e templates com os handlers
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(tera.clone()))
            // Logger de requisições
            .wrap(middleware::Logger::default())
            // Registra as rotas definidas no módulo handlers
            .configure(handlers::init_routes)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

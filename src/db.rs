use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

pub async fn init_db() -> PgPool {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL não definida no .env");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Falha ao conectar ao banco de dados")
}

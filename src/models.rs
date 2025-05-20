use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, Deserialize, FromRow)]
pub struct Usuario {
    pub id: i32,
    pub nome: String,
    pub senha_hash: String,
    pub email: String,
}
#[derive(Serialize, Deserialize, FromRow)]
pub struct Exercicio {
    pub id: i32,
    pub nome: String,
    pub grupo_muscular: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gif_url: Option<String>,
}
#[derive(Serialize, Deserialize, FromRow)]
pub struct ExercicioDetalhado {
    pub id: i32,
    pub nome: String,
    pub gif_url: Option<String>,
    pub series: i32,
    pub repeticoes: i32,
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct TreinoDetalhado {
    pub id: i32,
    pub nome: String,
    pub exercicios: Vec<ExercicioDetalhado>,
}
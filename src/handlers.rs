use actix_web::{web, HttpResponse, Responder, http::header, Error};
use sqlx::PgPool;
use crate::models::{Usuario, Exercicio, ExercicioDetalhado, TreinoDetalhado};
use tera::{Tera, Context};
use serde::{Deserialize};

use std::collections::HashMap;
use actix_files as fs;

pub async fn lista_alunos(
    tmpl: web::Data<Tera>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let usuarios = sqlx::query_as::<_, Usuario>(
        "SELECT id, nome, senha_hash, email FROM usuario"
    )
    .fetch_all(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar usuários: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar usuários")
    })?;

    let mut context = Context::new();
    context.insert("titulo", "Usuários da Academia");
    context.insert("usuarios", &usuarios);

    let rendered = tmpl.render("tela_inicial.html", &context).map_err(|e| {
        eprintln!("Erro ao renderizar template: {}", e);
        actix_web::error::ErrorInternalServerError("Erro ao renderizar")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

pub async fn montar_treino(
    tmpl: web::Data<Tera>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let exercicios = sqlx::query_as::<_, Exercicio>(
        "SELECT id, nome, grupo_muscular, gif_url FROM exercicio"
    )
    .fetch_all(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar exercícios: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar exercícios")
    })?;

    let mut context = Context::new();
    context.insert("exercicios", &exercicios);

    let rendered = tmpl.render("montar_treino.html", &context).map_err(|e| {
        eprintln!("Erro ao renderizar template: {}", e);
        actix_web::error::ErrorInternalServerError("Erro ao renderizar")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[derive(Deserialize)]
pub struct TreinoForm {
    pub nome_treino: String,
    pub exercicios_json: String,
}

#[derive(Deserialize)]
pub struct ExercicioInput {
    pub id: i32,
    pub series: i32,
    pub repeticoes: i32,
}

pub async fn salvar_treino(
    form: web::Form<TreinoForm>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    // Extrai o JSON dos exercícios do campo do formulário
    let exercicios_json = &form.exercicios_json;

    // Desserializa o JSON para a struct Vec<ExercicioInput>
    let exercicios: Vec<ExercicioInput> = serde_json::from_str(exercicios_json)
        .map_err(|e| {
            eprintln!("Erro ao desserializar JSON: {:?}", e);
            actix_web::error::ErrorBadRequest("JSON inválido")
        })?;

    // Usuário genérico (exemplo)
    let usuario_id = 1;

    // Insere o treino e obtém o ID
    let treino = sqlx::query!(
        "INSERT INTO treino (nome, usuario_id) VALUES ($1, $2) RETURNING id",
        form.nome_treino,
        usuario_id
    )
    .fetch_one(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao salvar treino: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao salvar treino")
    })?;

    // Insere os exercícios relacionados
    for ex in exercicios {
        sqlx::query!(
            "INSERT INTO treino_exercicio (treino_id, exercicio_id, series, repeticoes) VALUES ($1, $2, $3, $4)",
            treino.id,
            ex.id,
            ex.series,
            ex.repeticoes
        )
        .execute(db_pool.get_ref())
        .await
        .map_err(|e| {
            eprintln!("Erro ao salvar exercício: {:?}", e);
            actix_web::error::ErrorInternalServerError("Erro ao salvar exercício")
        })?;
    }

    // Redireciona para a tela inicial após salvar
    Ok(HttpResponse::Found()
        .append_header(("Location", "/tela_inicial"))
        .finish())
}

pub async fn escolher_treinos_prontos(
    tmpl: web::Data<Tera>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let exercicios = sqlx::query_as::<_, Exercicio>(
        "SELECT id, nome, grupo_muscular, gif_url FROM exercicio ORDER BY grupo_muscular, nome"
    )
    .fetch_all(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar exercícios: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar exercícios")
    })?;

    use std::collections::HashMap;
    let mut grupos_map: HashMap<String, Vec<&Exercicio>> = HashMap::new();

    for ex in &exercicios {
        grupos_map.entry(ex.grupo_muscular.clone())
            .or_default()
            .push(ex);
    }

    let grupos_ordenados = vec!["superior", "inferior", "costas"];

    let mut context = Context::new();
    context.insert("grupos", &grupos_ordenados);
    context.insert("exercicios_por_grupo", &grupos_map);

    let rendered = tmpl.render("treinos_prontos.html", &context).map_err(|e| {
        eprintln!("Erro ao renderizar template: {}", e);
        actix_web::error::ErrorInternalServerError("Erro ao renderizar")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

pub async fn visualizar_treinos(
    tmpl: web::Data<Tera>,
    db_pool: web::Data<PgPool>,
    // Aqui você pode receber o usuário logado, por enquanto fixo:
) -> Result<HttpResponse, Error> {
    let usuario_id = 1; // Ajuste para pegar o usuário logado

    // Buscar os treinos do usuário
    let treinos = sqlx::query!(
        "SELECT id, nome FROM treino WHERE usuario_id = $1",
        usuario_id
    )
    .fetch_all(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar treinos: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar treinos")
    })?;

    let mut treinos_detalhados = Vec::new();

    for treino in treinos {
        // Buscar exercícios relacionados a cada treino
        let exercicios = sqlx::query_as!(
            ExercicioDetalhado,
            r#"
            SELECT e.id, e.nome, e.gif_url, te.series, te.repeticoes
            FROM treino_exercicio te
            JOIN exercicio e ON e.id = te.exercicio_id
            WHERE te.treino_id = $1
            "#,
            treino.id
        )
        .fetch_all(db_pool.get_ref())
        .await
        .map_err(|e| {
            eprintln!("Erro ao buscar exercícios do treino: {:?}", e);
            actix_web::error::ErrorInternalServerError("Erro ao buscar exercícios")
        })?;

        treinos_detalhados.push(TreinoDetalhado {
            id: treino.id,
            nome: treino.nome,
            exercicios,
        });
    }

    let mut context = Context::new();
    context.insert("treinos", &treinos_detalhados);

    let rendered = tmpl.render("visualizar_treinos.html", &context).map_err(|e| {
        eprintln!("Erro ao renderizar template: {}", e);
        actix_web::error::ErrorInternalServerError("Erro ao renderizar")
    })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(fs::Files::new("/static", "./static").show_files_listing());
    cfg.route("/tela_inicial", web::get().to(lista_alunos));
    cfg.route("/montar_treino", web::get().to(montar_treino));
    cfg.route("/escolher_treinos_prontos", web::get().to(escolher_treinos_prontos));
    cfg.route("/salvar_treino", web::post().to(salvar_treino));
    cfg.route("/meus_treinos", web::get().to(visualizar_treinos));
}

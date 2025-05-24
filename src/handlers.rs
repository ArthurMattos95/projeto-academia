use actix_web::{post, web, HttpResponse, Error};
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

pub async fn visualizar_treinos(
    tmpl: web::Data<Tera>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let usuario_id = 1;

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

pub async fn editar_treinos_salvos(
    tmpl: web::Data<Tera>,
    db_pool: web::Data<PgPool>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    // 1. Pega o id do treino da query string
    let treino_id: i32 = match query.get("id").and_then(|s| s.parse().ok()) {
        Some(id) => id,
        None => return Ok(HttpResponse::BadRequest().body("ID do treino ausente ou inválido")),
    };

    let usuario_id = 1;

    // 2. Busca o treino no banco
    let treino_row = sqlx::query!(
        "SELECT id, nome FROM treino WHERE id = $1 AND usuario_id=$2",
        treino_id,usuario_id
    )
    .fetch_optional(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar treino: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar treino")
    })?;

    let treino_row = match treino_row {
        Some(row) => row,
        None => return Ok(HttpResponse::NotFound().body("Treino não encontrado")),
    };

    // 3. Busca os exercícios detalhados desse treino
    let exercicios = sqlx::query_as!(
        ExercicioDetalhado,
        r#"
        SELECT 
            e.id, 
            e.nome, 
            e.gif_url,
            te.series,
            te.repeticoes
        FROM exercicio e
        INNER JOIN treino_exercicio te ON te.exercicio_id = e.id
        WHERE te.treino_id = $1
        ORDER BY e.nome
        "#,
        treino_id
    )
    .fetch_all(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar exercícios do treino: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar exercícios")
    })?;

    // 4. Monta o TreinoDetalhado
    let treino = TreinoDetalhado {
        id: treino_row.id,
        nome: treino_row.nome,
        exercicios,
    };

    // 5. Passa para o template
    let mut context = tera::Context::new();
context.insert("treinos", &vec![treino]);

    let rendered = tmpl.render("editar_treinos_salvos.html", &context)
        .map_err(|e| {
            eprintln!("Erro ao renderizar template: {}", e);
            actix_web::error::ErrorInternalServerError("Erro ao renderizar página")
        })?;

    Ok(HttpResponse::Ok().content_type("text/html").body(rendered))
}

#[derive(Deserialize)]
struct DeletarTreinoForm {
    treino_id: i32,
}

#[post("/deletar_treino")]
async fn deletar_treino(
    db_pool: web::Data<PgPool>,
    form: web::Form<DeletarTreinoForm>,
) -> Result<HttpResponse, Error> {
    let usuario_id = 1;

    // Verifica se o treino pertence ao usuário
    let treino = sqlx::query!(
        "SELECT id FROM treino WHERE id = $1 AND usuario_id = $2",
        form.treino_id,
        usuario_id
    )
    .fetch_optional(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar treino: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar treino")
    })?;

    if treino.is_none() {
        return Ok(HttpResponse::NotFound().body("Treino não encontrado ou não pertence ao usuário"));
    }

    // Deleta o treino
    sqlx::query!(
        "DELETE FROM treino WHERE id = $1 AND usuario_id = $2",
        form.treino_id,
        usuario_id
    )
    .execute(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao deletar treino: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao deletar treino")
    })?;

    // Retorna redirecionamento para a página principal ou lista de treinos
    Ok(HttpResponse::SeeOther()
        .header("Location", "/meus_treinos")
        .finish())
}

#[derive(Deserialize)]
pub struct SalvarEdicaoForm {
    treino_id: i32,
    series: HashMap<i32, i32>,
    repeticoes: HashMap<i32, i32>,
}

#[post("/salvar_edicao_treinos")]
pub async fn salvar_edicao_treinos(
    db_pool: web::Data<PgPool>,
    form: web::Json<SalvarEdicaoForm>,
) -> Result<HttpResponse, Error> {
    let usuario_id = 1; // Ajuste conforme sua autenticação

    // Verifica se o treino pertence ao usuário
    let treino = sqlx::query!(
        "SELECT id FROM treino WHERE id = $1 AND usuario_id = $2",
        form.treino_id,
        usuario_id
    )
    .fetch_optional(db_pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Erro ao buscar treino: {:?}", e);
        actix_web::error::ErrorInternalServerError("Erro ao buscar treino")
    })?;

    if treino.is_none() {
        return Ok(HttpResponse::NotFound().body("Treino não encontrado ou não pertence ao usuário"));
    }

    // Atualiza séries e repetições para cada exercício
    for (exercicio_id, series) in &form.series {
        if let Some(repeticoes) = form.repeticoes.get(exercicio_id) {
            sqlx::query!(
                "UPDATE treino_exercicio SET series = $1, repeticoes = $2 WHERE treino_id = $3 AND exercicio_id = $4",
                series,
                repeticoes,
                form.treino_id,
                exercicio_id
            )
            .execute(db_pool.get_ref())
            .await
            .map_err(|e| {
                eprintln!("Erro ao atualizar treino_exercicio: {:?}", e);
                actix_web::error::ErrorInternalServerError("Erro ao atualizar exercício")
            })?;
        } else {
            eprintln!("Exercício {} não tem repetições definidas", exercicio_id);
        }
    }

    Ok(HttpResponse::SeeOther()
        .append_header(("Location", "/meus_treinos"))
        .finish())
}

#[derive(Deserialize)]
pub struct DeletarExercicioForm {
    treino_id: i32,
    exercicio_id: i32,
}

#[post("/deletar_exercicio")]
pub async fn deletar_exercicio(
    db_pool: web::Data<PgPool>,
    form: web::Form<DeletarExercicioForm>,
) -> Result<HttpResponse, Error> {
    let result = sqlx::query!(
        "DELETE FROM treino_exercicio WHERE treino_id = $1 AND exercicio_id = $2",
        form.treino_id,
        form.exercicio_id
    )
    .execute(db_pool.get_ref())
    .await;

    match result {
        Ok(_) => Ok(HttpResponse::Ok().body("Exercício deletado com sucesso")),
        Err(e) => {
            eprintln!("Erro ao deletar exercício: {:?}", e);
            Err(actix_web::error::ErrorInternalServerError("Erro ao deletar exercício"))
        }
    }
}



pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(fs::Files::new("/static", "./static").show_files_listing());
    cfg.service(web::resource("/editar_treinos_salvos").route(web::get().to(editar_treinos_salvos)));
     cfg.service(deletar_treino);
     cfg.service(salvar_edicao_treinos);
     cfg.service(deletar_exercicio);
    cfg.route("/tela_inicial", web::get().to(lista_alunos));
    cfg.route("/montar_treino", web::get().to(montar_treino));
    cfg.route("/salvar_treino", web::post().to(salvar_treino));
    cfg.route("/meus_treinos", web::get().to(visualizar_treinos));
    

}

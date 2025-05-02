use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use dotenv::dotenv;
use std::env;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Series {
    id: i64,
    title: String,
    status: String,
    last_episode_watched: i32,
    total_episodes: i32,
    ranking: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
struct SeriesForm {
    title: String,
    status: String,
    #[serde(rename = "lastEpisodeWatched")]
    last_episode_watched: i32,
    #[serde(rename = "totalEpisodes")]
    total_episodes: i32,
    ranking: i32,
}

#[derive(Debug, Deserialize)]
struct StatusUpdate {
    status: String,
}

struct AppState {
    db_pool: Pool<Postgres>
}

async fn get_all_series(data: web::Data<AppState>) -> Result<HttpResponse> {
    let result = sqlx::query_as::<_, Series>(
        "SELECT id, title, status, last_episode_watched, total_episodes, ranking, created_at FROM series"
    )
    .fetch_all(&data.db_pool)
    .await;

    match result {
        Ok(series) => Ok(HttpResponse::Ok().json(series)),
        Err(e) => {
            eprintln!("Error fetching series: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                    serde_json::json!({"error": "Error al obtener las series"})
            ))
        }
    }
}

async fn get_series_by_id(
    data: web::Data<AppState>,
    path: web::Path<i64>
) -> Result<HttpResponse> {
    let id = path.into_inner();

    let result = sqlx::query_as::<_, Series>(
        "SELECT id, title, status, last_episode_watched, total_episodes, ranking, created_at
         FROM series
         WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&data.db_pool)
    .await;

    match result {
        Ok(Some(series)) => Ok(HttpResponse::Ok().json(series)),
        Ok(None) => Ok(HttpResponse::NotFound().json(
                serde_json::json!({"error": "Serie no encontrada"})
        )),
        Err(e) => {
            eprintln!("Error fetching series {}", e);
            Ok(HttpResponse::InternalServerError().json(
                    serde_json::json!({"error": "Error al obtener la serie"})
            ))
        }
    }
}

async fn create_series(
    data: web::Data<AppState>,
    form: web::Json<SeriesForm>
) -> Result<HttpResponse> {
    // validaciones
    if form.title.is_empty() {
        return Ok(HttpResponse::BadRequest().json(
            serde_json::json!({"error": "El titulo es obligatorio"})
        ));
    }

    let result = sqlx::query_as::<_, Series>(
        "INSERT INTO series (title, status, last_episode_watched, total_episodes, ranking, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, title, status, last_episode_watched, total_episodes, ranking, created_at"
    )
    .bind(&form.title)
    .bind(&form.status)
    .bind(form.last_episode_watched)
    .bind(form.total_episodes)
    .bind(form.ranking)
    .bind(chrono::Utc::now())
    .fetch_one(&data.db_pool)
    .await;

    match result {
        Ok(new_series) => Ok(HttpResponse::Created().json(new_series)),
        Err(e) => {
            eprintln!("Error creating series: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Error al crear la serie"})
            ))
        }
    }
}

async fn update_series(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    form: web::Json<SeriesForm>
) -> Result<HttpResponse> {
    let id = path.into_inner();

    if form.title.is_empty() {
        return Ok(HttpResponse::BadRequest().json(
            serde_json::json!({"error": "El titulo es obligatorio"})
        ));
    }

    let exists = sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM series WHERE id = $1)")
        .bind(id)
        .fetch_one(&data.db_pool)
        .await;

    match exists {
        Ok(true) => {
            let result = sqlx::query_as::<_, Series>(
                "UPDATE series
                 SET title = $1, status = $2, last_episode_watched = $3, total_episodes = $4, ranking = $5
                 WHERE id = $6
                 RETURNING id, title, status, last_episode_watched, total_episodes, ranking, created_at"
            )
            .bind(&form.title)
            .bind(&form.status)
            .bind(form.last_episode_watched)
            .bind(form.total_episodes)
            .bind(form.ranking)
            .bind(id)
            .fetch_one(&data.db_pool)
            .await;

            match result {
                Ok(updated_series) => Ok(HttpResponse::Ok().json(updated_series)),
                Err(e) => {
                    eprintln!("Error updating series: {}", e);
                    Ok(HttpResponse::InternalServerError().json(
                        serde_json::json!({"error": "Error al actualizar la serie"})
                    ))
                }
            }
        },
        Ok(false) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "Serie no encontrada"})
        )),
        Err(e) => {
            eprintln!("Error checking if series exists: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Error al verificar la existencia de la serie"})
            ))
        }
    }
}

async fn delete_series(
    data: web::Data<AppState>,
    path: web::Path<i64>
) -> Result<HttpResponse> {
    let id = path.into_inner();

    let result = sqlx::query("DELETE FROM series WHERE id = $1 RETURNING id")
        .bind(id)
        .fetch_optional(&data.db_pool)
        .await;

    match result {
        Ok(Some(_)) => Ok(HttpResponse::Ok().json(
            serde_json::json!({"message": "Serie eliminada correctamente"})
        )),
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "Serie no encontrada"})
        )),
        Err(e) => {
            eprintln!("Error detecting series: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Error al eliminar la serie"})
            ))
        }
    }
}

async fn update_status(
    data: web::Data<AppState>,
    path: web::Path<i64>,
    status_update: web::Json<StatusUpdate>
) -> Result<HttpResponse> {
    let id = path.into_inner();

    if status_update.status.is_empty() {
        return Ok(HttpResponse::BadRequest().json(
            serde_json::json!({"error": "El estado no puede estar vacio"})
        ));
    }

    let result = sqlx::query_as::<_, Series>(
        "UPDATE series
         SET status = $1
         WHERE id = $2
         RETURNING id, title, status, last_episode_watched, total_episodes, ranking, created_at"
    )
    .bind(&status_update.status)
    .bind(id)
    .fetch_optional(&data.db_pool)
    .await;

    match result {
        Ok(Some(updated_series)) => Ok(HttpResponse::Ok().json(updated_series)),
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "Serie no encontrada"})
        )),
        Err(e) => {
            eprintln!("Error updating series status: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Error al actualizar el estado de la serie"})
            ))
        }
    }
}

async fn increment_episode(
    data: web::Data<AppState>,
    path: web::Path<i64>
) -> Result<HttpResponse> {
    let id = path.into_inner();

    let series = sqlx::query_as::<_, Series>(
        "SELECT id, title, status, last_episode_watched, total_episodes, ranking, created_at 
         FROM series 
         WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&data.db_pool)
    .await;
    
    match series {
        Ok(Some(series)) => {
            if series.last_episode_watched >= series.total_episodes {
                return Ok(HttpResponse::BadRequest().json(
                    serde_json::json!({"error": "No se puede incrementar mas alla del total de episodios"})
                ));
            }
            
            let result = sqlx::query_as::<_, Series>(
                "UPDATE series 
                 SET last_episode_watched = last_episode_watched + 1
                 WHERE id = $1
                 RETURNING id, title, status, last_episode_watched, total_episodes, ranking, created_at"
            )
            .bind(id)
            .fetch_one(&data.db_pool)
            .await;
            
            match result {
                Ok(updated_series) => Ok(HttpResponse::Ok().json(updated_series)),
                Err(e) => {
                    eprintln!("Error incrementing episode: {}", e);
                    Ok(HttpResponse::InternalServerError().json(
                        serde_json::json!({"error": "Error al incrementar el episodio"})
                    ))
                }
            }
        },
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "Serie no encontrada"})
        )),
        Err(e) => {
            eprintln!("Error fetching series: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Error al obtener la serie"})
            ))
        }
    }
}

async fn upvote_series(
    data: web::Data<AppState>,
    path: web::Path<i64>
) -> Result<HttpResponse> {
    let id = path.into_inner();

    let result = sqlx::query_as::<_, Series>(
        "UPDATE series
         SET ranking = ranking + 1
         WHERE id = $1
         RETURNING id, title, status, last_episode_watched, total_episodes, ranking, created_at"
    )
    .bind(id)
    .fetch_optional(&data.db_pool)
    .await;

    match result {
        Ok(Some(updated_series)) => Ok(HttpResponse::Ok().json(updated_series)),
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "Serie no encontrada"})
        )),
        Err(e) => {
            eprintln!("Error upvoting series: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Error al aumentar la puntuacion de la serie"})
            ))
        }
    }
}

async fn downvote_series(
    data: web::Data<AppState>,
    path: web::Path<i64>
) -> Result<HttpResponse> {
    let id = path.into_inner();

    let result = sqlx::query_as::<_, Series>(
        "UPDATE series
         SET ranking = ranking - 1
         WHERE id = $1
         RETURNING id, title, status, last_episode_watched, total_episodes, ranking, created_at"
    )
    .bind(id)
    .fetch_optional(&data.db_pool)
    .await;

    match result {
        Ok(Some(updated_series)) => Ok(HttpResponse::Ok().json(updated_series)),
        Ok(None) => Ok(HttpResponse::NotFound().json(
            serde_json::json!({"error": "Serie no encontrada"})
        )),
        Err(e) => {
            eprintln!("Error downvoting series: {}", e);
            Ok(HttpResponse::InternalServerError().json(
                serde_json::json!({"error": "Error al disminuir la puntuacion de la serie"})
            ))
        }
    }
}

async fn init_db(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS series (
            id BIGSERIAL PRIMARY KEY,
            title VARCHAR(255) NOT NULL,
            status VARCHAR(50) NOT NULL,
            last_episode_watched INTEGER NOT NULL,
            total_episodes INTEGER NOT NULL,
            ranking INTEGER NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL
        )
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    unsafe { std::env::set_var("RUST_LOG", "actix_web=info"); }
    env_logger::init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL debe estar establecida en el archivo .env");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("No se pudo conectar a la base de datos");

    init_db(&db_pool)
        .await
        .expect("No se pudo inicializar la base de datos");

    let app_state = web::Data::new(AppState { db_pool });

    println!("Servidor corriendo en http://localhost:8080");


    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .service(
                web::scope("/api")
                    .route("/series", web::get().to(get_all_series))
                    .route("/series", web::post().to(create_series))
                    .route("/series/{id}", web::get().to(get_series_by_id))
                    .route("/series/{id}", web::put().to(update_series))
                    .route("/series/{id}", web::delete().to(delete_series))
                    .route("/series/{id}/status", web::patch().to(update_status))
                    .route("/series/{id}/episode", web::patch().to(increment_episode))
                    .route("/series/{id}/upvote", web::patch().to(upvote_series))
                    .route("/series/{id}/downvote", web::patch().to(downvote_series))
            )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}

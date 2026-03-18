mod db;
mod error;
mod git;
mod handlers;
mod models;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::{get, post, put, delete};
use axum::Router;
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

const INDEX_HTML: &str = include_str!("web/index.html");

pub struct AppState {
    pub db: db::Database,
    pub repo_path: PathBuf,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("gitnote_api=debug".parse().unwrap()))
        .init();

    let db_path = std::env::var("DATABASE_URL").unwrap_or_else(|_| "data/gitnote.db".into());
    let repo_path = std::env::var("GIT_REPOS_PATH").unwrap_or_else(|_| "data/repo.git".into());
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);

    // Ensure data directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let database = db::Database::new(&db_path).expect("Failed to open database");
    let repo_path = PathBuf::from(&repo_path);
    git::init_repo(&repo_path).expect("Failed to init git repo");

    let state = Arc::new(AppState {
        db: database,
        repo_path,
    });

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/health", get(handlers::health))
        .route("/api/pages", post(handlers::create_page))
        .route("/api/pages", get(handlers::list_pages))
        .route("/api/pages/{id}", get(handlers::get_page))
        .route("/api/pages/{id}", put(handlers::update_page))
        .route("/api/pages/{id}", delete(handlers::delete_page))
        .route("/api/pages/{id}/history", get(handlers::page_history))
        .route("/api/pages/{id}/revisions/{oid}", get(handlers::page_at_revision))
        .route("/api/pages/{id}/restore/{oid}", post(handlers::restore_page))
        .route("/api/search", get(handlers::search))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr: SocketAddr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("GitNote API listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], INDEX_HTML)
}

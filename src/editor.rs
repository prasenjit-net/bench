use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};

use crate::ui_assets::UiAssets;

#[derive(Clone, PartialEq)]
pub enum ServerMode {
    Editor,
    Report,
}

// Shared state
#[derive(Clone)]
struct AppState {
    mode: ServerMode,
    file_path: Arc<PathBuf>,
    _content: Arc<Mutex<String>>,
}

// ── Asset serving ─────────────────────────────────────────────────────────────

fn embedded_file(path: &str) -> Response {
    match UiAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn serve_index() -> Response { embedded_file("index.html") }

async fn serve_asset(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    let full = format!("assets/{path}");
    match UiAssets::get(&full) {
        Some(_) => embedded_file(&full),
        None    => StatusCode::NOT_FOUND.into_response(),
    }
}

// ── API handlers ──────────────────────────────────────────────────────────────

/// GET /api/mode — tells the UI which app to render
async fn get_mode(State(state): State<AppState>) -> Response {
    let mode_str = match state.mode {
        ServerMode::Editor => "editor",
        ServerMode::Report => "report",
    };
    Json(json!({ "mode": mode_str })).into_response()
}

/// GET /api/scenario — return scenario file (editor mode)
async fn get_scenario(State(state): State<AppState>) -> Response {
    match std::fs::read_to_string(state.file_path.as_ref()) {
        Ok(content) => ([(header::CONTENT_TYPE, "application/json")], content).into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Json(json!({
                "run": { "concurrency": 10, "timeout_ms": 5000, "requests": 100,
                         "output_format": "json", "output": "report.json" },
                "requests": {},
                "scenarios": []
            })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot read file: {e}")).into_response(),
    }
}

/// PUT /api/scenario — write scenario file (editor mode)
async fn put_scenario(State(state): State<AppState>, body: axum::body::Bytes) -> Response {
    if let Err(e) = serde_json::from_slice::<serde_json::Value>(&body) {
        return (StatusCode::BAD_REQUEST, format!("Invalid JSON: {e}")).into_response();
    }
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let pretty = match serde_json::to_string_pretty(&value) {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    if let Some(parent) = state.file_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot create directory: {e}")).into_response();
        }
    }
    if let Err(e) = std::fs::write(state.file_path.as_ref(), pretty) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot write file: {e}")).into_response();
    }
    StatusCode::OK.into_response()
}

/// GET /api/report — return JSON report file (report mode)
async fn get_report(State(state): State<AppState>) -> Response {
    match std::fs::read_to_string(state.file_path.as_ref()) {
        Ok(content) => ([(header::CONTENT_TYPE, "application/json")], content).into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound =>
            (StatusCode::NOT_FOUND, format!("Report file not found: {}", state.file_path.display())).into_response(),
        Err(e) =>
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot read report: {e}")).into_response(),
    }
}

// ── Server bootstrap ──────────────────────────────────────────────────────────

fn find_free_port(start: u16) -> u16 {
    (start..=65535)
        .find(|&p| TcpListener::bind(("127.0.0.1", p)).is_ok())
        .expect("No free ports available")
}

async fn start_server(mode: ServerMode, file_path: PathBuf, label: &str, emoji: &str) -> Result<()> {
    let port = find_free_port(7878);
    let url  = format!("http://localhost:{port}");

    let state = AppState {
        mode,
        file_path: Arc::new(file_path.clone()),
        _content: Arc::new(Mutex::new(String::new())),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/mode",     get(get_mode))
        .route("/api/scenario", get(get_scenario).put(put_scenario))
        .route("/api/report",   get(get_report))
        .route("/",             get(serve_index))
        .route("/assets/*path", get(serve_asset))
        .fallback(serve_index)
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;

    println!("\n{emoji}  bench {label}");
    println!("   File : {}", file_path.display());
    println!("   URL  : {url}");
    println!("\n   Press Ctrl+C to stop\n");

    if let Err(e) = open::that(&url) {
        eprintln!("   (Could not open browser automatically: {e})");
    }

    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn run_editor(file_path: PathBuf) -> Result<()> {
    start_server(ServerMode::Editor, file_path, "editor", "🖊 ").await
}

pub async fn run_report_viewer(file_path: PathBuf) -> Result<()> {
    start_server(ServerMode::Report, file_path, "report viewer", "📊").await
}

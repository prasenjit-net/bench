use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use rust_embed::Embed;
use tower_http::cors::{Any, CorsLayer};

// Embed the compiled React dist/ into the binary
#[derive(Embed)]
#[folder = "ui/dist/"]
struct UiAssets;

/// Serve an embedded file, guessing MIME type from extension.
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

// Shared state: path to the scenario file being edited
#[derive(Clone)]
struct AppState {
    file_path: Arc<PathBuf>,
    // Cache the last known content to detect external changes (not critical, nice to have)
    _content: Arc<Mutex<String>>,
}

/// GET /api/scenario — return current scenario file contents as JSON
async fn get_scenario(State(state): State<AppState>) -> Response {
    match std::fs::read_to_string(state.file_path.as_ref()) {
        Ok(content) => (
            [(header::CONTENT_TYPE, "application/json")],
            content,
        )
            .into_response(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Return an empty template the UI can work with
            let template = serde_json::json!({
                "run": { "concurrency": 10, "timeout_ms": 5000, "requests": 100,
                         "output_format": "html", "output": "report.html" },
                "requests": {},
                "scenarios": []
            });
            Json(template).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Cannot read file: {e}"),
        )
            .into_response(),
    }
}

/// PUT /api/scenario — validate & write scenario file
async fn put_scenario(
    State(state): State<AppState>,
    body: axum::body::Bytes,
) -> Response {
    // Validate it's parseable JSON (we keep it as-is to preserve formatting)
    if let Err(e) = serde_json::from_slice::<serde_json::Value>(&body) {
        return (StatusCode::BAD_REQUEST, format!("Invalid JSON: {e}")).into_response();
    }
    // Pretty-print for readability
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let pretty = match serde_json::to_string_pretty(&value) {
        Ok(s) => s,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    // Create parent dirs if needed
    if let Some(parent) = state.file_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot create directory: {e}"))
                .into_response();
        }
    }
    if let Err(e) = std::fs::write(state.file_path.as_ref(), pretty) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Cannot write file: {e}")).into_response();
    }
    StatusCode::OK.into_response()
}

/// Serve React SPA assets
async fn serve_index() -> Response {
    embedded_file("index.html")
}

async fn serve_asset(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    // Try exact path first, then fall back to index.html for SPA routing
    match UiAssets::get(&path) {
        Some(_) => embedded_file(&path),
        None => embedded_file("index.html"),
    }
}

/// Find the lowest available TCP port starting from `start`.
fn find_free_port(start: u16) -> u16 {
    (start..=65535)
        .find(|&p| TcpListener::bind(("127.0.0.1", p)).is_ok())
        .expect("No free ports available")
}

pub async fn run_editor(file_path: PathBuf) -> Result<()> {
    let port = find_free_port(7878);
    let url = format!("http://localhost:{port}");

    let state = AppState {
        file_path: Arc::new(file_path.clone()),
        _content: Arc::new(Mutex::new(String::new())),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/scenario", get(get_scenario).put(put_scenario))
        .route("/", get(serve_index))
        .route("/assets/{*path}", get(serve_asset))
        .fallback(serve_index)
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;

    let file_display = file_path.display().to_string();
    println!("\n🖊  bench editor");
    println!("   File : {file_display}");
    println!("   URL  : {url}");
    println!("\n   Press Ctrl+C to stop\n");

    // Open browser (non-blocking, ignore errors)
    if let Err(e) = open::that(&url) {
        eprintln!("   (Could not open browser automatically: {e})");
    }

    axum::serve(listener, app).await?;
    Ok(())
}

//! # api-server
//!
//! REST API over the simulation engine. The engine owns all physical state; this crate
//! only validates requests, manages sessions and serialises results.

pub mod error;
pub mod openapi;
pub mod routes;
pub mod session;

use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::routes::*;
use crate::session::SessionStore;

pub fn app_state() -> AppState {
    AppState {
        store: Arc::new(SessionStore::default()),
        started: std::time::Instant::now(),
    }
}

/// Build the API router. When `static_dir` is given, the built frontend is served
/// from it with an SPA fallback to `index.html`.
pub fn router(state: AppState, static_dir: Option<PathBuf>) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi::openapi))
        .route("/config/default", get(default_config))
        .route("/config/validate", post(validate_config))
        .route("/metrics/definitions", get(metric_defs))
        .route("/presets/scenarios", get(preset_scenarios))
        .route("/presets/experiments", get(preset_experiments))
        .route("/flight-path/templates", post(path_templates))
        .route(
            "/simulations",
            get(list_simulations).post(create_simulation),
        )
        .route(
            "/simulations/{id}",
            axum::routing::delete(delete_simulation),
        )
        .route("/simulations/{id}/status", get(get_status))
        .route("/simulations/{id}/start", post(start))
        .route("/simulations/{id}/pause", post(pause))
        .route("/simulations/{id}/resume", post(resume))
        .route("/simulations/{id}/stop", post(stop))
        .route("/simulations/{id}/reset", post(reset))
        .route("/simulations/{id}/speed", post(set_speed))
        .route("/simulations/{id}/telemetry", get(telemetry))
        .route("/simulations/{id}/results", get(results))
        .route("/simulations/{id}/export.csv", get(export_csv))
        .route("/simulations/{id}/export.json", get(export_json))
        .route("/experiments/run", post(run_batch))
        .route("/experiments/compare", post(compare))
        .route("/experiments/presets/{id}/run", post(run_preset))
        .with_state(state);

    let mut app = Router::new().nest("/api", api);
    if let Some(dir) = static_dir {
        let index = dir.join("index.html");
        app = app.fallback_service(ServeDir::new(dir).fallback(ServeFile::new(index)));
    }
    app.layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

//! Compact OpenAPI 3.1 description of the endpoints. Request and response bodies are
//! described by reference to the Rust types (see `docs/api.md`); the schema objects
//! themselves are not generated.

use axum::Json;
use serde_json::{Value, json};

fn op(summary: &str, request: Option<&str>, response: &str) -> Value {
    let mut o = json!({
        "summary": summary,
        "responses": {
            "200": { "description": response },
            "4XX": { "description": "Structured error: {\"error\": {\"code\", \"message\", \"details\"}}" }
        }
    });
    if let Some(r) = request {
        o["requestBody"] = json!({ "required": true, "content": { "application/json": { "schema": { "description": r } } } });
    }
    o
}

pub async fn openapi() -> Json<Value> {
    let id = json!([{ "name": "id", "in": "path", "required": true, "schema": { "type": "string", "format": "uuid" } }]);
    let with_id = |mut v: Value| {
        v["parameters"] = id.clone();
        v
    };
    Json(json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Robotic airship autopilot simulator API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Simulation-only research API. Results are simulated data from a simplified model."
        },
        "paths": {
            "/api/health": { "get": op("Service health", None, "status, version, uptime and session count") },
            "/api/config/default": { "get": op("Default simulation configuration", None, "SimulationConfig") },
            "/api/config/validate": { "post": op("Validate a configuration and compute derived properties", Some("SimulationConfig"), "valid, issues[], derived, approach geometry") },
            "/api/metrics/definitions": { "get": op("Metric definitions with units", None, "list of metric definitions") },
            "/api/presets/scenarios": { "get": op("Environment scenarios 1-5", None, "Scenario[]") },
            "/api/presets/experiments": { "get": op("Research experiments A-G", None, "Experiment[]") },
            "/api/flight-path/templates": { "post": op("Generate predefined flight paths for a configuration", Some("SimulationConfig"), "template, description, path") },
            "/api/simulations": {
                "get": op("List sessions", None, "SessionStatus[]"),
                "post": op("Create a session (validated)", Some("{config: SimulationConfig, speed?: number}"), "201 SessionStatus")
            },
            "/api/simulations/{id}": { "delete": with_id(op("Delete a session", None, "204")) },
            "/api/simulations/{id}/status": { "get": with_id(op("Session status with latest telemetry sample", None, "SessionStatus")) },
            "/api/simulations/{id}/start": { "post": with_id(op("Start (optional {speed})", Some("{speed?: number}"), "SessionStatus")) },
            "/api/simulations/{id}/pause": { "post": with_id(op("Pause", None, "SessionStatus")) },
            "/api/simulations/{id}/resume": { "post": with_id(op("Resume", None, "SessionStatus")) },
            "/api/simulations/{id}/stop": { "post": with_id(op("Stop (results remain available)", None, "SessionStatus")) },
            "/api/simulations/{id}/reset": { "post": with_id(op("Reset to the initial conditions", None, "SessionStatus")) },
            "/api/simulations/{id}/speed": { "post": with_id(op("Change replay speed (does not change dt)", Some("{speed: number}"), "SessionStatus")) },
            "/api/simulations/{id}/telemetry": { "get": with_id(op("Telemetry samples (?since, ?stride, ?max_points)", None, "{total, next_index, samples[]}")) },
            "/api/simulations/{id}/results": { "get": with_id(op("Results report (?partial=true while running)", None, "ResultsReport")) },
            "/api/simulations/{id}/export.csv": { "get": with_id(op("Telemetry as CSV", None, "text/csv")) },
            "/api/simulations/{id}/export.json": { "get": with_id(op("Experiment as JSON (config, results, telemetry)", None, "application/json")) },
            "/api/experiments/run": { "post": op("Run one configuration to completion", Some("{config, max_points?}"), "ComparisonReport") },
            "/api/experiments/compare": { "post": op("Run and compare configurations", Some("{runs: [{label, config}], max_points?}"), "ComparisonReport") },
            "/api/experiments/presets/{id}/run": { "post": op("Run a predefined experiment (A-G)", None, "ComparisonReport") }
        }
    }))
}

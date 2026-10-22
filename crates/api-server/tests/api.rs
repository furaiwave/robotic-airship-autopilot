//! HTTP-level tests of the API router (no network socket; requests go through
//! `tower::ServiceExt::oneshot`).

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

fn app() -> Router {
    api_server::router(api_server::app_state(), None)
}

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value, String) {
    let mut req = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            req = req.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let resp = app.clone().oneshot(req.body(body).unwrap()).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&bytes).to_string();
    let json = serde_json::from_str(&text).unwrap_or(Value::Null);
    (status, json, text)
}

async fn default_config(app: &Router) -> Value {
    call(app, Method::GET, "/api/config/default", None).await.1
}

#[tokio::test]
async fn health_and_default_config() {
    let app = app();
    let (s, v, _) = call(&app, Method::GET, "/api/health", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["status"], "ok");
    let cfg = default_config(&app).await;
    assert!(cfg["airship"]["length_m"].as_f64().unwrap() > 0.0);
    let (s, v, _) = call(&app, Method::GET, "/api/openapi.json", None).await;
    assert_eq!(s, StatusCode::OK);
    assert!(v["paths"]["/api/simulations"].is_object());
}

#[tokio::test]
async fn validation_reports_issues_and_derived_properties() {
    let app = app();
    let mut cfg = default_config(&app).await;
    let (s, v, _) = call(
        &app,
        Method::POST,
        "/api/config/validate",
        Some(cfg.clone()),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["valid"], true);
    assert!(v["derived"]["buoyancy_n"].as_f64().unwrap() > 0.0);

    cfg["airship"]["length_m"] = json!(-5.0);
    let (_, v, _) = call(
        &app,
        Method::POST,
        "/api/config/validate",
        Some(cfg.clone()),
    )
    .await;
    assert_eq!(v["valid"], false);
    assert!(
        v["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["field"] == "airship.length_m")
    );

    // Creating a simulation from an invalid config is rejected with 422 and details.
    let (s, v, _) = call(
        &app,
        Method::POST,
        "/api/simulations",
        Some(json!({ "config": cfg })),
    )
    .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(v["error"]["code"], "validation_failed");
    assert!(!v["error"]["details"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn malformed_json_is_a_structured_400() {
    let app = app();
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/config/validate")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let v: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn unknown_session_is_404() {
    let app = app();
    let (s, v, _) = call(
        &app,
        Method::GET,
        "/api/simulations/00000000-0000-0000-0000-000000000000/status",
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(v["error"]["code"], "not_found");
    let (s, _, _) = call(
        &app,
        Method::GET,
        "/api/simulations/not-a-uuid/status",
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_lifecycle_runs_to_landing_and_exports() {
    let app = app();
    let cfg = default_config(&app).await;
    let (s, v, _) = call(
        &app,
        Method::POST,
        "/api/simulations",
        Some(json!({ "config": cfg, "speed": 0 })),
    )
    .await;
    assert_eq!(s, StatusCode::CREATED);
    let id = v["id"].as_str().unwrap().to_string();
    assert_eq!(v["state"], "created");

    // Results are not available before the run finishes.
    let (s, _, _) = call(
        &app,
        Method::GET,
        &format!("/api/simulations/{id}/results"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);

    let (s, v, _) = call(
        &app,
        Method::POST,
        &format!("/api/simulations/{id}/start"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["state"], "running");

    // Pause and resume while running.
    let (s, v, _) = call(
        &app,
        Method::POST,
        &format!("/api/simulations/{id}/pause"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(v["state"] == "paused" || v["state"] == "finished");
    let (s, _, _) = call(
        &app,
        Method::POST,
        &format!("/api/simulations/{id}/resume"),
        None,
    )
    .await;
    assert!(s == StatusCode::OK || s == StatusCode::CONFLICT);

    let mut state = Value::Null;
    for _ in 0..600 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let (_, v, _) = call(
            &app,
            Method::GET,
            &format!("/api/simulations/{id}/status"),
            None,
        )
        .await;
        state = v["state"].clone();
        if state == "finished" {
            assert_eq!(v["finished"], "landed");
            break;
        }
    }
    assert_eq!(state, "finished");

    let (s, v, _) = call(
        &app,
        Method::GET,
        &format!("/api/simulations/{id}/results"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["landing"]["classification"], "successful");
    assert!(
        v["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["key"] == "landing_point_error_m")
    );

    let (_, v, _) = call(
        &app,
        Method::GET,
        &format!("/api/simulations/{id}/telemetry?since=10&max_points=50"),
        None,
    )
    .await;
    assert!(v["samples"].as_array().unwrap().len() <= 52);
    assert!(v["total"].as_u64().unwrap() > 100);

    let (s, _, csv) = call(
        &app,
        Method::GET,
        &format!("/api/simulations/{id}/export.csv"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(csv.lines().nth(1).unwrap().starts_with("t_s,phase"));

    let (s, v, _) = call(
        &app,
        Method::GET,
        &format!("/api/simulations/{id}/export.json"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["format"], "robotic-airship-experiment/v1");
    assert!(v["telemetry"].as_array().unwrap().len() > 100);

    // A finished run cannot be started again until it is reset.
    let (s, _, _) = call(
        &app,
        Method::POST,
        &format!("/api/simulations/{id}/start"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    let (s, v, _) = call(
        &app,
        Method::POST,
        &format!("/api/simulations/{id}/reset"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["state"], "created");

    let (s, _, _) = call(
        &app,
        Method::DELETE,
        &format!("/api/simulations/{id}"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_marks_run_finished_and_allows_partial_results() {
    let app = app();
    let cfg = default_config(&app).await;
    let (_, v, _) = call(
        &app,
        Method::POST,
        "/api/simulations",
        Some(json!({ "config": cfg, "speed": 5 })),
    )
    .await;
    let id = v["id"].as_str().unwrap().to_string();
    call(
        &app,
        Method::POST,
        &format!("/api/simulations/{id}/start"),
        None,
    )
    .await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let (_, v, _) = call(
        &app,
        Method::GET,
        &format!("/api/simulations/{id}/results?partial=true"),
        None,
    )
    .await;
    assert_eq!(v["partial"], true);
    let (s, v, _) = call(
        &app,
        Method::POST,
        &format!("/api/simulations/{id}/stop"),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["state"], "finished");
    assert_eq!(v["finished"], "stopped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn controller_comparison_reports_identical_conditions() {
    let app = app();
    let mut a = default_config(&app).await;
    a["simulation"]["duration_s"] = json!(60.0);
    let mut b = a.clone();
    b["controller"]["kind"] = json!("gain_scheduled");
    let (s, v, _) = call(
        &app,
        Method::POST,
        "/api/experiments/compare",
        Some(json!({ "runs": [{ "label": "A", "config": a }, { "label": "B", "config": b }], "max_points": 200 })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["identical_conditions"], true);
    let runs = v["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[1]["results"]["controller"], "gain_scheduled");
    assert!(runs[0]["telemetry"].as_array().unwrap().len() <= 202);

    // Changing the wind breaks the identical-conditions property and is reported.
    let mut c = a.clone();
    c["environment"]["wind"]["speed_m_s"] = json!(3.0);
    let (_, v, _) = call(
        &app,
        Method::POST,
        "/api/experiments/compare",
        Some(json!({ "runs": [{ "label": "A", "config": a }, { "label": "C", "config": c }] })),
    )
    .await;
    assert_eq!(v["identical_conditions"], false);
    assert!(
        v["differences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d == "environment.wind.speed_m_s")
    );
}

#[tokio::test]
async fn presets_and_templates() {
    let app = app();
    let (_, v, _) = call(&app, Method::GET, "/api/presets/scenarios", None).await;
    assert_eq!(v.as_array().unwrap().len(), 5);
    let (_, v, _) = call(&app, Method::GET, "/api/presets/experiments", None).await;
    let ids: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["A", "B", "C", "D", "E", "F", "G"]);
    let cfg = default_config(&app).await;
    let (s, v, _) = call(&app, Method::POST, "/api/flight-path/templates", Some(cfg)).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 6);
    let (_, v, _) = call(&app, Method::GET, "/api/metrics/definitions", None).await;
    assert!(
        v.as_array()
            .unwrap()
            .iter()
            .all(|m| m["unit"].is_string() && m["definition"].is_string())
    );
}

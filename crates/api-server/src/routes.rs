//! Request handlers.

use std::sync::Arc;

use airship_model::{DerivedProperties, ValidationIssue};
use autopilot::{
    ApproachGeometry, FlightPath, PathTemplate, PhaseTransition, SafetyEvent, path_template,
};
use axum::Json;
use axum::extract::{FromRequest, Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use simulation_engine::metrics::metric_definitions;
use simulation_engine::telemetry::decimate;
use simulation_engine::{
    ComparisonReport, Experiment, ResultsReport, Scenario, SimulationConfig, SimulationStatus,
    TelemetrySample, compare_runs, experiments, scenarios, telemetry_csv,
};
use uuid::Uuid;

use crate::error::ApiError;
use crate::session::{RunState, Session, SessionRef, SessionStore, run_session};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<SessionStore>,
    pub started: std::time::Instant,
}

/// JSON extractor that maps body errors to structured [`ApiError`]s.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct ApiJson<T>(pub T);

type ApiResult<T> = Result<Json<T>, ApiError>;

fn parse_id(id: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(id).map_err(|_| ApiError::NotFound(id.to_string()))
}

fn session(state: &AppState, id: &str) -> Result<SessionRef, ApiError> {
    let uuid = parse_id(id)?;
    state
        .store
        .get(&uuid)
        .ok_or_else(|| ApiError::NotFound(id.to_string()))
}

fn lock(s: &SessionRef) -> Result<std::sync::MutexGuard<'_, Session>, ApiError> {
    s.lock()
        .map_err(|_| ApiError::Internal("session lock poisoned".into()))
}

fn validated(config: &SimulationConfig) -> Result<(), ApiError> {
    let report = config.validate();
    if report.has_errors() {
        return Err(ApiError::Validation(report.issues));
    }
    Ok(())
}

// ---------------------------------------------------------------- health & config

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
    version: &'static str,
    uptime_s: f64,
    sessions: usize,
}

pub async fn health(State(st): State<AppState>) -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_s: st.started.elapsed().as_secs_f64(),
        sessions: st.store.len(),
    })
}

pub async fn default_config() -> Json<SimulationConfig> {
    Json(SimulationConfig::default())
}

#[derive(Serialize)]
pub struct ValidationResponse {
    valid: bool,
    issues: Vec<ValidationIssue>,
    /// Derived vehicle properties at ground-level density (present when the vehicle section is valid).
    derived: Option<DerivedProperties>,
    ground_density_kg_m3: f64,
    approach: Option<ApproachGeometry>,
}

pub async fn validate_config(
    ApiJson(config): ApiJson<SimulationConfig>,
) -> Json<ValidationResponse> {
    let report = config.validate();
    let airship_ok = !config.airship.validate().has_errors();
    let env_ok = !config.environment.validate().has_errors();
    let approach =
        (env_ok && !config.autopilot.validate(&config.airship).has_errors()).then(|| {
            ApproachGeometry::from_config(
                config.environment.landing_site,
                config.environment.landing_area.heading_deg,
                &config.autopilot,
            )
        });
    Json(ValidationResponse {
        valid: !report.has_errors(),
        issues: report.issues,
        derived: (airship_ok && env_ok).then(|| config.derived()),
        ground_density_kg_m3: config.environment.ground_density(),
        approach,
    })
}

#[derive(Serialize)]
pub struct MetricDefinition {
    key: &'static str,
    label: &'static str,
    group: &'static str,
    unit: &'static str,
    definition: &'static str,
}

pub async fn metric_defs() -> Json<Vec<MetricDefinition>> {
    Json(
        metric_definitions()
            .into_iter()
            .map(|(key, label, group, unit, definition)| MetricDefinition {
                key,
                label,
                group,
                unit,
                definition,
            })
            .collect(),
    )
}

// ---------------------------------------------------------------- presets

pub async fn preset_scenarios() -> Json<Vec<Scenario>> {
    Json(scenarios())
}

pub async fn preset_experiments() -> Json<Vec<Experiment>> {
    Json(experiments())
}

#[derive(Serialize)]
pub struct TemplateResponse {
    template: PathTemplate,
    description: &'static str,
    path: FlightPath,
}

pub async fn path_templates(
    ApiJson(config): ApiJson<SimulationConfig>,
) -> ApiResult<Vec<TemplateResponse>> {
    let env = &config.environment;
    if env.validate().has_errors() {
        return Err(ApiError::Validation(env.validate().issues));
    }
    let geometry = ApproachGeometry::from_config(
        env.landing_site,
        env.landing_area.heading_deg,
        &config.autopilot,
    );
    Ok(Json(
        PathTemplate::ALL
            .iter()
            .map(|t| TemplateResponse {
                template: *t,
                description: t.description(),
                path: path_template(*t, env.takeoff_site, &geometry, &config.autopilot),
            })
            .collect(),
    ))
}

// ---------------------------------------------------------------- simulations

#[derive(Deserialize)]
pub struct CreateRequest {
    config: SimulationConfig,
    /// Simulated seconds per wall-clock second (default 1; <= 0 = as fast as possible).
    #[serde(default = "default_speed")]
    speed: f64,
}

fn default_speed() -> f64 {
    1.0
}

#[derive(Serialize)]
pub struct SessionStatus {
    id: Uuid,
    name: String,
    controller: control_algorithms::ControllerKind,
    state: RunState,
    speed: f64,
    created_at_ms: u64,
    progress: f64,
    #[serde(flatten)]
    sim: SimulationStatus,
    latest: Option<TelemetrySample>,
    last_transition: Option<PhaseTransition>,
    recent_events: Vec<SafetyEvent>,
}

fn status_of(s: &Session) -> SessionStatus {
    let st = s.sim.status();
    let ap = s.sim.autopilot();
    let events = &ap.safety.events;
    SessionStatus {
        id: s.id,
        name: s.config.name.clone(),
        controller: s.config.controller.kind,
        state: s.state,
        speed: s.speed,
        created_at_ms: s.created_at_ms,
        progress: if st.finished.is_some() {
            1.0
        } else {
            (st.time_s / st.duration_s).clamp(0.0, 1.0)
        },
        latest: s.sim.telemetry().last().cloned(),
        last_transition: ap.transitions.last().cloned(),
        recent_events: events.iter().rev().take(5).rev().cloned().collect(),
        sim: st,
    }
}

pub async fn list_simulations(
    State(st): State<AppState>,
) -> Result<Json<Vec<SessionStatus>>, ApiError> {
    let mut out = Vec::new();
    for s in st.store.all() {
        out.push(status_of(&*lock(&s)?));
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.created_at_ms));
    Ok(Json(out))
}

pub async fn create_simulation(
    State(st): State<AppState>,
    ApiJson(req): ApiJson<CreateRequest>,
) -> Result<Response, ApiError> {
    validated(&req.config)?;
    if !req.speed.is_finite() || req.speed > 1000.0 {
        return Err(ApiError::BadRequest(
            "speed must be finite and at most 1000".into(),
        ));
    }
    let session =
        Session::new(req.config, req.speed).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let r = st
        .store
        .insert(session)
        .map_err(ApiError::TooManySessions)?;
    let status = status_of(&*lock(&r)?);
    tracing::info!(id = %status.id, name = %status.name, "simulation created");
    Ok((StatusCode::CREATED, Json(status)).into_response())
}

pub async fn get_status(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<SessionStatus> {
    let s = session(&st, &id)?;
    Ok(Json(status_of(&*lock(&s)?)))
}

#[derive(Deserialize, Default)]
pub struct StartRequest {
    speed: Option<f64>,
}

fn spawn_runner(st: &AppState, s: &mut Session) {
    if !s.runner_active {
        s.runner_active = true;
        tokio::spawn(run_session(st.store.clone(), s.id));
    }
}

pub async fn start(
    State(st): State<AppState>,
    Path(id): Path<String>,
    body: Option<Json<StartRequest>>,
) -> ApiResult<SessionStatus> {
    let r = session(&st, &id)?;
    let mut s = lock(&r)?;
    if let Some(Json(StartRequest { speed: Some(v) })) = body {
        if !v.is_finite() || v > 1000.0 {
            return Err(ApiError::BadRequest(
                "speed must be finite and at most 1000".into(),
            ));
        }
        s.speed = v;
    }
    match s.state {
        RunState::Created | RunState::Paused => {
            s.state = RunState::Running;
            spawn_runner(&st, &mut s);
            Ok(Json(status_of(&s)))
        }
        RunState::Running => Ok(Json(status_of(&s))),
        RunState::Finished => Err(ApiError::Conflict(
            "simulation has finished; reset it to run again".into(),
        )),
    }
}

pub async fn pause(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<SessionStatus> {
    let r = session(&st, &id)?;
    let mut s = lock(&r)?;
    match s.state {
        RunState::Running => {
            s.state = RunState::Paused;
            Ok(Json(status_of(&s)))
        }
        RunState::Paused => Ok(Json(status_of(&s))),
        other => Err(ApiError::Conflict(format!(
            "cannot pause a simulation in state {other:?}"
        ))),
    }
}

pub async fn resume(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<SessionStatus> {
    let r = session(&st, &id)?;
    let mut s = lock(&r)?;
    match s.state {
        RunState::Paused => {
            s.state = RunState::Running;
            spawn_runner(&st, &mut s);
            Ok(Json(status_of(&s)))
        }
        RunState::Running => Ok(Json(status_of(&s))),
        other => Err(ApiError::Conflict(format!(
            "cannot resume a simulation in state {other:?}"
        ))),
    }
}

pub async fn stop(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<SessionStatus> {
    let r = session(&st, &id)?;
    let mut s = lock(&r)?;
    if s.state == RunState::Finished {
        return Ok(Json(status_of(&s)));
    }
    s.sim.stop();
    s.state = RunState::Finished;
    Ok(Json(status_of(&s)))
}

pub async fn reset(State(st): State<AppState>, Path(id): Path<String>) -> ApiResult<SessionStatus> {
    let r = session(&st, &id)?;
    let mut s = lock(&r)?;
    s.reset().map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(status_of(&s)))
}

#[derive(Deserialize)]
pub struct SpeedRequest {
    speed: f64,
}

pub async fn set_speed(
    State(st): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<SpeedRequest>,
) -> ApiResult<SessionStatus> {
    if !req.speed.is_finite() || req.speed > 1000.0 {
        return Err(ApiError::BadRequest(
            "speed must be finite and at most 1000".into(),
        ));
    }
    let r = session(&st, &id)?;
    let mut s = lock(&r)?;
    s.speed = req.speed;
    Ok(Json(status_of(&s)))
}

pub async fn delete_simulation(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let uuid = parse_id(&id)?;
    if let Some(s) = st.store.get(&uuid)
        && let Ok(mut s) = s.lock()
    {
        s.state = RunState::Finished;
    }
    if st.store.remove(&uuid) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(id))
    }
}

#[derive(Deserialize)]
pub struct TelemetryQuery {
    /// Index of the first sample to return.
    #[serde(default)]
    since: usize,
    /// Return every `stride`-th sample.
    #[serde(default)]
    stride: Option<usize>,
    /// Maximum number of samples (uniformly decimated when exceeded).
    #[serde(default)]
    max_points: Option<usize>,
}

#[derive(Serialize)]
pub struct TelemetryResponse {
    total: usize,
    next_index: usize,
    samples: Vec<TelemetrySample>,
}

pub async fn telemetry(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<TelemetryQuery>,
) -> ApiResult<TelemetryResponse> {
    let r = session(&st, &id)?;
    let s = lock(&r)?;
    let all = s.sim.telemetry();
    let start = q.since.min(all.len());
    let stride = q.stride.unwrap_or(1).max(1);
    let mut samples: Vec<TelemetrySample> = all[start..].iter().step_by(stride).cloned().collect();
    if let Some(m) = q.max_points {
        samples = decimate(&samples, m.max(2));
    }
    Ok(Json(TelemetryResponse {
        total: all.len(),
        next_index: all.len(),
        samples,
    }))
}

#[derive(Deserialize)]
pub struct ResultsQuery {
    #[serde(default)]
    partial: bool,
}

pub async fn results(
    State(st): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ResultsQuery>,
) -> ApiResult<ResultsReport> {
    let r = session(&st, &id)?;
    let s = lock(&r)?;
    if s.sim.finished().is_none() && !q.partial {
        return Err(ApiError::Conflict(
            "simulation has not finished; use ?partial=true for interim results".into(),
        ));
    }
    Ok(Json(s.sim.report()))
}

fn attachment(name: &str, ext: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("attachment; filename=\"{safe}.{ext}\"")
}

pub async fn export_csv(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let r = session(&st, &id)?;
    let s = lock(&r)?;
    let body = format!(
        "# {}\n{}",
        simulation_engine::DISCLAIMER,
        telemetry_csv(s.sim.telemetry())
    );
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (
                header::CONTENT_DISPOSITION,
                attachment(&s.config.name, "csv"),
            ),
        ],
        body,
    )
        .into_response())
}

#[derive(Serialize)]
struct ExperimentExport<'a> {
    format: &'static str,
    disclaimer: &'static str,
    config: &'a SimulationConfig,
    results: ResultsReport,
    telemetry: &'a [TelemetrySample],
}

pub async fn export_json(
    State(st): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let r = session(&st, &id)?;
    let s = lock(&r)?;
    let doc = ExperimentExport {
        format: "robotic-airship-experiment/v1",
        disclaimer: simulation_engine::DISCLAIMER,
        config: &s.config,
        results: s.sim.report(),
        telemetry: s.sim.telemetry(),
    };
    let body = serde_json::to_string(&doc).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (
                header::CONTENT_DISPOSITION,
                attachment(&s.config.name, "json"),
            ),
        ],
        body,
    )
        .into_response())
}

// ---------------------------------------------------------------- batch experiments

#[derive(Deserialize)]
pub struct CompareRun {
    label: String,
    config: SimulationConfig,
}

#[derive(Deserialize)]
pub struct CompareRequest {
    runs: Vec<CompareRun>,
    #[serde(default = "default_points")]
    max_points: usize,
}

fn default_points() -> usize {
    1500
}

async fn run_comparison(
    runs: Vec<(String, SimulationConfig)>,
    max_points: usize,
) -> Result<ComparisonReport, ApiError> {
    if runs.is_empty() || runs.len() > 6 {
        return Err(ApiError::BadRequest(
            "between 1 and 6 runs can be compared".into(),
        ));
    }
    let mut issues = Vec::new();
    for (label, c) in &runs {
        let r = c.validate();
        if r.has_errors() {
            issues.extend(r.issues.into_iter().map(|mut i| {
                i.field = format!("{label}: {}", i.field);
                i
            }));
        }
    }
    if !issues.is_empty() {
        return Err(ApiError::Validation(issues));
    }
    tokio::task::spawn_blocking(move || compare_runs(runs, max_points.clamp(10, 20_000)))
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .map_err(|e| ApiError::BadRequest(e.to_string()))
}

pub async fn compare(ApiJson(req): ApiJson<CompareRequest>) -> ApiResult<ComparisonReport> {
    let runs = req.runs.into_iter().map(|r| (r.label, r.config)).collect();
    Ok(Json(run_comparison(runs, req.max_points).await?))
}

#[derive(Deserialize)]
pub struct RunRequest {
    config: SimulationConfig,
    #[serde(default = "default_points")]
    max_points: usize,
}

/// Run one configuration to completion synchronously (batch mode).
pub async fn run_batch(ApiJson(req): ApiJson<RunRequest>) -> ApiResult<ComparisonReport> {
    let label = req.config.name.clone();
    Ok(Json(
        run_comparison(vec![(label, req.config)], req.max_points).await?,
    ))
}

pub async fn run_preset(Path(id): Path<String>) -> ApiResult<ComparisonReport> {
    let exp = experiments()
        .into_iter()
        .find(|e| e.id.eq_ignore_ascii_case(&id))
        .ok_or_else(|| ApiError::NotFound(format!("experiment {id}")))?;
    let runs = exp.runs.into_iter().map(|r| (r.label, r.config)).collect();
    Ok(Json(run_comparison(runs, default_points()).await?))
}

//! Structured API errors: `{"error": {"code", "message", "details"}}`.

use airship_model::ValidationIssue;
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("simulation {0} not found")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("configuration is invalid")]
    Validation(Vec<ValidationIssue>),
    #[error("{0}")]
    TooManySessions(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorPayload<'a>,
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    code: &'a str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<&'a [ValidationIssue]>,
}

impl ApiError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::Conflict(_) => (StatusCode::CONFLICT, "invalid_state"),
            ApiError::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, "validation_failed"),
            ApiError::TooManySessions(_) => (StatusCode::TOO_MANY_REQUESTS, "too_many_sessions"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        if status.is_server_error() {
            tracing::error!(error = %self, "request failed");
        }
        let details = match &self {
            ApiError::Validation(issues) => Some(issues.as_slice()),
            _ => None,
        };
        let body = ErrorBody {
            error: ErrorPayload {
                code,
                message: self.to_string(),
                details,
            },
        };
        (status, Json(body)).into_response()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        ApiError::BadRequest(format!("invalid JSON body: {}", r.body_text()))
    }
}

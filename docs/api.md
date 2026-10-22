# REST API

Base path `/api`. JSON in snake_case; units are in the field names. Errors always have
the form

```json
{ "error": { "code": "validation_failed", "message": "configuration is invalid",
             "details": [{ "field": "airship.length_m", "message": "...", "severity": "error" }] } }
```

| Status | code | When |
|---|---|---|
| 400 | `bad_request` | malformed JSON, invalid speed, invalid batch request |
| 404 | `not_found` | unknown session or experiment |
| 409 | `invalid_state` | e.g. start a finished run, results before the run finished |
| 422 | `validation_failed` | configuration fails server-side validation |
| 429 | `too_many_sessions` | all 32 session slots are running |
| 500 | `internal` | unexpected failure |

## Endpoints

| Method | Path | Body | Response |
|---|---|---|---|
| GET | `/health` | | `{status, version, uptime_s, sessions}` |
| GET | `/openapi.json` | | OpenAPI 3.1 path summary |
| GET | `/config/default` | | `SimulationConfig` |
| POST | `/config/validate` | `SimulationConfig` | `{valid, issues[], derived, ground_density_kg_m3, approach}` |
| GET | `/metrics/definitions` | | `[{key, label, group, unit, definition}]` |
| GET | `/presets/scenarios` | | environment scenarios 1–5 |
| GET | `/presets/experiments` | | experiments A–G with full configurations |
| POST | `/flight-path/templates` | `SimulationConfig` | six predefined paths for the configuration |
| GET | `/simulations` | | session status list |
| POST | `/simulations` | `{config, speed?}` | 201 `SessionStatus` |
| DELETE | `/simulations/{id}` | | 204 |
| GET | `/simulations/{id}/status` | | `SessionStatus` (state, time, phase, guidance mode, latest sample, last transition, recent events) |
| POST | `/simulations/{id}/start` | `{speed?}` | `SessionStatus` |
| POST | `/simulations/{id}/pause` · `/resume` · `/stop` · `/reset` | | `SessionStatus` |
| POST | `/simulations/{id}/speed` | `{speed}` | `SessionStatus` (dt unchanged) |
| GET | `/simulations/{id}/telemetry?since=&stride=&max_points=` | | `{total, next_index, samples[]}` |
| GET | `/simulations/{id}/results?partial=` | | `ResultsReport` |
| GET | `/simulations/{id}/export.csv` | | telemetry CSV (first line: disclaimer comment) |
| GET | `/simulations/{id}/export.json` | | `{format, disclaimer, config, results, telemetry}` |
| POST | `/experiments/run` | `{config, max_points?}` | `ComparisonReport` (one run) |
| POST | `/experiments/compare` | `{runs: [{label, config}], max_points?}` | `ComparisonReport` with `identical_conditions` and `differences` |
| POST | `/experiments/presets/{id}/run` | | `ComparisonReport` |

`speed` is simulated seconds per wall-clock second (≤ 1000); `0` means as fast as
possible. Example:

```bash
curl -s localhost:8080/api/config/default > cfg.json
curl -s -X POST localhost:8080/api/experiments/run -H 'content-type: application/json' \
     -d "{\"config\": $(cat cfg.json)}" | jq '.runs[0].results.landing.classification'
```

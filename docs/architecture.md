# System architecture

## 1. Problem statement and objectives

Design, implement and evaluate a software autopilot that performs autonomous take-off,
climb, en-route path following, approach, descent, landing and safe abort (go-around)
for a robotic lighter-than-air vehicle in simulation. The objectives are to:

1. model the airship's buoyancy, gravity, aerodynamics, propulsion, wind and ground
   interaction with a documented, defensible simplified model;
2. implement an explicit, testable flight-phase state machine with safety monitoring;
3. implement and compare two control approaches (fixed-gain cascaded PID and gain
   scheduling) under identical, reproducible conditions;
4. quantify performance with defined metrics and a four-criterion landing assessment;
5. provide an engineering user interface for configuration, live observation, analysis
   and export.

## 2. Requirements (summary)

| ID | Requirement | Where |
|---|---|---|
| F1 | Configure vehicle, environment, flight path, autopilot, controller, numerics | UI pages, `SimulationConfig` |
| F2 | Physical model with buoyancy, gravity, drag relative to air, propulsion, wind, ground contact | `flight-dynamics` |
| F3 | Autopilot phases 1–11 with logged, state-based transitions and go-around | `autopilot::state_machine` |
| F4 | Two controllers, comparable under identical conditions | `control-algorithms`, `compare` |
| F5 | Metrics and landing classification from actual telemetry | `simulation-engine::metrics`, `landing` |
| F6 | Start / pause / resume / stop / reset; speed change without changing dt | `api-server::session` |
| F7 | CSV telemetry and JSON experiment export | API `export.csv`, `export.json`; UI comparison export |
| N1 | Determinism for identical configuration and seeds | in-crate PRNG, fixed step, tests |
| N2 | Server-side validation independent of the client | `validate()` in every config section |
| N3 | Non-blocking API | stepping runs on Tokio's blocking pool |

## 3. Components

```
apps/web (React + TypeScript)                       crates/ (Rust workspace)
┌──────────────────────────────┐   REST / JSON   ┌───────────────────────────────────────┐
│ pages: dashboard, config,    │ ───────────────▶│ api-server (Axum)                     │
│ flight-path editor, autopilot│ ◀─── polling ── │  sessions, runner, validation, export │
│ simulation, results, compare │                 └──────────────┬────────────────────────┘
│ stores (Zustand), forms (RHF │                                │
│ + Zod), charts (Recharts),   │                 ┌──────────────▼────────────────────────┐
│ SVG views                    │                 │ simulation-engine                      │
└──────────────────────────────┘                 │  fixed-step loop, telemetry, metrics,  │
                                                 │  landing assessment, presets, compare, │
                                                 │  CLI `airship-sim`                     │
                                                 └──────┬───────────────┬────────────────┘
                                                        │               │
                                   ┌────────────────────▼──┐   ┌────────▼──────────────┐
                                   │ autopilot             │   │ flight-dynamics        │
                                   │  guidance, phases,    │   │  forces, EoM, RK4,     │
                                   │  safety, preflight    │   │  actuators, ground,    │
                                   └─────────┬─────────────┘   │  wind, sensors         │
                                             │                 └────────┬──────────────┘
                                   ┌─────────▼─────────────┐            │
                                   │ control-algorithms    │            │
                                   │  PID, schedule,       │            │
                                   │  cascade, allocation  │            │
                                   └─────────┬─────────────┘            │
                                             └──────────┬───────────────┘
                                              ┌─────────▼──────────┐
                                              │ airship-model       │
                                              │  params, derived,   │
                                              │  atmosphere, math,  │
                                              │  validation         │
                                              └─────────────────────┘
```

Mapping to the suggested module list: `airship_model`, `atmosphere` → `airship-model`;
`flight_dynamics`, `propulsion`, `environment` → `flight-dynamics`; `control` →
`control-algorithms`; `guidance`, `autopilot`, `safety` → `autopilot`; `simulation`,
`telemetry`, `landing`, `experiments` → `simulation-engine`; `api_server` → `api-server`.

**Separation of concerns.** Physical dynamics (`flight-dynamics`) know nothing about
guidance or phases. Controllers (`control-algorithms`) know phase *classes* only through
the gain schedule. The state machine (`autopilot`) sees measurements, not truth. The
engine owns the authoritative state. The frontend renders engine output and never
simulates physics: derived quantities, validation, templates, metrics and the landing
classification all come from the backend.

## 4. Simulation lifecycle and threading

* `POST /simulations` validates the configuration and creates a session holding a
  `Simulation` (state at t = 0).
* `start` spawns a Tokio task that wakes every 50 ms and advances the simulation by
  `speed × 50 ms` of simulated time, as a whole number of fixed steps with the remainder
  carried over, on the blocking thread pool. Speed ≤ 0 means "as fast as possible"
  (4000 steps per tick).
* `pause` and `resume` toggle the state; `stop` finishes the run with reason *stopped*
  (results remain available); `reset` recreates the initial state from the stored
  configuration.
* The frontend polls `status` and `telemetry?since=N` at 4 Hz, decoupling the physics rate
  (100 Hz), control rate (20 Hz), telemetry rate (10 Hz) and rendering rate.
* Batch endpoints (`/experiments/run`, `/compare`, `/presets/{id}/run`) run configurations
  to completion in parallel threads inside `spawn_blocking`.

## 5. Data contracts

Rust structures are serialised with serde in snake_case. Units are part of field names
(`_m`, `_m_s`, `_deg`, `_n`). The TypeScript mirror is in
`apps/web/src/types/api.ts`. Frontend test fixtures are generated from the running engine
so that the contract is exercised with real payloads. See [api.md](api.md) for the
endpoints.

## 6. Technology

Rust 2024 edition: Axum 0.8, Tokio, Serde, tower-http, thiserror, tracing, Criterion
(benchmarks). Frontend: React 19, TypeScript 5.9 (strict), Vite 7, Tailwind CSS 4,
shadcn/ui-style components on Radix, Lucide icons, Zustand 5, React Hook Form 7 with Zod 4,
Recharts 3, Vitest 3 with React Testing Library.

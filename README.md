# Robotic Airship Autopilot Lab

**Development and research of an autonomous take-off and landing autopilot for a robotic
airship**: simulation software for a bachelor's thesis.

A Rust flight-simulation engine (airship dynamics, autopilot, two controllers, metrics)
with an Axum REST API, and a React/TypeScript engineering workbench for configuration,
live observation, analysis, controller comparison and export.

> **Simulation only.** The model is simplified and has not been validated against
> flight data. Results are simulated data. The software is not certified and must not be
> used to control a real aircraft.

## Workflow

Configure airship → configure environment → define flight path → select autopilot and
controller → run take-off (mooring release, vertical climb) → follow the path →
approach (join, gate, glide path) → flare and touchdown → landing assessment →
compare controllers → export CSV/JSON.

## Quick start

Prerequisites: Rust ≥ 1.85 (developed with 1.99), Node.js ≥ 20 with npm (developed with
Node 24 / npm 11).

```bash
npm --prefix apps/web install        # frontend dependencies
npm run build                        # cargo build --release + vite build
npm start                            # API + built UI on http://127.0.0.1:8080
```

Development mode, with hot reload on http://localhost:5173 and `/api` proxied to :8080:

```bash
node scripts/dev.mjs
```

Headless CLI:

```bash
cargo run --release -p simulation-engine --bin airship-sim -- experiment F --out-dir results/
cargo run --release -p simulation-engine --bin airship-sim -- run examples/experiments/experiment-d-pid.json --controller gain_scheduled --csv d.csv --json d.json
cargo run --release -p simulation-engine --example dt_study       # time-step sensitivity
```

Docker (provided but **not verified**, because Docker was unavailable in the development
environment): `docker compose up --build`, then open http://localhost:8080.

## Tests

```bash
cargo test --workspace --release                       # 126 Rust tests
cargo clippy --workspace --all-targets -- -D warnings
npm --prefix apps/web test                             # 33 Vitest / RTL tests
npm --prefix apps/web run typecheck
node scripts/e2e.mjs                                   # frontend client ↔ live Rust API
```

## Model and algorithms in brief

* **Vehicle:** 12 m × 3 m helium blimp; two reversible main thrusters (surge plus
  differential yaw), one reversible heave thruster, tail rudder; CG 0.7 m below the CV
  (passive pitch/roll stability); no elevator and no lateral actuator.
* **Physics:** Archimedes buoyancy with local ISA density; gravity; component-wise
  quadratic drag relative to the air; fin and rudder forces; Lamb added masses;
  translation in 3 DOF; yaw dynamics; pitch/roll as damped pendulum modes; actuator lag
  and rate limits; momentum-theory power; spring-damper ground contact with friction;
  a weathervaning mooring mast; fixed-step RK4.
* **Environment:** flat terrain; mean wind with shear; scheduled wind changes; 1-cosine
  gusts; seeded Gauss–Markov turbulence; seeded sensor noise.
* **Autopilot:** an explicit 14-phase state machine (preflight → take-off preparation →
  take-off → initial climb → climb → en-route → approach → descent → flare → touchdown →
  post-landing → landed, plus go-around and aborted). Transitions are state-based and
  logged with reasons, and a safety monitor runs alongside.
* **Controllers:** A is a cascaded PID with fixed gains. B uses the same cascade with
  gains from an explicit pole-placement law scheduled on airspeed, vertical speed, yaw
  rate, flight-phase class and distance to the landing point.
* **Landing success:** target area reached, ground contact confirmed, touchdown within
  position/speed/area limits, and sequence completed without critical events.

## Documentation

| Document | Content |
|---|---|
| [docs/architecture.md](docs/architecture.md) | objectives, requirements, components, lifecycle, data contracts |
| [docs/mathematical-model.md](docs/mathematical-model.md) | frames, sign conventions, equations of motion, forces, integration accuracy |
| [docs/control-algorithms.md](docs/control-algorithms.md) | cascade, PID details, Controller A and B, allocation, guidance |
| [docs/autopilot-design.md](docs/autopilot-design.md) | state machine, take-off/landing logic, safety monitor, landing classification |
| [docs/experimental-methodology.md](docs/experimental-methodology.md) | experiments A–G, metric definitions, simulated results |
| [docs/testing.md](docs/testing.md) | test strategy and coverage |
| [docs/api.md](docs/api.md) | REST endpoints and error format |
| [docs/limitations.md](docs/limitations.md) | feature status (implemented / simplified / not implemented), limitations, future work |

## Project structure

```text
robotic-airship-autopilot/
├── apps/web/                    React + TypeScript workbench (Vite, Tailwind, shadcn-style UI)
│   └── src/
│       ├── components/          ui primitives, layout shell, charts, SVG flight views, forms
│       ├── features/            dashboard, airship-config, environment, flight-path,
│       │                        autopilot, simulation, research
│       ├── lib/                 Zod schemas, formatting, export helpers
│       ├── services/api.ts      typed API client
│       ├── stores/              Zustand stores (config, simulation, research, ui)
│       ├── types/api.ts         TypeScript mirror of the Rust contracts
│       ├── e2e/                 end-to-end test against the live API
│       └── test/                setup and engine-generated fixtures
├── crates/
│   ├── airship-model/           parameters, derived properties, atmosphere, math, validation
│   ├── flight-dynamics/         forces, equations of motion, RK4, actuators, ground, wind, sensors
│   ├── control-algorithms/      PID, gain schedule, cascade, allocation, controllers A/B
│   ├── autopilot/               guidance, flight phases, state machine, safety, preflight
│   ├── simulation-engine/       simulation loop, telemetry, metrics, landing, presets, compare, CLI
│   └── api-server/              Axum REST API, sessions, runner, static UI hosting
├── docs/                        thesis-oriented technical documentation
├── examples/experiments/        JSON configurations of experiments A–G
├── scripts/                     dev.mjs, e2e.mjs
├── Cargo.toml  package.json  Dockerfile  docker-compose.yml
```

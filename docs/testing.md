# Test strategy

| Level | Location | What is verified |
|---|---|---|
| Rust unit tests | `crates/*/src/**` (`#[cfg(test)]`) | physics, numerics, controllers, guidance, safety, validation, metrics |
| Rust closed-loop integration | `crates/simulation-engine/tests/closed_loop.rs` | complete missions through physics + sensors + autopilot + controllers |
| Rust API integration | `crates/api-server/tests/api.rs` | HTTP contract, status codes, lifecycle, exports, comparison |
| Frontend unit and component | `apps/web/src/**/*.test.ts(x)` (Vitest, React Testing Library) | Zod validation, utilities, exports, API client error handling, stores, pages |
| End-to-end | `apps/web/src/e2e/workflow.e2e.test.ts` via `node scripts/e2e.mjs` | the frontend API client against the live Rust server, full research workflow |

## Coverage of the required test topics

| Topic | Tests |
|---|---|
| Buoyancy and gravity | `derived::archimedes_buoyancy_and_weight`, `dynamics::buoyancy_and_gravity_at_rest`, `buoyancy_state_classification` |
| Drag direction and magnitude | `drag_opposes_relative_wind_and_scales_quadratically`, `lateral_drag_exceeds_axial_drag` |
| Translational dynamics | `vertical_terminal_velocity_matches_drag_balance` (analytic drag balance), `thrust_and_yaw_moment_signs` |
| Attitude integration | `pendulum_restores_pitch_and_roll`, `fins_provide_weathercock_stability`, `moored_vehicle_weathervanes_into_the_wind` |
| Numerical integration and dt dependence | `fourth_order_convergence`, `rk4_converges_with_timestep`, `integration_error_decreases_with_time_step` (closed loop) |
| Propulsion and actuator limits | `thrust_lag_reaches_63_percent_after_tau`, `commands_are_saturated`, `rudder_is_rate_limited`, `hover_power_matches_ideal_formula` |
| PID and anti-windup | `integral_removes_steady_state_error`, `anti_windup_limits_integrator_during_saturation`, `without_anti_windup_comparison_overshoot_is_smaller`, `derivative_acts_on_measurement_without_setpoint_kick`, `invalid_dt_holds_output_and_state`, `gain_change_is_bumpless` |
| Gain scheduling | `closed_loop_poles_match_design_at_operating_point`, `proportional_gain_decreases_with_aerodynamic_damping`, `landing_phase_has_higher_vertical_bandwidth`, `position_gain_scheduled_on_distance` |
| Control allocation | `yaw_has_priority_over_surge`, `yaw_cannot_add_large_forward_thrust_while_braking`, `rudder_takes_the_moment_at_speed` |
| Phase transitions | `reference_mission_lands_with_both_controllers` (exact phase sequence), `transitions_carry_reasons_and_timestamps`, `go_around_is_executed_when_approach_becomes_infeasible` |
| Take-off and landing classification | `landing::*` (proximity alone is not success; hard, fast, far or out-of-area touchdowns fail), `insufficient_heave_authority_fails_preflight`, `statically_light_vehicle_takes_off_and_is_held_down_after_landing` |
| Wind disturbances | `constant_wind_direction`, `wind_change_ramps_linearly`, `distance_triggered_gust_is_one_minus_cosine`, `shear_increases_with_height`, `turbulence_is_reproducible_and_has_expected_intensity` |
| Ground contact | `normal_force_is_unilateral_and_opposes_penetration`, `friction_opposes_sliding`, `heavy_vehicle_settles_on_gear_without_penetrating`, `hard_impact_is_detected_as_fatal` |
| Safety constraints | `airspeed_warning_escalates_and_clears`, `critical_event_in_approach_requests_go_around`, `minimum_altitude_only_enroute`, `low_battery_in_cruise_proceeds_to_landing` |
| Performance metrics | `rms_of_constant_error`, `every_metric_has_a_unique_definition`, metric assertions in the closed-loop tests |
| Deterministic execution | `identical_configuration_gives_bit_identical_results` (seeded turbulence and noise, irregular chunking), `steps_per_tick_preserve_fixed_dt` |
| Invalid configuration | `rejects_unphysical_geometry`, `physics_validation_rejects_infeasible_vtol`, `rejects_invalid_time_steps`, `invalid_configurations_are_rejected`, API 422/400 tests |
| Frontend: configuration validation | `lib.test.ts` (Zod schemas incl. cross-field rules) |
| Frontend: airship, environment and flight-path editing | `features.test.tsx` |
| Frontend: lifecycle controls, telemetry display, API failures | `features.test.tsx`, `simulationStore.test.ts`, `api.test.ts` |
| Frontend: comparison, CSV and JSON export | `features.test.tsx`, `lib.test.ts` |
| Frontend ↔ Rust | `workflow.e2e.test.ts` (configure → validate → reject invalid → run → land → results → CSV/JSON → reset and repeat → compare) |

## Commands

```bash
cargo test --workspace --release        # Rust unit, integration and API tests
cargo clippy --workspace --all-targets -- -D warnings
pnpm --dir apps/web test                # Vitest
pnpm --dir apps/web typecheck           # tsc --noEmit (strict)
node scripts/e2e.mjs                    # builds and starts the API, runs the e2e test
cargo bench -p control-algorithms -p simulation-engine   # Criterion benchmarks
```

# Experimental methodology, metrics and simulated results

All results in this document are **simulated data** from the simplified model described in
[mathematical-model.md](mathematical-model.md). They do not demonstrate real-world flight
performance.

## 1. Reproducibility protocol

* Every run is fully described by one `SimulationConfig` JSON (vehicle, environment,
  flight path, autopilot, controller, numerical settings). The JSON of each experiment run
  is in `examples/experiments/` and is reproduced by
  `cargo run --release -p simulation-engine --bin airship-sim -- export-presets examples/experiments`.
* Results record an FNV-1a hash of the configuration and the turbulence and noise seeds.
* Physics use a fixed time step independent of replay speed, chunking of execution and
  wall-clock time. Identical configurations give **bit-identical telemetry**
  (test `identical_configuration_gives_bit_identical_results`, which also steps the run
  in irregular chunks).
* Turbulence and sensor noise use independent seeded generators. Disturbances triggered
  by distance to the landing point occur at the same point of the manoeuvre for every
  controller.
* Controller comparisons run configurations that differ only in the `controller` section.
  The engine checks this and reports any other differing field
  (`identical_conditions`, `differences`).

## 2. Experiments

| ID | Title | Set-up | Focus metrics |
|---|---|---|---|
| A | Take-off in calm air | reference mission (moored start, 3 waypoints, approach from the East) | time to target altitude, take-off duration, altitude RMS, vz RMS, max heave effort |
| B | Take-off under horizontal wind | reference mission, 5 m/s northerly at 10 m, shear α = 0.14 | time to target altitude, cross-track RMS, altitude RMS, max roll, saturation |
| C | Altitude tracking | airborne start at 40 m; legs with altitude steps 40–60–35–45 m at 5 m/s | altitude RMS/max, vz RMS, heave effort, energy |
| D | Approach and landing in calm air | airborne start 450 m out on the extended final course, 30 m off the centre line | landing-point error, touchdown speeds, approach duration, cross-track RMS |
| E | Landing under changing wind | as D; within 180 m of the landing point the wind ramps over 20 s from 1.5 m/s (270°) to 5 m/s (200°); light turbulence (seed 7) | landing-point error, touchdown speed, cross-track max, go-arounds, saturation |
| F | Controller comparison | reference mission in scenario 4 (3 m/s wind, turbulence σ = 0.7 m/s seed 42, two discrete gusts) with A and B | all tracking, landing, effort and energy metrics |
| G | Go-around | as D; a 9 m/s 1-cosine crosswind gust (40 s) starts 110 m before the landing point; landing wind limit 7 m/s | go-arounds, landing attempts, landing-point error, mission duration |

Environment scenarios 1–5 (calm, constant wind, wind change during approach, gusts,
limited ground distance) are available as presets in the UI and API.

## 3. Metric definitions

Notation: telemetry samples `k` at spacing `Δt` (telemetry period). For an error `e` over
a sample window `W`:
`RMS(e) = sqrt(Σ_W e_k² Δt / Σ_W Δt)`, `MAX(e) = max_W |e_k|`, `INT(x) = Σ_k x_k Δt`.
"Airborne" samples are those in airborne phases without ground contact.

| Metric | Unit | Definition |
|---|---|---|
| Altitude tracking RMS / max error | m | RMS / MAX of `h_ref − h` over airborne samples with an active altitude reference (rate-limited reference) |
| Position (cross-track) RMS / max | m | RMS / MAX of the signed cross-track error to the active leg or final line (climb, cruise, approach line, descent) |
| Horizontal velocity RMS error | m/s | RMS of `|v_ref − v_ground|` over airborne samples |
| Vertical velocity RMS error | m/s | RMS of `vz_ref − vz` over airborne samples |
| Heading RMS error | deg | RMS of `wrap(ψ_ref − ψ)` over airborne samples |
| Max pitch / roll | deg | MAX `|θ|`, `|φ|` over airborne samples |
| Time to target altitude | s | first time `|h − h_target| ≤ tol` after mooring release, minus release time |
| Take-off duration | s | `t(Climb entry) − t(Take-off entry)` |
| Approach duration | s | `t(touchdown) − t(last Approach entry)` |
| Time to touchdown | s | time of first contact of the assessed touchdown |
| Landing-point error | m | horizontal distance between the touchdown point and the target |
| Touchdown horizontal / vertical speed | m/s | `|v_ground|` and `−vz` at contact onset |
| Max surge / heave / yaw effort | – | MAX of the realised virtual command normalised by its limit |
| Control effort index | s | `INT(u_surge² + u_heave² + u_yaw²)` with normalised commands |
| Control saturation duration (total, heave, yaw) | s | `INT(1[channel saturated])` |
| Electrical energy | Wh | `INT(P)`, momentum-theory thruster power plus avionics |
| Propulsive impulse | N s | `INT(|T_L| + |T_R| + |T_h|)` |
| Final battery state of charge | – | `1 − E_used / E_capacity` |
| Landing attempts, go-arounds, bounces | count | Approach entries, GoAround transitions, contact losses after touchdown |
| Warning / critical / fatal events | count | safety events by severity |
| Numerical execution time, real-time factor | ms, – | wall-clock stepping time; simulated / wall-clock time |

The landing classification (four criteria) is defined in
[autopilot-design.md](autopilot-design.md#5-landing-classification).

## 4. Simulated results (default parameters)

Generated by running every experiment with both controllers from the exported JSON
configurations
(`airship-sim run examples/experiments/experiment-<id>-pid.json --controller <kind>`),
with the default dt = 0.01 s and 20 Hz control. "—" marks a metric that does not apply,
such as take-off timing for airborne starts.

| Exp. | Ctrl | Landing | LP error [m] | TD vz [m/s] | TD vh [m/s] | Alt. RMS [m] | XT RMS [m] | t to alt. [s] | Duration [s] | Saturation [s] | Energy [Wh] | Go-arounds | Effort idx [s] |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| A | A | successful | 1.48 | 0.28 | 0.11 | 0.16 | 6.42 | 38.5 | 365.7 | 53.0 | 40.8 | 0 | 33.8 |
| A | B | successful | 0.86 | 0.30 | 0.06 | 0.15 | 6.44 | 38.7 | 366.1 | 59.4 | 42.1 | 0 | 35.5 |
| B | A | successful | 1.16 | 0.28 | 0.36 | 0.20 | 5.37 | 38.3 | 373.1 | 24.8 | 45.5 | 0 | 40.3 |
| B | B | successful | 1.47 | 0.30 | 0.36 | 0.16 | 5.40 | 38.7 | 372.6 | 25.7 | 45.9 | 0 | 41.6 |
| C | A | successful | 1.43 | 0.28 | 0.11 | 0.29 | 3.94 | — | 384.8 | 33.7 | 34.4 | 0 | 30.3 |
| C | B | successful | 0.82 | 0.30 | 0.06 | 0.32 | 3.94 | — | 385.0 | 38.9 | 35.7 | 0 | 32.0 |
| D | A | successful | 1.41 | 0.29 | 0.13 | 0.07 | 3.83 | — | 170.2 | 15.2 | 10.7 | 0 | 6.7 |
| D | B | successful | 2.12 | 0.28 | 0.23 | 0.05 | 3.81 | — | 170.0 | 20.2 | 11.6 | 0 | 9.0 |
| E | A | successful | 2.12 | 0.16 | 0.58 | 0.16 | 5.05 | — | 175.3 | 1.0 | 15.2 | 0 | 14.8 |
| E | B | successful | 2.52 | 0.28 | 0.50 | 0.10 | 4.85 | — | 173.4 | 3.5 | 16.3 | 0 | 18.8 |
| F | A | successful | 3.98 | 0.24 | 0.34 | 0.19 | 5.44 | 38.4 | 362.3 | 89.5 | 60.8 | 0 | 85.1 |
| F | B | successful | 1.70 | 0.09 | 0.89 | 0.17 | 5.30 | 38.9 | 368.3 | 100.9 | 63.8 | 0 | 95.3 |
| G | A | successful | 1.77 | 0.28 | 0.14 | 0.37 | 5.48 | — | 344.6 | 61.6 | 35.1 | 1 | 28.3 |
| G | B | successful | 2.50 | 0.30 | 0.25 | 0.47 | 5.45 | — | 345.3 | 66.7 | 35.9 | 1 | 32.3 |

LP = landing point, TD = touchdown, XT = cross-track.

### Observations (simulated, single deterministic runs)

* All 14 runs complete the landing sequence successfully under the four criteria.
* In Experiment G both controllers go around once during descent (t ≈ 109 s), and the
  second approach lands. The triggering constraint differs. With A, the cross-track error
  reached the 12 m limit first. With B, which held the line slightly better, the
  estimated wind reached the 7 m/s landing limit first ("approach infeasible").
  Both are logged with their measured values.
* The gain-scheduled controller (B) gives **lower altitude RMS error** in 5 of 7
  experiments. It uses **more control effort, longer saturation and slightly more
  energy** in every experiment, consistent with its higher landing-phase bandwidths.
* Landing-point error does not favour either controller consistently: B is closer in A,
  C and F, and A is closer in B, D, E and G. Differences are around 1 m against a 5 m
  tolerance.
* In F (gusts), B touched down with a horizontal speed of 0.89 m/s, near the 1.0 m/s
  limit. The flare's touchdown-speed gate kept it inside the limit.
* Cross-track RMS is dominated by waypoint turns (LOS overshoot at the turn radius) and is
  nearly identical for both controllers, because guidance, not the inner loops, limits it.

These are single deterministic runs per configuration. Statistical conclusions require
Monte-Carlo runs over turbulence seeds and parameter variations, for example a script
around `airship-sim run` that varies `environment.wind.turbulence.seed`. Such a campaign
has not been performed here. See [limitations.md](limitations.md).

## 5. How to reproduce

```bash
cargo build --release
cargo run --release -p simulation-engine --bin airship-sim -- experiment F --out-dir results/
# or: UI → Experiments & comparison → Run, then export "Metrics CSV" / "Comparison JSON"
```

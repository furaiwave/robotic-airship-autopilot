# Limitations, feature status and future work

## 1. Scope statement

This software is a simulation and research tool for a bachelor's thesis. It is **not
certified**, has **not been validated against flight data**, and must not be used to
control a real aircraft. Its safety checks are part of the simulation study and do not
replace certified flight software, independent flight-termination systems or hardware
safety mechanisms.

## 2. Feature status

| Feature | Status | Notes |
|---|---|---|
| Buoyancy (Archimedes, local ISA density), gravity | Implemented | constant-volume hull; no superheat, ballonet or pressure-height effects |
| Direction-dependent quadratic drag relative to air | Implemented | coefficients are estimates |
| Added mass (Lamb coefficients) | Simplified | quasi-static diagonal; Coriolis coupling and Munk moment neglected |
| Translational dynamics (3 DOF) | Implemented | world frame, RK4 |
| Yaw dynamics (fins, rudder, differential thrust, hull damping) | Implemented | fin model is linear lift plus flat-plate cross-flow drag |
| Pitch and roll | Simplified | decoupled small-angle pendulum modes; no elevator; Euler-angle kinematics without cross-coupling |
| Full 6-DOF rigid-body dynamics with quaternions | Not implemented | |
| Propulsion lag, saturation, reverse thrust; rudder rate limit | Implemented | thrust independent of inflow speed |
| Power and battery estimate | Simplified | ideal momentum theory with constant efficiency |
| Ground contact (spring-damper, friction), mooring mast with weathervaning | Implemented | flat terrain only; single contact point |
| Wind: mean, shear, scheduled changes, 1-cosine gusts, Gauss–Markov turbulence | Implemented (test harness) | not a validated meteorological model; no Dryden/von Kármán spectra; uniform over the hull |
| Sensor noise (white Gaussian, seeded) | Simplified | no bias, drift, latency or state estimator; attitude and yaw rate exact |
| Cascaded PID (Controller A) | Implemented | |
| Gain-scheduled PID (Controller B) | Implemented | scheduling on airspeed, vertical speed, yaw rate, phase class, distance; not adaptive |
| Model predictive control (Controller C) | Not implemented | |
| Explicit flight-phase state machine with logged reasons | Implemented | 14 phases |
| Go-around, bounce handling, preflight checks, safety monitor | Implemented | |
| Precise lateral station keeping | Simplified | no lateral actuator; calm-air heading re-pointing logic with hysteresis |
| REST API, live telemetry by incremental polling | Implemented | WebSocket streaming not implemented (polling at 4 Hz is sufficient) |
| OpenAPI | Simplified | hand-written path summary at `/api/openapi.json`; no generated schemas |
| Monte-Carlo campaigns | Not implemented | single deterministic runs; scriptable via the CLI |
| Docker image | Provided, unverified | Docker was not available in the development environment |

## 3. Known modelling limitations and their effect

* **Pitch/roll decoupling.** Attitude excursions stay moderate for the default geometry:
  maximum pitch 2–9.5° and maximum roll 0.9–11.3° across experiments A–G, the largest
  in the wind and gust cases B and F. At these angles the small-angle decoupling is a
  reasonable approximation. Larger thrust offsets, lower BG or stronger gusts would require
  coupled rigid-body dynamics.
* **No Munk moment.** Real hulls are directionally unstable without fins. The model's
  weathercock stability therefore comes only from the fins and is optimistic.
* **Actuator zero-order hold.** The closed-loop simulation converges at first order in dt
  (see [mathematical-model.md](mathematical-model.md#12-numerical-integration)), with errors
  below 1 mm at dt = 0.01 s.
* **Wind uniform over the hull.** Gust penetration and shear across the hull length are
  not modelled.
* **Nominal-model feed-forward.** The controllers know the nominal mass, volume and drag
  coefficients. `trim_mass_bias_kg` allows simple robustness studies, but no parameter
  uncertainty campaign has been performed.
* **Under-actuation.** Without a lateral thruster, lateral position errors near the
  landing point are corrected by heading changes. In calm air this takes noticeable time
  (flare durations of 10–40 s).
* **Inertia estimates** assume a uniformly filled ellipsoid.

## 4. Future research directions

1. 6-DOF dynamics with quaternions, Munk moment and coupled added mass; parameter
   identification from wind-tunnel or flight data.
2. A state estimator (EKF) with realistic sensor models (GNSS, barometer, IMU,
   magnetometer, air data), replacing direct noisy measurements.
3. A model-predictive controller for the final approach and flare with explicit
   touchdown-speed and position constraints, solved by QP at each step.
4. Monte-Carlo evaluation over turbulence seeds, wind conditions and parameter
   uncertainty, with confidence intervals for the controller comparison.
5. A lateral thruster or vectored thrust, and the corresponding control allocation.
6. Dryden or von Kármán turbulence, terrain and obstacle models, and moving mooring
   masts.
7. Hardware-in-the-loop: the same autopilot crate running against a real flight
   controller interface.

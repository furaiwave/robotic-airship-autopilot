# Autopilot state machine, take-off and landing logic, safety monitoring

Implemented in `crates/autopilot/src/state_machine.rs`, `safety.rs`, `preflight.rs`.
The autopilot only sees **measurements** (`flight_dynamics::Measurement`), never the
true simulated state.

## 1. Control step

1. Low-pass filter the wind estimate.
2. Run the safety monitor; apply requested responses (go-around, proceed to landing).
3. Evaluate the exit conditions of the current phase; at most one transition per step.
4. Generate references for the (possibly new) phase, shape the altitude reference and
   run the selected controller.

Each transition is logged as `(time, from, to, reason, state snapshot)`. The reason
quotes the measured values that satisfied the condition, for example
*"mooring load 6.1 N ≥ release threshold 6.0 N with heave thrust 21.8 N; wind 0.0 m/s
≤ 8.0 m/s, nose 0° off the wind; mooring released"*.

**Timers never cause nominal progress.** Timeouts only lead to failure handling
(abort), and dwell times appear only as additional conditions (settled for 3 s).

## 2. Phases

| Phase | Entry | References / active loops | Nominal exit | Failure / abort |
|---|---|---|---|---|
| Preflight | start | none | all checks pass → Take-off prep. (moored), En-route or Approach (airborne start) | any failed check → Aborted |
| Take-off preparation | moored | open-loop heave ramp to estimated heaviness + margin, then a slow increase until the **measured mooring load** confirms lift; with wind, yaw into the wind by differential thrust | load ≥ release threshold, wind ≤ limit, hull ≤ 20° off the wind → release mooring | timeout → Aborted, reporting the failing condition(s) |
| Take-off | mooring released | vz = take-off climb rate; position hold over the site; nose into wind | gear height ≥ clearance and vz > 0.1 m/s | timeout below clearance → Aborted |
| Initial climb | | same | gear height ≥ initial-climb height | — |
| Climb | | altitude → target (rate-limited); LOS path or position hold; airspeed | \|h − h_target\| ≤ tol and \|vz\| ≤ tol | unplanned ground contact → Touchdown |
| En-route | | altitude and LOS along the waypoints | all waypoints captured | battery critical → Approach |
| Approach | | approach altitude; direct to the join point, then the final-approach line | at the gate: cross-track, altitude, track error, ground speed and wind all within limits | passing the gate unestablished → Go-around (failed conditions listed) |
| Descent | | glide-path altitude with vz feed-forward; line tracking; decelerating speed | gear ≤ flare height + 0.75 m, within the flare radius, ground speed ≤ limit | above the decision height: cross-track, glide-path error, wind > landing limit, overshoot → Go-around; contact → Touchdown |
| Flare | | position hold over the point; vertical descent at the touchdown sink rate once centred (< 40 % of tolerance, hysteresis); descent paused while ground speed > 60 % of the touchdown limit | **weight on gear** from the ground-contact model | above DH: drift > 2 × tolerance or wind > limit → Go-around |
| Touchdown | contact | open-loop hold-down thrust `trim − F_hold` | on ground, slow, \|vz\| ≤ 0.2 m/s | contact lost above 0.5 m → Flare (bounce) |
| Post-landing | | hold-down thrust | settled (ground speed ≤ 0.2 m/s, \|vz\| ≤ 0.1 m/s) for 3 s → mooring engaged → **Landed** | lift-off → Flare |
| Go-around | | climb to approach altitude straight ahead along the course | approach altitude reached → Approach (next attempt) | attempts exhausted → Aborted |
| Landed / Aborted | | terminal (Aborted holds position and altitude if airborne) | — | — |

Touchdown is entered **only** after the simulated ground-contact model confirms contact
(normal force > 2 N), never on proximity to the target.

## 3. Preflight checks

sensors finite · vehicle physics (buoyancy ratio, gas mass, thrust authority) · take-off
lift margin (heave capacity × authority fraction ≥ heaviness + margin) · landing descent
authority (down-thrust > static lift) · wind ≤ take-off/landing limit · battery ≥ minimum
· flight path free of validation errors.

## 4. Safety monitor

Rising-edge events with levels *warning* (limit exceeded) and *critical* (limit ×
critical factor, default 1.3). Each event records time, phase, value, limit, response
and a state snapshot, and is closed with a `cleared_at` time when the condition ends.

| Constraint | Source | Response |
|---|---|---|
| airspeed, vertical speed (airborne) | vehicle limits | log; critical in approach/descent/flare above DH → go-around |
| pitch, roll (not on ground) | vehicle limits | as above |
| minimum en-route altitude (climb, cruise, approach) | safety config | critical |
| continuous actuator saturation | safety config (8 s) | warning: requested manoeuvre exceeds control authority |
| battery state of charge | safety config | critical in climb/cruise → proceed to landing |
| approach feasibility | state machine | warning logged with each go-around |
| touchdown speed, landing zone | engine, at contact onset | critical |
| unplanned ground contact | state machine | critical |
| ground collision, numerical validity | engine (truth state) | **fatal**, run terminated |

Warnings are recoverable; fatal events end the simulation and mark the outcome. These
software checks belong to a simulation study and do not replace certified flight
software, independent flight-termination systems or hardware safety mechanisms.

## 5. Landing classification

A landing is **successful only if all four criteria hold**:

1. *Target area reached:* the gear was within the landing-point tolerance below flare
   height + 1 m.
2. *Ground contact:* the contact model registered contact during the final landing
   sequence.
3. *Touchdown within limits:* at the first contact of the final attempt, the distance,
   the sink rate and the horizontal speed are within their limits and the point lies
   inside the prepared landing area.
4. *Sequence completed safely:* the state machine reached Landed (settled and moored),
   and no critical or fatal event occurred after the final Approach entry.

Otherwise the landing is *unsuccessful*, with each failed criterion explained, or *not
attempted*.

# Guidance and control algorithms

Implemented in `crates/control-algorithms` (loops, schedule, allocation) and
`crates/autopilot/src/guidance.rs` (path and approach guidance).

## 1. Architecture

```
            references from the state machine / guidance
                              │
  ┌───────────────────────────┴───────────────────────────────┐
  │ Vertical:  h_ref ─[altitude PI]→ vz_ref (+ff, limited) ─[vz PI]→ F_z + trim + drag ff
  │ Horizontal: p_ref ─[position P]→ v_ground,ref (+guidance ff, limited)
  │            wind triangle: v_air,ref = v_ground,ref − ŵ
  │            → ψ_ref = atan2(v_air,ref),  u_ref = |v_air,ref| · gate(ψ_ref − ψ)
  │ Heading:   ψ_ref ─[heading P]→ r_ref (≤ r_max) ─[yaw-rate PI]→ N
  │ Speed:     u_ref ─[airspeed PI]→ F_x + axial drag ff
  └──────────────────────────┬────────────────────────────────┘
                virtual commands (F_x, F_z, N)
                             │ control allocation
              T_L, T_R (main), T_h (heave), δ (rudder)
```

**Altitude vs vertical-speed control.** The altitude loop is an *outer* loop whose
output is the vertical-speed set-point of the *inner* loop. When no altitude reference is
active (take-off, flare) the inner loop tracks a vertical-speed reference directly.
Altitude references are rate-limited to 80 % of the vehicle's climb/descent limits, so
step changes become feasible trajectories; while the limiter is active its rate is the
vertical-speed feed-forward.

**Feed-forward terms** (shared by both controllers): estimated static heaviness
`(m + Δm) g − ρ V g` from the nominal mass (with an optional bias for robustness
studies) and measured density; quadratic drag `½ρ C_D A x|x|` of the reference
vertical speed and airspeed.

**Wind triangle.** The airship has no lateral actuator. Lateral position is controlled by
pointing the nose along the required *air-relative* velocity; in wind the vehicle crabs.
Hysteresis (engage above 0.5 m/s, release below 0.25 m/s) avoids heading chatter.
In calm-air station keeping, along-axis errors are corrected with reversible thrust while
the heading is held; the nose is re-pointed at the hold position only when the lateral
offset exceeds 1.5 m (released below 0.5 m). Forward speed is gated by the heading error
("turn, then go": zero beyond 45° in station keeping, `cos` otherwise). Without this
gating, pure pursuit of a nearby point with a turn-rate limit ends in an orbit, which was
observed and fixed during development.

## 2. PID element (both controllers)

$$u = \mathrm{sat}\big(K_p e + I + D + u_{ff}\big),\quad I_k = \mathrm{clamp}(I_{k-1} + K_i e\,\Delta t),\quad D = -K_d\,\mathrm{LPF}_\tau(\dot y)$$

* **Anti-windup:** conditional integration. The integral is not updated when the
  candidate output exceeds a limit and the error would push it further. The integral
  itself is bounded.
* **Derivative on measurement** (no set-point kick), first-order filtered.
* **Saturation and rate limiting** of the output.
* **Bumpless gain changes:** the integrator stores the integral *term* (`Σ K_i e Δt`),
  so a gain change does not rescale it. On release of an open-loop heave override
  (moored, on ground) the integrator is preset.
* **Invalid inputs:** non-positive or non-finite `dt` or non-finite errors leave the state
  untouched and hold the previous output.

## 3. Controller A: cascaded PID with fixed gains

Gains are user-configurable and constant for all phases and speeds. The defaults come
from the pole-placement formulas of Controller B evaluated **once** at a nominal cruise
operating point (airspeed 4 m/s, |w| = 0.5 m/s, cruise bandwidths). A unit test
(`default_fixed_gains_match_schedule_at_design_point`) documents the derivation.

| Loop | Kp | Ki |
|---|---|---|
| altitude → vz_ref | 0.25 1/s | 0.006 1/s² |
| vz → F_z | 125 N s/m | 46 N/m |
| heading → r_ref | 0.4 1/s | 0 |
| yaw rate → N | 1540 N m s | 1230 N m |
| airspeed → F_x | 79 N s/m | 27 N/m |
| position → v_ground | 0.2 1/s | — |

## 4. Controller B: gain-scheduled PID

Same loop structure, feed-forward, wind triangle and allocation. Only the gains differ:
they are recomputed every control step from an explicit scheduling law.

Each inner loop acts on a first-order plant `M ẋ = u − d(σ) x`, where `M` is the effective
mass or inertia including added mass and `d(σ)` is the **linearised aerodynamic damping
at the current operating point** σ. Matching `s² + (d + K_p)/M s + K_i/M` to
`s² + 2ζω s + ω²` gives

$$K_p(\sigma) = \max\big(2\zeta\omega M - d(\sigma),\; f_{min}\,2\zeta\omega M\big),\qquad K_i = \omega^2 M$$

| Loop | M | d(σ) | scheduling variable |
|---|---|---|---|
| vertical speed | m + k₂ρV | ρ C_Dz A_z \|w\| | measured vertical speed |
| yaw rate | I_z + k′ρV(L²+D²)/20 | ½ρ S C_Lα l_f² V_air + 2 c_h \|r\| | airspeed, yaw rate |
| airspeed | m + k₁ρV | ρ C_Dx A_x \|u\| | axial airspeed |

* The bandwidth ω of each loop is looked up per **flight-phase class** (ground, take-off,
  climb, cruise, approach, landing). Defaults give higher vertical bandwidth during landing
  (1.2 rad/s versus 0.6 rad/s in cruise).
* Outer loops: `K_h = ω_v / n`, `K_ψ = ω_r / n` (n = 3); altitude integral
  `0.1 (ω_v/n)²`.
* Position gain scheduled on the distance `d` to the landing point:
  `K_pos(d) = K_far + (K_near − K_far) clamp(1 − d/d_s, 0, 1)` (0.12 → 0.35 1/s within
  40 m).

Every input to the law is measured (airspeed, vertical speed, yaw rate, distance) or a
nominal model constant. This is **gain scheduling**, not adaptive control: nothing is
estimated or learned online. Unit tests check the closed-loop pole placement at an
operating point, the monotonic decrease of `K_p` with damping, the phase-dependent
bandwidth and the distance schedule.

## 5. Control allocation (shared)

1. Heave: `T_h = sat(F_z)`.
2. Rudder first: moment per radian `N_δ = ½ρ u|u| S C_Lδ l_f` from the measured axial
   airspeed; `δ = sat(N / N_δ)` if the rudder has meaningful authority.
3. Differential thrust supplies the remainder: `T_R − T_L = (N − N_δ δ)/b`.
   Yaw has priority over surge, but only in one direction. The common-mode part may be
   *reduced* freely, while it may be *increased* by at most 10 % of the motor maximum.
   Because reverse thrust is weaker than forward thrust, unrestricted yaw priority added
   net forward thrust during braking turns. During development this made the vehicle
   accelerate to 7 m/s in a turn and miss the final approach by 76 m.

A channel is reported **saturated** when the allocation cannot realise the virtual
command or when the loop output is clamped (magnitude or rate).

## 6. Guidance

**Line-of-sight (LOS) path following** on the leg `a → b` with tangent `t`: aim at
`a + t·min(s + Δ, L)`, with `s` the along-track position. The look-ahead is
`Δ = max(Δ₀, 1.5 V / r_max)` so that the demanded turn is flyable. The altitude
reference is interpolated linearly along the leg (constant flight-path angle), with a
vertical-speed feed-forward `gradient × speed`. Waypoints are captured inside the
acceptance radius or when passed abeam. Hover waypoints (speed 0) require position,
altitude and optional heading capture.

**Final approach.** A line along the approach course `u` ending at the landing point.
Glide path `h(d) = h_flare + d·tan γ` (capped at the approach altitude), gate at
`d_gate = (h_app − h_flare)/tan γ`, pre-gate a further 80 m out and join point 40 m
beyond that. During descent the speed decreases linearly over the last 60 m (minimum
0.5 m/s). The LOS look-ahead shrinks with the distance to go
(`min(Δ₀, max(5, 0.3 d))`), so the cross-track error converges before the vehicle
reaches the point.

## 7. Optional MPC

Not implemented. A model-predictive controller is listed as future work in
[limitations.md](limitations.md).

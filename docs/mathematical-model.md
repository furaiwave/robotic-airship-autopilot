# Mathematical model

This document states the physical model implemented in `crates/airship-model` and
`crates/flight-dynamics`. Each statement is classified as **assumption** (modelling
choice), **implemented** (equation as coded) or **estimate** (parameter that would
need identification on a real vehicle). No part of the model has been validated
against measured flight data.

## 1. Vehicle configuration

A small non-rigid research airship ("robotic blimp"):

| Element | Description | Default |
|---|---|---|
| Envelope | prolate ellipsoid, length `L`, max. diameter `D`, helium | 12 m × 3 m, V = 56.5 m³ |
| Gondola | below the envelope; CG lies `BG` below the centre of volume (CV) | BG = 0.7 m |
| Main thrusters | two reversible propellers on outriggers at lateral arm `b`, thrust line `z_T` below the CG | 2 × 60 N fwd / 30 N rev, b = 1.2 m |
| Heave thruster | one reversible vertical propeller through the CG | 100 N up / 100 N down |
| Rudder | on the vertical tail fins, authority ∝ dynamic pressure | ±25°, 60 °/s |
| Landing gear | skid, contact point `h_g` below the CV | 2.1 m |
| Mass | total, including gas, structure, payload, battery | 70 kg (≈ 11 N heavy at 120 m MSL) |

There is **no elevator**; pitch and roll are passively stabilised by the pendulum
arrangement (CG below CV). Vertical motion is commanded with the heave thruster,
horizontal motion with the main thrusters, heading with differential thrust and rudder.
There is **no lateral actuator**, so the vehicle is under-actuated horizontally.

## 2. Coordinate frames and sign conventions

* **World frame (ENU):** `x` East, `y` North, `z` **up**. `z` is the altitude above mean
  sea level of the CV. "Altitude" in the user interface is the height of the gear
  contact point above the (flat) ground: `h = z − h_g − z_ground`.
* **Body frame (FLU):** `x` forward, `y` left, `z` up.
* **Attitude:** yaw `ψ` counter-clockwise from East, pitch `θ` positive nose-up, roll `φ`
  positive right side down. Rotation body→world:
  $$R = R_z(\psi)\,R_y(-\theta)\,R_x(\phi)$$
* **Compass quantities** (headings, wind direction) are clockwise from North:
  `heading = 90° − ψ`. Wind direction is meteorological ("blowing from").

## 3. Atmosphere (implemented)

Either constant density, or the ISA troposphere with an optional temperature offset `ΔT`:

$$T(h) = T_0 + \Delta T - L h,\qquad p(h) = p_0\left(1 - \frac{L h}{T_0}\right)^{gM/(RL)},\qquad \rho(h) = \frac{p(h)}{R_{air}\,T(h)}$$

with `T0 = 288.15 K`, `p0 = 101 325 Pa`, `L = 0.0065 K/m`, `R_air = 287.05 J/(kg K)`.
Buoyancy uses the **local** density at the vehicle altitude.

## 4. Buoyancy and gravity (implemented)

Archimedes' principle with the displaced volume `V = V_hull + V_gondola`:

$$F_b = \rho_{air}\,V\,g, \qquad F_g = m\,g, \qquad F_{vertical,static} = F_b - F_g$$

`V_hull` is either user input or the ellipsoid estimate `π L D² / 6`. The static
state is classified as *light* (`B/W > 1.005`), *neutral* or *heavy* (`B/W < 0.995`).
Validation rejects `B/W` outside 0.6–1.4, masses below the contained gas mass
(`ρ_gas/ρ_air · ρ V`), heave thrust that cannot overcome the heaviness (no vertical
take-off possible) and down-thrust that cannot overcome the static lift (no controlled
landing possible). **Buoyancy alone never causes take-off in the model**: a moored
vehicle stays attached until the autopilot releases the mooring, and lift-off requires
`F_b − F_g + T_heave > 0`.

## 5. Translational dynamics (implemented)

Total force in the world frame:

$$\mathbf F = \begin{pmatrix}0\\0\\F_b\end{pmatrix} + \begin{pmatrix}0\\0\\-mg\end{pmatrix} + R\,(\mathbf F_{aero,b} + \mathbf F_{thrust,b}) + \mathbf F_{ground}$$

Added (virtual) mass is applied as an anisotropic quasi-static inertia in body axes:

$$\mathbf a_b = \left(\frac{F_{b,x}}{m + k_1\rho V},\ \frac{F_{b,y}}{m + k_2\rho V},\ \frac{F_{b,z}}{m + k_2\rho V}\right),\quad \mathbf F_b = R^T\mathbf F,\qquad \dot{\mathbf v} = R\,\mathbf a_b,\quad \dot{\mathbf p} = \mathbf v$$

`k1, k2, k'` are Lamb's inertia coefficients of a prolate spheroid of eccentricity
`e = sqrt(1 − (D/L)²)`:

$$\alpha_0 = \frac{2(1-e^2)}{e^3}\left(\tfrac12\ln\frac{1+e}{1-e} - e\right),\quad \beta_0 = \frac1{e^2} - \frac{1-e^2}{2e^3}\ln\frac{1+e}{1-e}$$
$$k_1 = \frac{\alpha_0}{2-\alpha_0},\quad k_2 = \frac{\beta_0}{2-\beta_0},\quad k' = \frac{e^4(\beta_0-\alpha_0)}{(2-e^2)\,[2e^2-(2-e^2)(\beta_0-\alpha_0)]}$$

(For `L/D = 4`: `k1 = 0.082`, `k2 = 0.860`, `k' = 0.608`, verified by a unit test.)
**Simplification:** Coriolis-type added-mass coupling and the Munk moment are neglected.

## 6. Aerodynamic forces (implemented, coefficients are estimates)

All aerodynamic forces use the velocity **relative to the air**:
`v_r = R^T (v − w) = (u, v, w)`, `V = |v_r|`.

Hull drag with direction-dependent coefficients and reference areas
(`A_x = πD²/4`, `A_y = A_z = πLD/4`):

$$\mathbf D = -\tfrac12\rho\,\big(C_{D,x}A_x|u|u,\ C_{D,y}A_y|v|v,\ C_{D,z}A_z|w|w\big)$$

This is the component-wise form of `F_drag = −½ ρ C_D A |v_rel| v_rel`.

Vertical tail fins (area `S`, lift slope `C_Lα`, arm `l_f` behind the CG). The local
lateral velocity at the fin includes the yaw rate, `v_f = v − r l_f`; at small incidence
the fin produces lift, at large incidence (e.g. rotation at hover) flat-plate drag:

$$Y_f = -\tfrac12\rho S\,(C_{L\alpha}|u|\,v_f + C_{D,plate}|v_f|\,v_f), \qquad C_{D,plate} = 1.2$$

Rudder (deflection `δ`, effectiveness `C_Lδ`), dynamic pressure from the axial airspeed:

$$Y_r = -\tfrac12\rho\,u|u|\,S\,C_{L\delta}\,\delta$$

Rudder authority therefore vanishes at zero airspeed; differential thrust is the only
yaw effector at hover.

## 7. Rotational dynamics (implemented, simplified)

Yaw has full dynamics; pitch and roll are treated as decoupled small-angle pendulum
modes. Kinematics: `ψ̇ = r`, `θ̇ = q`, `φ̇ = p`.

$$I_z'\,\dot r = \underbrace{-l_f\,Y_f}_{\text{fins}} \underbrace{- l_f\,Y_r}_{\text{rudder}} + \underbrace{b\,(T_R - T_L)}_{\text{diff. thrust}} - c_h\,r|r| + N_{ground}, \qquad c_h = \tfrac12\rho C_{D,y} D\,\frac{L^4}{32}$$

$$I_y'\,\dot q = z_T (T_L + T_R) - BG\,X_{aero} - F_b\,BG\,\sin\theta - c_\theta q + M_{ground}$$

$$I_x\,\dot p = -BG\,Y_{aero} - F_b\,BG\,\sin\phi - c_\phi p + L_{ground}$$

The aerodynamic forces act at the CV (BG above the CG). Inertia estimates (uniform
ellipsoid): `I_x = mD²/10`, `I_y = I_z = m(L² + D²)/20`, plus added inertia
`k' ρV (L² + D²)/20` for pitch/yaw. Pitch/roll damping `c = 2ζ sqrt(F_b BG · I)` with a
configurable damping ratio ζ (estimate). The hull-rotation damping `c_h` follows from
integrating cross-flow drag `½ρ C_Dy D |r x| r x` over the hull length.

## 8. Propulsion and actuators (implemented)

* Thrust: saturated first-order lag `Ṫ = (sat(T_cmd) − T)/τ`, discretised exactly per
  step: `T ← T + (1 − e^{−dt/τ})(sat(T_cmd) − T)`.
* Rudder: position limit and slew-rate limit.
* Electrical power per thruster from actuator-disk momentum theory with axial inflow `V`:
  $$v_i = -\tfrac V2 + \sqrt{\tfrac{V^2}4 + \tfrac{|T|}{2\rho A}},\qquad P = \frac{|T|\,(V + v_i)}{\eta}$$
  plus a constant avionics power; battery state of charge `SoC = 1 − E_used/E_cap`.
  When the battery is empty all thrust commands are forced to zero.

## 9. Ground contact and mooring (implemented)

Gear compression `δ = z_ground − (z − h_g)`; unilateral spring-damper normal force,
regularised Coulomb friction, rotational spring-damper on pitch/roll (gear footprint)
and a friction moment in yaw:

$$N = \max(0, k\delta - c\,v_z),\qquad \mathbf F_f = -\mu N \tanh\!\left(\frac{|\mathbf v_h|}{v_s}\right)\frac{\mathbf v_h}{|\mathbf v_h|}$$

`k = m_z ω²` with `ω = 4 rad/s`, `c = 2·0.8·sqrt(k m_z)`, `μ = 0.5`, `v_s = 0.05 m/s`.

**Mooring mast:** while moored, translation, pitch and roll are restrained and the hull
can weathervane in yaw (no ground friction). The mast reaction equals the negative of
all other forces; its vertical component is the measured *mooring load* used by the
take-off logic.

## 10. Wind model (implemented test-harness model, not meteorology)

$$\mathbf w(t,h) = s(h)\,\mathbf w_{mean,h}(t) + w_{mean,z}(t)\,\hat z + \mathbf w_{gust}(t) + \mathbf w_{turb}(t),\qquad s(h) = (\max(h,1)/h_{ref})^{\alpha}$$

* Mean wind changes ramp linearly over a configurable time after a trigger (time or
  first approach within a distance of the landing point).
* Discrete gusts: `A/2 (1 − cos(2π τ/T))`.
* Turbulence: first-order Gauss–Markov per axis, exact discretisation
  `x_{k+1} = a x_k + σ sqrt(1 − a²) n_k`, `a = e^{−dt/T_c}`, `n_k` from a seeded
  SplitMix64 generator (implemented in-crate so sequences stay reproducible).

## 11. Sensors (implemented)

Measured quantities = truth + optional zero-mean Gaussian noise (seeded, independent of
the turbulence stream). The wind estimate is formed as measured ground velocity minus
measured air-relative velocity and low-pass filtered (τ = 3 s) in the autopilot.

## 12. Numerical integration (implemented)

Classical RK4 with a fixed step `dt` (default 0.01 s) on the 12 continuous states
`(p, v, ψ, r, θ, q, φ, p)`. Actuator states and the turbulence state are held
constant over a step (zero-order hold); the wind field is evaluated at each RK4 stage.
The controller runs every `1/f_c` (default 20 Hz) and the control period must be an
integer multiple of `dt`.

**Accuracy (measured, `cargo run --release -p simulation-engine --example dt_study`).**
Closed-loop position deviation of Experiment D at `t = 100 s` relative to `dt = 0.001 s`:

| dt [s] | position deviation [m] |
|---|---|
| 0.025 | 1.4 × 10⁻³ |
| 0.01 | 5.3 × 10⁻⁴ |
| 0.005 | 2.4 × 10⁻⁴ |
| 0.0025 | 9.0 × 10⁻⁵ |

The coupled system converges at approximately **first order** because the actuator
outputs are held constant over each step; the rigid-body RK4 alone shows fourth-order
convergence (unit test `integrator::fourth_order_convergence`, and
`dynamics::rk4_converges_with_timestep`). At the default step the discretisation error is
three orders of magnitude below the landing-point tolerance.

**Safeguards:** invalid `dt` and rate combinations are rejected by validation;
non-finite states, speeds above 60 m/s or attitudes above 75° stop the run as a fatal
*numerical validity* event; ground penetration deeper than half the gear height and
impacts faster than 2.5 m/s are fatal *ground collision* events.

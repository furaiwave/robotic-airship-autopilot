use airship_model::{AirshipParams, DerivedProperties, Vec2};
use control_algorithms::{
    ControlInput, ControlMeasurement, ControlReferences, ControllerConfig, ControllerKind,
    ForceLimits, HorizontalReference, PhaseClass, VerticalReference, build_controller,
};
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench(c: &mut Criterion) {
    let p = AirshipParams::default();
    let d = DerivedProperties::compute(&p, 1.21);
    let limits = ForceLimits {
        surge_min_n: -d.max_reverse_thrust_n,
        surge_max_n: d.max_forward_thrust_n,
        heave_min_n: -d.max_heave_down_n,
        heave_max_n: d.max_heave_up_n,
        yaw_max_nm: d.max_differential_yaw_moment_nm,
    };
    let input = ControlInput {
        phase: PhaseClass::Cruise,
        meas: ControlMeasurement {
            altitude_agl: 30.0,
            airspeed: 4.0,
            airspeed_forward: 4.0,
            air_density: 1.21,
            ..Default::default()
        },
        refs: ControlReferences {
            vertical: VerticalReference {
                altitude_agl: Some(40.0),
                vertical_speed_ff: 0.0,
                vertical_speed_min: -1.2,
                vertical_speed_max: 1.5,
            },
            horizontal: HorizontalReference {
                ground_velocity_ff: Vec2::new(3.0, 1.0),
                hold_position: None,
                max_ground_speed: 8.0,
                preferred_yaw: None,
                yaw_override: None,
            },
            heave_trim_n: 10.0,
            heave_override_n: None,
            max_airspeed: 10.0,
            max_yaw_rate: 0.17,
            horizontal_active: true,
            surge_enabled: true,
        },
        distance_to_landing_m: 200.0,
    };
    for kind in [ControllerKind::CascadedPid, ControllerKind::GainScheduled] {
        let mut ctl = build_controller(
            &ControllerConfig {
                kind,
                ..Default::default()
            },
            &p,
            &d,
            limits,
        );
        c.bench_function(&format!("controller_step_{kind:?}"), |b| {
            b.iter(|| black_box(ctl.compute(black_box(&input), 0.05)))
        });
    }
}

criterion_group!(benches, bench);
criterion_main!(benches);

use criterion::{Criterion, criterion_group, criterion_main};
use simulation_engine::{Simulation, SimulationConfig};

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("simulation");
    group.sample_size(10);
    group.bench_function("reference_mission_100s", |b| {
        b.iter(|| {
            let mut sim = Simulation::new(SimulationConfig::default()).expect("valid config");
            sim.advance_time(100.0);
        })
    });
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);

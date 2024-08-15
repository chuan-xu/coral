use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;

fn get_config() -> Criterion {
    Criterion::default().sample_size(10)
}

#[allow(unused)]
fn bench(c: &mut Criterion) {}

criterion_group!(name = benches; config = get_config(); targets = bench);
criterion_main!(benches);

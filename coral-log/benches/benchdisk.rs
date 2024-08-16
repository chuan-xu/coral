use coral_log::logs::logger::Logger;
use coral_log::logs::Record;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use log::info;

fn get_config() -> Criterion {
    Criterion::default().sample_size(10)
}

#[allow(unused)]
fn bench(c: &mut Criterion) {
    c.bench_function("test proto log size", |b| {
        b.iter(|| {
            let fd = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open("/root/tmp/benchlog.log")
                .unwrap();
            let logger = Logger::<Record>::new(log::Level::Info, None, fd).unwrap();
            log::set_boxed_logger(Box::new(logger));
            log::set_max_level(log::LevelFilter::Info);
            let mut ths = Vec::new();
            for i in 0..4 {
                let th_name = String::from("th-") + i.to_string().as_str();

                ths.push(
                    std::thread::Builder::new()
                        .name(th_name)
                        .spawn(|| {
                            for _ in 0..250000 {
                                info!(e = "some err info"; "XXX-xxx-aaa");
                            }
                        })
                        .unwrap(),
                );
            }
            while let Some(th) = ths.pop() {
                th.join().unwrap();
            }
        })
    });
}

criterion_group!(name = benches; config = get_config(); targets = bench);
criterion_main!(benches);

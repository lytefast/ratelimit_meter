use criterion::{black_box, Criterion, ParameterizedBenchmark, Throughput};
use ratelimit_meter::test_utilities::variants::Variant;
use std::time::{Duration, Instant};

pub fn bench_all(c: &mut Criterion) {
    bench_plain_algorithm_1elem(c);
    bench_plain_algorithm_multi(c);
}

fn bench_plain_algorithm_1elem(c: &mut Criterion) {
    let id = "algorithm/1";
    let bm = ParameterizedBenchmark::new(
        id,
        move |b, ref v| {
            bench_with_algorithm_variants!(v, algo, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let state = algo.state();

                let mut i = 0;
                b.iter(|| {
                    i += 1;
                    black_box(algo.check(&state, now + (ms * i)).is_ok());
                });
            });
        },
        Variant::ALL,
    )
    .throughput(|_s| Throughput::Elements(1));
    c.bench(id, bm);
}

fn bench_plain_algorithm_multi(c: &mut Criterion) {
    let id = "algorithm/multi";
    let elements: u32 = 10;
    let bm = ParameterizedBenchmark::new(
        id,
        move |b, ref v| {
            bench_with_algorithm_variants!(v, algo, {
                let now = Instant::now();
                let ms = Duration::from_millis(20);
                let state = algo.state();

                let mut i = 0;
                b.iter(|| {
                    i += 1;
                    black_box(algo.check_n(&state, elements, now + (ms * i)).is_ok());
                });
            });
        },
        Variant::ALL,
    )
    .throughput(|_s| Throughput::Elements(1));
    c.bench(id, bm);
}

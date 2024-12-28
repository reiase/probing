use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

use probing_proto::types::Series;

fn series_append(n: u64) -> u64 {
    let mut series = Series::default();
    for i in 0..n {
        series.append(i as i64).unwrap();
    }
    series.slices.len() as u64
}

fn series_iter(s: &Series) -> u64 {
    let mut result = 0;
    for value in s.into_iter() {
        let value: i64 = value.try_into().unwrap();
        result += value as u64;
    }
    result
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut series = Series::default();
    for i in 0..60000 {
        series.append(i as i64).unwrap();
    }

    c.bench_function("series_append", |b| b.iter(|| series_append(black_box(60000))));
    c.bench_function("series_iter", |b| b.iter(|| series_iter(black_box(&series))));

}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
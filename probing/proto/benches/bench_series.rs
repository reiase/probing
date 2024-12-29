use std::hint::black_box;

use arrow::array::Int64Array;

use criterion::{criterion_group, criterion_main, Criterion};

use probing_proto::types::{Array, Series};

fn vec_append(n: u64) -> u64 {
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    vec.len() as u64
}

fn page_append(n: u64) -> u64 {
    let mut page =
        probing_proto::types::series::Page::Raw(Array::Int64Array(Vec::with_capacity(10000)));
    for i in 0..n {
        match page {
            probing_proto::types::series::Page::Raw(ref mut array) => {
                if let Array::Int64Array(ref mut int_array) = array {
                    int_array.push(i as i64);
                }
            }
            probing_proto::types::series::Page::Compressed{dtype, buffer} => todo!(),
            probing_proto::types::series::Page::Ref => todo!(),
        }
    }
    0
}

fn series_append(n: u64) -> u64 {
    let mut series = Series::default();
    for i in 0..n {
        series.append(i as i64).unwrap();
    }
    series.slices.len() as u64
}

fn series_iter(s: &Series, expected_sum: u64) -> u64 {
    let mut result = 0;
    for value in s.iter() {
        let value: i64 = value.try_into().unwrap();
        result += value as u64;
    }
    assert!(result == expected_sum, "expected sum: {}, got: {}", expected_sum, result);
    result
}

fn arrow_array_append(n: u64) -> u64 {
    let mut builder = Int64Array::builder(10000);
    for i in 0..n {
        builder.append_value(i as i64);
    }
    let array = builder.finish();
    array.len() as u64
}

fn arrow_array_iter(array: &Int64Array) -> u64 {
    let mut result = 0;
    for value in array.iter() {
        if let Some(v) = value {
            result += v as u64;
        }
    }
    result
}

fn criterion_benchmark(c: &mut Criterion) {

    let expected_sum = (0..60000).sum::<u64>();

    let mut series = Series::builder().with_chunk_size(256).build();
    for i in 0..60000 {
        series.append(i as i64).unwrap();
    }

    // Create arrow array for benchmarking
    let mut builder = Int64Array::builder(60000);
    for i in 0..60000 {
        builder.append_value(i as i64);
    }
    let arrow_array = builder.finish();

    c.bench_function("vec_append", |b| b.iter(|| vec_append(black_box(60000))));
    c.bench_function("page_append", |b| b.iter(|| page_append(black_box(60000))));
    c.bench_function("series_append", |b| {
        b.iter(|| series_append(black_box(60000)))
    });
    c.bench_function("arrow_array_append", |b| {
        b.iter(|| arrow_array_append(black_box(60000)))
    });

    c.bench_function("series_iter", |b| {
        b.iter(|| series_iter(black_box(&series), expected_sum))
    });
    c.bench_function("arrow_array_iter", |b| {
        b.iter(|| arrow_array_iter(black_box(&arrow_array)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

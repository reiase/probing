use probing_proto::types::Series;

fn main() {
    for dtype in vec!["i64", "i32", "f64", "f32"] {
        for level in vec![0, 8, 12] {
            for seq in vec!["zero", "linear", "sin(x)", "x+sin(x)", "exp(x)"] {
                // test for int seq
                let mut series = Series::builder()
                    .with_chunk_size(10000)
                    .with_compression_threshold(100)
                    .with_compression_level(level)
                    .build();
                for i in 0..1_000_000 {
                    let val = match seq {
                        "zero" => 0 as f64,
                        "linear" => i as f64,
                        "sin(x)" => (i as f64).sin(),
                        "x+sin(x)" => (i as f64) + (i as f64).sin(),
                        "exp(x)" => (i as f64).exp(),
                        _ => unreachable!(),
                    };
                    match dtype {
                        "i64" => {
                            let _ = series.append(val as i64);
                        }
                        "i32" => {
                            let _ = series.append(val as i32);
                        }
                        "f64" => {
                            let _ = series.append(val);
                        }
                        "f32" => {
                            let _ = series.append(val as f32);
                        }
                        _ => unreachable!(),
                    }
                }
                println!(
                    "{} series ({}) with compress level 0 ({}): {} => {}",
                    dtype,
                    seq,
                    series.nbytes() as f64 / 1_000_000.0 / 8.0,
                    1_000_000
                        * match dtype {
                            "i64" => 8,
                            "i32" => 4,
                            "f64" => 8,
                            "f32" => 4,
                            _ => unreachable!(),
                        },
                    series.nbytes()
                );
            }
        }
    }
}

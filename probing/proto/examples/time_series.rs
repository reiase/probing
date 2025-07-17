use probing_proto::prelude::{DiscardStrategy, Series};

/// Data type information for series benchmarking
#[derive(Clone, Copy)]
struct DataTypeInfo {
    name: &'static str,
    size_bytes: usize,
}

impl DataTypeInfo {
    const I64: Self = Self {
        name: "i64",
        size_bytes: 8,
    };
    const I32: Self = Self {
        name: "i32",
        size_bytes: 4,
    };
    const F64: Self = Self {
        name: "f64",
        size_bytes: 8,
    };
    const F32: Self = Self {
        name: "f32",
        size_bytes: 4,
    };

    const ALL: &'static [Self] = &[Self::I64, Self::I32, Self::F64, Self::F32];
}

/// Sequence generation functions
enum SeqGenerator {
    Zero,
    Linear,
    Sin,
    LinearPlusSin,
    Exp,
}

impl SeqGenerator {
    const ALL: &'static [(Self, &'static str)] = &[
        (Self::Zero, "zero"),
        (Self::Linear, "linear"),
        (Self::Sin, "sin(x)"),
        (Self::LinearPlusSin, "x+sin(x)"),
        (Self::Exp, "exp(x)"),
    ];

    fn generate(&self, i: usize) -> f64 {
        let x = i as f64;
        match self {
            Self::Zero => 0.0,
            Self::Linear => x,
            Self::Sin => x.sin(),
            Self::LinearPlusSin => x + x.sin(),
            Self::Exp => x.exp(),
        }
    }
}

/// Append value to series based on data type
fn append_to_series(
    series: &mut Series,
    val: f64,
    dtype: DataTypeInfo,
) -> Result<(), Box<dyn std::error::Error>> {
    match dtype.name {
        "i64" => series.append(val as i64).map_err(|e| e.into()),
        "i32" => series.append(val as i32).map_err(|e| e.into()),
        "f64" => series.append(val).map_err(|e| e.into()),
        "f32" => series.append(val as f32).map_err(|e| e.into()),
        _ => unreachable!("Invalid data type: {}", dtype.name),
    }
}

/// Create and populate a series with generated data
fn create_series(
    dtype: DataTypeInfo,
    seq_gen: &SeqGenerator,
    level: usize,
    count: usize,
) -> Series {
    let mut series = Series::builder()
        .with_discard_strategy(DiscardStrategy::base_memory_size_with_custom_chunk(10000))
        .with_compression_threshold(100)
        .with_compression_level(level)
        .build();

    for i in 0..count {
        let val = seq_gen.generate(i);
        let _ = append_to_series(&mut series, val, dtype);
    }

    series
}

fn main() {
    const DATA_COUNT: usize = 1_000_000;
    const COMPRESSION_LEVELS: &[usize] = &[0, 8, 12];

    for &dtype in DataTypeInfo::ALL {
        for &level in COMPRESSION_LEVELS {
            for &(ref seq_gen, seq_name) in SeqGenerator::ALL {
                let series = create_series(dtype, seq_gen, level, DATA_COUNT);

                let compression_ratio = series.nbytes() as f64 / DATA_COUNT as f64 / 8.0;
                let uncompressed_size = DATA_COUNT * dtype.size_bytes;

                println!(
                    "{} series ({}) with compress level {} ({}): {} => {}",
                    dtype.name,
                    seq_name,
                    level,
                    compression_ratio,
                    uncompressed_size,
                    series.nbytes()
                );
            }
        }
    }
}

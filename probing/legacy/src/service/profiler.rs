use crate::handlers::PPROF_HOLDER;

pub fn flamegraph() -> String {
    PPROF_HOLDER
        .flamegraph()
        .unwrap_or("no profile data".to_string())
}

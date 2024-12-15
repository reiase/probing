use std::sync::{LazyLock, RwLock};

pub static PROBING_ADDRESS: LazyLock<RwLock<String>> =
    LazyLock::new(|| RwLock::new(Default::default()));

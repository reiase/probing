fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    pyo3_build_config::add_extension_module_link_args();

    println!("cargo::rustc-check-cfg=cfg(probing_snapshot_unwind)");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_os == "linux" && target_arch == "x86_64" {
        // The ABI is declared directly in Rust so cargo-zigbuild does not need
        // to compile C code or locate target-specific libunwind headers.
        println!("cargo::rustc-link-lib=unwind-x86_64");
        println!("cargo::rustc-link-lib=unwind");
        println!("cargo::rustc-cfg=probing_snapshot_unwind");
    }
}

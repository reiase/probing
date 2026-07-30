use std::path::PathBuf;

fn main() {
    pyo3_build_config::use_pyo3_cfgs();
    pyo3_build_config::add_extension_module_link_args();

    println!("cargo::rustc-check-cfg=cfg(probing_snapshot_unwind)");
    println!("cargo::rustc-check-cfg=cfg(probing_bundled_libunwind)");
    println!("cargo::rerun-if-env-changed=PROBING_LIBUNWIND_STATIC_DIR");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if target_os == "linux" && target_arch == "x86_64" {
        println!("cargo::rustc-cfg=probing_snapshot_unwind");

        if let Some(directory) = std::env::var_os("PROBING_LIBUNWIND_STATIC_DIR") {
            let directory = PathBuf::from(directory);
            for archive in ["libunwind-x86_64.a", "libunwind.a"] {
                let path = directory.join(archive);
                assert!(
                    path.is_file(),
                    "PROBING_LIBUNWIND_STATIC_DIR is missing {}",
                    path.display()
                );
            }

            println!("cargo::rustc-cfg=probing_bundled_libunwind");
            println!("cargo::rustc-link-search=native={}", directory.display());
            println!("cargo::rustc-link-lib=static=unwind-x86_64");
            println!("cargo::rustc-link-lib=static=unwind");
            println!("cargo::rustc-link-arg=-Wl,--exclude-libs,libunwind-x86_64.a:libunwind.a");
        }
    }
}

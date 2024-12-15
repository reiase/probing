use std::ffi::{c_char, CString};

use anyhow::anyhow;
use anyhow::Result;

use nix::libc::dlclose;
use nix::libc::dlopen;
use nix::libc::dlsym;
use nix::libc::RTLD_LAZY;
use plt_rs::collect_modules;
use plt_rs::DynamicLibrary;
use plt_rs::RelocationTable;

use crate::ctrl::StringBuilder;
use crate::ctrl::StringBuilderAppend;

type QueryFn = fn() -> *mut c_char;

pub fn call_func(name: String) -> Result<String> {
    let ret = unsafe {
        let handle = dlopen(std::ptr::null::<i8>(), RTLD_LAZY);
        if handle.is_null() {
            return Err(anyhow!("unable of dlopen self"));
        }
        let symbol = dlsym(handle, name.as_ptr() as *const i8);
        if symbol.is_null() {
            return Err(anyhow!("symbol not found: {}", name));
        }
        let symbol: QueryFn = std::mem::transmute(symbol);
        let ptr = symbol();
        if ptr.is_null() {
            return Err(anyhow!("ffi function returned null pointer"));
        }
        let ret = CString::from_raw(ptr)
            .to_owned()
            .to_string_lossy()
            .to_string();
        dlclose(handle);
        ret
    };
    Ok(ret)
}

pub fn read_plt() -> Result<String> {
    let entries = collect_modules();
    let builder: &mut StringBuilder = &mut Default::default();

    for entry in entries.into_iter() {
        format!("[{:?}] Addr: {:#X?}", entry.name(), entry.addr()).append_line(builder);
        if let Ok(dynamic_lib) = DynamicLibrary::initialize(entry) {
            format!(
                "Dynamic String Table Length: {}",
                dynamic_lib.string_table().total_size()
            )
            .append_line(builder);

            let dynamic_symbols = dynamic_lib.symbols().expect("symbols...");
            let string_table = dynamic_lib.string_table();

            "dynamic addend relocations:".append_line(builder);
            if let Some(dyn_relas) = dynamic_lib.addend_relocs() {
                dyn_relas
                    .entries()
                    .iter()
                    .flat_map(|e| {
                        dynamic_symbols.resolve_name(e.symbol_index() as usize, string_table)
                    })
                    .filter(|s| !s.is_empty())
                    .for_each(|s| format!("\t{}", s).append_line(builder));
            }

            "dynamic relocations:".append_line(builder);
            if let Some(dyn_relocs) = dynamic_lib.relocs() {
                dyn_relocs
                    .entries()
                    .iter()
                    .flat_map(|e| {
                        dynamic_symbols.resolve_name(e.symbol_index() as usize, string_table)
                    })
                    .filter(|s| !s.is_empty())
                    .for_each(|s| format!("\t{}", s).append_line(builder));
            }

            "plt:".append_line(builder);
            if let Some(plt) = dynamic_lib.plt() {
                match plt {
                    RelocationTable::WithAddend(rel) => {
                        rel.entries()
                            .iter()
                            .flat_map(|e| {
                                dynamic_symbols
                                    .resolve_name(e.symbol_index() as usize, string_table)
                            })
                            .filter(|s| !s.is_empty())
                            .for_each(|s| format!("\t{}", s).append_line(builder));
                    }
                    RelocationTable::WithoutAddend(rel) => {
                        rel.entries()
                            .iter()
                            .flat_map(|e| {
                                dynamic_symbols
                                    .resolve_name(e.symbol_index() as usize, string_table)
                            })
                            .filter(|s| !s.is_empty())
                            .for_each(|s| format!("\t{}", s).append_line(builder));
                    }
                }
            }
        }
        "".append_line(builder);
    }

    Ok(builder.to_string())
}

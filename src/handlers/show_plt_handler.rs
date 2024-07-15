use anyhow::Result;
use plt_rs::{collect_modules, DynamicLibrary, RelocationTable};

use crate::ctrl::{StringBuilder, StringBuilderAppend};

pub(crate) fn show_plt() -> Result<()> {
    read_plt().map(|plt| {
        println!("{}", plt);
    })
}

pub(crate) fn read_plt() -> Result<String> {
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

            format!("dynamic addend relocations:").append_line(builder);
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

            format!("dynamic relocations:").append_line(builder);
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

            format!("plt:").append_line(builder);
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
        format!("").append_line(builder);
    }

    Ok(builder.to_string())
}

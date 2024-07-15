use anyhow::Result;
use probing_common::cli::ShowCommand;

use crate::handlers::read_plt;

use super::not_implemented;

pub fn handle(topic: ShowCommand) -> Result<String> {
    match topic {
        ShowCommand::Memory => not_implemented(),
        ShowCommand::Threads => not_implemented(),
        ShowCommand::Objects => not_implemented(),
        ShowCommand::Tensors => not_implemented(),
        ShowCommand::Modules => not_implemented(),
        ShowCommand::PLT => read_plt(),
    }
}

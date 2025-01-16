use leptos::prelude::*;
use probing_proto::{types::Table, Process};
use thaw::*;

use crate::components::tableview::TableView;

#[component]
pub fn ProcessCard(process: Process) -> impl IntoView {
    let tbl = Table::new(
        vec!["name", "value"],
        vec![
            vec!["Process ID(pid)".to_string(), process.pid.to_string()],
            vec!["Executable Path(exe)".to_string(), process.exe.to_string()],
            vec!["Command Line(cmd)".to_string(), process.cmd.to_string()],
            vec![
                "Current Working Dirctory(cwd)".to_string(),
                process.cwd.to_string(),
            ],
        ],
    );
    view! { <TableView tbl /> }
}

#[component]
pub fn ThreadsCard(threads: Vec<u64>) -> impl IntoView {
    let threads = threads
        .iter()
        .map(|t| {
            let tid = *t;
            let url = format!("/activity/{}", tid);
            view! { <Link href=url>{tid}</Link> }
        })
        .collect::<Vec<_>>();

    view! { <Flex style="flex-wrap: wrap;">{threads}</Flex> }
}

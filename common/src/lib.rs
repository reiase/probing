use leptos::*;
use leptos_struct_table::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Process {
    pub pid: i32,
    pub exe: String,
    pub env: String,
    pub cmd: String,
    pub cwd: String,
}

#[derive(TableRow, Clone)]
#[table(impl_vec_data_provider)]
pub struct KeyValuePair {
    pub name: String,
    #[table(renderer = "PreCellRenderer")]
    pub value: String,
}

#[component]
fn PreCellRenderer<F>(
    class: String,
    #[prop(into)] value: MaybeSignal<String>,
    on_change: F,
    index: usize,
) -> impl IntoView
where
    F: Fn(String) + 'static,
{
    view! {
        <td class=class>
            <pre style="white-space: pre-wrap; word-break: break-word;">{value}</pre>
        </td>
    }
}

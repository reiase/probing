use std::collections::HashMap;

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

#[derive(TableRow, Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[table(impl_vec_data_provider)]
pub struct Object {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct CallStack {
    pub file: String,
    pub func: String,
    pub locals: HashMap<String, String>,
}

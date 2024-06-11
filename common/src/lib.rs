use std::collections::HashMap;

#[cfg(feature = "leptos")]
use leptos::*;

#[cfg(feature = "leptos")]
use leptos_struct_table::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Process {
    pub pid: i32,
    pub exe: String,
    pub env: String,
    pub cmd: String,
    pub cwd: String,
    pub main_thread: u64,
    pub threads: Vec<u64>,
}

#[derive(Clone)]
#[cfg_attr(feature = "leptos", derive(TableRow))]
#[cfg_attr(feature = "leptos", table(impl_vec_data_provider))]
pub struct KeyValuePair {
    pub name: String,
    #[cfg_attr(feature = "leptos", table(renderer = "PreCellRenderer"))]
    pub value: String,
}

#[cfg(feature = "leptos")]
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

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct CallStack {
    pub file: String,
    pub func: String,
    pub lineno: i64,
    pub locals: HashMap<String, Object>,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Object {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
    pub value: Option<String>,
}

use leptonic::prelude::*;
use leptos::*;
use leptos_struct_table::*;
use serde::{Deserialize, Serialize};

use serde_json;

#[derive(TableRow, Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[table(impl_vec_data_provider)]
pub struct Module {
    pub id: u64,
    pub class: String,
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

#[component]
pub fn ModuList(text: Option<String>) -> impl IntoView {
    text.map(|text| serde_json::from_str::<Vec<Module>>(text.as_str()))
        .map(|objs| match objs {
            Ok(rows) => view! {
                <Box style="width: 100%">
                    <Table bordered=true hoverable=true>
                        <TableContent rows=rows/>
                    </Table>
                </Box>
            },
            Err(err) => view! { <Box style="width: 100%">{err.to_string()}</Box> },
        })
        .unwrap_or(view! { <Box style="width: 100%">{"no objects found!"}</Box> })
}

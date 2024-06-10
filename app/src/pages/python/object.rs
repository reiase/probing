use leptonic::prelude::*;
use leptos::*;
use leptos_struct_table::*;
use serde::{Deserialize, Serialize};

use serde_json;

#[derive(TableRow, Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[table(impl_vec_data_provider)]
pub struct PyObj {
    pub id: u64,
    pub class: String,
}

#[component]
pub fn ObjectList(text: Option<String>) -> impl IntoView {
    text.map(|text| serde_json::from_str::<Vec<PyObj>>(text.as_str()))
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

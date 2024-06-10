use leptonic::prelude::*;
use leptos::*;
use leptos_struct_table::*;
use serde::{Deserialize, Serialize};

use serde_json;

#[derive(TableRow, Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[table(impl_vec_data_provider)]
pub struct Tensor {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
}

#[component]
pub fn TensorList(text: Option<String>) -> impl IntoView {
    text.map(|text| serde_json::from_str::<Vec<Tensor>>(text.as_str()))
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

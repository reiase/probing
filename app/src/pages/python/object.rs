use leptonic::prelude::*;
use leptos::*;
use leptos_struct_table::*;
use probe_common::Object;
use serde::{Deserialize, Serialize};

use serde_json;

use crate::pages::common::ObjectView;

#[derive(TableRow, Debug, Default, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[table(impl_vec_data_provider)]
pub struct PyObj {
    pub id: u64,
    pub class: String,
}

#[component]
pub fn ObjectList(text: Option<String>) -> impl IntoView {
    let header = view! {
        <TableRow>
            <TableHeaderCell min_width=true>"#"</TableHeaderCell>
            <TableHeaderCell>"Class"</TableHeaderCell>
            <TableHeaderCell>"Value"</TableHeaderCell>
        </TableRow>
    };
    let body: Vec<_> = text
        .map(|text| serde_json::from_str::<Vec<Object>>(text.as_str()).unwrap_or_default())
        .unwrap_or_default()
        .iter()
        .map(|obj| {
            let id = obj.id;
            let class = obj.class.clone();
            let obj = obj.clone();
            view! {
                <TableRow>
                    <TableCell>{id}</TableCell>
                    <TableCell>{class}</TableCell>
                    <TableCell>
                        <ObjectView obj=obj/>
                    </TableCell>
                </TableRow>
            }
        })
        .collect();
    view! {
        <TableContainer>
            <Table bordered=true hoverable=true>
                <TableHeader>{header}</TableHeader>
                <TableBody>{body}</TableBody>
            </Table>
        </TableContainer>
    }
}

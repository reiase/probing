use leptonic::components::prelude::*;
use leptos::*;
use ppp::Object;

use serde_json;

use crate::pages::common::ObjectKind;
use crate::pages::common::ObjectView;

#[component]
pub fn ObjectList(
    #[prop(into)] text: Option<String>,
    #[prop(into)] kind: ObjectKind,
) -> impl IntoView {
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
                        <ObjectView obj=obj kind=kind/>
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

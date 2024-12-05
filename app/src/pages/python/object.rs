use leptos::*;
use thaw::*;

use probing_dpp::Object;

use serde_json;

use crate::pages::common::ObjectKind;

#[component]
pub fn ObjectList(
    #[prop(into)] text: Option<String>,
    #[prop(into)] kind: ObjectKind,
) -> impl IntoView {
    let rows: Vec<_> = text
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
                        // <ObjectView obj=obj kind=kind/>
                        "123"
                    </TableCell>
                </TableRow>
            }
        })
        .collect();
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell resizable=true min_width=10.0 max_width=40.0>
                        "#"
                    </TableHeaderCell>
                    <TableHeaderCell resizable=true>"Class"</TableHeaderCell>
                    <TableHeaderCell>"Value"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>{rows}</TableBody>
        </Table>
    }
}

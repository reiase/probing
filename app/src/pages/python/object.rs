use leptos::*;
use thaw::*;

use dpp::Object;

use serde_json;

use crate::pages::common::ObjectKind;
use crate::pages::common::ObjectView;

#[component]
pub fn ObjectList(
    #[prop(into)] text: Option<String>,
    #[prop(into)] kind: ObjectKind,
) -> impl IntoView {
    let header = view! {
        <tr>
            <td min_width=true>"#"</td>
            <td>"Class"</td>
            <td>"Value"</td>
        </tr>
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
                <tr>
                    <td>{id}</td>
                    <td>{class}</td>
                    <td>
                        // <ObjectView obj=obj kind=kind/>
                        "123"
                    </td>
                </tr>
            }
        })
        .collect();
    view! {
        <Table>
            <thead>{header}</thead>
            <tbody>{body}</tbody>
        </Table>
    }
}

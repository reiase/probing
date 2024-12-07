use leptos::prelude::*;
use thaw::*;

use probing_dpp::protocol::dataframe::Table;

#[component]
pub fn TableView(tbl: Table) -> impl IntoView {
    let head = view! {
        <TableHeader>
            <TableRow>
                <For
                    each=move || tbl.names.clone().into_iter()
                    key=|name| name.clone()
                    children=move |name| {
                        view! { <TableHeaderCell>{name}</TableHeaderCell> }
                    }
                />
            </TableRow>
        </TableHeader>
    };
    let rows = tbl
        .rows
        .into_iter()
        .map(|row| {
            view! {
                <TableRow>
                    {row
                        .into_iter()
                        .map(|val| {
                            view! { <TableCell>{val.to_string()}</TableCell> }
                        })
                        .collect::<Vec<_>>()}
                </TableRow>
            }
        })
        .collect::<Vec<_>>();
    view! { <Table>{head} <TableBody>{rows}</TableBody></Table> }
}

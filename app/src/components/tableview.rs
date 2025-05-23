use leptos::prelude::*;
use leptos_meta::Style;
use thaw::*;

use probing_proto::types::Ele;

#[derive(Debug, PartialEq, Clone)]
pub struct Table {
    pub names: Vec<String>,
    pub rows: Vec<Vec<Ele>>,
}

impl Table {
    pub fn new<N: Into<String>, V: Into<Ele>>(names: Vec<N>, rows: Vec<Vec<V>>) -> Self {
        let names = names.into_iter().map(|x| x.into()).collect();
        let rows = rows
            .into_iter()
            .map(|r| r.into_iter().map(|x| x.into()).collect())
            .collect();
        Self { names, rows }
    }
}

#[component]
pub fn TableView(tbl: Table) -> impl IntoView {
    let head = view! {
        <TableHeader>
            <TableRow>
                <For
                    each=move || tbl.names.clone().into_iter()
                    key=|name| name.clone()
                    children=move |name| {
                        view! { <TableHeaderCell resizable=true>{name}</TableHeaderCell> }
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
    view! {
        <Style>
            "
            .tbl-content {
                table-layout: auto;
            }
            "
        </Style>
        <Table class="tbl-content">{head} <TableBody>{rows}</TableBody></Table>
    }
}

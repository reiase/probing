use leptos::prelude::*;
use thaw::*;

use probing_proto::protocol::dataframe::DataFrame;
use probing_proto::protocol::dataframe::Value;

#[component]
pub fn DataFrameView(df: DataFrame) -> impl IntoView {
    let head = view! {
        <TableHeader>
            <TableRow>
                <For
                    each=move || df.names.clone().into_iter()
                    key=|name| name.clone()
                    children=move |name| {
                        view! { <TableHeaderCell>{name}</TableHeaderCell> }
                    }
                />
            </TableRow>
        </TableHeader>
    };
    let nrows = df.cols.clone().iter().map(|x| x.len()).max().unwrap_or(0);
    let rows = (0..nrows)
        .map(|i| {
            let row = df.cols
                .iter()
                .map(move |col| match col.get(i) {
                    Value::Nil => view! { <TableCell>{"nil".to_string()}</TableCell> },
                    Value::Int32(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Value::Int64(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Value::Float32(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Value::Float64(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Value::Text(x) => view! { <TableCell>{x.to_string()}</TableCell> },
                    Value::Url(x) => view! {
                        <TableCell>
                            <Link href=x.to_string()>{x.to_string()}</Link>
                        </TableCell>
                    },
                })
                .collect::<Vec<_>>();
            view! { <TableRow>{row}</TableRow> }
        })
        .collect::<Vec<_>>();
    view! { <Table>{head} <TableBody>{rows}</TableBody></Table> }
}

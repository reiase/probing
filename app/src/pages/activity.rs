use leptonic::prelude::*;
use leptos::*;
use leptos_struct_table::*;

use gloo_net::http::Request;
use probe_common::{CallStack, KeyValuePair};

#[component]
pub fn Activity() -> impl IntoView {
    let callstacks = create_resource(
        move || (),
        move |_| async move {
            let resp = Request::get("/apis/callstack")
                .send()
                .await
                .map_err(|err| {
                    eprintln!("error getting callstack: {}", err);
                })
                .unwrap()
                .json::<Vec<CallStack>>()
                .await
                .map_err(|err| {
                    eprintln!("error decoding callstack: {}", err);
                })
                .ok();
            resp.unwrap_or(Default::default())
        },
    );

    let activity_info = move || {
        callstacks
            .get()
            .map(|callstacks| {
                let rows: Vec<KeyValuePair> = callstacks
                    .iter()
                    .map(|callstack| KeyValuePair {
                        name: callstack.file.to_string(),
                        value: callstack.func.to_string(),
                    })
                    .collect();
                view! {
                    <Table bordered=true hoverable=true>
                        <TableContent rows/>
                    </Table>
                }
            })
            .unwrap_or(view! {
                <Table>
                    <TableRow>""</TableRow>
                </Table>
            })
    };

    view! {
        <Box>
            <TableContainer>{activity_info}</TableContainer>
        </Box>
    }
}

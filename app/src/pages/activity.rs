use leptonic::prelude::*;
use leptos::*;
use leptos_router::use_params_map;
use leptos_struct_table::*;

use gloo_net::http::Request;
use probe_common::{CallStack, KeyValuePair};

#[component]
pub fn Activity() -> impl IntoView {
    let params = use_params_map();
    let url = if let Some(tid) = params.get().get("tid") {
        format!("/apis/callstack?tid={}", tid)
    } else {
        "/apis/callstack".to_string()
    };
    let callstacks = create_resource(
        move || url.clone(),
        move |url| async move {
            let resp = Request::get(url.as_str())
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
                let views = callstacks
                    .iter()
                    .map(|callstack| {
                        let file = callstack.file.clone();
                        let func = callstack.func.clone();
                        let locals: Vec<KeyValuePair> = callstack
                            .locals
                            .clone()
                            .iter()
                            .map(|(k, v)| KeyValuePair {
                                name: k.to_string(),
                                value: v.to_string(),
                            })
                            .collect();
                        view! {
                            <Collapsible>
                                <CollapsibleHeader slot>{func} {"@"} {file}</CollapsibleHeader>
                                <CollapsibleBody class="my-body" slot>
                                    <TableContainer>
                                        <Table bordered=true hoverable=true>
                                            <TableContent rows=locals/>
                                        </Table>
                                    </TableContainer>
                                </CollapsibleBody>
                            </Collapsible>
                        }
                    })
                    .collect::<Vec<_>>();
                view! { <Stack spacing=Size::Em(0.6)>{views}</Stack> }
            })
            .unwrap_or(view! {
                <Stack spacing=Size::Em(0.6)>
                    <span>{"no call stack"}</span>
                </Stack>
            })
    };

    view! {
        <Collapsibles default_on_open=OnOpen::CloseOthers>
            <TableContainer>{activity_info}</TableContainer>
        </Collapsibles>
    }
}

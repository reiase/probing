use leptonic::prelude::*;
use leptos::*;
use leptos_struct_table::*;

use gloo_net::http::Request;

use probe_common::{KeyValuePair, Process};

#[component]
pub fn Overview() -> impl IntoView {
    let resp = create_resource(
        move || {},
        move |_| async move {
            let resp = Request::get("/apis/overview")
                .send()
                .await
                .map_err(|err| {
                    eprintln!("error getting overview: {}", err);
                })
                .unwrap()
                .json::<Process>()
                .await
                .map_err(|err| {
                    eprintln!("error decoding overview: {}", err);
                })
                .ok();
            resp.unwrap_or(Default::default())
        },
    );
    let process_info = move || {
        resp.get()
            .map(|proc| {
                let rows = vec![
                    KeyValuePair {
                        name: "pid".to_string(),
                        value: proc.pid.to_string(),
                    },
                    KeyValuePair {
                        name: "exe".to_string(),
                        value: proc.exe,
                    },
                    KeyValuePair {
                        name: "cmd".to_string(),
                        value: proc.cmd,
                    },
                    KeyValuePair {
                        name: "cwd".to_string(),
                        value: proc.cwd,
                    },
                ];
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
    let environments = move || {
        resp.get()
            .map(|proc| {
                let rows: Vec<KeyValuePair> = proc
                    .env
                    .split_terminator('\n')
                    .filter_map(|kv| {
                        if let Some((name, value)) = kv.split_once('=') {
                            Some(KeyValuePair {
                                name: name.to_string(),
                                value: value.to_string(),
                            })
                        } else {
                            None
                        }
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
        <Collapsibles default_on_open=OnOpen::CloseOthers>
            <Stack spacing=Size::Em(0.6)>
                <Collapsible>
                    <CollapsibleHeader slot>{"Process Infomation"}</CollapsibleHeader>
                    <CollapsibleBody class="my-body" slot>
                        <TableContainer>{process_info}</TableContainer>
                    </CollapsibleBody>
                </Collapsible>
                <Collapsible>
                    <CollapsibleHeader slot>{"Environment Variables"}</CollapsibleHeader>
                    <CollapsibleBody class="my-body" slot>
                        <TableContainer>{environments}</TableContainer>
                    </CollapsibleBody>
                </Collapsible>
            </Stack>
        </Collapsibles>
    }
}

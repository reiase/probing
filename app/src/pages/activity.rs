use std::collections::HashMap;

use leptonic::prelude::*;
use leptos::*;
use leptos_router::use_params_map;

use gloo_net::http::Request;
use probe_common::{CallStack, Object};

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
                    logging::log!("error getting callstack: {}", err);
                })
                .unwrap()
                .json::<Vec<CallStack>>()
                .await
                .map_err(|err| {
                    logging::log!("error decoding callstack: {}", err);
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
                        view! { <CallStackView callstack=callstack.clone()/> }
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
        <H3>"Call Stacks"</H3>
        <Collapsibles default_on_open=OnOpen::CloseOthers>{activity_info}</Collapsibles>
    }
}

#[component]
fn CallStackView(#[prop(into)] callstack: CallStack) -> impl IntoView {
    let file = callstack.file.clone();
    let func = callstack.func.clone();
    let lineno = callstack.lineno;
    let locals = callstack.locals.clone();
    view! {
        <Collapsible>
            <CollapsibleHeader slot>
                <Chip>{func} "@" {file} {":"} {lineno}</Chip>
            </CollapsibleHeader>
            <CollapsibleBody class="my-body" slot>
                <VariablesView variables=locals/>
            </CollapsibleBody>
        </Collapsible>
    }
}

#[component]
fn VariablesView(#[prop(into)] variables: HashMap<String, Object>) -> impl IntoView {
    let header = view! {
        <TableRow>
            <TableHeaderCell min_width=true>"#"</TableHeaderCell>
            <TableHeaderCell>"Name"</TableHeaderCell>
            <TableHeaderCell>"Value"</TableHeaderCell>
        </TableRow>
    };
    let body = variables
        .iter()
        .map(|(name, obj)| {
            let id = obj.id;
            let name = name.clone();
            let obj = obj.clone();
            logging::log!("debug: {:?}", obj);
            view! {
                <TableRow>
                    <TableCell>{id}</TableCell>
                    <TableCell>{name.clone()}</TableCell>
                    <TableCell>
                        <ObjectView obj=obj/>
                    </TableCell>
                </TableRow>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <TableContainer>
            <Table bordered=true hoverable=true>
                <TableHeader>{header}</TableHeader>
                <TableBody>{body}</TableBody>
            </Table>
        </TableContainer>
    }
}

#[component]
fn ObjectView(#[prop(into)] obj: Object) -> impl IntoView {
    let id = obj.id;
    let class = obj.class.clone();
    let value = obj.value.clone();
    let shape = move || {
        let shape = obj.shape.clone();
        if let Some(shape) = shape {
            view! {
                <Box>
                    <Chip>{shape}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    let dtype = move || {
        let dtype = obj.dtype.clone();
        if let Some(dtype) = dtype {
            view! {
                <Box>
                    <Chip>{dtype}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    let device = move || {
        let device = obj.device.clone();
        if let Some(device) = device {
            view! {
                <Box>
                    <Chip>{device}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    view! {
        <span>{value}</span>
        <Chip>{class}</Chip>
        {shape}
        {dtype}
        {device}
    }
}

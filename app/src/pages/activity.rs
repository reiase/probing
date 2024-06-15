use leptonic::prelude::*;
use leptos::*;
use leptos_router::use_params_map;

use gloo_net::http::Request;
use probe_common::CallStack;

use super::common::*;

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
    let url = format!("/apis/files?path={}", file.clone());
    let route_url = format!("/files?path={}", file);
    view! {
        <Collapsible>
            <CollapsibleHeader slot>
                <Chip>
                    {func} "@" <a href=url target="_blank">
                        {file}
                    </a> {":"} {lineno}
                </Chip>
                <Button on_click=move |_| {
                    let navigate = leptos_router::use_navigate();
                    navigate(route_url.as_str(), Default::default());
                }>
                    <Icon icon=icondata::FaFileRegular/>
                </Button>
            </CollapsibleHeader>
            <CollapsibleBody class="my-body" slot>
                <VariablesView variables=locals/>
            </CollapsibleBody>
        </Collapsible>
    }
}

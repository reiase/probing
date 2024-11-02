use leptos::*;
use leptos_meta::Style;
use leptos_router::use_params_map;
use log::info;
use thaw::*;

use dpp::CallStack;

use crate::{components::header_bar::HeaderBar, url_read::url_read_resource};

use super::common::*;

#[component]
pub fn Activity() -> impl IntoView {
    let params = use_params_map();
    let url = if let Some(tid) = params.get().get("tid") {
        format!("/apis/callstack?tid={}", tid)
    } else {
        "/apis/callstack".to_string()
    };

    let callstacks = url_read_resource::<Vec<CallStack>>(url.as_str());

    let callstacks = move || {
        callstacks
            .and_then(|callstacks| {
                info!("output some activity");
                callstacks
                    .iter()
                    .map(|callstack| {
                        view! { <CallStackView callstack=callstack.clone()/> }
                    })
                    .collect::<Vec<_>>()
            })
            .map(|x| x.ok())
            .flatten()
    };

    view! {
        <Style>
            "
            .doc-content {
                margin: 0 auto;
                width: 100%;
                display: grid;
            }
            @media screen and (max-width: 1200px) {
                .doc-content {
                    width: 100%;
                }
            }
            "
        </Style>
        <HeaderBar/>
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <Space align=SpaceAlign::Center vertical=true class="doc-content">
                <h3>"Call Stacks"</h3>
                <Collapse>{callstacks}</Collapse>
            </Space>
        </Layout>
    }
}

#[component]
fn CallStackView(#[prop(into)] callstack: CallStack) -> impl IntoView {
    if let Some(cstack) = callstack.cstack {
        view! {
            <CollapseItem title="C/C++ Call Stack" key="C/C++">
                <pre>{cstack}</pre>
            </CollapseItem>
        }
    } else {
        let file = callstack.file.clone();
        let func = callstack.func.clone();
        let lineno = callstack.lineno;
        let locals = callstack.locals.clone();
        let url = format!("/apis/files?path={}", file.clone());
        let route_url = format!("/files?path={}", file);
        let key = format!("{func} @ {file}: {lineno}");
        view! {
            <CollapseItem title=key.clone() key=key>
                // <title slot>
                // {func} "@" <a href=url target="_blank">
                // {file}
                // </a> {":"} {lineno}
                // <Button on_click=move |_| {
                // let navigate = leptos_router::use_navigate();
                // navigate(route_url.as_str(), Default::default());
                // }>
                // <Icon icon=icondata::BiFileRegular/>
                // </Button>
                // </title>
                <VariablesList variables=locals/>
            </CollapseItem>
        }
    }
}

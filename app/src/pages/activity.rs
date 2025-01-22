use leptos::prelude::*;
use leptos_meta::Style;
use leptos_router::hooks::use_params_map;
use thaw::*;

use probing_proto::protocol::probe::ProbeCall;
use probing_proto::protocol::process::CallFrame;

use crate::{components::header_bar::HeaderBar, url_read::url_read_resource};

use super::common::*;

#[component]
pub fn Activity() -> impl IntoView {
    log::info!("Activity Page");
    let params = use_params_map();
    let url = if let Some(tid) = params.get().get("tid") {
        format!("/apis/callstack?tid={}", tid)
    } else {
        "/apis/callstack".to_string()
    };

    let reply = url_read_resource::<ProbeCall>(url.as_str());

    let callstacks = move || {
        view! {
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || Suspend::new(async move {
                    let callstacks = match reply.await {
                        Ok(ProbeCall::ReturnBacktrace(callstacks)) => callstacks,
                        _other => Default::default(),
                    };
                    log::info!("callstacks: {:?}", callstacks);
                    callstacks
                        .iter()
                        .map(|callstack| {
                            view! { <CallStackView callstack=callstack.clone() /> }
                        })
                        .collect::<Vec<_>>()
                })}

            </Suspense>
        }
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
        <HeaderBar />
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <Space align=SpaceAlign::Center vertical=true class="doc-content">
                <h3>"Call Stacks"</h3>
                <Accordion multiple=true>{callstacks}</Accordion>
            </Space>
        </Layout>
    }
}

#[component]
fn CallStackView(#[prop(into)] callstack: CallFrame) -> impl IntoView {
    match callstack {
        CallFrame::CFrame { .. } => {
            view! {
                <AccordionItem value="C/C++">
                    <AccordionHeader slot>"C/C++ Call Stack"</AccordionHeader>
                    <pre>{"..."}</pre>
                </AccordionItem>
            }
        }
        CallFrame::PyFrame {
            file,
            func,
            lineno,
            locals,
        } => {
            let url = format!("/apis/files?path={}", file.clone());
            // let route_url = format!("/files?path={}", file);
            let key = format!("{func} @ {file}: {lineno}");
            view! {
                <AccordionItem value=key.clone()>
                    <AccordionHeader slot>{key.clone()}</AccordionHeader>
                    <b>"local:"</b>
                    <span style="padding: 5px">
                        {func} "@" <a href=url target="_blank">
                            {file}
                        </a> {":"} {lineno}
                    // <Button on_click=move |_| {
                    // let navigate = leptos_router::use_navigate();
                    // navigate(route_url.as_str(), Default::default());
                    // }>
                    // <Icon icon=icondata::BiFileRegular/>
                    // </Button>
                    </span>
                    <ValueList variables=locals />
                </AccordionItem>
            }
        }
    }
}

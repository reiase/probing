use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use thaw::*;

use probing_proto::protocol::process::CallFrame;

use crate::components::page_layerout::PageLayout;
use crate::url_read::url_read_resource;

use super::common::*;

#[component]
pub fn Activity() -> impl IntoView {
    log::info!("Activity Page");
    let params = use_params_map();
    let url = if let Some(tid) = params.get().get("tid") {
        format!("/apis/pythonext/callstack?tid={}", tid)
    } else {
        "/apis/pythonext/callstack".to_string()
    };

    let reply = url_read_resource::<Vec<CallFrame>>(url.as_str());

    let callstacks = move || {
        view! {
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || Suspend::new(async move {
                    let callstacks = reply.await.unwrap_or_default();
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
        <PageLayout>
            <Space align=SpaceAlign::Center vertical=true class="doc-content">
                <h3>"Call Stacks"</h3>
                <Accordion multiple=true>{callstacks}</Accordion>
            </Space>
        </PageLayout>
    }
}

#[component]
fn CallStackView(#[prop(into)] callstack: CallFrame) -> impl IntoView {
    match callstack {
        CallFrame::CFrame { ip, file, func, lineno } => {
            let key = format!("{ip}: {func} @ {file}: {lineno}");
            view! {
                <AccordionItem value=key.clone()>
                    <AccordionHeader slot>
                        <Icon icon=icondata::SiCplusplus />
                        <span style="margin-left: 8px;">{key.clone()}</span>
                    </AccordionHeader>
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
                    <AccordionHeader slot>
                        <Icon icon=icondata::SiPython />
                        <span style="margin-left: 8px;">{key.clone()}</span>
                    </AccordionHeader>
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

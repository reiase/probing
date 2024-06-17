use gloo_net::http::Request;
use leptonic::prelude::*;
use leptos::*;
use leptos_router::use_query_map;

#[component]
pub fn Profiler() -> impl IntoView {
    let params = use_query_map();
    let mid = params.get().get("mid").cloned();

    if let Some(mid) = mid {
        let profile = create_resource(
            move || mid.clone(),
            move |mid| async move {
                let resp = Request::get(format!("/apis/profile?mid={}", mid).as_str())
                    .send()
                    .await
                    .map_err(|err| {
                        logging::log!("error getting callstack: {}", err);
                    })
                    .unwrap()
                    .text()
                    .await
                    .map_err(|err| {
                        logging::log!("error decoding callstack: {}", err);
                    })
                    .ok();
                resp.unwrap_or_default()
            },
        );
        view! {
            <Box style="display: flex; flex-direction: column; align-items: center; min-width: 100%">
                <H2>"Profiler"</H2>
                <pre>{profile.get().unwrap_or_default()}</pre>
            </Box>
        }
    } else {
        view! {
            <Box style="display: flex; flex-direction: column; align-items: center; min-width: 100%">
                <H2>"Profiler"</H2>
                <object data="/flamegraph.svg" style="width: 100%; border: none;"></object>
            </Box>
        }
    }
}

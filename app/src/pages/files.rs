use leptonic::prelude::*;
use leptos::*;
use leptos_router::use_query_map;

use gloo_net::http::Request;


#[component]
pub fn Files() -> impl IntoView {
    let params = use_query_map();
    let path = params.get().get("path").map(|path| path.clone());

    let content = create_resource(move || path.clone(), move |path| async move {
        if path.is_none() {
            logging::log!("no path");
            "".to_string()
        } else {
            let path = path.as_ref().unwrap();
            let resp = Request::get(format!("/apis/files?path={}", path).as_str())
               .send()
               .await
               .map_err(|err| {
                    logging::log!("error getting files: {}", err);
                })
               .unwrap()
               .text()
               .await
               .map_err(|err| {
                    logging::log!("error decoding files: {}", err);
                })
               .ok();
            resp.unwrap_or_default()
        }
    });

    // let code = content.get().unwrap_or("".to_string());
    let code = content.into_view();
    logging::log!("loading code: {:?}", code);
    view! {
        <Box>
        <pre>
            {code}
        </pre>
        </Box>
    }
}

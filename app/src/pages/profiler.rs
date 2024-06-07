use leptonic::prelude::*;
use leptos::*;

#[component]
pub fn Profiler() -> impl IntoView {
    #[cfg(feature="debug")]
    let prefix = "http://127.0.0.1:9922";

    #[cfg(not(feature="debug"))]
    let prefix = "";
    view! {
        <Box style="display: flex; flex-direction: column; align-items: center; min-width: 100%">
            <H2>"Profiler"</H2>
            <object
                data=format!("{}/flamegraph.svg", prefix)
                style="width: 100%; border: none;"
            ></object>
        </Box>
    }
}

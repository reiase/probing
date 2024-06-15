use leptonic::prelude::*;
use leptos::*;

#[component]
pub fn Profiler() -> impl IntoView {
    view! {
        <Box style="display: flex; flex-direction: column; align-items: center; min-width: 100%">
            <H2>"Profiler"</H2>
            <object
                data="/flamegraph.svg"
                style="width: 100%; border: none;"
            ></object>
        </Box>
    }
}

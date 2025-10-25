use leptos::{prelude::*, reactive::wrappers::write::SignalSetter};
use leptos_meta::provide_meta_context;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use thaw::*;

use crate::pages::profiler::Profiler;
use crate::pages::timeseries::Timeseries;
use crate::pages::{activity::Activity, cluster::Cluster, overview::Overview, python::Python};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <ConfigProvider>
            <ToasterProvider>
                <LoadingBarProvider>
                    <TheRouter />
                </LoadingBarProvider>
            </ToasterProvider>
        </ConfigProvider>
    }
}

#[component]
fn TheRouter() -> impl IntoView {
    let loading_bar = LoadingBarInjection::expect_context();
    let is_routing = RwSignal::new(false);
    let set_is_routing = SignalSetter::map(move |is_routing_data| {
        is_routing.set(is_routing_data);
    });

    Effect::watch(
        move || is_routing.get(),
        move |is_routing, _, _| {
            if *is_routing {
                loading_bar.start();
            } else {
                loading_bar.finish();
            }
        },
        false,
    );

    view! {
        <Router set_is_routing>
            <Routes fallback=|| "404">
                <Route path=path!("/") view=Overview />
                <Route path=path!("/cluster") view=Cluster />
                <Route path=path!("/activity") view=Activity />
                <Route path=path!("/activity/:tid") view=Activity />
                // <Route path="/debug" view=|| view! { <DebugView/> }/>
                <Route path=path!("/profiler") view=|| view! { <Profiler /> } />
                <Route path=path!("/timeseries") view=|| view! { <Timeseries /> } />
                <Route path=path!("/inspect") view=|| view! { <Python /> } />
            // <Route path="/files" view=|| view! { <Files/> }/>
            </Routes>
        </Router>
    }
}

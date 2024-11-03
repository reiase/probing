use leptos::*;
use leptos_meta::provide_meta_context;
use leptos_router::*;
use thaw::*;


use crate::pages::{activity::Activity, overview::Overview, python::Python};

#[component]
pub fn App() -> impl IntoView {
    let is_routing = create_rw_signal(false);
    let set_is_routing = SignalSetter::map(move |is_routing_data| {
        is_routing.set(is_routing_data);
    });
    provide_meta_context();

    view! {
        <Router set_is_routing>
            // fallback=|| {
            // let mut outside_errors = Errors::default();
            // outside_errors.insert_with_default_key(AppError::NotFound);
            // view! { <ErrorTemplate outside_errors/> }
            // }

            <TheProvider>
                <TheRouter is_routing/>
            </TheProvider>
        </Router>
    }
}

#[component]
fn TheRouter(is_routing: RwSignal<bool>) -> impl IntoView {
    let loading_bar = use_loading_bar();
    _ = is_routing.watch(move |is_routing| {
        if *is_routing {
            loading_bar.start();
        } else {
            loading_bar.finish();
        }
    });

    view! {
        <Routes>
            <Route path="/" view=Overview/>
            <Route path="/activity" view=Activity/>
            <Route path="/activity/:tid" view=Activity/>
            // <Route path="/debug" view=|| view! { <DebugView/> }/>
            // <Route path="/profiler" view=|| view! { <Profiler/> }/>
            <Route path="/inspect" view=|| view! { <Python/> }/>
        // <Route path="/files" view=|| view! { <Files/> }/>
        </Routes>
    }
}

#[component]
fn TheProvider(children: Children) -> impl IntoView {
    fn use_query_value(key: &str) -> Option<String> {
        let query_map = use_query_map();
        query_map.with_untracked(|query| query.get(key).cloned())
    }
    let theme = use_query_value("theme").map_or_else(Theme::light, |name| {
        if name == "light" {
            Theme::light()
        } else if name == "dark" {
            Theme::dark()
        } else {
            Theme::light()
        }
    });
    let theme = create_rw_signal(theme);

    view! {
        <ThemeProvider theme>
            <GlobalStyle/>
            <MessageProvider>
                <LoadingBarProvider>{children()}</LoadingBarProvider>
            </MessageProvider>
        </ThemeProvider>
    }
}

use leptos::prelude::*;

use probing_proto::prelude::*;

use crate::components::card_view::{ProcessCard, ThreadsCard};
use crate::components::page_layerout::PageLayout;
use crate::components::panel::Panel;
use crate::components::tableview::TableView;
use crate::errors::AppError;
use crate::url_read::url_read_resource;

/// Helper function to parse environment variables string into a Table structure.
fn parse_env_vars(env_str: &str) -> Table {
    let names = vec!["name", "value"];
    let rows = env_str
        .lines() // Use lines() for better handling of line endings
        .filter(|line| !line.is_empty()) // Filter out empty lines
        .map(|kv| {
            kv.split_once('=')
                .map(|(name, value)| vec![name.to_string(), value.to_string()])
                .unwrap_or_else(|| vec![String::new(), kv.to_string()]) // Use String::new() for empty name
        })
        .collect::<Vec<_>>();
    Table::new(names, rows)
}

/// Helper component to render data from a resource within a Suspense boundary.
#[component]
fn SuspendedView<T, F, IV>(
    resource: LocalResource<Result<T, AppError>>,
    view_fn: F,
) -> impl IntoView
where
    T: Clone + Default + Sync + Send + 'static,
    F: Fn(T) -> IV + Copy + 'static + std::marker::Send,
    IV: IntoView + 'static,
{
    view! {
        <Suspense fallback=|| {
            view! { <p>"Loading..."</p> }
        }>
            {move || {
                resource
                    .get()
                    .map(|result| match result.take() {
                        Ok(data) => view_fn(data).into_view(),
                        Err(_) => view_fn(T::default()).into_view(),
                    })
            }}
        </Suspense>
    }
}

#[component]
pub fn Overview() -> impl IntoView {
    // Fetch process data once
    let resource = url_read_resource::<Process>("/apis/overview");

    view! {
        <PageLayout>
            // Process Information Panel
            <Panel title="Process Information">
                <SuspendedView resource view_fn=|process| view! { <ProcessCard process /> } />
            </Panel>

            // Threads Information Panel
            <Panel title="Threads Information">
                <SuspendedView
                    resource
                    view_fn=|process| view! { <ThreadsCard threads=process.threads /> }
                />
            </Panel>

            // Environment Variables Panel
            <Panel title="Environment Variables">
                <SuspendedView
                    resource
                    view_fn=|process| view! { <TableView tbl=parse_env_vars(&process.env) /> }
                />
            </Panel>
        </PageLayout>
    }
}

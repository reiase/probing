use leptos::prelude::*;
use leptos_meta::Style;
use thaw::*;

use probing_proto::prelude::*;

use crate::components::card_view::ProcessCard;
use crate::components::card_view::ThreadsCard;
use crate::components::header_bar::HeaderBar;
use crate::components::panel::Panel;
use crate::components::tableview::TableView;
use crate::url_read::url_read_resource;

#[component]
pub fn Overview() -> impl IntoView {
    let resp = url_read_resource::<Process>("/apis/overview");

    let process_info = view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                let process = resp.await.unwrap_or_default();
                view! { <ProcessCard process=process /> }
            })}
        </Suspense>
    };

    let thread_info = view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                let process = resp.await.unwrap_or_default();
                let threads = process.threads;
                view! { <ThreadsCard threads /> }
            })}

        </Suspense>
    };

    let environments = view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                let process = resp.await.unwrap_or_default();
                let names = vec!["name", "value"];
                let rows = process
                    .env
                    .split('\n')
                    .map(|kv| {
                        if let Some((name, value)) = kv.split_once('=') {
                            vec![name.to_string(), value.to_string()]
                        } else {
                            vec!["".to_string(), kv.to_string()]
                        }
                    })
                    .collect::<Vec<_>>();

                view! { <TableView tbl=Table::new(names, rows) /> }
            })}
        </Suspense>
    };

    view! {
        <Style>
            "
            .doc-content {
                display: flex;
                flex-direction: column;
                flex: 1;
                gap: 16px;
                max-width: 100%;
                box-sizing: border-box;
                padding: 0 24px;
            }
            "
        </Style>
        <HeaderBar />
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <Panel title="Process Information">{process_info}</Panel>
            <Panel title="Threads Information">{thread_info}</Panel>
            <Panel title="Environment Variables">{environments}</Panel>
        </Layout>
    }
}

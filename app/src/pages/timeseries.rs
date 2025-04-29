use leptos::prelude::*;
use leptos_meta::Style;

use thaw::*;

use crate::{components::header_bar::HeaderBar, url_read::read_query_resource};

#[component]
pub fn Timeseries() -> impl IntoView {
    let table = read_query_resource("show tables");

    let table = table.get_untracked();
    log::info!("{:?}", table.clone());

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
            <span>"123"</span>
        </Layout>
    }
}
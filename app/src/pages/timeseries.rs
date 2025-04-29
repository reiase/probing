use leptos::prelude::*;
use leptos_meta::Style;

use thaw::*;

use crate::{components::header_bar::HeaderBar, url_read::read_query_resource};
use crate::components::dataframe_view::DataFrameView;

#[component]
pub fn Timeseries() -> impl IntoView {
    let table = read_query_resource("show tables");

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
            <div class="table-container">
                <h2 class="table-title">"数据表列表"</h2>
                <Suspense fallback=move || view! { <p class="loading-text">"加载数据中..."</p> }>
                    {move || Suspend::new(async move {
                        let df = table.await.unwrap_or_default();
                        view! {
                            <DataFrameView df />
                        }  
                    })}
                </Suspense>
            </div>
        </Layout>
    }
}

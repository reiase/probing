use leptos::prelude::*;
use leptos_meta::Style;

use thaw::*;

use crate::components::dataframe_view::DataFrameView;
use crate::{components::header_bar::HeaderBar, url_read::read_query_resource};

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
            .query-form {
                display: flex;
                flex-direction: column;
                flex: 1;
                gap: 16px;
                max-width: 100%;
                box-sizing: border-box;
                padding: 0 24px;
            }
            .query-actions {
                display: flex;
                justify-content: flex-end;
            }
            .query-editor {
                min-height: 120px;
                font-family: monospace;
                padding: 12px;
                border-radius: 4px;
            }
            "
        </Style>
        <HeaderBar />
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <div class="doc-container">
                <h2>"数据表列表"</h2>
                <Suspense fallback=move || view! { <p class="loading-text">"加载数据中..."</p> }>
                    {move || Suspend::new(async move {
                        let df = table.await.unwrap_or_default();
                        view! {
                            <DataFrameView df />
                        }
                    })}
                </Suspense>
            </div>

            <div class="doc-container">
                <h2>"SQL 查询工具"</h2>
                <SqlQueryPanel />
            </div>
        </Layout>
    }
}

#[component]
fn SqlQueryPanel() -> impl IntoView {
    // State for the SQL query
    let (sql, set_sql) = RwSignal::new(String::new()).split();

    // Create a trigger to execute the query when the button is clicked
    let (query_trigger, set_query_trigger) = RwSignal::new(0).split();

    // Create a resource that depends on both the SQL and the trigger
    let query_resource = Resource::new(
        move || query_trigger.get(),
        move |_| async move {
            let query_text = sql.get();
            if query_text.trim().is_empty() {
                return Err("请输入 SQL 查询".to_string());
            }
            Ok(read_query_resource(&query_text).await)
        },
    );

    // Function to execute the query
    let run_query = move |_| {
        set_query_trigger.update(|count| *count += 1);
    };

    view! {
        <Card>
            <div class="query-form">
                <Textarea
                    class="query-editor"
                    placeholder="输入 SQL 查询，例如: SELECT * FROM table_name LIMIT 10"
                    prop:value=move || sql.get()
                    on:input=move |ev| {
                        set_sql.set(event_target_value(&ev));
                    }
                />
                <div class="query-actions">
                    <Button
                        appearance=ButtonAppearance::Primary
                        on:click=run_query
                    >
                        "执行查询"
                    </Button>
                </div>
            </div>

            <div>
                <Suspense fallback=move || {
                    if query_trigger.get() > 0 {
                        view! { <p class="loading-text">"执行查询中..."</p> }
                    } else {
                        view! { <p class="loading-text">"输入 SQL 语句并点击执行查询"</p> }
                    }
                }>
                {move || Suspend::new(async move {
                    let df = match query_resource.await {
                        Ok(df) => df.unwrap_or_default(),
                        Err(_) => Default::default(),
                    };
                    view! {
                        <DataFrameView df />
                    }
                })}
                </Suspense>
            </div>
        </Card>
    }
}

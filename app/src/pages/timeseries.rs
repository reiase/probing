use leptos::prelude::*;
use leptos_meta::Style;
use thaw::*;

use crate::components::dataframe_view::{DataFrameChartView, DataFrameView};
use crate::components::page_layerout::PageLayout;
use crate::errors::AppError;
use crate::url_read::read_query_resource;

#[component]
pub fn Timeseries() -> impl IntoView {
    let table = read_query_resource("show tables");

    view! {
        <Style>
            "
            .timeseries-page-content {
                display: flex;
                flex-direction: column;
                flex: 1;
                gap: 24px;
                max-width: 100%;
                box-sizing: border-box;
            }
            
            .section-container {
                padding: 16px;
                border: 1px solid #e0e0e0;
                border-radius: 4px;
                background-color: #fff;
            }
            
            .loading-text {
                color: #757575;
                padding: 16px;
            }
            .error-text {
                color: red;
                padding: 16px;
                white-space: pre-wrap;
            }
            
            .query-form {
                display: flex;
                flex-direction: column;
                gap: 16px;
                width: 100%;
            }
            .query-actions {
                display: flex;
                justify-content: flex-end;
            }
            .query-editor {
                min-height: 120px;
                font-family: monospace;
                width: 100%;
                padding: 12px;
                border-radius: 4px;
                border: 1px solid #ccc;
            }
            "
        </Style>
        <PageLayout>
            <Space vertical=true>
                <div class="section-container">
                    <h2>"数据表列表"</h2>
                    <Suspense fallback=|| {
                        view! { <p class="loading-text">"加载数据中..."</p> }
                    }>
                        {move || Suspend::new(async move {
                            match table.await {
                                Ok(df) => view! { <DataFrameView df /> }.into_any(),
                                Err(e) => {
                                    view! {
                                        <p class="error-text">
                                            {format!("加载表失败: {}", e)}
                                        </p>
                                    }
                                        .into_any()
                                }
                            }
                        })}
                    </Suspense>
                </div>

                <div class="section-container">
                    <h2>"查询工具"</h2>
                    <SqlQueryPanel />
                </div>
            </Space>
        </PageLayout>
    }
}

#[component]
fn SqlQueryPanel() -> impl IntoView {
    let (sql, set_sql) = RwSignal::new(String::new()).split();
    let (query_trigger, set_query_trigger) = RwSignal::new(0).split();

    let query_resource = Resource::new(
        move || query_trigger.get(),
        move |trigger_value| async move {
            if trigger_value == 0 {
                return Err(AppError::QueryError(
                    "请在上方输入 SQL 查询并点击执行".to_string(),
                ));
            }

            let query_text = sql.get();
            if query_text.trim().is_empty() {
                return Err(AppError::QueryError("请输入 SQL 查询".to_string()));
            }

            read_query_resource(&query_text)
                .await
                .map_err(|e| AppError::QueryError(format!("查询执行失败: {}", e)))
        },
    );

    let run_query = move |_| set_query_trigger.update(|count| *count += 1);

    view! {
        <div class="query-form">
            <Textarea
                class="query-editor"
                placeholder="输入 SQL 查询，例如: SELECT * FROM table_name LIMIT 10"
                prop:value=move || sql.get()
                on:input=move |ev| set_sql.set(event_target_value(&ev))
            />
            <div class="query-actions">
                <Button appearance=ButtonAppearance::Primary on:click=run_query>
                    "执行查询"
                </Button>
            </div>
        </div>

        <div style="margin-top: 16px;">
            <Suspense fallback=move || {
                let is_loading = query_trigger.get() > 0;
                let message = if is_loading {
                    "执行查询中..."
                } else {
                    "等待执行查询..."
                };
                view! { <p class="loading-text">{message}</p> }
            }>
                {move || match query_resource.get() {
                    Some(Ok(df)) => {
                        let df1 = df.clone();
                        view! {
                            <DataFrameChartView df=df1 />
                            <DataFrameView df />
                        }
                            .into_any()
                    }
                    Some(Err(e)) => view! { <p class="error-text">{e.to_string()}</p> }.into_any(),
                    None => view! { <p class="loading-text">"加载中..."</p> }.into_any(),
                }}
            </Suspense>
        </div>
    }
}

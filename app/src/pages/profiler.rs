use leptos::prelude::*;
use leptos_meta::Style;

use thaw::*;

use crate::components::header_bar::HeaderBar;

#[component]
pub fn Profiler() -> impl IntoView {
    view! {
        <Style>
            "
            .doc-content {
                margin: 0 auto;
                width: 100%;
                display: grid;
            }
            @media screen and (max-width: 1200px) {
                .doc-content {
                    width: 100%;
                }
            }
            "
        </Style>
        <HeaderBar />
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <Space align=SpaceAlign::Center vertical=true class="doc-content">
                <h3>"Torch Profiler"</h3>
                <object data="/apis/flamegraph" style="width: 100%; border: none;"></object>
            </Space>
        </Layout>
    }
}

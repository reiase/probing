use leptos::prelude::*;
use leptos_meta::Style;

use thaw::*;

use crate::components::header_bar::HeaderBar;

#[component]
pub fn Profiler() -> impl IntoView {
    let selected_tab = RwSignal::new(String::from("pprof"));
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
                <TabList selected_value=selected_tab>
                    <Tab value="pprof">"PProf"</Tab>
                    <Tab value="torch">"Torch"</Tab>
                </TabList>
                <div>
                    {move || {
                        let selected = selected_tab.read();
                        let selected = format!("{}", selected);
                        let url = match selected.as_str() {
                            "pprof" => "/apis/flamegraph/pprof",
                            "torch" => "/apis/flamegraph/torch",
                            _ => "",
                        };
                        view! {
                            <div>
                                <h3>
                                    {move || {
                                        match selected.clone().as_str() {
                                            "pprof" => "PProf Profiler",
                                            "torch" => "Torch Profiler",
                                            _ => "Unknown Profiler",
                                        }
                                    }}
                                </h3>
                                <object
                                    data=move || { url }
                                    style="width: 100%; border: none;"
                                ></object>
                            </div>
                        }
                    }}
                </div>
            </Space>
        </Layout>
    }
}

use leptos::prelude::*;

use thaw::*;

use crate::components::page_layerout::PageLayout;

#[component]
pub fn Profiler() -> impl IntoView {
    let selected_tab = RwSignal::new(String::from("pprof"));
    view! {
        <PageLayout>
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
        </PageLayout>
    }
}

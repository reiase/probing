use leptos::prelude::*;

use thaw::*;

use crate::{components::page_layerout::PageLayout, url_read::read_query_resource};

#[component]
pub fn Profiler() -> impl IntoView {
    let selected_tab = RwSignal::new(String::from("pprof"));

    let config = read_query_resource(
        "select name, value from information_schema.df_settings where name like 'probing.%';",
    );

    let pprof_enabled = RwSignal::new(false); // pprof profiler 的开关状态
    let pprof_freq = RwSignal::new("".to_string());

    let torch_enabled = RwSignal::new(false); // torch profiler 的开关状态
    let torch_ratio = RwSignal::new("".to_string());

    let _ = Effect::new_sync(
        move || {
        match config.get().as_deref() {
        Some(Ok(df)) => {
            assert!(df.names[0] == "name");
            assert!(df.names[1] == "value");

            for ele in df.iter() {
                match ele[0].to_string().as_str() {
                    "probing.pprof.sample_freq" => {
                        if ele[1].to_string() != "" {
                            pprof_enabled.set(true);
                            pprof_freq.set(ele[1].to_string());
                        }
                    }
                    "probing.torch.sample_ratio" => {
                        if ele[1].to_string() != "" {
                            torch_enabled.set(true);
                            torch_ratio.set(ele[1].to_string());    
                        }
                    }
                    _ => {}
                }
            }
            log::info!("config: {:?}", df);
        }
        Some(Err(e)) => {
            log::warn!("Failed to read config: {}", e);
        }
        None => {
            log::warn!("Failed to read probing config");
        }};
    });

    view! {
        <PageLayout>
            <Divider />
            <Flex class="doc-content" style="width: 100%">
                <NavDrawer selected_value=selected_tab>
                    <NavItem icon=icondata::CgPerformance value="pprof">
                        "Pprof Profiling"
                    </NavItem>
                    <NavItem icon=icondata::SiPytorch value="torch">
                        "Torch Profiling"
                    </NavItem>
                    <NavItem
                        icon=icondata::AiGithubOutlined
                        value="github"
                        href="https://github.com/reiase/probing"
                        attr:target="_blank"
                    >
                        "Github"
                    </NavItem>
                </NavDrawer>

                // <Flex align=FlexAlign::Start vertical=false class="doc-content" style="width: 100%">
                // <Field label="PProf Sample Frequency" orientation=FieldOrientation::Horizontal>
                // <Input value=pprof_freq />
                // </Field>
                // <Field label="PProf Profiler" orientation=FieldOrientation::Horizontal>
                // <Switch checked=pprof_enabled />
                // </Field>
                // <Divider vertical=true />
                // <Field label="Torch Sample Ratio" orientation=FieldOrientation::Horizontal>
                // <Input value=torch_ratio />
                // </Field>
                // <Field label="Torch Profiler" orientation=FieldOrientation::Horizontal>
                // <Switch checked=torch_enabled />
                // </Field>
                // </Flex>
                // <TabList selected_value=selected_tab>
                // {move || {
                // if pprof_enabled.get() {
                // view! { <Tab value="pprof">"PProf"</Tab> }.into_any()
                // } else {
                // view! { <></> }.into_any()
                // }
                // }}
                // {move || {
                // if torch_enabled.get() {
                // view! { <Tab value="torch">"Torch"</Tab> }.into_any()
                // } else {
                // view! { <></> }.into_any()
                // }
                // }}
                // </TabList>
                <Flex align=FlexAlign::Center vertical=true class="doc-content" style="width: 100%">
                    {move || {
                        if !pprof_enabled.get() && !torch_enabled.get() {
                            return view! {
                                <div>
                                    "No profilers are currently enabled. Enable a profiler using the switches above."
                                </div>
                            }
                                .into_any();
                        }
                        let selected = selected_tab.read();
                        let selected = format!("{}", selected);
                        let active_profiler = match selected.as_str() {
                            "pprof" if pprof_enabled.get() => "pprof",
                            "torch" if torch_enabled.get() => "torch",
                            _ => {
                                if pprof_enabled.get() {
                                    selected_tab.set("pprof".to_string());
                                    "pprof"
                                } else if torch_enabled.get() {
                                    selected_tab.set("torch".to_string());
                                    "torch"
                                } else {
                                    ""
                                }
                            }
                        };
                        if active_profiler.is_empty() {
                            return // 确保选中的 profiler 是已启用的，否则自动选择另一个

                            view! { <></> }
                                .into_any();
                        }
                        let url = match active_profiler {
                            "pprof" => "/apis/flamegraph/pprof",
                            "torch" => "/apis/flamegraph/torch",
                            _ => "",
                        };
                        log::info!("Profiler URL: {}", url);
                        let setting_view = match active_profiler {
                            "pprof" => {

                                view! {
                                    <Flex align=FlexAlign::Center>
                                        <Field
                                            label="PProf Sample Frequency"
                                            orientation=FieldOrientation::Horizontal
                                        >
                                            <Input value=pprof_freq />
                                        </Field>
                                        <Field
                                            label="PProf Profiler"
                                            orientation=FieldOrientation::Horizontal
                                        >
                                            <Switch checked=pprof_enabled />
                                        </Field>
                                    </Flex>
                                }
                                    .into_any()
                            }
                            "torch" => {

                                view! {
                                    <Flex align=FlexAlign::Center>
                                        <Field
                                            label="Torch Sample Ratio"
                                            orientation=FieldOrientation::Horizontal
                                        >
                                            <Input value=torch_ratio />
                                        </Field>
                                        <Field
                                            label="Torch Profiler"
                                            orientation=FieldOrientation::Horizontal
                                        >
                                            <Switch checked=torch_enabled />
                                        </Field>
                                    </Flex>
                                }
                                    .into_any()
                            }
                            _ => view! { <></> }.into_any(),
                        };

                        view! {
                            {setting_view}
                            <object data=url style="width: 100%; border: none;"></object>
                        }
                            .into_any()
                    }}
                </Flex>
            </Flex>
        </PageLayout>
    }.into_view()
}

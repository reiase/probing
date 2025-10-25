use leptos::prelude::*;
use thaw::*;

use probing_proto::prelude::Value;

use crate::components::page_layerout::PageLayout;
use crate::{pages::common::ObjectList, url_read::url_read_resource};

// mod module;
mod object;

#[component]
pub fn SelectedObjectList(
    #[prop(into)] selected: RwSignal<String>,
    #[prop(into)] limits: RwSignal<Option<i32>>,
) -> impl IntoView {
    // 创建一个响应式资源，当selected或limits改变时重新获取数据
    let objects_resource = Resource::new(
        move || (selected.get(), limits.get()),
        move |(selected_type, limit_value)| async move {
            let path = match selected_type.as_str() {
                "Python" => "/objects",
                "Tensor" => "/torch/tensors",
                "Module" => "/torch/modules",
                _ => "/objects",
            };

            let limit = limit_value.unwrap_or(-1);
            let url = if limit > 0 {
                format!("{path}?limit={}", limit)
            } else {
                path.to_string()
            };

            log::info!("Fetching objects from: {}", url);
            url_read_resource::<Vec<Value>>(url.as_str()).await
        },
    );

    view! {
        <Suspense fallback=move || {
            view! {
                <div style="padding: 20px; text-align: center;">
                    <p>"Loading {selected.get()} objects..."</p>
                </div>
            }
        }>
            {move || {
                match objects_resource.get() {
                    Some(Ok(objects)) => {
                        let count = objects.len();
                        let selected_type = selected.get();
                        if objects.is_empty() {
                            view! {
                                <div style="padding: 20px; text-align: center; color: #666;">
                                    <p>"No {selected_type} objects found"</p>
                                </div>
                            }
                                .into_any()
                        } else {
                            view! {
                                <div style="width: 100%;">
                                    <div style="margin-bottom: 12px; padding: 8px; background: #f5f5f5; border-radius: 4px; text-align: center;">
                                        <strong>{count}</strong>
                                        " {selected_type} objects found"
                                    </div>
                                    <ObjectList objects />
                                </div>
                            }
                                .into_any()
                        }
                    }
                    Some(Err(error)) => {
                        view! {
                            <div style="padding: 20px; color: red; border: 1px solid red; border-radius: 4px; margin: 8px;">
                                <p>
                                    <strong>"Error loading {selected.get()} objects: "</strong>
                                    {error.to_string()}
                                </p>
                            </div>
                        }
                            .into_any()
                    }
                    None => {
                        view! {
                            <div style="padding: 20px; text-align: center;">
                                <p>"Loading..."</p>
                            </div>
                        }
                            .into_any()
                    }
                }
            }}
        </Suspense>
    }
}

#[component]
pub fn Python() -> impl IntoView {
    let limits = RwSignal::new(Some(100));
    let selected_type = RwSignal::new(String::from("Python"));
    let search_query = RwSignal::new(String::new());
    let limit_string = RwSignal::new("100".to_string());
    let refresh_trigger = RwSignal::new(0);

    // 当选择类型或限制改变时，触发刷新
    Effect::new(move |_| {
        selected_type.get();
        limits.get();
        refresh_trigger.update(|x| *x += 1);
    });

    let refresh_data = move |_| {
        refresh_trigger.update(|x| *x += 1);
        log::info!("Refreshing object data for type: {}", selected_type.get());
    };

    view! {
        <PageLayout>
            <Space vertical=true>
                <Flex align=FlexAlign::Center justify=FlexJustify::SpaceBetween>
                    <div>
                        <h2>"Python Object Inspector"</h2>
                        <p>"Inspect Python objects, tensors, and models in the target process"</p>
                    </div>
                    <Button appearance=ButtonAppearance::Primary on_click=refresh_data>
                        <Icon icon=icondata::AiReloadOutlined />
                        "Refresh"
                    </Button>
                </Flex>

                <div style="background: #f8f9fa; border-radius: 8px; padding: 16px; margin-bottom: 20px;">
                    <Flex align=FlexAlign::Center gap=FlexGap::Medium>
                        <Flex align=FlexAlign::Center gap=FlexGap::Small>
                            <span>"Object Type: "</span>
                            <Select value=selected_type>
                                <option value="Python">"Python Objects"</option>
                                <option value="Tensor">"Tensors"</option>
                                <option value="Module">"Models"</option>
                            </Select>
                        </Flex>

                        <Flex align=FlexAlign::Center gap=FlexGap::Small>
                            <span>"Limit: "</span>
                            <Select value=limit_string>
                                <option value="10">"10"</option>
                                <option value="100">"100"</option>
                                <option value="1000">"1000"</option>
                                <option value="-1">"ALL"</option>
                            </Select>
                        </Flex>

                        <Flex align=FlexAlign::Center gap=FlexGap::Small>
                            <span>"Search: "</span>
                            <Input placeholder="Search objects..." value=search_query />
                        </Flex>
                    </Flex>
                </div>

                <div style="border: 1px solid #e0e0e0; border-radius: 8px; padding: 20px; background: white;">
                    <Flex align=FlexAlign::Center gap=FlexGap::Medium style="margin-bottom: 16px;">
                        {move || {
                            let icon = match selected_type.get().as_str() {
                                "Python" => icondata::SiPython,
                                "Tensor" => icondata::SiPytorch,
                                "Module" => icondata::AiCodeOutlined,
                                _ => icondata::SiPython,
                            };
                            view! {
                                <Icon icon=icon style="font-size: 24px;" />
                                <h3 style="margin: 0;">{selected_type.get()}</h3>
                            }
                        }}
                    </Flex>

                    <SelectedObjectList selected=selected_type limits />
                </div>

            </Space>
        </PageLayout>
    }
}

#[component]
fn ObjectSelector(
    #[prop(into)] set_selected: WriteSignal<&'static str>,
    #[prop(into)] selected: &'static str,
    #[prop(into)] target: &'static str,
) -> impl IntoView {
    let on_click = move |_| {
        set_selected.update(|x| *x = target);
    };
    if selected.eq(target) {
        view! {
            <Button on_click appearance=ButtonAppearance::Primary>
                {target}
            </Button>
        }
    } else {
        view! {
            <Button on_click appearance=ButtonAppearance::Secondary>
                {target}
            </Button>
        }
    }
}

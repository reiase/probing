use probing_dpp::Object;
use leptos::*;

use leptos_meta::Style;
use thaw::*;

use crate::{
    components::header_bar::HeaderBar, pages::common::ObjectList, url_read::url_read_resource,
};

// mod module;
mod object;

#[component]
pub fn SelectedObjectList(
    #[prop(into)] selected: String,
    #[prop(into)] limits: RwSignal<Option<i32>>,
) -> impl IntoView {
    let path = match selected.as_str() {
        "Python" => "/objects",
        "Tensor" => "/torch/tensors",
        "Module" => "/torch/modules",
        _ => "/objects",
    };
    let limits = limits.get().unwrap_or(-1);
    let path = if limits > 0 {
        format!("{path}?limit={}", limits)
    } else {
        path.to_string()
    };
    let objects = url_read_resource::<Vec<Object>>(path.as_str());
    let objects = move || {
        objects.and_then(|objs| objs.clone()).map(|x| x.ok()).flatten().map(|objects| {
            view! {
                <Space>
                    <ObjectList objects/>
                </Space>
            }
        }).unwrap_or(view! { <Space>"no python objects"</Space> })
    };
    view! { {objects} }
}

#[component]
pub fn Python() -> impl IntoView {
    let limits = create_rw_signal(Some((100)));
    let selected = create_rw_signal(String::from("Python"));

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
        <HeaderBar/>
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <h3>Object Inspection</h3>

            <Space align=SpaceAlign::Center>
                <span>"limits: "</span>
                <Select
                    value=limits
                    options=vec![
                        SelectOption::new("10", 10),
                        SelectOption::new("100", 100),
                        SelectOption::new("1000", 1000),
                        SelectOption::new("ALL", -1),
                    ]
                >

                    <SelectLabel slot>"limits:"</SelectLabel>
                </Select>
            </Space>
            <Tabs value=selected>
                <Tab key="Python" label="Python">
                    <div style="width: 100%">
                        <SelectedObjectList selected="Python".to_string() limits/>
                    </div>
                </Tab>
                <Tab key="Tensor" label="Tensor">
                    <div style="width: 100%">
                        <SelectedObjectList selected="Tensor".to_string() limits/>
                    </div>
                </Tab>
                <Tab key="Module" label="Module">
                    <div style="width: 100%">
                        <SelectedObjectList selected="Module".to_string() limits/>
                    </div>
                </Tab>
            </Tabs>

        </Layout>
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
            <Button on_click color=ButtonColor::Primary>
                {target}
            </Button>
        }
    } else {
        view! {
            <Button on_click color=ButtonColor::Success>
                {target}
            </Button>
        }
    }
}

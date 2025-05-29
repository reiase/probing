use leptos::prelude::*;
use thaw::*;

use probing_proto::prelude::Value;


use crate::components::page_layerout::PageLayout;
use crate::{pages::common::ObjectList, url_read::url_read_resource};

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
    let objects = url_read_resource::<Vec<Value>>(path.as_str());
    view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                objects
                    .await
                    .map(|objects| {
                        view! {
                            <Flex>
                                <ObjectList objects />
                            </Flex>
                        }
                    })
                    .unwrap_or(view! { <Flex>"no python objects"</Flex> })
            })}

        </Suspense>
    }
}

#[component]
pub fn Python() -> impl IntoView {
    let limits = RwSignal::new(Some(100));
    let selected = RwSignal::new(String::from("Python"));

    view! {
        <PageLayout>
            <h3>Object Inspection</h3>

        // <Flex align=SpaceAlign::Center>
        // <span>"limits: "</span>
        // <Select
        // value=limits
        // options=vec![
        // SelectOption::new("10", 10),
        // SelectOption::new("100", 100),
        // SelectOption::new("1000", 1000),
        // SelectOption::new("ALL", -1),
        // ]
        // >

        // <SelectLabel slot>"limits:"</SelectLabel>
        // </Select>
        // </Flex>
        // <TabList selected_value=selected>
        // <Tab value="Python" label="Python">
        // <div style="width: 100%">
        // <SelectedObjectList selected="Python".to_string() limits/>
        // </div>
        // </Tab>
        // <Tab key="Tensor" label="Tensor">
        // <div style="width: 100%">
        // <SelectedObjectList selected="Tensor".to_string() limits/>
        // </div>
        // </Tab>
        // <Tab key="Module" label="Module">
        // <div style="width: 100%">
        // <SelectedObjectList selected="Module".to_string() limits/>
        // </div>
        // </Tab>
        // </TabList>
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

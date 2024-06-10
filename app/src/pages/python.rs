use leptonic::prelude::*;
use leptos::*;
use leptos_struct_table::*;

use gloo_net::http::Request;
use probe_common::Object;
use serde_json;

mod module;
mod object;
mod tensor;

#[component]
pub fn Python() -> impl IntoView {
    let (selected, set_selected) = create_signal("Python");
    let (limits, set_limits) = create_signal(100);

    let objects = create_resource(
        move || (selected.get(), limits.get()),
        move |(selected, limits)| async move {
            let path = match selected {
                "Python" => "/objects",
                "Tensor" => "/torch/tensors",
                "Module" => "/torch/modules",
                _ => "/objects",
            };
            let path = if limits > 0 {
                format!("{path}?limit={limits}")
            } else {
                format!("{path}")
            };
            let resp = Request::get(path.as_str())
                .send()
                .await
                .map_err(|err| {
                    eprintln!("error getting overview: {}", err);
                })
                .unwrap()
                .text()
                .await
                .ok();
            resp.unwrap_or("".to_string())
        },
    );

    let object_info = move || {
        let selected = selected.get();
        match selected {
            "Tensor" => {
                view! { <tensor::TensorList text=objects.get()></tensor::TensorList> }
            }
            "Module" => {
                view! { <module::ModuList text=objects.get()></module::ModuList> }
            }
            _ => {
                view! { <object::ObjectList text=objects.get()></object::ObjectList> }
            }
        }
    };

    view! {
        <div>
            <H3>Object Inspection</H3>
        </div>
        <Stack orientation=StackOrientation::Horizontal spacing=Size::Em(1.0) style="float: left;">
            <span>"Select object to inspect: "</span>
            <ButtonGroup>
                {move || {
                    view! {
                        <ObjectSelector set_selected selected=selected.get() target="Python"/>
                        <ObjectSelector set_selected selected=selected.get() target="Tensor"/>
                        <ObjectSelector set_selected selected=selected.get() target="Module"/>
                    }
                }}

            </ButtonGroup>
            <span>"limits: "</span>
            <Select
                options=vec![10, 100, 1000, -1]
                search_text_provider=move |o| {
                    if o != -1 { format!("{o}") } else { String::from("All") }
                }

                render_option=move |o| if o != -1 { format!("{o}") } else { String::from("All") }
                selected=limits
                set_selected=move |v| set_limits.set(v)
            />
        </Stack>
        <div style="width: 100%">{object_info}</div>
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
            <Button on_click=on_click color=ButtonColor::Primary>
                {target}
            </Button>
        }
    } else {
        view! {
            <Button on_click=on_click color=ButtonColor::Secondary>
                {target}
            </Button>
        }
    }
}

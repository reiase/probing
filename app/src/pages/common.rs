use std::collections::HashMap;

use leptos::*;
use leptos_struct_table::*;

use dpp::Object;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectKind {
    Object,
    Tensor,
    Module,
}

#[derive(TableRow, Clone)]
#[table(impl_vec_data_provider)]
pub struct VariableView {
    id: u64,
    name: String,
    value: String,
}

#[component]
pub fn VariablesList(#[prop(into)] variables: HashMap<String, Object>) -> impl IntoView {
    let rows = variables
        .iter()
        .map(|kv| VariableView {
            id: kv.1.id,
            name: kv.0.clone(),
            value: match &kv.1.value {
                Some(v) => v.clone(),
                None => "None".to_string(),
            },
        })
        .collect::<Vec<_>>();
    view! {
        <table>
            <TableContent rows/>
        </table>
    }
}

#[derive(TableRow, Clone)]
#[table(impl_vec_data_provider)]
pub struct ObjectView {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
    pub value: Option<String>,
}

#[component]
pub fn ObjectList(#[prop(into)] objects: Vec<Object>) -> impl IntoView {
    let rows = objects
        .iter()
        .map(|obj| ObjectView {
            id: obj.id,
            class: obj.class.clone(),
            shape: obj.shape.clone(),
            dtype: obj.dtype.clone(),
            device: obj.device.clone(),
            value: obj.value.clone(),
        })
        .collect::<Vec<_>>();

    view! {
        <table>
            <TableContent rows/>
        </table>
    }
}

// #[component]
// pub fn ModuleView(#[prop(into)] obj: Object) -> impl IntoView {
//     let id = obj.id;
//     let value = obj.value.clone();
//     let device = move || {
//         let device = obj.device.clone();
//         if let Some(device) = device {
//             view! {
//                 <Box>
//                     <Chip>{device}</Chip>
//                 </Box>
//             }
//         } else {
//             view! { <Box>""</Box> }
//         }
//     };
//     let act1 = move |_| {
//         let url1 = format!("/apis/start_profile?mid={}&steps=1", id);
//         spawn_local(async move {
//             let url = url1.clone();
//             let _ = Request::get(url.as_str()).send().await;
//         });
//         let route_url = format!("/profiler?mid={}", id);
//         let navigate = leptos_router::use_navigate();
//         navigate(route_url.as_str(), Default::default());
//     };
//     let act5 = move |_| {
//         let url5 = format!("/apis/start_profile?mid={}&steps=5", id);
//         spawn_local(async move {
//             let url = url5.clone();
//             let _ = Request::get(url.as_str()).send().await;
//         });
//         let route_url = format!("/profiler?mid={}", id);
//         let navigate = leptos_router::use_navigate();
//         navigate(route_url.as_str(), Default::default());
//     };
//     let act10 = move |_| {
//         let url10 = format!("/apis/start_profile?mid={}&steps=10", id);
//         spawn_local(async move {
//             let url = url10.clone();
//             let _ = Request::get(url.as_str()).send().await;
//         });
//         let route_url = format!("/profiler?mid={}", id);
//         let navigate = leptos_router::use_navigate();
//         navigate(route_url.as_str(), Default::default());
//     };
//     view! {
//         <pre style="white-space: pre-wrap; word-break: break-word;">{value}</pre>
//         <Button on_press=act1>"profile 1 step"</Button>
//         <Button on_press=act5>"profile 5 steps"</Button>
//         <Button on_press=act10>"profile 10 steps"</Button>
//         {device}
//     }
// }

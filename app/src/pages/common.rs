use std::collections::HashMap;

use leptos::prelude::*;
use thaw::*;

use probing_proto::prelude::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectKind {
    // Object,
    // Tensor,
    // Module,
}

#[derive(Clone)]
pub struct VariableView {
    id: u64,
    name: String,
    value: String,
}

#[component]
pub fn ValueList(#[prop(into)] variables: HashMap<String, Value>) -> impl IntoView {
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
        .map(|v| {
            view! {
                <TableRow>
                    <TableCell>
                        <TableCellLayout truncate=true>{v.id}</TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout truncate=true>{v.name}</TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout>{v.value}</TableCellLayout>
                    </TableCell>
                </TableRow>
            }
        })
        .collect::<Vec<_>>();
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell resizable=true min_width=10.0 max_width=40.0>
                        "#"
                    </TableHeaderCell>
                    <TableHeaderCell resizable=true>"Name"</TableHeaderCell>
                    <TableHeaderCell>"Value"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>{rows}</TableBody>
        </Table>
    }
}

#[derive(Clone)]
pub struct ObjectView {
    pub id: u64,
    pub class: String,
    pub shape: Option<String>,
    pub dtype: Option<String>,
    pub device: Option<String>,
    pub value: Option<String>,
}

#[component]
pub fn ObjectList(#[prop(into)] objects: Vec<Value>) -> impl IntoView {
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
        .map(|v| {
            let device_text = v.device.as_ref().map(|s| s.clone()).unwrap_or_else(|| "N/A".to_string());

            let shape_text = v.shape.as_ref().map(|s| s.clone()).unwrap_or_else(|| "N/A".to_string());
            let dtype_text = v.dtype.as_ref().map(|s| s.clone()).unwrap_or_else(|| "N/A".to_string());
            
            let obj = Value {
                id: v.id,
                class: v.class.clone(),
                shape: v.shape.clone(),
                dtype: v.dtype.clone(),
                device: v.device.clone(),
                value: v.value.clone(),
            };

            view! {
                <TableRow>
                    <TableCell>
                        <TableCellLayout truncate=true>
                            <strong>{v.id}</strong>
                        </TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout truncate=true>
                            <code>{v.class}</code>
                        </TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout>{shape_text}</TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout>
                            <code>{dtype_text}</code>
                        </TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout>{device_text}</TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout truncate=true>
                            <span style="max-width: 200px; overflow: hidden; text-overflow: ellipsis;">
                                {v
                                    .value
                                    .as_ref()
                                    .map(|s| s.clone())
                                    .unwrap_or_else(|| "N/A".to_string())}
                            </span>
                        </TableCellLayout>
                    </TableCell>
                    <TableCell>
                        <TableCellLayout>
                            <Button
                                appearance=ButtonAppearance::Transparent
                                on_click=move |_| {
                                    log::info!("Viewing object: {}", obj.id);
                                }
                            >
                                <Icon icon=icondata::AiEyeOutlined />
                                "View"
                            </Button>
                        </TableCellLayout>
                    </TableCell>
                </TableRow>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <div style="width: 100%; overflow-x: auto;">
            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell resizable=true min_width=60.0 max_width=80.0>
                            "ID"
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=120.0>
                            "Class"
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=100.0>
                            "Shape"
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=80.0>
                            "Dtype"
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=100.0>
                            "Device"
                        </TableHeaderCell>
                        <TableHeaderCell resizable=true>"Value"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=120.0>
                            "Actions"
                        </TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>{rows}</TableBody>
            </Table>
        </div>
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

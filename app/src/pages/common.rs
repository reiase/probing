use std::collections::HashMap;

use leptonic::components::prelude::*;
use leptos::*;

use gloo_net::http::Request;
use ppp::Object;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectKind {
    Object,
    Tensor,
    Module,
}

#[component]
pub fn VariablesView(#[prop(into)] variables: HashMap<String, Object>) -> impl IntoView {
    let header = view! {
        <TableRow>
            <TableHeaderCell min_width=true>"#"</TableHeaderCell>
            <TableHeaderCell>"Name"</TableHeaderCell>
            <TableHeaderCell>"Value"</TableHeaderCell>
        </TableRow>
    };
    let body = variables
        .iter()
        .map(|(name, obj)| {
            let id = obj.id;
            let name = name.clone();
            let obj = obj.clone();
            view! {
                <TableRow>
                    <TableCell>{id}</TableCell>
                    <TableCell>{name.clone()}</TableCell>
                    <TableCell>
                        <ObjectView obj=obj kind=ObjectKind::Object/>
                    </TableCell>
                </TableRow>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <TableContainer>
            <Table bordered=true hoverable=true>
                <TableHeader>{header}</TableHeader>
                <TableBody>{body}</TableBody>
            </Table>
        </TableContainer>
    }
}

#[component]
pub fn ObjectView(#[prop(into)] obj: Object, #[prop(into)] kind: ObjectKind) -> impl IntoView {
    if kind == ObjectKind::Tensor {
        return view! { <TensorView obj=obj/> };
    }
    if kind == ObjectKind::Module {
        return view! { <ModuleView obj=obj/> };
    }

    let class = obj.class.clone();
    let value = obj.value.clone();
    let shape = move || {
        let shape = obj.shape.clone();
        if let Some(shape) = shape {
            view! {
                <Box>
                    <Chip>{shape}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    let dtype = move || {
        let dtype = obj.dtype.clone();
        if let Some(dtype) = dtype {
            view! {
                <Box>
                    <Chip>{dtype}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    let device = move || {
        let device = obj.device.clone();
        if let Some(device) = device {
            view! {
                <Box>
                    <Chip>{device}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    view! {
        <Box>
            <span>{value}</span>
            <Chip>{class}</Chip>
            {shape}
            {dtype}
            {device}
        </Box>
    }
}

#[component]
pub fn TensorView(#[prop(into)] obj: Object) -> impl IntoView {
    let class = obj.class.clone();
    let value = obj.value.clone();
    let shape = move || {
        let shape = obj.shape.clone();
        if let Some(shape) = shape {
            view! {
                <Box>
                    <Chip>{shape}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    let dtype = move || {
        let dtype = obj.dtype.clone();
        if let Some(dtype) = dtype {
            view! {
                <Box>
                    <Chip>{dtype}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    let device = move || {
        let device = obj.device.clone();
        if let Some(device) = device {
            view! {
                <Box>
                    <Chip>{device}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    view! {
        <span>{value}</span>
        <Chip>{class}</Chip>
        {shape}
        {dtype}
        {device}
    }
}

#[component]
pub fn ModuleView(#[prop(into)] obj: Object) -> impl IntoView {
    let id = obj.id;
    let value = obj.value.clone();
    let device = move || {
        let device = obj.device.clone();
        if let Some(device) = device {
            view! {
                <Box>
                    <Chip>{device}</Chip>
                </Box>
            }
        } else {
            view! { <Box>""</Box> }
        }
    };
    let act1 = move |_| {
        let url1 = format!("/apis/start_profile?mid={}&steps=1", id);
        spawn_local(async move {
            let url = url1.clone();
            let _ = Request::get(url.as_str()).send().await;
        });
        let route_url = format!("/profiler?mid={}", id);
        let navigate = leptos_router::use_navigate();
        navigate(route_url.as_str(), Default::default());
    };
    let act5 = move |_| {
        let url5 = format!("/apis/start_profile?mid={}&steps=5", id);
        spawn_local(async move {
            let url = url5.clone();
            let _ = Request::get(url.as_str()).send().await;
        });
        let route_url = format!("/profiler?mid={}", id);
        let navigate = leptos_router::use_navigate();
        navigate(route_url.as_str(), Default::default());
    };
    let act10 = move |_| {
        let url10 = format!("/apis/start_profile?mid={}&steps=10", id);
        spawn_local(async move {
            let url = url10.clone();
            let _ = Request::get(url.as_str()).send().await;
        });
        let route_url = format!("/profiler?mid={}", id);
        let navigate = leptos_router::use_navigate();
        navigate(route_url.as_str(), Default::default());
    };
    view! {
        <pre style="white-space: pre-wrap; word-break: break-word;">{value}</pre>
        <Button on_press=act1>"profile 1 step"</Button>
        <Button on_press=act5>"profile 5 steps"</Button>
        <Button on_press=act10>"profile 10 steps"</Button>
        {device}
    }
}

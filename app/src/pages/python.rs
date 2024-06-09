use leptonic::prelude::*;
use leptos::*;

#[component]
pub fn Python() -> impl IntoView {
    let (selectd, set_selected) = create_signal("Python");
    view !{
        <Stack 
            orientation=StackOrientation::Horizontal
            spacing=Size::Em(1.0) 
            style="margin-left: 2em"
        >
            <ButtonGroup>
                {move || {
                    if selectd.get() == "Python" {
                        view! {<Button on_click=move |_| {} color=ButtonColor::Primary>"Python"</Button>}
                    } else {
                        view! {<Button on_click=move |_| {} color=ButtonColor::Secondary>"Python"</Button>}
                    }
                }}
                <Button on_click=move |_| {}>"Python"</Button>
                <Button on_click=move |_| { set_selected.update(|x| *x = "Tensor"); }>"torch.Tensors"</Button>
                <Button on_click=move |_| {}>"Modules"</Button>
            </ButtonGroup>
        </Stack>
        <div>
            <H2>"Python"</H2>
        </div>
    }
}
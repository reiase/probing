use leptonic::prelude::*;
use leptos::*;
use leptos_meta::{provide_meta_context, Meta, Stylesheet, Title};
use leptos_router::*;

use crate::{
    error_template::{AppError, ErrorTemplate},
    pages::welcome::Welcome,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let (count, set_count) = create_signal(0);

    let f = move |_| {
        set_count.update(|c| *c += 1);
    };
    view! {
        <Meta name="charset" content="UTF-8"/>
        <Meta name="description" content="Leptonic CSR template"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>
        <Meta name="theme-color" content="#e66956"/>

        <Stylesheet id="leptos" href="/pkg/leptonic-template-ssr.css"/>
        <Stylesheet href="https://fonts.googleapis.com/css?family=Roboto&display=swap"/>

        <Title text="Probe"/>

        <Root default_theme=LeptonicTheme::default()>
            <Box style="position: relative; width: 100%; overflow: auto;">
                <AppBar
                    height=Size::Px(36)
                    style="z-index: 1; background: var(--brand-color); color: white; align=left"
                >
                    <Stack
                        orientation=StackOrientation::Horizontal
                        spacing=Size::Em(1.0)
                        style="margin-left: 1em"
                    >
                        <H1 style="margin-left: 1em; color: white;">"Probe"</H1>
                        <Button on_click=f>"Overview"</Button>
                        <Button on_click=f>"Call Stacks"</Button>
                        <Button on_click=f>"Profiling"</Button>
                    </Stack>
                    <Stack
                        orientation=StackOrientation::Horizontal
                        spacing=Size::Em(1.0)
                        style="margin-right: 1em"
                    >
                        <span> {move || count.get()} </span>
                        <Icon icon=icondata::BsGithub/>
                        <Icon icon=icondata::BsPower/>
                    </Stack>
                </AppBar>
            </Box>
            <Box>
                <Router fallback=|| {
                    let mut outside_errors = Errors::default();
                    outside_errors.insert_with_default_key(AppError::NotFound);
                    view! { <ErrorTemplate outside_errors/> }
                }>
                    <Routes>
                        <Route path="" view=|| view! { <Welcome/> }/>
                    </Routes>
                </Router>
            </Box>
        </Root>
    }
}

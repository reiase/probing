use leptos::*;
use leptos_meta::Style;
use leptos_meta::provide_meta_context;
use leptos_router::*;
use thaw::*;

use crate::{
    error_template::{AppError, ErrorTemplate},
    // pages::{
    //     activity::Activity, debug::DebugView, files::Files, overview::Overview, profiler::Profiler,
    //     python::Python,
    // },
    pages::overview::Overview,
};

#[component]
pub fn App() -> impl IntoView {
    let is_routing = create_rw_signal(false);
    let set_is_routing = SignalSetter::map(move |is_routing_data| {
        is_routing.set(is_routing_data);
    });
    provide_meta_context();

    view! {
        <Router
            set_is_routing
            fallback=|| {
                let mut outside_errors = Errors::default();
                outside_errors.insert_with_default_key(AppError::NotFound);
                view! { <ErrorTemplate outside_errors/> }
            }
        >

            <TheProvider>
                <HeaderBar/>
                <TheRouter is_routing/>
            // <Routes>
            // <Route path="/" view=|| view! { <Overview/> }/>
            // // <Route path="/activity" view=|| view! { <Activity/> }/>
            // // <Route path="/activity/:tid" view=|| view! { <Activity/> }/>
            // // <Route path="/debug" view=|| view! { <DebugView/> }/>
            // // <Route path="/profiler" view=|| view! { <Profiler/> }/>
            // // <Route path="/inspect" view=|| view! { <Python/> }/>
            // // <Route path="/files" view=|| view! { <Files/> }/>
            // </Routes>
            </TheProvider>
        </Router>
    }
}

#[component]
fn TheRouter(is_routing: RwSignal<bool>) -> impl IntoView {
    let loading_bar = use_loading_bar();
    _ = is_routing.watch(move |is_routing| {
        if *is_routing {
            loading_bar.start();
        } else {
            loading_bar.finish();
        }
    });

    view! {
        <Routes>
            <Route path="/" view=|| view! { <Overview/> }/>
        </Routes>
    }
}
#[component]
pub fn HeaderBar() -> impl IntoView {
    let navigate = use_navigate();
    let theme = use_rw_theme();
    let theme_name = create_memo(move |_| {
        theme.with(|theme| {
            if theme.name == *"light" {
                "Dark".to_string()
            } else {
                "Light".to_string()
            }
        })
    });
    let change_theme = Callback::new(move |_| {
        if theme_name.get_untracked() == "Light" {
            theme.set(Theme::light());
        } else {
            theme.set(Theme::dark());
        }
    });
    let style = create_memo(move |_| {
        theme.with(|theme| format!("border-bottom: 1px solid {}", theme.common.border_color))
    });
    let menu_value = use_menu_value(change_theme);
    view! {
        <Style id="header-bar">
            "
            .header-bar {
                    height: 64px;
                    display: flex;
                    align-items: center;
                    justify-content: space-between;
                    padding: 0 20px;
            }
            .header-name {
                    cursor: pointer;
                    display: flex;
                    align-items: center;
                    height: 100%;
                    font-weight: 600;
                    font-size: 20px;
                }
            @media screen and (max-width: 1200px) {
                .header-bar-right {
                    display: none !important;
                }
            }
            "
        </Style>
        <LayoutHeader class="header-bar" style>
            <Space on:click=move |_| {
                navigate("/", Default::default());
            }>
                <img src="/logo.png" style="width: 36px"/>
                <div class="header-name">"Probing"</div>
            </Space>
            <Space class="header-bar-right" align=SpaceAlign::Center>
                <Button
                    variant=ButtonVariant::Text
                    on_click=move |_| {
                        let navigate = use_navigate();
                        navigate("/", Default::default());
                    }
                >

                    "Overview"
                </Button>
                <Button
                    variant=ButtonVariant::Text
                    on_click=Callback::new(move |_| change_theme.call(()))
                >
                    {move || theme_name.get()}
                </Button>
                <Button
                    variant=ButtonVariant::Text
                    icon=icondata::AiGithubOutlined
                    round=true
                    style="font-size: 22px; padding: 0px 6px;"
                    on_click=move |_| {
                        _ = window().open_with_url("http://github.com/reiase/probing");
                    }
                />

                <Space>
                    <Popover
                        placement=PopoverPlacement::BottomEnd
                        class="demo-header__menu-popover-mobile"
                    >
                        <PopoverTrigger slot class="demo-header__menu-mobile">
                            <Button
                                variant=ButtonVariant::Text
                                icon=icondata::AiUnorderedListOutlined
                                style="font-size: 22px; padding: 0px 6px;"
                            />
                        </PopoverTrigger>
                        <div style="height: 70vh; overflow: auto;">
                            <Menu value=menu_value>
                                <MenuItem key=theme_name label=theme_name/>
                                <MenuItem key="github" label="Github"/>
                            </Menu>
                        </div>
                    </Popover>
                </Space>
            </Space>
        </LayoutHeader>
    }
}

fn use_menu_value(change_theme: Callback<()>) -> RwSignal<String> {
    let navigate = use_navigate();
    let loaction = use_location();

    let menu_value = create_rw_signal({
        let mut pathname = loaction.pathname.get_untracked();
        if pathname.starts_with("/components/") {
            pathname.drain(12..).collect()
        } else if pathname.starts_with("/guide/") {
            pathname.drain(7..).collect()
        } else {
            String::new()
        }
    });

    _ = menu_value.watch(move |name| {
        if name == "Dark" || name == "Light" {
            change_theme.call(());
            return;
        } else if name == "github" {
            _ = window().open_with_url("http://github.com/reiase/probing");
            return;
        }
        let pathname = loaction.pathname.get_untracked();
    });

    menu_value
}

#[component]
fn TheProvider(children: Children) -> impl IntoView {
    fn use_query_value(key: &str) -> Option<String> {
        let query_map = use_query_map();
        query_map.with_untracked(|query| query.get(key).cloned())
    }
    let theme = use_query_value("theme").map_or_else(Theme::light, |name| {
        if name == "light" {
            Theme::light()
        } else if name == "dark" {
            Theme::dark()
        } else {
            Theme::light()
        }
    });
    let theme = create_rw_signal(theme);

    view! {
        <ThemeProvider theme>
            <GlobalStyle/>
            <MessageProvider>
                <LoadingBarProvider>{children()}</LoadingBarProvider>
            </MessageProvider>
        </ThemeProvider>
    }
}
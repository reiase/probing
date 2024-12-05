use hooks::use_navigate;
use leptos::prelude::*;
use leptos_meta::Style;
use leptos_router::*;
use thaw::*;

#[component]
pub fn HeaderBar() -> impl IntoView {
    let navigate = use_navigate();
    let navigate_signal = RwSignal::new(use_navigate());
    let theme = Theme::use_rw_theme();
    let theme_name = Memo::new(move |_| {
        theme.with(|theme| {
            if theme.name == *"light" {
                "Dark".to_string()
            } else {
                "Light".to_string()
            }
        })
    });
    let change_theme = move |_| {
        if theme_name.get_untracked() == "Light" {
            theme.set(Theme::light());
        } else {
            theme.set(Theme::dark());
        }
    };
    // let style = Memo::new(move |_| {
    //     theme.with(|theme| format!("border-bottom: 1px solid {}", theme.common.border_color))
    // });
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
        <LayoutHeader class="header-bar">
            <Space on:click=move |_| navigate_signal.get()("/", Default::default())>
                <img src="/logo.png" style="width: 36px"/>
                <div class="header-name">"Probing"</div>
            </Space>
            <Space class="header-bar-right" align=SpaceAlign::Center>
                <Button
                    appearance=ButtonAppearance::Transparent
                    on_click=move |_| navigate_signal.get()("/", Default::default())
                >
                    "Overview"
                </Button>
                <Button
                    appearance=ButtonAppearance::Transparent
                    on_click=move |_| navigate_signal.get()("/cluster", Default::default())
                >
                    "Cluster"
                </Button>
                <Button
                    appearance=ButtonAppearance::Transparent
                    on_click=move |_| navigate_signal.get()("/activity", Default::default())
                >
                    "Activity"
                </Button>
                <Button
                    appearance=ButtonAppearance::Transparent
                    on_click=move |_| navigate_signal.get()("/inspect", Default::default())
                >
                    "Inspect"
                </Button>
                <Button appearance=ButtonAppearance::Primary on_click=change_theme>
                    {move || theme_name.get()}
                </Button>
                <Button
                    appearance=ButtonAppearance::Transparent
                    icon=icondata::AiGithubOutlined
                    // style="font-size: 22px; padding: 0px 6px;"
                    on_click=move |_| {
                        _ = window().open_with_url("http://github.com/reiase/probing");
                    }
                />

            </Space>
        </LayoutHeader>
    }
}

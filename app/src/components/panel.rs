use leptos::prelude::*;
use leptos_meta::Style;
use thaw::*;

#[component]
pub fn Panel(
    #[prop(into)] title: Signal<String>,
    #[prop(optional)] children: Option<Children>
) -> impl IntoView {
    let theme = Theme::use_theme(Theme::light);
    let css_vars = Memo::new(move |_| {
        let mut css_vars = String::new();
        theme.with(|theme| {
            if theme.color.color_scheme == "dark" {
                css_vars.push_str("--panel-border-color: #383f52;");
                css_vars.push_str("--panel-background-color: #242832;");
            } else {
                css_vars.push_str("--panel-border-color: var(--colorNeutralStroke2);");
                css_vars.push_str("--panel-background-color: #f9fafb;");
            }
        });
        css_vars
    });

    let styles = use_context::<PanelStyle>().is_none().then(|| {
        view! { <Style id="panel-panel">{include_str!("./panel.css")}</Style> }
    });
    provide_context(PanelStyle);
    view! {
        {styles}
        <div class="panel-panel" style=move || css_vars.get()>
            <div class="panel-panel__head">
                <h4>{title}</h4>
            </div>
            <Divider />
            {if let Some(children) = children {
                view! { <Flex class="panel-panel__view">{children()}</Flex> }
            } else {
                view! { <Flex class="panel-panel__view">""</Flex> }
            }}

        </div>
    }
}

#[derive(Clone)]
pub struct PanelStyle;

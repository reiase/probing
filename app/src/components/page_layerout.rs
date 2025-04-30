// filepath: src/components/page_layout.rs
use leptos::prelude::*;
use leptos_meta::Style;

use thaw::*;
use crate::components::header_bar::HeaderBar;

#[component]
pub fn PageLayout(children: Children) -> impl IntoView {
    view! {
        // Common styles could be defined here or in a global CSS
        <Style>
            "
            .common-page-content {
                display: flex;
                flex-direction: column;
                flex: 1;
                gap: 16px; // Default gap, can be overridden
                max-width: 100%;
                box-sizing: border-box;
                padding: 24px; // Default padding
            }
            "
        </Style>
        <HeaderBar />
        <Layout
            content_style="display: flex; flex-direction: column; flex: 1;" // Basic flex layout
            class="common-page-content" // Apply common class
        >
            {children()}
        </Layout>
    }
}
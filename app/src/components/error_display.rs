use crate::errors::AppError;
use leptos::prelude::*;

#[component]
pub fn ErrorDisplay(#[prop(into)] error: AppError) -> impl IntoView {
    view! {
        <div
            class="error-message"
            style="padding: 16px; color: red; border: 1px solid red; border-radius: 4px; margin: 8px;"
        >
            <p>
                <strong>"Error: "</strong>
                {error.to_string()}
            </p>
        </div>
    }
}

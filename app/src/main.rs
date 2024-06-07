use leptos::*;

mod app;
mod error_template;
mod pages;

use crate::app::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! { <App/> }
    });
}

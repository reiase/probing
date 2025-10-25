use leptos::prelude::*;

mod app;
mod components;
mod errors;
mod pages;
mod url_read;

use crate::app::*;

fn main() {
    let _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! { <App /> }
    });
}

use leptos::prelude::*;
use leptos_meta::Style;

use thaw::*;

use crate::{components::header_bar::HeaderBar, url_read::read_query_resource};

#[component]
pub fn Timeseries() -> impl IntoView {
    let tables = read_query_resource("show tables");
}
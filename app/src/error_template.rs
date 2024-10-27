use leptos_router::use_navigate;
use thaw::*;
use leptos::*;
use leptos_meta::Style;
use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum AppError {
    #[error("Not Found")]
    NotFound,
}

// A basic function to display errors served by the error boundaries.
// Feel free to do more complicated things here than just displaying the error.
#[component]
pub fn ErrorTemplate(
    #[prop(optional)] outside_errors: Option<Errors>,
    #[prop(optional)] errors: Option<RwSignal<Errors>>,
) -> impl IntoView {
    let errors = match outside_errors {
        Some(e) => create_rw_signal(e),
        None => match errors {
            Some(e) => e,
            None => panic!("No Errors found and we expected errors!"),
        },
    };
    // Get Errors from Signal
    let errors = errors.get_untracked();

    // Downcast lets us take a type that implements `std::error::Error`
    let errors: Vec<AppError> = errors
        .into_iter()
        .filter_map(|(_k, v)| v.downcast_ref::<AppError>().cloned())
        .collect();
    println!("Errors: {errors:#?}");

    let num_errors = errors.len();

    view! {
        <Style id="header-bar">
            "
            .header-bar {
                display: flex;
                flex-direction: column;
                align-items:center;
            }
            "
        </Style>
        <Space class="header-bar">
            <h1>
                {match num_errors {
                    1 => "Error",
                    _ => "Errors",
                }}

            </h1>

            <For
                each=move || { errors.clone().into_iter().enumerate() }
                key=|(index, _error)| *index
                children=move |(_index, error)| {
                    match error {
                        AppError::NotFound => view! { <p>"404 - Not Found"</p> },
                    }
                }
            />

            <Button on_click=move |_| {
                let navigate = use_navigate();
                navigate("/", Default::default());
            }>"Back"</Button>
        </Space>
    }
}

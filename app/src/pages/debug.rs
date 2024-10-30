use leptonic::components::prelude::*;
use leptos::*;

use gloo_net::http::Request;
use dpp::DebugState;

#[component]
pub fn DebugView() -> impl IntoView {
    let status = create_resource(
        move || {},
        move |_| async move {
            let resp = Request::get("/apis/debug")
                .send()
                .await
                .map_err(|err| {
                    logging::log!("error getting debug status: {}", err);
                })
                .unwrap()
                .json::<DebugState>()
                .await
                .map_err(|err| {
                    logging::log!("error decoding debug status: {}", err);
                })
                .ok();
            resp.unwrap_or_default()
        },
    );
    let debug_info = move || {
        status
            .get()
            .map(|status| {
                if !status.debugger_installed {
                    return view! {
                        <Box>
                            <Alert variant=AlertVariant::Danger>
                                <AlertTitle slot>"debugger not installed"</AlertTitle>
                                <AlertContent slot>
                                    <span>
                                        "execute `pip install debugpy` to install debugger. or click the following button:"
                                        <Button
                                            on_press=move |_| {
                                                spawn_local(async move {
                                                    let _ = Request::get("/apis/debug/install").send().await;
                                                });
                                            }

                                            size=ButtonSize::Small
                                        >
                                            "install debugger"
                                        </Button>
                                    </span>
                                </AlertContent>
                            </Alert>
                        </Box>
                    };
                }
                if let Some(addr) = status.debugger_address {
                    let cfg = if addr.contains(':') {
                        let addr = addr.split(':').collect::<Vec<&str>>();
                        let host = addr[0];
                        let port = addr[1];
                        format!(r#"
                        {{
                            "name": "Attach to Probing Server",
                            "port": {},
                            "host": "{}",
                            "request": "attach",
                            "type": "python",
                        }}
                        "#, port, host)
                    } else {
                        format!(r#"
                        {{
                            "name": "Attach to Probing Server",
                            "port": {},
                            "host": "127.0.0.1",
                            "request": "attach",
                            "type": "python",
                        }}
                        "#, addr)
                    };

                     view! {
                         <Box>
                             <Alert variant=AlertVariant::Success>
                                 <AlertTitle slot>"debugger is enabled"</AlertTitle>
                                 <AlertContent slot>
                                     "debugger can be connected via:" {addr}
                                 </AlertContent>
                             </Alert>
                             <p>
                                 <H4>"Connect to debugger with vscode:"</H4>
                                 <span>"Open vscode, change to \"Run and Debug\""</span>
                                 <span>"Add the following configuration to `launch.json`"</span>
                                 <pre>{cfg}</pre>
                             </p>
                         </Box>
                     }
                } else {
                     view! {
                         <Box>
                             <Alert variant=AlertVariant::Warn>
                                 <AlertTitle slot>"debugger not enabled"</AlertTitle>
                                 <AlertContent slot>
                                     <span>
                                         "click the following button to enable debugger:"
                                         <Button
                                             on_press=move |_| {
                                                 spawn_local(async move {
                                                     let _ = Request::get("/apis/debug/enable").send().await;
                                                 });
                                             }

                                             size=ButtonSize::Small
                                         >
                                             "enable debugger"
                                         </Button>
                                     </span>
                                 </AlertContent>
                             </Alert>
                         </Box>
                     }
                }
            })
            .unwrap_or(view! {
                <Box>
                    <span>{"no debug status"}</span>
                </Box>
            })
    };

    view! {
        <Box>
            <H3>Debug</H3>
            {debug_info}
        </Box>
    }
}

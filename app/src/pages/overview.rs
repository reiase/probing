use leptonic::components::prelude::*;
use leptos::*;

use gloo_net::http::Request;

use probing_ppp::Process;

#[component]
pub fn Overview() -> impl IntoView {
    let resp = create_resource(
        move || {},
        move |_| async move {
            let resp = Request::get("/apis/overview")
                .send()
                .await
                .map_err(|err| {
                    eprintln!("error getting overview: {}", err);
                })
                .unwrap()
                .json::<Process>()
                .await
                .map_err(|err| {
                    eprintln!("error decoding overview: {}", err);
                })
                .ok();
            resp.unwrap_or(Default::default())
        },
    );

    let thread_info = move || {
        resp.get()
            .map(|proc| {
                let threads: Vec<_> = proc
                    .threads
                    .iter()
                    .map(|t| {
                        let tid = *t;
                        let url = format!("/activity/{}", tid);

                        if tid == proc.main_thread {
                            view! {
                                <Chip color=ChipColor::Primary>
                                    <a href=url>{tid}</a>
                                </Chip>
                            }
                        } else {
                            view! {
                                <Chip color=ChipColor::Secondary>
                                    <a href=url>{tid}</a>
                                </Chip>
                            }
                        }
                    })
                    .collect();
                view! { <Box>{threads}</Box> }
            })
            .unwrap_or(view! {
                <Box>
                    <span>{"no threads found"}</span>
                </Box>
            })
    };

    let process_info = move || {
        resp.get()
            .map(|proc| {
                view! {
                    <Box>
                        <Ul>
                            <Li slot>
                                <b>"Process ID(pid):"</b>
                                <span style="float:right;">{proc.pid.to_string()}</span>
                            </Li>
                            <Li slot>
                                <b>"Executable Path(exe):"</b>
                                <span style="float:right;">{proc.exe.to_string()}</span>
                            </Li>
                            <Li slot>
                                <b>"Command Line(cmd):"</b>
                                <span style="float:right;">{proc.cmd.to_string()}</span>
                            </Li>
                            <Li slot>
                                <b>"Current Working Dirctory(cwd):"</b>
                                <span style="float:right;">{proc.cwd.to_string()}</span>
                            </Li>
                        </Ul>
                    </Box>
                }
            })
            .unwrap_or(view! {
                <Box>
                    <span>"no process information"</span>
                </Box>
            })
    };

    let environments = move || {
        resp.get()
            .map(|proc| {
                let envs: Vec<_> = proc
                    .env
                    .split_terminator('\n')
                    .filter_map(|kv| {
                        if let Some((name, value)) = kv.split_once('=') {
                            Some(view! {
                                <li>
                                    <b>{name.to_string()} " :"</b>
                                    {value.to_string()}
                                </li>
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                view! {
                    <Box>
                        <ul>{envs}</ul>
                    </Box>
                }
            })
            .unwrap_or(view! {
                <Box>
                    <span>"no environment variables"</span>
                </Box>
            })
    };
    view! {
        <H3>"Process Information"</H3>
        {process_info}
        <H3>"Threads"</H3>
        <span>"click to show thread call stack"</span>
        {thread_info}
        <H3>"Environment Variables"</H3>
        {environments}
    }
}

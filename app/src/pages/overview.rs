use leptos::*;
use leptos_meta::Style;
use leptos_router::use_navigate;
use thaw::*;

use dpp::Process;

use crate::{components::header_bar::HeaderBar, url_read::url_read_resource};

#[component]
pub fn Overview() -> impl IntoView {
    let resp = url_read_resource::<Process>("/apis/overview");

    let process_info = move || {
        resp.and_then(|process| {
            let process = process.clone();
            view! {
                <Space>
                    <Table>
                        <tbody>
                            <tr>
                                <td>"Process ID(pid)"</td>
                                <td>{process.pid.to_string()}</td>
                            </tr>
                            <tr>
                                <td>"Executable Path(exe)"</td>
                                <td>{process.exe.to_string()}</td>
                            </tr>
                            <tr>
                                <td>"Command Line(cmd)"</td>
                                <td>{process.cmd.to_string()}</td>
                            </tr>
                            <tr>
                                <td>"Current Working Dirctory(cwd)"</td>
                                <td>{process.cwd.to_string()}</td>
                            </tr>
                        </tbody>
                    </Table>

                </Space>
            }
        })
        .map(|x| x.ok())
        .flatten()
        .unwrap_or(view! {
            <Space>
                <span>"no process information"</span>
            </Space>
        })
    };

    let thread_info = move || {
        resp.and_then(|process| {
            let threads = process
                .threads
                .iter()
                .map(|t| {
                    let tid = *t;
                    let url = format!("/activity/{}", tid);

                    if tid == process.main_thread {
                        view! {
                            <Button
                                color=ButtonColor::Primary
                                style="margin: 5px"
                                on_click=move |_| use_navigate()(url.as_str(), Default::default())
                            >
                                {tid}
                            </Button>
                        }
                    } else {
                        view! {
                            <Button
                                color=ButtonColor::Success
                                style="margin: 5px"
                                on_click=move |_| use_navigate()(url.as_str(), Default::default())
                            >
                                {tid}
                            </Button>
                        }
                    }
                })
                .collect::<Vec<_>>();
            view! { <Space>{threads}</Space> }
        })
        .map(|x| x.ok())
        .flatten()
        .unwrap_or(view! {
            <Space>
                <span>{"no threads found"}</span>
            </Space>
        })
    };

    let environments = move || {
        resp.and_then(|process| {
            let envs: Vec<_> = process
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
                <Space>
                    <ul>{envs}</ul>
                </Space>
            }
        })
        .map(|x| x.ok())
        .flatten()
        .unwrap_or(view! {
            <Space>
                <span>"no environment variables"</span>
            </Space>
        })
    };

    view! {
        <Style>
            "
            .doc-content {
                    margin: 0 auto;
                    width: 100%;
                    display: grid;
            }
            @media screen and (max-width: 1200px) {
                .doc-content {
                    width: 100%;
                }
            }
            "
        </Style>
        <HeaderBar/>
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <Space align=SpaceAlign::Center vertical=true class="doc-content">
                <Card title="Process Information">{process_info}</Card>
                <Card title="Threads">
                    {thread_info} <CardFooter slot>"click to show thread call stack"</CardFooter>
                </Card>
                <Card title="Environment Variables">{environments}</Card>
            </Space>
        </Layout>
    }
}

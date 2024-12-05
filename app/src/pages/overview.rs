use leptos::prelude::*;
use leptos_meta::Style;
use leptos_router::hooks::use_navigate;
use thaw::*;

use probing_dpp::Process;

use crate::{components::header_bar::HeaderBar, url_read::url_read_resource};

#[component]
pub fn Overview() -> impl IntoView {
    let resp = url_read_resource::<Process>("/apis/overview");

    let process_info = move || {
        view! {
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || Suspend::new(async move {
                    resp.await
                        .map(|process| {
                            view! {
                                <Flex>
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
                                </Flex>
                            }
                        })
                        .unwrap_or(
                            view! {
                                <Flex>
                                    <span>"no process information"</span>
                                </Flex>
                            },
                        )
                })}

            </Suspense>
        }
    };

    let thread_info = move || {
        view! {
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || Suspend::new(async move {
                    let process = resp.await;
                    process
                        .map(|process| {
                            let threads = process
                                .threads
                                .iter()
                                .map(|t| {
                                    let tid = *t;
                                    let url = format!("/activity/{}", tid);
                                    if tid == process.main_thread {
                                        view! {
                                            <Button
                                                appearance=ButtonAppearance::Primary
                                                on_click=move |_| use_navigate()(
                                                    url.as_str(),
                                                    Default::default(),
                                                )
                                            >

                                                {tid}
                                            </Button>
                                        }
                                    } else {
                                        view! {
                                            <Button
                                                appearance=ButtonAppearance::Secondary
                                                on_click=move |_| use_navigate()(
                                                    url.as_str(),
                                                    Default::default(),
                                                )
                                            >

                                                {tid}
                                            </Button>
                                        }
                                    }
                                })
                                .collect::<Vec<_>>();
                            view! { <Flex>{threads}</Flex> }
                        })
                        .unwrap_or(
                            view! {
                                <Flex>
                                    <span>{"no threads found"}</span>
                                </Flex>
                            },
                        );
                })}

            </Suspense>
        }
    };

    let environments = move || {
        view! {
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || Suspend::new(async move {
                    let process = resp.await;
                    process
                        .map(|process| {
                            let envs: Vec<_> = process
                                .env
                                .split_terminator('\n')
                                .filter_map(|kv| {
                                    if let Some((name, value)) = kv.split_once('=') {
                                        Some(
                                            view! {
                                                <li>
                                                    <b>{name.to_string()} " :"</b>
                                                    {value.to_string()}
                                                </li>
                                            },
                                        )
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            view! {
                                <Flex>
                                    <ul>{envs}</ul>
                                </Flex>
                            }
                        })
                })}

            </Suspense>
        }
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
                <Card>
                    <CardHeader>"Process Information"</CardHeader>
                    <CardPreview>{process_info}</CardPreview>
                </Card>
                <Card>
                    <CardHeader>"Threads"</CardHeader>
                    <CardPreview>{thread_info}</CardPreview>
                    <CardFooter>"click to show thread call stack"</CardFooter>
                </Card>
                <Card>
                    <CardHeader>"Environment Variables"</CardHeader>
                    <CardPreview>{environments}</CardPreview>
                </Card>
            </Space>
        </Layout>
    }
}

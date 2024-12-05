use std::cmp::{max, min};

use leptos::prelude::*;
use leptos_meta::Style;
use thaw::*;

use probing_dpp::Process;

use crate::components::header_bar::HeaderBar;
use crate::components::panel::Panel;
use crate::url_read::url_read_resource;

#[component]
pub fn Overview() -> impl IntoView {
    let resp = url_read_resource::<Process>("/apis/overview");

    let process_info = view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                resp.await
                    .map(|process| {
                        view! {
                            <Flex>
                                <Table>
                                    <TableBody>
                                        <TableRow>
                                            <TableCell>"Process ID(pid)"</TableCell>
                                            <TableCell>{process.pid.to_string()}</TableCell>
                                        </TableRow>
                                        <TableRow>
                                            <TableCell>"Executable Path(exe)"</TableCell>
                                            <TableCell>{process.exe.to_string()}</TableCell>
                                        </TableRow>
                                        <TableRow>
                                            <TableCell>"Command Line(cmd)"</TableCell>
                                            <TableCell>{process.cmd.to_string()}</TableCell>
                                        </TableRow>
                                        <TableRow>
                                            <TableCell>"Current Working Dirctory(cwd)"</TableCell>
                                            <TableCell>{process.cwd.to_string()}</TableCell>
                                        </TableRow>
                                    </TableBody>
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
    };

    let thread_info = view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                resp.await
                    .map(|process| {
                        let threads = process
                            .threads
                            .iter()
                            .map(|t| {
                                let tid = *t;
                                let url = format!("/activity/{}", tid);
                                view! { <Link href=url>{tid}</Link> }
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
                    )
            })}

        </Suspense>
    };

    let environments = view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                let process = resp.await;
                process
                    .map(|process| {
                        view! {
                            <Flex>
                                <EnvironmentTable envs=process.env />
                            </Flex>
                        }
                    })
            })}
        </Suspense>
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
        <HeaderBar />
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <Flex align=FlexAlign::Center vertical=true class="doc-content">
                <Panel title="Process Information">{process_info}</Panel>
                <Panel title="Threads Information">{thread_info}</Panel>
                <Panel title="Environment Variables">{environments}</Panel>
            </Flex>
        </Layout>
    }
}

#[component]
pub fn EnvironmentTable(#[prop(into)] envs: String) -> impl IntoView {
    let mut max_name_width = 10;
    let envs = str2map(envs)
        .iter()
        .map(|kv| {
            let (name, value) = kv;
            let name = name.clone();
            let value = value.clone();
            max_name_width = max(max_name_width, name.len());
            view! {
                <TableRow>
                    <TableCell>{name}</TableCell>
                    <TableCell>{value}</TableCell>
                </TableRow>
            }
        })
        .collect::<Vec<_>>();
    let max_name_width = min(50, max_name_width) as f64;
    view! {
        <Table>
            <TableHeader>
                <TableRow>
                    <TableHeaderCell resizable=true min_width=100.0 max_width=10. * max_name_width>
                        "name"
                    </TableHeaderCell>
                    <TableHeaderCell>"Value"</TableHeaderCell>
                </TableRow>
            </TableHeader>
            <TableBody>{envs}</TableBody>
        </Table>
    }
}

fn str2map(s: String) -> Vec<(String, String)> {
    s.split('\n')
        .map(|kv| {
            if let Some((name, value)) = kv.split_once('=') {
                (name.to_string(), value.to_string())
            } else {
                ("".to_string(), kv.to_string())
            }
        })
        .collect::<Vec<_>>()
}

use leptos::prelude::*;
use leptos_meta::Style;
use thaw::*;

use probing_proto::prelude::*;
use probing_proto::Process;

use crate::components::header_bar::HeaderBar;
use crate::components::panel::Panel;
use crate::components::tableview::TableView;
use crate::url_read::url_read_resource;

#[component]
pub fn Overview() -> impl IntoView {
    let resp = url_read_resource::<Process>("/apis/overview");

    let process_info = view! {
        <Suspense fallback=move || {
            view! { <p>"Loading..."</p> }
        }>
            {move || Suspend::new(async move {
                let process = resp.await.unwrap_or_default();
                let tbl = Table::new(
                    vec!["name", "value"],
                    vec![
                        vec!["Process ID(pid)".to_string(), process.pid.to_string()],
                        vec!["Executable Path(exe)".to_string(), process.exe.to_string()],
                        vec!["Command Line(cmd)".to_string(), process.cmd.to_string()],
                        vec!["Current Working Dirctory(cwd)".to_string(), process.cwd.to_string()],
                    ],
                );
                view! { <TableView tbl /> }
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
                let process = resp.await.unwrap_or_default();
                let names = vec!["name", "value"];
                let rows = process
                    .env
                    .split('\n')
                    .map(|kv| {
                        if let Some((name, value)) = kv.split_once('=') {
                            vec![name.to_string(), value.to_string()]
                        } else {
                            vec!["".to_string(), kv.to_string()]
                        }
                    })
                    .collect::<Vec<_>>();

                view! { <TableView tbl=Table::new(names, rows) /> }
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
            // <Flex align=FlexAlign::Center vertical=true class="doc-content">
            <Panel title="Process Information">{process_info}</Panel>
            <Panel title="Threads Information">{thread_info}</Panel>
            <Panel title="Environment Variables">{environments}</Panel>
        // </Flex>
        </Layout>
    }
}

use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_meta::Style;
use thaw::*;

use probing_proto::prelude::*;

use crate::components::{header_bar::HeaderBar, panel::Panel};
use crate::url_read::url_read_resource;

#[component]
pub fn Cluster() -> impl IntoView {
    let resp = url_read_resource::<Vec<Node>>("/apis/nodes");

    let node_info = move || {
        view! {
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || Suspend::new(async move {
                    resp.await
                        .map(|nodes| {
                            nodes
                                .iter()
                                .map(|node| {
                                    let node = node.clone();
                                    let datetime: DateTime<Utc> = (SystemTime::UNIX_EPOCH
                                        + Duration::from_micros(node.timestamp))
                                        .into();
                                    let timestamp = datetime.to_rfc3339();
                                    let url = format!("http://{}", node.addr);
                                    view! {
                                        <TableRow>
                                            <TableCell>{node.host.to_string()}</TableCell>
                                            <TableCell>
                                                <a href=url>{node.addr.to_string()}</a>
                                            </TableCell>
                                            <TableCell>
                                                {node.local_rank.unwrap_or(-1).to_string()}
                                            </TableCell>
                                            <TableCell>{node.rank.unwrap_or(-1).to_string()}</TableCell>
                                            <TableCell>
                                                {node.world_size.unwrap_or(-1).to_string()}
                                            </TableCell>
                                            <TableCell>
                                                {node.group_rank.unwrap_or(-1).to_string()}
                                            </TableCell>
                                            <TableCell>
                                                {node.group_world_size.unwrap_or(-1).to_string()}
                                            </TableCell>
                                            <TableCell>{node.role_name}</TableCell>
                                            <TableCell>
                                                {node.role_rank.unwrap_or(-1).to_string()}
                                            </TableCell>
                                            <TableCell>
                                                {node.role_world_size.unwrap_or(-1).to_string()}
                                            </TableCell>
                                            <TableCell>{node.status}</TableCell>
                                            <TableCell>{timestamp}</TableCell>
                                        </TableRow>
                                    }
                                })
                                .collect::<Vec<_>>()
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
        <HeaderBar />
        <Layout
            content_style="padding: 8px 12px 28px; display: flex; flex-direction: column;"
            class="doc-content"
        >
            <Panel title="Nodes">
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell>host</TableHeaderCell>
                            <TableHeaderCell>address</TableHeaderCell>
                            <TableHeaderCell>local_rank</TableHeaderCell>
                            <TableHeaderCell>rank</TableHeaderCell>
                            <TableHeaderCell>world_size</TableHeaderCell>
                            <TableHeaderCell>group_rank</TableHeaderCell>
                            <TableHeaderCell>group_world_size</TableHeaderCell>
                            <TableHeaderCell>role_name</TableHeaderCell>
                            <TableHeaderCell>role_rank</TableHeaderCell>
                            <TableHeaderCell>role_world_size</TableHeaderCell>
                            <TableHeaderCell>status</TableHeaderCell>
                            <TableHeaderCell>timestamp</TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>{node_info}</TableBody>
                </Table>
            </Panel>
        </Layout>
    }
}

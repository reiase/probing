use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use leptos::*;
use leptos_meta::Style;
use thaw::*;

use probing_dpp::protocol::cluster::Node;

use crate::{components::header_bar::HeaderBar, url_read::url_read_resource};

#[component]
pub fn Cluster() -> impl IntoView {
    let resp = url_read_resource::<Vec<Node>>("/apis/nodes");

    let cluster_info = move || {
        resp.and_then(|nodes| {
            let nnodes = nodes.len();
            view! {
                <Table>
                    <tbody>
                        <tr>
                            <td>"Number of Nodes"</td>
                            <td>{nnodes.to_string()}</td>
                        </tr>
                    </tbody>
                </Table>
            }
        })
    };

    let node_info = move || {
        resp.and_then(|nodes| {
            nodes
                .iter()
                .map(|node| {
                    let node = node.clone();
                    logging::log!("node: {:?}", node);
                    let datetime: DateTime<Utc> = (SystemTime::UNIX_EPOCH
                        + Duration::from_micros(node.timestamp as u64))
                    .into();
                    let timestamp = datetime.to_rfc3339();
                    let url = format!("http://{}", node.addr);
                    view! {
                        <tr>
                            <td>{node.host.to_string()}</td>
                            <td>
                                <a href=url>{node.addr.to_string()}</a>
                            </td>
                            <td>{node.local_rank.unwrap_or(-1).to_string()}</td>
                            <td>{node.rank.unwrap_or(-1).to_string()}</td>
                            <td>{node.world_size.unwrap_or(-1).to_string()}</td>
                            <td>{node.group_rank.unwrap_or(-1).to_string()}</td>
                            <td>{node.group_world_size.unwrap_or(-1).to_string()}</td>
                            <td>{node.role_name}</td>
                            <td>{node.role_rank.unwrap_or(-1).to_string()}</td>
                            <td>{node.role_world_size.unwrap_or(-1).to_string()}</td>
                            <td>{node.status}</td>
                            <td>{timestamp}</td>
                        </tr>
                    }
                })
                .collect::<Vec<_>>()
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
                <Card title="Cluster Information">{cluster_info}</Card>
                <Card title="Nodes Information">
                    <Table>
                        <thead>
                            <tr>
                                <th>host</th>
                                <th>address</th>
                                <th>local_rank</th>
                                <th>rank</th>
                                <th>world_size</th>
                                <th>group_rank</th>
                                <th>group_world_size</th>
                                <th>role_name</th>
                                <th>role_rank</th>
                                <th>role_world_size</th>
                                <th>status</th>
                                <th>timestamp</th>
                            </tr>
                        </thead>
                        <tbody>{node_info}</tbody>
                    </Table>
                </Card>
            </Space>
        </Layout>
    }
}

use anyhow::Result;
use once_cell::sync::Lazy;

use probing_common::Process;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::style::Stylize;
use ratatui::prelude::*;
use ratatui::widgets::Scrollbar;
use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::cli::panel::{AppTab, APP};

use super::activity_tab::ACTIVITY_TAB;
use super::app_style;
use super::read_info::{read_callstack_info, read_process_info};

#[derive(Default, Debug)]
pub struct ProcessTab {
    threads: Vec<u64>,
    state: TreeState<String>,
    items: Vec<TreeItem<'static, String>>,
}

pub static mut PROCESS_TAB: Lazy<ProcessTab> = Lazy::new(ProcessTab::default);

pub fn handle_key_event(code: KeyCode) -> Result<()> {
    unsafe {
        match code {
            KeyCode::Char('\n') | KeyCode::Enter => {
                PROCESS_TAB.state.toggle_selected();
                match PROCESS_TAB.state.selected() {
                    [toplevel, id] => {
                        if toplevel == "threads" {
                            let tid: i32 = PROCESS_TAB.threads[id.parse::<usize>().unwrap()] as i32;
                            ACTIVITY_TAB.set_tid(tid);
                            ACTIVITY_TAB.callstacks =
                                read_callstack_info(tid).unwrap_or_default();
                            APP.selected_tab = AppTab::Activity;
                        }
                    }
                    _ => {}
                }
                false
            }
            KeyCode::Up => PROCESS_TAB.state.key_up(),
            KeyCode::Down => PROCESS_TAB.state.key_down(),
            KeyCode::Left => PROCESS_TAB.state.key_left(),
            KeyCode::Right => PROCESS_TAB.state.key_right(),
            _ => false,
        };
    }
    Ok(())
}

fn format_json_key(key: &str, val: String) -> TreeItem<'static, String> {
    TreeItem::new_leaf(
        key.to_string(),
        format!("{} {}{}", key.blue(), ":".dark_grey(), val.dark_grey(),),
    )
}

fn format_json_key_longstr(
    key: &str,
    val: String,
    sep1: &str,
    sep2: &str,
) -> TreeItem<'static, String> {
    let children: Vec<_> = val
        .split_terminator(sep1)
        .filter_map(|kv| {
            if let Some((name, value)) = kv.split_once(sep2) {
                Some(format_json_key(name, value.to_string()))
            } else {
                None
            }
        })
        .collect();
    TreeItem::new(
        key.to_string(),
        format!(
            "{}{}",
            key.blue().bold(),
            format!(":{} children", children.len()).dark_grey().dim()
        ),
        children,
    )
    .unwrap()
}

fn format_json_key_array(key: &str, val: Vec<String>) -> TreeItem<'static, String> {
    let children: Vec<_> = val
        .iter()
        .enumerate()
        .map(|(i, v)| {
            TreeItem::new_leaf(
                format!("{i}"),
                format!(
                    "{}{}",
                    format!("[{}]=", i).dark_grey().dim(),
                    v.clone().blue().bold(),
                ),
            )
        })
        .collect();
    TreeItem::new(
        key.to_string(),
        format!(
            "{} {}",
            key.blue().bold(),
            format!(":{} children", children.len()).dark_grey().dim()
        ),
        children,
    )
    .unwrap()
}

impl ProcessTab {
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if self.items.is_empty() {
            let info = read_process_info();
            let info = serde_json::from_str::<Process>(&info).unwrap_or_default();
            self.items = vec![
                format_json_key("pid", format!("{}", info.pid)),
                format_json_key("exe", info.exe),
                format_json_key("cmd", info.cmd),
                format_json_key("cwd", info.cwd),
                format_json_key_longstr("env", info.env, "\n", "="),
                format_json_key("main_thread", format!("{}", info.main_thread)),
                format_json_key_array(
                    "threads",
                    info.threads.iter().map(|t| format!("{}", t)).collect(),
                ),
            ];
            self.threads.clone_from(&info.threads);
        }
        let tree = Tree::new(&self.items)
            .expect("all item identifiers are unique")
            .block(app_style::border_header(Some(
                "Process Info (`Enter` to select)",
            )))
            .experimental_scrollbar(
                Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .track_symbol(None)
                    .end_symbol(None)
                    .into(),
            )
            .node_closed_symbol(" +")
            .node_open_symbol(" -")
            .highlight_symbol(">");
        ratatui::prelude::StatefulWidget::render(tree, area, buf, &mut self.state);
    }
}

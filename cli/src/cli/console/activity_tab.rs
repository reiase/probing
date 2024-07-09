use anyhow::Result;
use once_cell::sync::Lazy;
use probing_common::CallStack;
use ratatui::{crossterm::event::KeyCode, prelude::*, widgets::Scrollbar};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use nu_ansi_term::Color::Blue;
use nu_ansi_term::Color::DarkGray;

use super::app_style;

#[derive(Default, Debug)]
pub struct ActivityTab {
    pub tid: i32,
    pub callstacks: Vec<CallStack>,
    state: TreeState<String>,
    items: Vec<TreeItem<'static, String>>,
}

pub static mut ACTIVITY_TAB: Lazy<ActivityTab> = Lazy::new(|| ActivityTab::default());
pub fn handle_key_event(code: KeyCode) -> Result<()> {
    unsafe {
        match code {
            KeyCode::Char('\n') | KeyCode::Enter => ACTIVITY_TAB.state.toggle_selected(),
            KeyCode::Up => ACTIVITY_TAB.state.key_up(),
            KeyCode::Down => ACTIVITY_TAB.state.key_down(),
            KeyCode::Left => ACTIVITY_TAB.state.key_left(),
            KeyCode::Right => ACTIVITY_TAB.state.key_right(),
            _ => false,
        };
    }
    Ok(())
}

fn format_frame(i: usize, frame: &CallStack) -> TreeItem<'static, String> {
    if let Some(cframe) = &frame.cstack {
        TreeItem::new(
            format!("{}", i),
            "C/C++ Frame".to_string(),
            vec![TreeItem::new_leaf(format!("{}", i), cframe.clone())],
        )
        .unwrap()
    } else {
        TreeItem::new(
            format!("{}", i),
            format!(
                "{}{} @ {}:{}",
                DarkGray.dimmed().paint(format!("#[{}]:", i)),
                Blue.bold().paint(frame.func.clone()),
                Blue.bold().paint(frame.file.clone()),
                Blue.bold().paint(format!("{}", frame.lineno)),
            ),
            frame
                .locals
                .iter()
                .map(|(name, value)| {
                    TreeItem::new_leaf(
                        name.clone(),
                        if value.value.is_some() {
                            format!(
                                "{} = {} as {}",
                                name,
                                value.value.clone().unwrap(),
                                value.class
                            )
                        } else {
                            "None".to_string()
                        },
                    )
                })
                .collect(),
        )
        .unwrap()
    }
}

fn format_callstacks(callstacks: &Vec<CallStack>) -> Vec<TreeItem<'static, String>> {
    callstacks
        .iter()
        .enumerate()
        .map(|(i, frame)| format_frame(i, frame))
        .collect()
}

impl ActivityTab {
    pub fn set_tid(&mut self, tid: i32) -> &Self {
        self.tid = tid;
        return self;
    }
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        self.items = format_callstacks(&self.callstacks);

        let tree = Tree::new(&self.items)
            .expect("all item identifiers are unique")
            .block(app_style::border_header(Some(format!(
                "Call Stacks for thread {}{}",
                self.tid,
                DarkGray.paint(format!(":stack deepth={}", self.callstacks.len()))
            ))))
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

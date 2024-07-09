use anyhow::Result;
use once_cell::sync::Lazy;
use probing_common::CallStack;
use ratatui::{crossterm::event::KeyCode, prelude::*};
use tui_tree_widget::{TreeItem, TreeState};

use super::read_info::read_callstack_info;

#[derive(Default, Debug)]
pub struct ActivityTab {
    tid: i32,
    callstacks: Vec<CallStack>,
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
impl ActivityTab {
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if self.callstacks.len() == 0 {
            self.callstacks = read_callstack_info(self.tid).unwrap_or(Default::default());
        }
    }
}

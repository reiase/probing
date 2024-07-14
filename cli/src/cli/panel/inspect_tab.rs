use anyhow::Result;
use once_cell::sync::Lazy;
use probing_common::Object;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::style::Stylize;
use ratatui::{prelude::*, widgets::Scrollbar};
use tui_tree_widget::{Tree, TreeItem, TreeState};

use super::app_style;
use super::read_info::read_object_info;

#[derive(Default, Debug)]
pub enum ObjectType {
    #[default]
    OBJECT,
    TENSOR,
    MODULE,
}

#[derive(Default, Debug)]
pub struct InspectTab {
    pub selector: ObjectType,
    pub objects: Vec<Object>,
    state: TreeState<String>,
    items: Vec<TreeItem<'static, String>>,
}

pub static mut INSPECT_TAB: Lazy<InspectTab> = Lazy::new(|| InspectTab::default());

pub fn handle_key_event(code: KeyCode) -> Result<()> {
    unsafe {
        match code {
            KeyCode::Char('\n') | KeyCode::Enter => INSPECT_TAB.state.toggle_selected(),
            KeyCode::Char('p') => {
                INSPECT_TAB.selector = ObjectType::OBJECT;
                INSPECT_TAB.objects = read_object_info(match INSPECT_TAB.selector {
                    ObjectType::OBJECT => "objects",
                    ObjectType::TENSOR => "torch/tensors",
                    ObjectType::MODULE => "torch/modules",
                })
                .unwrap_or(Default::default());
                false
            }
            KeyCode::Char('t') => {
                INSPECT_TAB.selector = ObjectType::TENSOR;
                INSPECT_TAB.objects = read_object_info(match INSPECT_TAB.selector {
                    ObjectType::OBJECT => "objects",
                    ObjectType::TENSOR => "torch/tensors",
                    ObjectType::MODULE => "torch/modules",
                })
                .unwrap_or(Default::default());
                false
            }
            KeyCode::Char('m') => {
                INSPECT_TAB.selector = ObjectType::MODULE;
                INSPECT_TAB.objects = read_object_info(match INSPECT_TAB.selector {
                    ObjectType::OBJECT => "objects",
                    ObjectType::TENSOR => "torch/tensors",
                    ObjectType::MODULE => "torch/modules",
                })
                .unwrap_or(Default::default());
                false
            }
            KeyCode::Up => INSPECT_TAB.state.key_up(),
            KeyCode::Down => INSPECT_TAB.state.key_down(),
            KeyCode::Left => INSPECT_TAB.state.key_left(),
            KeyCode::Right => INSPECT_TAB.state.key_right(),
            _ => false,
        };
    }
    Ok(())
}

fn format_objects(objects: &Vec<Object>) -> Vec<TreeItem<'static, String>> {
    objects
        .iter()
        .enumerate()
        .map(|(idx, obj)| {
            let mut children = vec![];
            if let Some(shape) = obj.shape.clone() {
                children.push(TreeItem::new_leaf(
                    "shape".to_string(),
                    format!("shape={shape}"),
                ));
            }
            if let Some(dtype) = obj.dtype.clone() {
                children.push(TreeItem::new_leaf(
                    "dtype".to_string(),
                    format!("dtype={dtype}"),
                ));
            }
            if let Some(device) = obj.device.clone() {
                children.push(TreeItem::new_leaf(
                    "device".to_string(),
                    format!("device={device}"),
                ));
            }
            if let Some(value) = obj.value.clone() {
                children.push(TreeItem::new_leaf(
                    "value".to_string(),
                    format!("value={value}"),
                ));
            }
            TreeItem::new(
                format!("{}", idx),
                format!("id={}, class={}", obj.id, obj.class),
                children,
            )
            .unwrap()
        })
        .collect()
}

impl InspectTab {
    pub fn draw(&mut self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        if self.items.len() == 0 {
            self.objects = read_object_info(match self.selector {
                ObjectType::OBJECT => "objects",
                ObjectType::TENSOR => "torch/tensors",
                ObjectType::MODULE => "torch/modules",
            })
            .unwrap_or(Default::default());
        }
        self.items = format_objects(&self.objects);

        let tree = Tree::new(&self.items)
            .expect("all item identifiers are unique")
            .block(app_style::border_header(Some(format!(
                "Inspect Objects of type {:?}{}",
                self.selector,
                format!(":total={}", self.objects.len()).dark_grey()
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

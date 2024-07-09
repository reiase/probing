use anyhow::Result;

use once_cell::sync::Lazy;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Padding, Paragraph, Tabs};

use hyperparameter::*;
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

mod app_style;
mod process_tab;
mod activity_tab;
mod read_info;
mod utils;

pub fn console_main(pid: i32) -> Result<()> {
    utils::init_error_hooks()?;
    let mut terminal = utils::init_terminal()?;

    unsafe {
        APP.set_pid(pid).run(&mut terminal).unwrap();
    }
    utils::restore_terminal()?;
    Ok(())
}

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum AppTab {
    #[default]
    Process,
    Activity,
    Debug,
    Inspect,
}

impl AppTab {
    fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(app_style::fgcolor().c200)
            .bg(app_style::bgcolor().c900)
            .into()
    }
    fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(app_style::bgcolor().c400)
    }
}

#[derive(Default)]
pub struct App {
    pid: Option<i32>,
    is_quit: bool,
    selected_tab: AppTab,
}

impl App {
    pub fn set_pid(&mut self, pid: i32) -> &mut Self {
        self.pid = Some(pid);
        self
    }
    fn run(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        while !self.is_quit {
            self.draw(terminal)?;
            self.handle_event()?;
        }
        Ok(())
    }

    fn draw(&self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal.draw(|f| f.render_widget(self, f.size()))?;
        Ok(())
    }

    fn handle_event(&mut self) -> Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Tab => {
                        self.selected_tab = AppTab::from_repr(self.selected_tab as usize + 1)
                            .unwrap_or(AppTab::default())
                    }
                    KeyCode::Char('q') => self.is_quit = true,
                    code => self.route_key_event(code)?,
                }
            }
        }
        Ok(())
    }

    fn route_key_event(&mut self, code: KeyCode) -> Result<()> {
        match self.selected_tab {
            AppTab::Process => process_tab::handle_key_event(code),
            AppTab::Activity => Ok(()),
            AppTab::Debug => Ok(()),
            AppTab::Inspect => Ok(()),
        }
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = AppTab::iter().map(|t| t.title());
        let highlight_style = (Color::default(), app_style::bgcolor().c400);
        let selected_tab_index = self.selected_tab as usize;
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }
}

pub static mut APP: Lazy<App> = Lazy::new(|| App::default());

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        use Constraint::Length;
        use Constraint::Min;
        use Constraint::Percentage;
        let toplevel = Layout::vertical([Length(1), Min(0)]);
        let [header, body] = toplevel.areas(area);
        let [title_area, tab_area] =
            Layout::horizontal([Length(10), Percentage(100)]).areas(header);
        "Probing".bold().render(title_area, buf);
        self.render_tabs(tab_area, buf);
        with_params! {
            set probing.process.pid = self.pid.unwrap() as i64;

            self.selected_tab.render(body, buf);
        }
    }
}

impl Widget for AppTab {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self {
            AppTab::Process => unsafe { process_tab::PROCESS_TAB.draw(area, buf) },
            AppTab::Activity => Paragraph::new("Hello, World!!")
                .block(self.block())
                .render(area, buf),
            AppTab::Debug => Paragraph::new("Hello, World!!!")
                .block(self.block())
                .render(area, buf),
            AppTab::Inspect => Paragraph::new("Hello, World!!!!")
                .block(self.block())
                .render(area, buf),
        }
    }
}

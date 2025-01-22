use std::os::fd::AsFd;

use tabled::builder::Builder;
use tabled::grid::config::Position;
use tabled::grid::records::{
    vec_records::{Text, VecRecords},
    ExactRecords, Records,
};
use tabled::settings::{
    object::Segment,
    peaker::{PriorityMax, PriorityMin},
    Alignment, Settings, Style, Width,
};

use probing_proto::types::DataFrame;
use probing_proto::types::Value;

pub struct Table {
    data: VecRecords<Text<String>>,
}

impl Table {
    pub fn new(ncol: usize, nrow: usize) -> Self {
        Self {
            data: VecRecords::new(vec![vec![Text::default(); ncol]; nrow + 1]),
        }
    }

    pub fn count_rows(&self) -> usize {
        self.data.count_rows()
    }

    pub fn count_columns(&self) -> usize {
        self.data.count_columns()
    }

    pub fn put(&mut self, pos: Position, text: String) {
        self.data[pos.0][pos.1] = Text::new(text)
    }

    pub fn draw(self, termwidth: usize) -> Option<String> {
        if self.count_columns() == 0 || self.count_rows() == 0 {
            return Some(Default::default());
        }

        let data: Vec<Vec<_>> = self.data.into();
        let mut table = Builder::from(data).build();
        table.with(Style::sharp());
        table.modify(
            Segment::all(),
            Settings::new(Alignment::left(), Alignment::top()),
        );

        table.with((
            Width::wrap(termwidth).priority(PriorityMax::default()),
            Width::increase(termwidth).priority(PriorityMin::default()),
        ));
        Some(table.to_string())
    }
}

pub fn render_dataframe(df: &DataFrame) {
    let ncol = df.names.len();
    let nrow = df.cols.iter().map(|col| col.len()).max().unwrap_or(0);

    let mut table = Table::new(ncol, nrow);

    for (col, name) in df.names.iter().enumerate() {
        table.put((0, col), name.clone());
    }

    for (col, col_data) in df.cols.iter().enumerate() {
        for row in 0..col_data.len() {
            let value = match col_data.get(row) {
                Value::Nil => "nil".to_string(),
                Value::Int32(x) => x.to_string(),
                Value::Int64(x) => x.to_string(),
                Value::Float32(x) => x.to_string(),
                Value::Float64(x) => x.to_string(),
                Value::Text(x) => x.to_string(),
                Value::Url(x) => x.to_string(),
                Value::DataTime(x) => x.to_string(),
            };
            table.put((row + 1, col), value);
        }
    }
    println!(
        "{}",
        table.draw(terminal_width().unwrap_or(80) as usize).unwrap()
    );
}

fn terminal_width() -> Option<u32> {
    terminal_size_of(std::io::stdout())
}

fn terminal_size_of<Fd: AsFd>(fd: Fd) -> Option<u32> {
    use rustix::termios::{isatty, tcgetwinsize};

    if !isatty(&fd) {
        return None;
    }

    let winsize = tcgetwinsize(&fd).ok()?;
    let cols = winsize.ws_col;

    if cols > 0 {
        Some(cols as u32)
    } else {
        None
    }
}

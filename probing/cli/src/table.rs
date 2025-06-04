use nix::ioctl_read;
use nix::libc;
use std::os::fd::AsFd;
use std::os::fd::AsRawFd;

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

use probing_proto::prelude::{DataFrame, Ele};

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
                Ele::Nil => "nil".to_string(),
                Ele::BOOL(x) => x.to_string(),
                Ele::I32(x) => x.to_string(),
                Ele::I64(x) => x.to_string(),
                Ele::F32(x) => x.to_string(),
                Ele::F64(x) => x.to_string(),
                Ele::Text(x) => x.to_string(),
                Ele::Url(x) => x.to_string(),
                Ele::DataTime(x) => x.to_string(),
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

ioctl_read!(get_winsize, libc::TIOCGWINSZ, 0, libc::winsize);

fn terminal_size_of<Fd: AsFd>(fd: Fd) -> Option<u32> {
    use nix::unistd::isatty;
    if isatty(fd.as_fd()).is_err() {
        return None;
    }

    let winsize = unsafe {
        let mut winsize = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        get_winsize(fd.as_fd().as_raw_fd(), &mut winsize).ok()?;
        winsize
    };
    let cols = winsize.ws_col;

    if cols > 0 {
        Some(cols as u32)
    } else {
        None
    }
}

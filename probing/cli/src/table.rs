use std::os::fd::AsFd;
// use std::time::{Duration, SystemTime};

// // use arrow::array::RecordBatch;
// use chrono::{DateTime, Utc};
use tabled::builder::Builder;
use tabled::grid::{
    config::Position,
    records::{
        vec_records::{Text, VecRecords},
        ExactRecords, Records,
    },
};
use tabled::settings::{
    object::Segment,
    peaker::{PriorityMax, PriorityMin},
    Alignment, Settings, Style, Width,
};

use probing_proto::protocol::dataframe::{DataFrame, Value};

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
            Width::wrap(termwidth).priority(PriorityMax),
            Width::increase(termwidth).priority(PriorityMin),
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

// pub fn render_table(data: &[RecordBatch]) {
//     if data.is_empty() {
//         return;
//     }
//     let ncol = data[0].schema().fields().len();
//     let nrow = data.iter().map(|batch| batch.num_rows()).sum();
//     let mut table = Table::new(ncol, nrow);

//     let mut row = 0;
//     for batch in data {
//         if row == 0 {
//             for (col, field) in batch.schema().fields().iter().enumerate() {
//                 table.put((0, col), field.name().clone());
//             }
//             row = 1;
//         }
//         for col in 0..ncol {
//             let field = batch.column(col);
//             if let Some(array) = field
//                 .as_any()
//                 .downcast_ref::<arrow::array::TimestampMicrosecondArray>()
//             {
//                 array.iter().enumerate().for_each(|(i, value)| {
//                     let datetime_str = value.map_or_else(
//                         || String::new(),
//                         |v| {
//                             let datetime: DateTime<Utc> =
//                                 (SystemTime::UNIX_EPOCH + Duration::from_micros(v as u64)).into();
//                             datetime.to_rfc3339()
//                         },
//                     );
//                     table.put((row + i, col), datetime_str);
//                 });
//             }
//             if let Some(array) = field.as_any().downcast_ref::<arrow::array::StringArray>() {
//                 array.iter().enumerate().for_each(|(i, value)| {
//                     table.put((row + i, col), value.unwrap_or_default().to_string());
//                 });
//             }
//             if let Some(array) = field.as_any().downcast_ref::<arrow::array::Int32Array>() {
//                 array.iter().enumerate().for_each(|(i, value)| {
//                     table.put((row + i, col), value.unwrap_or_default().to_string());
//                 });
//             }
//             if let Some(array) = field.as_any().downcast_ref::<arrow::array::Int64Array>() {
//                 array.iter().enumerate().for_each(|(i, value)| {
//                     table.put((row + i, col), value.unwrap_or_default().to_string());
//                 });
//             }
//             if let Some(array) = field.as_any().downcast_ref::<arrow::array::Float64Array>() {
//                 array.iter().enumerate().for_each(|(i, value)| {
//                     table.put((row + i, col), value.unwrap_or_default().to_string());
//                 });
//             }
//             if let Some(array) = field.as_any().downcast_ref::<arrow::array::StringArray>() {
//                 array.iter().enumerate().for_each(|(i, value)| {
//                     table.put((row + i, col), value.unwrap_or_default().to_string());
//                 });
//             }
//         }
//         row += batch.num_rows();
//     }
//     println!(
//         "{}",
//         table.draw(terminal_width().unwrap_or(80) as usize).unwrap()
//     );
// }

fn terminal_width() -> Option<u32> {
    if let Some(width) = terminal_size_of(std::io::stdout()) {
        Some(width)
    } else if let Some(width) = terminal_size_of(std::io::stderr()) {
        Some(width)
    } else if let Some(width) = terminal_size_of(std::io::stdin()) {
        Some(width)
    } else {
        None
    }
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

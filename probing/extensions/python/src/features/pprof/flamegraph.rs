use crate::features::pprof::PPROF_CACHE;
use probing_proto::prelude::CallFrame;

pub(crate) struct Report {
    offset: i64,
}

impl Report {
    pub fn new(offset: i64) -> Self {
        Self { offset }
    }

    #[allow(static_mut_refs)]
    fn process_records(&self) -> Vec<Vec<CallFrame>> {
        let records: Vec<super::PProfRecord> = unsafe {
            PPROF_CACHE
                .try_read()
                .map(|cache| cache.iter().cloned().collect())
                .unwrap_or_default()
        };

        records.iter().map(|r| r.resolve()).collect::<Vec<_>>()
    }

    fn stack_to_line(&self, stack: Vec<CallFrame>) -> String {
        let line = stack
            .into_iter()
            .rev()
            .map(|loc| {
                (match loc {
                    CallFrame::CFrame {
                        ip: _,
                        file: _,
                        func,
                        lineno: _,
                    } => func.to_string(),
                    CallFrame::PyFrame {
                        file: _,
                        func,
                        lineno: _,
                        locals: _,
                    } => {
                        format!("py:{func}")
                    }
                })
                .to_string()
            })
            .collect::<Vec<_>>()
            .join(";");
        format!("{line} 1")
    }

    pub fn flamegraph(&self) -> String {
        let records = self.process_records();

        let mut buffer = Vec::new();
        let graph = {
            let lines = records
                .iter()
                .map(|s| self.stack_to_line(s.clone()))
                .collect::<Vec<_>>();
            inferno::flamegraph::from_lines(
                &mut inferno::flamegraph::Options::default(),
                lines.iter().map(|s| s.as_str()),
                &mut buffer,
            )
        };
        match graph {
            Ok(_) => String::from_utf8(buffer).unwrap(),
            Err(e) => {
                println!("Error: {e}");
                String::default()
            }
        }
    }
}

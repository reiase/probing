use probing_proto::protocol::process::CallFrame;

use crate::Probe;

pub struct CCProbe {}

impl Probe for CCProbe {
    // fn backtrace(
    //     &self,
    //     _depth: Option<i32>,
    // ) -> anyhow::Result<Vec<probing_proto::protocol::process::CallFrame>> {
    //     let mut frames = vec![];
    //     backtrace::trace(|frame| {
    //         let ip = frame.ip();
    //         let symbol_address = frame.symbol_address() as usize;
    //         backtrace::resolve_frame(frame, |symbol| {
    //             let func = symbol.name().and_then(|name| name.as_str());
    //             let func = func
    //                 .map(|x| x.to_string())
    //                 .unwrap_or(format!("unknown@{:#x}", symbol_address));

    //             let file = symbol.filename();
    //             let file = file
    //                 .map(|x| x.to_string_lossy().to_string())
    //                 .unwrap_or_default();

    //             frames.push(CallFrame::CFrame {
    //                 ip: ip as usize,
    //                 file,
    //                 func,
    //                 lineno: symbol.lineno().unwrap_or_default() as i64,
    //             });
    //         });
    //         true
    //     });
    //     Ok(frames)
    // }

    fn eval(&self, _code: &str) -> anyhow::Result<String> {
        todo!()
    }
}

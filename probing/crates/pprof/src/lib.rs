#[macro_use]
extern crate ctor;

mod signal;
mod timer;

pub use signal::get_pprof_signal_handler;
pub use signal::set_pprof_signal_handler;
pub use signal::SignalHandler;
pub use timer::reset_pprof_timer;

use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Default)]
pub struct PProf {
    freq: i64,
}

impl PProf {
    pub fn new() -> Self {
        PProf { freq: 0 }
    }

    pub fn start(&mut self, freq: Option<i64>) {
        if let Some(freq) = freq {
            self.freq = freq;
        }
        reset_pprof_timer(self.freq);
    }

    pub fn stop(&self) {
        reset_pprof_timer(0);
    }

    pub fn set_handler(&mut self, handler: SignalHandler) {
        self.stop();
        std::thread::sleep(std::time::Duration::from_secs(1));
        set_pprof_signal_handler(handler);
        self.start(None);
    }
}

// pub static mut PPROF: std::sync::LazyLock<PProf> = std::sync::LazyLock::new(Default::default);
pub static PPROF: Lazy<Mutex<PProf>> = Lazy::new(|| Mutex::new(PProf::new()));

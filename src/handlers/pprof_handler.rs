use once_cell::sync::Lazy;
use pprof::ProfilerGuard;
use pprof::ProfilerGuardBuilder;
use std::sync::Mutex;

use hyperparameter::*;

pub struct PprofHolder(Mutex<Option<ProfilerGuard<'static>>>);

impl PprofHolder {
    pub fn reset(&self) {
        let _ = self.0.lock().map(|mut holder| {
            *holder = None;
        });
    }

    pub fn setup(&self, freq: i32) {
        let _ = self.0.lock().map(|mut holder| {
            match ProfilerGuardBuilder::default().frequency(freq).build() {
                Ok(ph) => holder.replace(ph),
                Err(_) => todo!(),
            };
        });
    }

    // pub fn report(&self) -> Option<String> {
    //     self.0.lock().ok().and_then(|pp| match pp.as_ref() {
    //         Some(pp) => {
    //             if let Ok(report) = pp.report().build() {
    //                 Some(format!("report: {:?}", &report))
    //             } else {
    //                 None
    //             }
    //         }
    //         None => None,
    //     })
    // }

    pub fn flamegraph(&self) -> Option<String> {
        self.0.lock().ok().and_then(|pp| match pp.as_ref() {
            Some(pp) => {
                if let Ok(report) = pp.report().build() {
                    let mut graph: Vec<u8> = vec![];
                    report.flamegraph(&mut graph).unwrap();
                    String::from_utf8(graph).ok()
                } else {
                    None
                }
            }
            None => None,
        })
    }
}

pub static PPROF_HOLDER: Lazy<PprofHolder> = Lazy::new(|| PprofHolder(Mutex::new(None)));

pub fn pprof_handler() {
    with_params! {
        get freq = probing.pprof.freq or 100;

        PPROF_HOLDER.setup(freq as i32);
    }
}

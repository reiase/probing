use anyhow::Result;

use once_cell::sync::Lazy;
use pprof::ProfilerGuard;
use pprof::ProfilerGuardBuilder;
use std::sync::Mutex;

pub struct PprofHolder(Mutex<Option<ProfilerGuard<'static>>>);

impl PprofHolder {
    pub fn reset(&self) {
        let _ = self.0.lock().map(|mut holder| {
            *holder = None;
        });
    }

    pub fn setup(&self, freq: i32) {
        log::debug!("setup pprof with sample freq: {freq}");
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

    pub fn flamegraph(&self) -> Result<String> {
        let holder = self.0.lock().unwrap();

        if let Some(pp) = holder.as_ref() {
            let report = pp.report().build()?;
            let mut graph: Vec<u8> = vec![];
            report.flamegraph(&mut graph).unwrap();
            let graph = String::from_utf8(graph)?;
            Ok(graph)
        } else {
            Err(anyhow::anyhow!("no pprof"))
        }
    }
}

pub static PPROF_HOLDER: Lazy<PprofHolder> = Lazy::new(|| PprofHolder(Mutex::new(None)));

pub fn pprof_handler() {
    PPROF_HOLDER.setup(100);
}

pub fn setup(freq: u64) -> Result<()> {
    PPROF_HOLDER.setup(freq as i32);
    Ok(())
}

pub fn flamegraph() -> Result<String> {
    PPROF_HOLDER.flamegraph()
}

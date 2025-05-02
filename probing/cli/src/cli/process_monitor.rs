use anyhow::Error;
use anyhow::Result;
use log::error;
use std::collections::HashSet;
use std::thread;
use std::time::Duration;

use super::ctrl::ProbeEndpoint;
use super::inject;

#[derive(Debug)]
pub struct ProcessMonitor {
    child: std::process::Child,
    recursive: bool,
    injected: HashSet<i32>,
}

impl ProcessMonitor {
    pub fn new(args: &[String], recursive: bool) -> Result<Self> {
        let child = std::process::Command::new(&args[0])
            .args(&args[1..])
            .spawn()?;

        Ok(Self {
            child,
            injected: HashSet::new(),
            recursive,
        })
    }

    async fn inject(&mut self, pid: i32) -> Result<()> {
        if self.injected.contains(&pid) {
            return Ok(());
        }

        let ctrl: ProbeEndpoint = pid.to_string().as_str().try_into()?;
        inject::InjectCommand::default().run(ctrl).await?;
        self.injected.insert(pid);
        Ok(())
    }

    pub async fn monitor(&mut self) -> Result<()> {
        if !self.recursive {
            thread::sleep(Duration::from_secs(1));
            self.inject(self.child.id() as i32).await?;

            return self.child.wait().map_err(Error::msg).map(|_| ());
        }

        while let Ok(None) = self.child.try_wait() {
            if let Ok(children) = get_descendant_pids(self.child.id() as i32) {
                let remain: Vec<i32> = children
                    .iter()
                    .filter(|pid| !self.injected.contains(pid))
                    .cloned()
                    .collect();
                for pid in remain {
                    if let Err(err) = self.inject(pid).await {
                        error!("failed to probe {}, retry later: {}", pid, err)
                    }
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
        Ok(())
    }
}

fn get_descendant_pids(pid: i32) -> Result<Vec<i32>> {
    let mut descendants = Vec::new();
    let processes = procfs::process::all_processes()?;
    for process in processes.filter_map(|x| x.ok()) {
        if let Ok(stat) = process.stat() {
            if stat.ppid == pid {
                let child_pid = process.pid();
                descendants.push(child_pid);
            }
        }
    }

    Ok(descendants)
}

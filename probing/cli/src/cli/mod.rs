use anyhow::Result;
use clap::Parser;
use probing_proto::prelude::Query;
use process_monitor::ProcessMonitor;

pub mod commands;
pub mod ctrl;
pub mod inject;
pub mod process_monitor;
pub mod store;

mod ptree;

use crate::cli::ctrl::ProbeEndpoint;
use commands::Commands;

/// Probing CLI - A performance and stability diagnostic tool for AI applications
#[derive(Parser, Debug)]
#[command(version = "0.2.0")]
pub struct Cli {
    /// Enable verbose mode
    #[arg(short, long, global = true)]
    verbose: bool,

    /// target process, PID (e.g., 1234) for local process, and <ip>:<port> for remote process
    #[arg()]
    target: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub fn run(&self) -> Result<()> {
        let target = self.target.clone().unwrap_or("0".to_string());
        let ctrl: ProbeEndpoint = target.as_str().try_into()?;

        self.execute_command(ctrl)
    }

    fn execute_command(&self, ctrl: ProbeEndpoint) -> Result<()> {
        if self.command.is_none() {
            inject::InjectCommand::default().run(ctrl.clone())?;
            return Ok(());
        }
        let command = self.command.as_ref().unwrap();
        match command {
            Commands::Inject(cmd) => cmd.run(ctrl),
            Commands::Config { setting } => {
                match *setting {
                    Some(ref setting) => {
                        let setting = if !setting.starts_with("set ") & !setting.starts_with("SET ") {
                            format!("set {}", setting)
                        } else {
                            setting.clone()
                        };
                        ctrl::query(ctrl, Query {
                            expr: setting,
                            opts: None,
                        })
                    },
                    None => {
                        ctrl::query(ctrl, Query {
                            expr: "select * from information_schema.df_settings where name like 'probing.%';".to_string(),
                            opts: None,
                        })
                    },
                }
            },
            Commands::Backtrace{tid} => {
                ctrl.backtrace(*tid)
            },
            Commands::Eval { code } => {
                ctrl.eval(code.clone())
            },
            Commands::Query { query } => ctrl::query(
                ctrl,
                Query {
                    expr: query.clone(),
                    opts: None,
                },
            ),
            Commands::Launch { recursive, args } => {
                ProcessMonitor::new(args, *recursive)?.monitor()
            }
            Commands::List { verbose, tree } => {
                match ptree::collect_probe_processes() {
                    Ok(processes) => {
                        if processes.is_empty() {
                            println!("No processes with injected probes found.");
                            return Ok(());
                        }
            
                        if *tree {
                            // Build and display process tree
                            let tree_nodes = ptree::build_process_tree(processes);
                            println!("Processes with injected probes (tree view):");
                            ptree::print_process_tree(&tree_nodes, *verbose, "", true);
                        } else {
                            // Display flat list
                            println!("Processes with injected probes:");
                            for process in processes {
                                if *verbose {
                                    println!("PID {} ({}): {}", 
                                        process.pid,
                                        if let Some(socket) = &process.socket_name { socket } else { "-" },
                                        process.cmd
                                    );
                                } else {
                                    println!("PID {}: {}", process.pid, process.cmd);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error listing processes: {}", e);
                    }
                }
                Ok(())
            },
            Commands::Store(cmd) => {
                cmd.run()
            }
        }
    }
}

pub fn run() -> Result<()> {
    Cli::parse().run()
}

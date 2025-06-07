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

    #[arg()]
    query: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub async fn run(&mut self) -> Result<()> {
        let target = self.target.clone().unwrap_or("0".to_string());

        if let Some(query) = &self.query {
            self.command = Some(Commands::Query {
                query: query.clone(),
            });
        }

        let ctrl: ProbeEndpoint = target.as_str().try_into()?;
        self.execute_command(ctrl).await
    }

    async fn execute_command(&self, ctrl: ProbeEndpoint) -> Result<()> {
        if self.command.is_none() {
            inject::InjectCommand::default().run(ctrl.clone()).await?;
            return Ok(());
        }
        let command = self.command.as_ref().unwrap();
        match command {
            Commands::Inject(cmd) => cmd.run(ctrl).await,
            Commands::Config { options, setting } => {
                let options_cfg = options.to_cfg();

                let query_expr = match (setting, options_cfg) {
                    (Some(setting_str), Some(opts_str)) => {
                        let setting = if !setting_str.starts_with("set ")
                            && !setting_str.starts_with("SET ")
                        {
                            format!("set {}", setting_str)
                        } else {
                            setting_str.clone()
                        };
                        format!("{}; {}", setting, opts_str)
                    }
                    (Some(setting_str), None) => {
                        if !setting_str.starts_with("set ") && !setting_str.starts_with("SET ") {
                            format!("set {}", setting_str)
                        } else {
                            setting_str.clone()
                        }
                    }
                    (None, Some(opts_str)) => opts_str,
                    (None, None) => {
                        "select * from information_schema.df_settings where name like 'probing.%';"
                            .to_string()
                    }
                };

                ctrl::query(
                    ctrl,
                    Query {
                        expr: query_expr,
                        opts: None,
                    },
                )
                .await
            }
            Commands::Backtrace { tid } => ctrl.backtrace(*tid).await,
            Commands::Eval { code } => ctrl.eval(code.clone()).await,
            Commands::Query { query } => ctrl::query(ctrl, Query::new(query.clone())).await,
            Commands::Launch { recursive, args } => {
                ProcessMonitor::new(args, *recursive)?.monitor().await
            }
            Commands::List { verbose, tree } => {
                match ptree::collect_probe_processes().await {
                    Ok(processes) => {
                        if processes.is_empty() {
                            println!("No processes with injected probes found.");
                            return Ok(());
                        }

                        if *tree {
                            let tree_nodes = ptree::build_process_tree(processes);
                            println!("Processes with injected probes (tree view):");
                            ptree::print_process_tree(&tree_nodes, *verbose, "");
                        } else {
                            println!("Processes with injected probes:");
                            for p in processes {
                                println!("{}", ptree::format_process(&p, *verbose));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error listing processes: {}", e);
                    }
                }
                Ok(())
            }
            Commands::Store(cmd) => cmd.run().await,
        }
    }
}

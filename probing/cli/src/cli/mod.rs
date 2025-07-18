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
use once_cell::sync::Lazy;

fn get_build_info() -> String {
    let mut info = "0.2.0".to_string();

    if let Some(timestamp) = option_env!("VERGEN_BUILD_TIMESTAMP") {
        info.push_str(&format!("\nBuild Timestamp: {timestamp}"));
    }

    if let Some(rustc_version) = option_env!("VERGEN_RUSTC_SEMVER") {
        info.push_str(&format!("\nrustc version: {rustc_version}"));
    }

    info
}

static BUILD_INFO: Lazy<String> = Lazy::new(get_build_info);

/// Probing CLI - A performance and stability diagnostic tool for AI applications
#[derive(Parser, Debug)]
#[command(version = BUILD_INFO.as_str())]
pub struct Cli {
    /// Enable verbose mode
    #[arg(short, long, global = true)]
    verbose: bool,

    /// target process, PID (e.g., 1234) for local process, and <ip>:<port> for remote process
    #[arg(short, long)]
    target: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

impl Cli {
    pub async fn run(&mut self) -> Result<()> {
        // Handle external commands first to avoid target requirement
        if let Some(Commands::External(args)) = &self.command {
            std::env::set_var("PROBING_ENDPOINT", self.target.clone().unwrap_or_default());
            return handle_external_command(args);
        }

        // Handle commands that don't need a target
        match &self.command {
            Some(Commands::List { verbose, tree }) => {
                return self.handle_list_command(*verbose, *tree).await;
            }
            Some(Commands::Launch { recursive, args }) => {
                return ProcessMonitor::new(args, *recursive)?.monitor().await;
            }
            Some(Commands::Store(cmd)) => {
                return cmd.run().await;
            }
            _ => {}
        }

        // For other commands, we need a target
        let target = self.target.clone().unwrap_or("0".to_string());
        let ctrl: ProbeEndpoint = target.as_str().try_into()?;
        self.execute_command(ctrl).await
    }

    async fn handle_list_command(&self, verbose: bool, tree: bool) -> Result<()> {
        match ptree::collect_probe_processes().await {
            Ok(processes) => {
                if processes.is_empty() {
                    println!("No processes with injected probes found.");
                    return Ok(());
                }

                if tree {
                    let tree_nodes = ptree::build_process_tree(processes);
                    println!("Processes with injected probes (tree view):");
                    ptree::print_process_tree(&tree_nodes, verbose, "");
                } else {
                    println!("Processes with injected probes:");
                    for p in processes {
                        println!("{}", ptree::format_process(&p, verbose));
                    }
                }
            }
            Err(e) => {
                eprintln!("Error listing processes: {e}");
            }
        }
        Ok(())
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
                            format!("set {setting_str}")
                        } else {
                            setting_str.clone()
                        };
                        format!("{setting}; {opts_str}")
                    }
                    (Some(setting_str), None) => {
                        if !setting_str.starts_with("set ") && !setting_str.starts_with("SET ") {
                            format!("set {setting_str}")
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
            Commands::Backtrace { tid, cpp, py } => {
                if *cpp && *py {
                    eprintln!("Cannot use both --cpp and --py options simultaneously.");
                    return Err(anyhow::anyhow!("Invalid options"));
                }
                ctrl.backtrace(*tid, *cpp, *py).await
            }
            Commands::Rdma { hca_name } => {
                let hca_name = hca_name.clone().unwrap_or_default();
                ctrl.rdma(hca_name).await
            }
            Commands::Eval { code } => ctrl.eval(code.clone()).await,
            Commands::Query { query } => ctrl::query(ctrl, Query::new(query.clone())).await,
            // These commands are handled in run() method and don't need a target
            Commands::Launch { .. }
            | Commands::List { .. }
            | Commands::Store(..)
            | Commands::External(..) => {
                unreachable!("These commands should be handled in run() method")
            }
        }
    }
}

fn handle_external_command(args: &[String]) -> Result<()> {
    if args.is_empty() {
        eprintln!("Command not specified. Please provide a subcommand.");
        std::process::exit(1);
    }

    let subcommand = &args[0];
    let external_bin = format!("probing-{subcommand}");

    let status = std::process::Command::new(&external_bin)
        .args(&args[1..])
        .status();

    match status {
        Ok(exit_status) => std::process::exit(exit_status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("Error finding external command '{external_bin}'\n\t{e}");
            std::process::exit(1);
        }
    }
}

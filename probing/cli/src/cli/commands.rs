use clap::{Args, Subcommand};

use super::store::StoreCommand;

#[derive(Args, Default, Debug)]
pub struct Settings {
    /// Probing mode - controls how the probe system is enabled
    ///
    /// Supported values:
    ///   - **0**: Disabled (default)
    ///   - **1** or **followed**: Enable only in current process
    ///   - **2** or **nested**: Enable in current and all child processes
    ///   - **regex:PATTERN**: Enable if script name matches regex pattern
    ///   - **SCRIPTNAME**: Enable if script name matches exactly
    ///   - **script:<init script>+[0|1|2]**: Run script and enable with level
    ///
    /// Examples:
    /// ```bash
    /// $ probing <endpoint> config --probing 1
    /// $ probing <endpoint> config --probing script:script.py+1
    /// ```
    #[arg(long, env = "PROBING")]
    probing_mode: Option<String>,

    /// Log level for the probing system
    ///
    /// Supported values:
    ///   - **debug**: Enable debug messages (verbose)
    ///   - **info**: Enable info messages (default)
    ///   - **warn**: Show only warnings and errors
    ///   - **error**: Show only errors
    #[arg(long, env = "PROBING_LOGLEVEL")]
    loglevel: Option<String>,

    /// Root path for assets used by the probing UI dashboard
    ///
    /// Examples:
    /// ```bash
    /// probing <endpoint> config --assets-root /path/to/ui/assets
    /// ```
    #[arg(long, env = "PROBING_ASSETS_ROOT")]
    assets_root: Option<String>,

    /// TCP port for the probing server to listen on
    ///
    /// ```bash
    /// probing <endpoint> config --server-port 8080
    /// ```
    #[arg(long, env = "PROBING_PORT")]
    server_port: Option<u64>,

    /// PyTorch profiling mode
    ///
    /// Supported values:
    ///   - **ordered**: Profiling with ordered sampling
    ///   - **random**: Profiling with random sampling
    #[arg(long, env = "PROBING_TORCH_PROFILING_MODE")]
    torch_profiling_mode: Option<String>,

    /// PyTorch profiling sample rate (range: 0.0-1.0)
    ///
    /// Example:
    /// ```bash
    /// probing <endpoint> config --torch-sample-rate 0.01  # 1% sampling
    /// ```
    #[arg(long, env = "PROBING_TORCH_SAMPLE_RATE")]
    torch_sample_rate: Option<f64>,

    /// Variables to capture during PyTorch profiling
    ///
    /// Format: `<variable name>@<function name>` (comma separated)
    ///
    /// Example:
    /// ```bash
    /// probing <endpoint> config --torch-watch "x@forward,y@backward"
    /// ```
    #[arg(long, env = "PROBING_TORCH_WATCH_VARS")]
    torch_watch_vars: Option<String>,
}

impl Settings {
    pub fn to_cfg(&self) -> Option<String> {
        let mut cfg = String::new();
        if let Some(probing_mode) = &self.probing_mode {
            cfg.push_str(&format!("set probing={};", probing_mode));
        }
        if let Some(log_level) = &self.loglevel {
            cfg.push_str(&format!("set server.log_level={};", log_level));
        }
        if let Some(assets_root) = &self.assets_root {
            cfg.push_str(&format!("set server.assets_root={};", assets_root));
        }
        if let Some(server_port) = &self.server_port {
            // Convert port to full address format
            cfg.push_str(&format!("set server.address=0.0.0.0:{};", server_port));
        }
        if let Some(torch_profiling_mode) = &self.torch_profiling_mode {
            cfg.push_str(&format!(
                "set torch.profiling_mode={};",
                torch_profiling_mode
            ));
        }
        if let Some(torch_sample_rate) = &self.torch_sample_rate {
            cfg.push_str(&format!("set torch.sample_rate={};", torch_sample_rate));
        }
        if let Some(torch_watch_variables) = &self.torch_watch_vars {
            cfg.push_str(&format!("set torch.watch_vars={};", torch_watch_variables));
        }

        if cfg.is_empty() {
            None
        } else {
            Some(cfg)
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(visible_aliases = ["in", "i"])]
    Inject(super::inject::InjectCommand),

    /// List all processes with injected probes
    #[command(visible_aliases = ["ls", "l"])]
    List {
        #[arg(short, long, help = "Show detailed information")]
        verbose: bool,

        #[arg(short, long, help = "Show processes as a tree structure")]
        tree: bool,
    },

    /// Fetch multi ranks' stack info
    #[command(visible_aliases = ["f"])]
    Fetch {
        #[arg(long, help = "Fetch all ranks")]
        all_pids: bool,

        #[arg(long, help = "Fetch only the specified rank")]
        rank: Option<String>,
    },

    /// Display or modify the configuration
    #[command(visible_aliases = ["cfg", "c"])]
    Config {
        #[command(flatten)]
        options: Settings,

        setting: Option<String>,
    },

    /// Show the backtrace of the target process or thread
    #[command(visible_aliases = ["bt", "b"])]
    Backtrace { tid: Option<i32> },

    /// Evaluate Python code in the target process
    #[command(visible_aliases = ["e"])]
    Eval {
        #[arg()]
        code: String,
    },

    /// Query data from the target process
    #[command(visible_aliases = ["q"])]
    Query {
        #[arg()]
        query: String,
    },

    /// Launch new Python process
    #[command()]
    Launch {
        #[arg(short, long)]
        recursive: bool,

        #[arg()]
        args: Vec<String>,
    },

    /// Access various storage backends
    #[command(subcommand = false, hide = true)]
    Store(StoreCommand),
}

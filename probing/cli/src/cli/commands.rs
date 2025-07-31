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

    #[arg(long, env = "PROBING_RDMA_SAMPLE_RATE")]
    rdma_sample_rate: Option<f64>,

    #[arg(long, env = "PROBING_RDMA_HCA_NAME")]
    rdma_hca_name: Option<String>,
}

impl Settings {
    pub fn to_cfg(&self) -> Option<String> {
        let mut cfg = String::new();
        macro_rules! set_if_some {
            ($field:expr, $key:expr) => {
                if let Some(value) = &$field {
                    cfg.push_str(&format!("set {}={};", $key, value));
                }
            };
            ($field:expr, $key:expr, $formatter:expr) => {
                if let Some(value) = &$field {
                    cfg.push_str(&format!("set {}={};", $key, $formatter(value)));
                }
            };
        }

        set_if_some!(self.probing_mode, "probing");
        set_if_some!(self.loglevel, "server.log_level");
        set_if_some!(self.assets_root, "server.assets_root");
        set_if_some!(self.server_port, "server.address", |p| format!(
            "0.0.0.0:{p}"
        ));
        set_if_some!(self.torch_profiling_mode, "torch.profiling_mode");
        set_if_some!(self.torch_sample_rate, "torch.sample_rate");
        set_if_some!(self.torch_watch_vars, "torch.watch_vars");

        set_if_some!(self.rdma_sample_rate, "rdma.sample_rate", |r| {
            format!("{r:.2}")
        });
        set_if_some!(self.rdma_hca_name, "rdma.hca_name");

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

    /// Get RDMA flow of the target process or thread
    #[command(visible_aliases = ["rd"])]
    Rdma { hca_name: Option<String> },

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

    #[command(external_subcommand)]
    External(Vec<String>),

    /// Access various storage backends
    #[command(subcommand = false, hide = true)]
    Store(StoreCommand),
}

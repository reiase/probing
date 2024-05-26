use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct ProbeFlags {
    /// signal libprobe to dump the calling stack of the target process
    #[arg(short, long, action)]
    pub dump: bool,

    /// signal libprobe to pause the target process and listen for remote connection
    #[arg(short, long, action)]
    pub pause: bool,

    /// signal libprobe to start profiling
    #[arg(short = 'P', long, action)]
    pub pprof: bool,

    /// signal libprobe to handle target process crash
    #[arg(short, long, action)]
    pub crash: bool,

    /// signal libprobe to start background server
    #[arg(short, long, action)]
    pub background: bool,

    /// signal libprobe to execute a script in the target process
    #[arg(short, long)]
    pub execute: Option<String>,

    /// address used for listening remote connection
    #[arg(short, long)]
    pub address: Option<String>,

    /// dll file to be injected into the target process, default: <location of probe cli>/libprobe.so
    #[arg(long)]
    pub dll: Option<std::path::PathBuf>,

    /// target process
    #[arg()]
    pub pid: Option<u32>,
}

impl Default for ProbeFlags {
    fn default() -> Self {
        Self {
            dump: Default::default(),
            pause: Default::default(),
            pprof: Default::default(),
            crash: Default::default(),
            background: Default::default(),
            execute: Default::default(),
            address: Default::default(),
            dll: Default::default(),
            pid: Default::default(),
        }
    }
}

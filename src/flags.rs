use argh::FromArgs;

/// flags for libprobe
#[derive(FromArgs, Debug)]
pub struct ProbeFlags {
    /// signal libprobe to dump the calling stack of the target process
    #[argh(switch, short = 'd')]
    pub dump: bool,

    /// signal libprobe to pause the target process and listen for remote connection
    #[argh(switch, short = 'p')]
    pub pause: bool,

    /// signal libprobe to start profiling
    #[argh(switch, short = 'P')]
    pub pprof: bool,

    /// signal libprobe to handle target process crash
    #[argh(switch, short = 'c')]
    pub crash: bool,

    /// signal libprobe to start background server
    #[argh(switch, short = 'b')]
    pub background: bool,

    /// signal libprobe to execute a script in the target process
    #[argh(option, short = 'e')]
    pub execute: Option<String>,

    /// address used for listening remote connection
    #[argh(option, short = 'a')]
    pub address: Option<String>,

    /// dll file to be injected into the target process, default: <location of probe cli>/libprobe.so
    #[argh(option)]
    pub dll: Option<std::path::PathBuf>,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argh_parse() {
        let argstr = "-P -b -a 127.0.0.1:8080 -e test";
        let split_args: Vec<&str> = argstr.split(" ").collect();
        let args = ProbeFlags::from_args(&["cmd"], split_args.as_slice()).unwrap();
        assert_eq!(args.pprof, true);
        assert_eq!(args.background, true);
    }
}
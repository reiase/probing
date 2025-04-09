use anyhow::Result;

use clap::Args;
use clap::Subcommand;
use clap::Parser;

use probing_core::store::TCPStore;

// 包装结构体，包含共享参数和子命令
#[derive(Parser, Debug)]
pub struct StoreCommand {
    #[command(flatten)]
    pub args: StoreArgs,
    
    #[command(subcommand)]
    pub command: StoreSubCommand,
}

#[derive(Subcommand, Debug)]
pub enum StoreSubCommand {
    #[command()]
    Set {
        /// The key to set
        #[arg()]
        key: String,

        /// The value to store
        #[arg()]
        value: String,
    },

    #[command()]
    Get {
        /// The key to get
        #[arg()]
        key: String,
    },
}

#[derive(Args, Debug)]
pub struct StoreArgs {
    /// Specify the store endpoint (host:port)
    #[arg(long, global = true)]
    endpoint: Option<String>,

    /// Specify the backend type (tcp, redis, etc.)
    #[arg(long, global = true)]
    backend: Option<String>,
}

impl StoreCommand {
    pub fn run(&self) -> Result<()> {
        let store = TCPStore::new(self.args.endpoint.clone().unwrap());

        match &self.command {
            StoreSubCommand::Set { key, value } => {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(store.set(&key, &value))?;
            }
            StoreSubCommand::Get { key } => {
                // let store = TCPStore::new(args.endpoint.clone().unwrap());
                let value = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(store.get(&key))?;
                println!("{}", value);
            }
        }
        Ok(())
    }
}

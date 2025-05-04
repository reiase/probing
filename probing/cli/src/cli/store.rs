use anyhow::Result;

use clap::Args;
use clap::Parser;
use clap::Subcommand;

use probing_store::store::TCPStore;

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
    pub async fn run(&self) -> Result<()> {
        let store = TCPStore::new(self.args.endpoint.clone().unwrap());

        match &self.command {
            StoreSubCommand::Set { key, value } => {
                store.set(key, value).await?;
                println!("Set key '{}'", key); // Add confirmation
            }
            StoreSubCommand::Get { key } => {
                let value = store.get(key).await?;
                println!("{}", value);
            }
        }
        Ok(())
    }
}

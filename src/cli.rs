use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "streamshare")]
#[command(about = "Video/audio streaming application")]
pub struct Args {

    #[command(subcommand)]
    pub command: Commands,
}


#[derive(Subcommand, Debug)]
pub enum Commands {

    /// Start the client
    Client {
        #[arg(long)]
        config_path: String,
    },


    /// Start the server
    Server {
        #[arg(long)]
        config_path: String,
    },
}

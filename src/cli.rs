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
        /// username
        #[arg(long)]
        username: String,
        /// Server address
        #[arg(long)]
        address: String,

        /// TCP port
        #[arg(long)]
        tcp_port: u16,

        /// UDP port
        #[arg(long)]
        udp_port: u16,
    },


    /// Start the server
    Server {
        /// TCP port
        #[arg(long)]
        tcp_port: u16,

        /// UDP port
        #[arg(long)]
        udp_port: u16,
    },
}

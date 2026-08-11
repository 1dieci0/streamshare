use clap::{Parser};

mod server;
use server::Server;
mod client;
use client::Client;
mod cli;
use cli::{Args, Commands};
mod protocol;
//mod media;
mod ui;

mod network;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let args = Args::parse();

    rustls::crypto::ring::default_provider()
    .install_default().expect("failed to install rustls crypto provider");

    
    
    match args.command {
    
        Commands::Client {
            config_path,
        } => {
    
            println!("Starting client");
    
            let client = Client::new(config_path)?;
            client.start().await?;
        }
    
    
        Commands::Server {
            config_path
        } => {
    
            println!("Starting server");
    
            let server = Server::new(&config_path)?;
            server.start().await?;
        }
    }

    Ok(())
}

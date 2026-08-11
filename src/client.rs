use std::sync::{Arc, RwLock};

mod endpoint;
pub mod config;
pub mod session;
pub mod command;

use tokio::sync::mpsc;
use quinn::{RecvStream, SendStream};

use anyhow::{Ok, anyhow};
use config::ClientConfig;

use crate::{
    client::session::ClientSession, 
    //media,
    network::{self, stream::{receive_packet, send_packet}}, protocol::{command::{ClientPacket, ServerPacket}, video::VideoPacket}, server::Server, ui};


pub struct Client {

    pub config: ClientConfig,
    
    // pub client_state: Arc<state::ClientState>,
    // pub app_state: Arc<RwLock<ui::state::AppState>>,
    // pub media_state: Arc<media::state::MediaState>,
}


impl Client {
    pub fn new(config_path: String) -> anyhow::Result<(Self)>{
        let config = ClientConfig::load_or_create(config_path)?;
        
        Ok(Self{
            config,
            // client_state: Arc::new(state::ClientState::new()),
            // app_state: Arc::new(RwLock::new(ui::state::AppState::new())),
            // media_state: Arc::new((media::state::MediaState::new())),
        })
    }

    pub async fn start(&self) -> anyhow::Result<()>{

        let (command_tx, command_rx) = tokio::sync::mpsc::channel(128);
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(128);


        let session = ClientSession::connect(
            &self.config,
            command_rx,
            event_tx,
        ).await?;

        // let ui = ui::new(
        //     session.command_tx.clone(),
        // );

        // ui.run().await?;

        let ui = ui::start(
            command_tx,
            event_rx,
        );


        Ok(())
    }

}

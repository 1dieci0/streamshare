use std::sync::{Arc, RwLock, atomic::AtomicBool};

mod endpoint;
pub mod config;
pub mod session;
pub mod command;

use tokio::sync::mpsc;
use quinn::{RecvStream, SendStream};

use anyhow::{Ok, anyhow};
use config::ClientConfig;

use crate::{
    client::session::ClientSession, media::{capture, state::Media}, network::{self, stream::{receive_packet, send_packet}}, protocol::{command::{ClientPacket, ServerPacket}, video::VideoPacket}, server::Server, ui};


pub struct Client {

    pub config: ClientConfig,
    pub media: Arc<Media>,
}


impl Client {
    pub fn new(config_path: String) -> anyhow::Result<(Self)>{
        let config = ClientConfig::load_or_create(config_path)?;
        
        Ok(Self{
            config,
            media: Arc::new(Media::new()),
        })
    }

    pub async fn start(&self) -> anyhow::Result<()>{

        let (command_tx, command_rx) = 
            tokio::sync::mpsc::channel(128);
        let (event_tx, event_rx) = 
            tokio::sync::mpsc::channel(128);
        let (video_tx, video_rx) = 
            tokio::sync::mpsc::channel(8);


        let (_session, uid) = ClientSession::connect(
            &self.config,
            self.media.clone(),
            command_rx,
            event_tx,
            video_rx,
        ).await?;

        capture::start_capture(self.media.clone(), video_tx);

        ui::start(
            command_tx,
            event_rx,
        );


        Ok(())
    }

}

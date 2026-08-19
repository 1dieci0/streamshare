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
    client::session::ClientSession, media::{capture, frame::RawFrame, state::Media}, network::{self, stream::{receive_packet, send_packet}}, protocol::{command::{ClientPacket, ServerPacket}, video::VideoPacket}, server::Server, ui};


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
        let (my_video_tx, my_video_rx) = 
            tokio::sync::mpsc::channel(2);
        let (encoder_tx, encoder_rx) = 
            tokio::sync::mpsc::channel(128);

        let (others_video_tx, others_video_rx) = tokio::sync::mpsc::channel::<RawFrame>(2);


        let (_session, uid) = ClientSession::connect(
            &self.config,
            self.media.clone(),
            command_rx,
            event_tx,
            my_video_rx,
            others_video_tx,
            encoder_tx,
        ).await?;

        capture::start_capture(self.media.clone(), my_video_tx, encoder_rx);

        ui::start(
            command_tx,
            event_rx,
            others_video_rx,
        );


        Ok(())
    }

}

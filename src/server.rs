use std::{
    collections::HashMap, net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, UdpSocket}, path::Path, sync::{Arc, RwLock}, thread,
};

mod endpoint;
//pub mod state;
mod config;

use quinn::{ClientConfig, SendStream, RecvStream};

use crate::{network::{self, stream::{receive_packet, send_packet}}, protocol::command::{ClientPacket, ServerPacket}};

//use state::ServerState;


pub struct Server{
    pub config: config::ServerConfig,
    //state: Arc<RwLock<ServerState>>,
}


impl Server{
    pub fn new(config_path: &str) -> anyhow::Result<Self>{
        let config =
            config::ServerConfig::load_or_create(
                config_path
        )?;

        Ok(Self{
            config,
            /* 
            state: Arc::new(RwLock::new(ServerState {
                users: HashMap::new(),
                streams: HashMap::new(),
            })),
            */
        })
    }

    pub async fn start(self) -> anyhow::Result<()> {

        let endpoint = endpoint::make_server_endpoint(
            self.config.clone()
        )?;

        let server = Arc::new(self);

        while let Some(connecting) = endpoint.accept().await {

            let server = Arc::clone(&server);

            println!("incoming connection");

            //let state = self.state.clone();

            tokio::spawn(async move {

                match connecting.await {

                    Ok(connection) => {
                        println!(
                            "Client connected: {}",
                            connection.remote_address()
                        );
                        
                        //if let Err(e) = handle_connection(connection, state).await
                        if let Err(e) = server.handle_connection(connection).await
                        {
                            eprintln!(
                                "connection error: {e}"
                            );
                        }
                    }


                    Err(e) => {
                        eprintln!(
                            "failed connection: {e}"
                        );
                    }
                }

            });
        }

        Ok(())
    }

    async fn handle_connection(
        &self,
        connection: quinn::Connection,
        //state: Arc<RwLock<ServerState>>,
    ) -> anyhow::Result<()> {

        let (mut send, mut recv) =
                connection.accept_bi().await?;
            
        self.login(&mut send, &mut recv).await?;

        println!("client authed");


        loop {

            tokio::task::yield_now().await;

            // read/write protocol messages here

        }

    }



    async fn login(
        &self,
        send: &mut SendStream,
        recv: &mut RecvStream
    ) -> anyhow::Result<()> {
        let packet: ClientPacket = receive_packet(recv).await?;

        match packet{
            ClientPacket::Authenticate { username, password } => {
                if password == self.config.security.password{

                    let uid = 0;

                    println!("about to send uid");

                    send_packet(send, &ServerPacket::AuthAccepted{uid}
                    ).await?;

                    println!("uid sent");

                    Ok(())

                }else{
                    send_packet(send, &ServerPacket::AuthDenied).await?;

                    Err(anyhow::anyhow!("Auth failed"))
                }
            }

            _ => {Err(anyhow::anyhow!("No auth yet"))}
        }
    }

}


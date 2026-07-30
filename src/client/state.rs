use std::sync::{atomic::{AtomicBool, AtomicU64}, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientCommand {
    None,
    StartStream,
    StopStream,
    Disconnect,
}

pub struct ClientState{
    pub uid: AtomicU64,
    pub streaming: AtomicBool,
    pub sequence: AtomicU64,
    pub command: Mutex<ClientCommand>,
}

impl ClientState{
    pub fn new() -> ClientState{
        ClientState {
            uid: AtomicU64::new(0),
            streaming: AtomicBool::new(false),
            sequence: AtomicU64::new(0),
            command: Mutex::new(ClientCommand::None),
        }
    }

    pub fn set_command(&self, command: ClientCommand) {
        *self.command.lock().unwrap() = command;
    }

    pub fn take_command(&self) -> ClientCommand {
        let mut command = self.command.lock().unwrap();
        std::mem::replace(&mut *command, ClientCommand::None)
    }
}
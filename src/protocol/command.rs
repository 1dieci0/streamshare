
pub trait Packet: Sized{
    fn encode(&self) -> Vec<u8>;
    fn decode(buf: &[u8]) -> Option<Self>;
}


pub enum ClientPacket {
    Authenticate{
        username: String,
        password: String,
    },

    StreamStarted,

    StreamStopped,

    WatchStream{
        uid: u64,
    },

    LeaveStream{
        uid: u64,
    },

    Disconnect,
}

#[derive(Clone)] 
pub enum ServerPacket {
    AuthAccepted{
        uid: u64,
    },

    AuthDenied,
    
    UserJoined{
        uid: u64,
        username: String,
    },

    UserLeft{
        uid: u64,
        username: String,
    },

    StreamStarted{
        uid: u64,
        username: String,
    },

    StreamStopped {
        uid: u64,
        username: String,
    },


    Error{
        error: String,
    }
}

impl Packet for ClientPacket {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();

        match self {
            Self::Authenticate{username, password} => {
                out.push(0x10);

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);

                let passwd = password.as_bytes();
                out.extend_from_slice(&(passwd.len() as u16).to_be_bytes());
                out.extend_from_slice(passwd);
            }

            Self::StreamStarted => {
                out.push(0x11);
            }

            Self::StreamStopped => {
                out.push(0x12);
            }

            Self::WatchStream { uid} => {
                out.push(0x13);
                out.extend_from_slice(&uid.to_be_bytes());
            }

            Self::LeaveStream { uid} => {
                out.push(0x14);
                out.extend_from_slice(&uid.to_be_bytes());
            }

            Self::Disconnect => {
                out.push(0x15);
            }
        }

        out
    }

    fn decode(buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        match buf[0] {

        0x10 => {

            if buf.len() < 5 {
                return None;
            }

            let username_len =
                u16::from_be_bytes(
                    buf[1..3].try_into().ok()?
                ) as usize;

            if buf.len() < 3 + username_len + 2 {
                return None;
            }

            let username =
                String::from_utf8(
                    buf[3..3 + username_len].to_vec()
                ).ok()?;

            let password_len_start =
                3 + username_len;

            let password_len =
                u16::from_be_bytes(
                    buf[
                        password_len_start
                        ..
                        password_len_start + 2
                    ]
                    .try_into()
                    .ok()?
                ) as usize;

            let password_start =
                password_len_start + 2;

            if buf.len() != password_start + password_len {
                return None;
            }

            let password =
                String::from_utf8(
                    buf[
                        password_start
                        ..
                        password_start + password_len
                    ]
                    .to_vec()
                ).ok()?;

            Some(Self::Authenticate {
                username,
                password,
            })
        }
            0x11 => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::StreamStarted)
            }


            0x12 => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::StreamStopped)
            }

            0x13 => {
                if buf.len() != 9 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                Some(Self::WatchStream { uid })
            }

            0x14 => {
                if buf.len() != 9 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                Some(Self::LeaveStream { uid })
            }

            0x15 => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::Disconnect)
            }

            _ => None,
        }
    }
}


impl Packet for ServerPacket{
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();

        match self {
            Self::AuthAccepted{ uid } => {
                out.push(0x1);
                out.extend_from_slice(&uid.to_be_bytes());
            }

            Self::AuthDenied => {
                out.push(0x2);
            }


            Self::UserJoined { uid, username} => {
                out.push(0x4);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }

            Self::UserLeft { uid, username} => {
                out.push(0x5);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }

            Self::StreamStarted { uid, username} => {
                out.push(0x6);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }
            Self::StreamStopped { uid, username} => {
                out.push(0x7);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }

            Self::Error{error} => {
                out.push(0x8);

                let err = error.as_bytes();
                out.extend_from_slice(&(err.len() as u16).to_be_bytes());
                out.extend_from_slice(err);

            }
        }

        out
    }

    fn decode(buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        match buf[0] {

            0x1 => {
                if buf.len() != 9 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                Some(Self::AuthAccepted{ uid })
            }

            0x2 => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::AuthDenied)
            }

            0x4 => {
                if buf.len() < 11 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                let name_len =
                    u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                if buf.len() != 11 + name_len {
                    return None;
                }

                let username =
                    String::from_utf8(buf[11..].to_vec()).ok()?;

                Some(Self::UserJoined {
                    uid,
                    username,
                })
            }

            0x5 => {
                if buf.len() < 11 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                let name_len =
                    u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                if buf.len() != 11 + name_len {
                    return None;
                }

                let username =
                    String::from_utf8(buf[11..].to_vec()).ok()?;

                Some(Self::UserLeft {
                    uid,
                    username,
                })
            }

            0x6 => {
                if buf.len() < 11 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                let name_len =
                    u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                if buf.len() != 11 + name_len {
                    return None;
                }

                let username =
                    String::from_utf8(buf[11..].to_vec()).ok()?;

                Some(Self::StreamStarted {
                    uid,
                    username,
                })
            }

            0x7 => {
                if buf.len() < 11 {
                    return None;
                }

                let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);

                let name_len =
                    u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                if buf.len() != 11 + name_len {
                    return None;
                }

                let username =
                    String::from_utf8(buf[11..].to_vec()).ok()?;

                Some(Self::StreamStopped {
                    uid,
                    username,
                })
            }

            0x8 => {
                if buf.len() < 3 {
                    return None;
                }

                let err_len =
                    u16::from_be_bytes(buf[1..3].try_into().ok()?) as usize;

                if buf.len() != 3 + err_len {
                    return None;
                }

                let error =
                    String::from_utf8(buf[3..].to_vec()).ok()?;

                Some(Self::Error {
                    error,
                })
            }

            _ => None,
        }
    }
}
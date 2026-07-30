pub enum TcpPacket {
    AuthAccepted,
    AuthDenied,
    SendUID {
        uid: u64,
    },

    UserJoined {
        uid: u64,
        username: String,
    },

    UserLeft {
        uid: u64,
        username: String,
    },

    StreamStarted {
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

impl TcpPacket {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();

        match self {
            Self::AuthAccepted => {
                out.push(0x10);
            }
                Self::AuthDenied=> {
                out.push(0x11);
            }

            Self::SendUID { uid } => {
                out.push(0x12);
                out.extend_from_slice(&uid.to_be_bytes());
            }

            Self::UserJoined { uid, username } => {
                out.push(0x13);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }

            Self::UserLeft { uid, username} => {
                out.push(0x14);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }

            Self::StreamStarted { uid, username} => {
                out.push(0x15);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }

            Self::StreamStopped { uid, username } => {
                out.push(0x16);
                out.extend_from_slice(&uid.to_be_bytes());

                let name = username.as_bytes();
                out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                out.extend_from_slice(name);
            }

            Self::Error { error } => {
                out.push(0x17);


                let err = error.as_bytes();
                out.extend_from_slice(&(err.len() as u16).to_be_bytes());
                out.extend_from_slice(err);
            }
        }

        out
    }

    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        match buf[0] {

            0x10 => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::AuthAccepted)
            }

            0x11 => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::AuthDenied)
            }


            0x12 => {
                if buf.len() != 9 {
                    return None;
                }

                Some(Self::SendUID {
                    uid: u64::from_be_bytes(buf[1..9].try_into().ok()?),
                })
            }

            0x13 => {
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

            0x14 => {
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

            0x15 => {
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

            0x16 => {
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
            0x17 => {
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
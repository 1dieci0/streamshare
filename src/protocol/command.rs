pub mod packet_id {
    pub const SERVER_AUTH_ACCEPTED: u8 = 0x01;
    pub const SERVER_AUTH_DENIED: u8 = 0x02;
    pub const SERVER_USER_JOINED: u8 = 0x04;
    pub const SERVER_USER_LEFT: u8 = 0x05;
    pub const SERVER_STREAM_STARTED: u8 = 0x06;
    pub const SERVER_STREAM_STOPPED: u8 = 0x07;
    pub const SERVER_ERROR: u8 = 0x08;
    pub const SERVER_INITIAL_STATE: u8 = 0x09;

    pub const CLIENT_AUTHENTICATE: u8 = 0x10;
    pub const CLIENT_STREAM_START: u8 = 0x11;
    pub const CLIENT_STREAM_STOP: u8 = 0x12;
    pub const CLIENT_WATCH_STREAM: u8 = 0x13;
    pub const CLIENT_LEAVE_STREAM: u8 = 0x14;
    pub const CLIENT_DISCONNECT: u8 = 0x15;
}



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

    InitialState{
        users: Vec<super::info::UserInfo>,
        streams: Vec<super::info::StreamInfo>,
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
                out.push(packet_id::CLIENT_AUTHENTICATE);

                // let name = username.as_bytes();
                // out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                // out.extend_from_slice(name);
                put_string(&mut out, username);

                // let passwd = password.as_bytes();
                // out.extend_from_slice(&(passwd.len() as u16).to_be_bytes());
                // out.extend_from_slice(passwd);

                put_string(&mut out, password);
            }

            Self::StreamStarted => {
                out.push(packet_id::CLIENT_STREAM_START);
            }

            Self::StreamStopped => {
                out.push(packet_id::CLIENT_STREAM_STOP);
            }

            Self::WatchStream { uid} => {
                out.push(packet_id::CLIENT_WATCH_STREAM);
                // out.extend_from_slice(&uid.to_be_bytes());
                put_u64(&mut out, *uid);
            }

            Self::LeaveStream { uid} => {
                out.push(packet_id::CLIENT_LEAVE_STREAM);
                // out.extend_from_slice(&uid.to_be_bytes());
                put_u64(&mut out, *uid);
            }

            Self::Disconnect => {
                out.push(packet_id::CLIENT_DISCONNECT);
            }
        }

        out
    }

    fn decode(mut buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        match buf[0] {

            packet_id::CLIENT_AUTHENTICATE => {
                // if buf.len() < 5 {
                //     return None;
                // }

                // let username_len =
                //     u16::from_be_bytes(
                //         buf[1..3].try_into().ok()?
                //     ) as usize;

                // if buf.len() < 3 + username_len + 2 {
                //     return None;
                // }

                // let username =
                //     String::from_utf8(
                //         buf[3..3 + username_len].to_vec()
                //     ).ok()?;
                buf = &buf[1..];

                let username = read_string(&mut buf)?;
                // let password_len_start =
                //     3 + username_len;

                // let password_len =
                //     u16::from_be_bytes(
                //         buf[
                //             password_len_start
                //             ..
                //             password_len_start + 2
                //         ]
                //         .try_into()
                //         .ok()?
                //     ) as usize;

                // let password_start =
                //     password_len_start + 2;

                // if buf.len() != password_start + password_len {
                //     return None;
                // }

                // let password =
                //     String::from_utf8(
                //         buf[
                //             password_start
                //             ..
                //             password_start + password_len
                //         ]
                //         .to_vec()
                //     ).ok()?;

                let password = read_string(&mut buf)?;

                Some(Self::Authenticate {
                    username,
                    password,
                })
            }
            
            packet_id::CLIENT_STREAM_START => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::StreamStarted)
            }


            packet_id::CLIENT_STREAM_STOP => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::StreamStopped)
            }

            packet_id::CLIENT_WATCH_STREAM => {
                // if buf.len() != 9 {
                //     return None;
                // }

                // let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);
                buf = &buf[1..];

                let uid = read_u64(&mut buf)?;

                Some(Self::WatchStream { uid })
            }

            packet_id::CLIENT_LEAVE_STREAM => {
                // if buf.len() != 9 {
                //     return None;
                // }

                // let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);
                buf = &buf[1..];

                let uid = read_u64(&mut buf)?;

                Some(Self::LeaveStream { uid })
            }

            packet_id::CLIENT_DISCONNECT => {
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
                out.push(packet_id::SERVER_AUTH_ACCEPTED);
                // out.extend_from_slice(&uid.to_be_bytes());
                put_u64(&mut out, *uid);
            }

            Self::AuthDenied => {
                out.push(packet_id::SERVER_AUTH_DENIED);
            }


            Self::UserJoined { uid, username} => {
                out.push(packet_id::SERVER_USER_JOINED);
                // out.extend_from_slice(&uid.to_be_bytes());
                put_u64(&mut out, *uid);

                // let name = username.as_bytes();
                // out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                // out.extend_from_slice(name);
                put_string(&mut out, username);
            }

            Self::UserLeft { uid, username} => {
                out.push(packet_id::SERVER_USER_LEFT);
                // out.extend_from_slice(&uid.to_be_bytes());
                put_u64(&mut out, *uid);


                // let name = username.as_bytes();
                // out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                // out.extend_from_slice(name);
                put_string(&mut out, username);

            }

            Self::StreamStarted { uid, username} => {
                out.push(packet_id::SERVER_STREAM_STARTED);
                // out.extend_from_slice(&uid.to_be_bytes());
                put_u64(&mut out, *uid);

                // let name = username.as_bytes();
                // out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                // out.extend_from_slice(name);
                put_string(&mut out, username);
            }
            Self::StreamStopped { uid, username} => {
                out.push(packet_id::SERVER_STREAM_STOPPED);
                // out.extend_from_slice(&uid.to_be_bytes());
                put_u64(&mut out, *uid);


                // let name = username.as_bytes();
                // out.extend_from_slice(&(name.len() as u16).to_be_bytes());
                // out.extend_from_slice(name);
                put_string(&mut out, username);
            }

            Self::Error{error} => {
                out.push(packet_id::SERVER_ERROR);

                // let err = error.as_bytes();
                // out.extend_from_slice(&(err.len() as u16).to_be_bytes());
                // out.extend_from_slice(err);
                put_string(&mut out, error);

            }

            Self::InitialState { users, streams } => {
                out.push(packet_id::SERVER_INITIAL_STATE);

                out.extend_from_slice(&(users.len() as u32).to_be_bytes());

                for user in users {
                    out.extend_from_slice(&user.uid.to_be_bytes());
                    put_string(&mut out, &user.username);
                }

                out.extend_from_slice(&(streams.len() as u32).to_be_bytes());

                for stream in streams {
                    out.extend_from_slice(&stream.uid.to_be_bytes());
                    put_string(&mut out, &stream.username);
                }
            }
        }

        out
    }

    fn decode(mut buf: &[u8]) -> Option<Self> {
        if buf.is_empty() {
            return None;
        }

        match buf[0] {

            packet_id::SERVER_AUTH_ACCEPTED => {
                // if buf.len() != 9 {
                //     return None;
                // }

                // let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);
                buf = &buf[1..];
                let uid = read_u64(&mut buf)?;

                Some(Self::AuthAccepted{ uid })
            }

            packet_id::SERVER_AUTH_DENIED => {
                if buf.len() != 1 {
                    return None;
                }

                Some(Self::AuthDenied)
            }

            packet_id::SERVER_USER_JOINED => {
                // if buf.len() < 11 {
                //     return None;
                // }

                // let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);
                buf = &buf[1..];
                let uid = read_u64(&mut buf)?;

                // let name_len =
                //     u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                // if buf.len() != 11 + name_len {
                //     return None;
                // }

                // let username =
                //     String::from_utf8(buf[11..].to_vec()).ok()?;
                let username = read_string(&mut buf)?;

                Some(Self::UserJoined {
                    uid,
                    username,
                })
            }

            packet_id::SERVER_USER_LEFT => {
                // if buf.len() < 11 {
                //     return None;
                // }

                // let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);
                buf = &buf[1..];

                let uid = read_u64(&mut buf)?;

                // let name_len =
                //     u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                // if buf.len() != 11 + name_len {
                //     return None;
                // }

                // let username =
                //     String::from_utf8(buf[11..].to_vec()).ok()?;

                let username = read_string(&mut buf)?;

                Some(Self::UserLeft {
                    uid,
                    username,
                })
            }

            packet_id::SERVER_STREAM_STARTED => {
                // if buf.len() < 11 {
                //     return None;
                // }

                // let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);
                buf = &buf[1..];

                let uid = read_u64(&mut buf)?;

                // let name_len =
                //     u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                // if buf.len() != 11 + name_len {
                //     return None;
                // }

                // let username =
                //     String::from_utf8(buf[11..].to_vec()).ok()?;

                let username = read_string(&mut buf)?;

                Some(Self::StreamStarted {
                    uid,
                    username,
                })
            }

            packet_id::SERVER_STREAM_STOPPED => {
                // if buf.len() < 11 {
                //     return None;
                // }

                // let uid = u64::from_be_bytes(buf[1..9].try_into().ok()?);
                buf = &buf[1..];

                let uid = read_u64(&mut buf)?;

                // let name_len =
                //     u16::from_be_bytes(buf[9..11].try_into().ok()?) as usize;

                // if buf.len() != 11 + name_len {
                //     return None;
                // }

                // let username =
                //     String::from_utf8(buf[11..].to_vec()).ok()?;

                let username = read_string(&mut buf)?;

                Some(Self::StreamStopped {
                    uid,
                    username,
                })
            }

            packet_id::SERVER_ERROR => {
                // if buf.len() < 3 {
                //     return None;
                // }

                // let err_len =
                //     u16::from_be_bytes(buf[1..3].try_into().ok()?) as usize;

                // if buf.len() != 3 + err_len {
                //     return None;
                // }

                // let error =
                //     String::from_utf8(buf[3..].to_vec()).ok()?;
                buf = &buf[1..];

                let error = read_string(&mut buf)?;

                Some(Self::Error {
                    error,
                })
            }

            packet_id::SERVER_INITIAL_STATE => {
                let mut data = &buf[1..];

                if data.len() < 4 {
                    return None;
                }

                let user_count =
                    u32::from_be_bytes(
                        data[..4].try_into().ok()?
                    ) as usize;

                data = &data[4..];

                let mut users = Vec::with_capacity(user_count);

                for _ in 0..user_count {
                    let uid = read_u64(&mut data)?;
                    let username = read_string(&mut data)?;

                    users.push(super::info::UserInfo {
                        uid,
                        username,
                    });
                }

                if data.len() < 4 {
                    return None;
                }

                let stream_count =
                    u32::from_be_bytes(
                        data[..4].try_into().ok()?
                    ) as usize;

                data = &data[4..];

                let mut streams = Vec::with_capacity(stream_count);

                for _ in 0..stream_count {
                    let uid = read_u64(&mut data)?;
                    let username = read_string(&mut data)?;

                    streams.push(super::info::StreamInfo {
                        uid,
                        username,
                    });
                }

                if !data.is_empty() {
                    return None;
                }

                Some(Self::InitialState {
                    users,
                    streams,
                })
            }

            _ => None,
        }
    }
}




fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn put_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();

    assert!(bytes.len() <= u16::MAX as usize);

    put_u16(out, bytes.len() as u16);
    out.extend_from_slice(bytes);
}



fn read_u16(buf: &mut &[u8]) -> Option<u16> {
    if buf.len() < 2 {
        return None;
    }

    let value = u16::from_be_bytes(buf[..2].try_into().ok()?);
    *buf = &buf[2..];

    Some(value)
}

fn read_u64(buf: &mut &[u8]) -> Option<u64> {
    if buf.len() < 8 {
        return None;
    }

    let value = u64::from_be_bytes(buf[..8].try_into().ok()?);
    *buf = &buf[8..];

    Some(value)
}

fn read_string(buf: &mut &[u8]) -> Option<String> {
    let len = read_u16(buf)? as usize;

    if buf.len() < len {
        return None;
    }

    let value = String::from_utf8(buf[..len].to_vec()).ok()?;

    *buf = &buf[len..];

    Some(value)
}
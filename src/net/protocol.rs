pub(super) const KIND_HEARTBEAT: u8 = 0;
pub(super) const KIND_CONNECT: u8 = 1;
pub(super) const KIND_CONNECT_ACK: u8 = 2;
pub(super) const KIND_DISCONNECT: u8 = 3;
pub(super) const KIND_DATA: u8 = 4;

pub(super) const HEARTBEAT_INTERVAL: f32 = 1.0;
pub(super) const TIMEOUT_SECS: f32 = 5.0;
pub(super) const MAX_PACKET: usize = 65507;

/// Opaque identifier for a connected client (server-side).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u32);

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum NetError {
    Io(std::io::Error),
    NotConnected,
    UnknownClient(ClientId),
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::Io(e) => write!(f, "io: {e}"),
            NetError::NotConnected => write!(f, "not connected"),
            NetError::UnknownClient(id) => write!(f, "unknown client {id}"),
        }
    }
}

impl From<std::io::Error> for NetError {
    fn from(e: std::io::Error) -> Self {
        NetError::Io(e)
    }
}

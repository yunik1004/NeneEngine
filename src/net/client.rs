use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::{net::UdpSocket, runtime::Runtime, sync::mpsc};

use super::protocol::*;

/// Events delivered to the game loop from the client.
#[derive(Debug)]
pub enum ClientEvent {
    Connected,
    Disconnected,
    Message(Vec<u8>),
}

impl ClientEvent {
    pub fn into_json<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        if let ClientEvent::Message(data) = self {
            serde_json::from_slice(data).ok()
        } else {
            None
        }
    }
}

/// UDP client. Connects to a server and delivers events each frame.
pub struct Client {
    _rt: Runtime,
    rx: mpsc::UnboundedReceiver<ClientEvent>,
    tx_out: mpsc::UnboundedSender<Vec<u8>>,
    tx_ctrl: mpsc::UnboundedSender<u8>,
    connected: Arc<AtomicBool>,
    heartbeat_timer: f32,
}

impl Client {
    /// Connect to a UDP server at `addr` (e.g. `"127.0.0.1:7777"`).
    pub fn connect(server_addr: &str) -> Result<Self, NetError> {
        let server_addr: SocketAddr = server_addr
            .parse()
            .map_err(|_| NetError::Io(std::io::Error::other("invalid address")))?;

        let rt = Runtime::new().map_err(NetError::Io)?;

        let (tx_event, rx_event) = mpsc::unbounded_channel::<ClientEvent>();
        let (tx_out, mut rx_out) = mpsc::unbounded_channel::<Vec<u8>>();
        let (tx_ctrl, mut rx_ctrl) = mpsc::unbounded_channel::<u8>();

        let connected = Arc::new(AtomicBool::new(false));
        let connected_recv = Arc::clone(&connected);

        let socket = rt
            .block_on(async { UdpSocket::bind("0.0.0.0:0").await })
            .map_err(NetError::Io)?;
        rt.block_on(async { socket.connect(server_addr).await })
            .map_err(NetError::Io)?;
        let socket = Arc::new(socket);
        let socket_send = Arc::clone(&socket);

        let tx = tx_event;

        rt.spawn(async move {
            let mut buf = vec![0u8; MAX_PACKET];
            loop {
                let len = match socket.recv(&mut buf).await {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                if len == 0 {
                    continue;
                }
                match buf[0] {
                    KIND_CONNECT_ACK => {
                        connected_recv.store(true, Ordering::Relaxed);
                        if tx.send(ClientEvent::Connected).is_err() {
                            break;
                        }
                    }
                    KIND_DISCONNECT => {
                        connected_recv.store(false, Ordering::Relaxed);
                        if tx.send(ClientEvent::Disconnected).is_err() {
                            break;
                        }
                    }
                    KIND_DATA => {
                        if tx.send(ClientEvent::Message(buf[1..len].to_vec())).is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        rt.spawn(async move {
            loop {
                tokio::select! {
                    Some(kind) = rx_ctrl.recv() => {
                        let _ = socket_send.send(&[kind]).await;
                    }
                    Some(data) = rx_out.recv() => {
                        let mut pkt = Vec::with_capacity(1 + data.len());
                        pkt.push(KIND_DATA);
                        pkt.extend_from_slice(&data);
                        let _ = socket_send.send(&pkt).await;
                    }
                    else => break,
                }
            }
        });

        tx_ctrl.send(KIND_CONNECT).ok();

        Ok(Self {
            _rt: rt,
            rx: rx_event,
            tx_out,
            tx_ctrl,
            connected,
            heartbeat_timer: 0.0,
        })
    }

    pub fn poll(&mut self) -> Vec<ClientEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            if matches!(ev, ClientEvent::Disconnected) {
                self.connected.store(false, Ordering::Relaxed);
            }
            events.push(ev);
        }
        events
    }

    pub fn send(&self, data: &[u8]) -> Result<(), NetError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(NetError::NotConnected);
        }
        self.tx_out
            .send(data.to_vec())
            .map_err(|_| NetError::Io(std::io::Error::other("send task closed")))?;
        Ok(())
    }

    pub fn send_json<T: serde::Serialize>(&self, value: &T) -> Result<(), NetError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        self.send(&bytes)
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub fn update(&mut self, dt: f32) {
        if !self.is_connected() {
            return;
        }
        self.heartbeat_timer += dt;
        if self.heartbeat_timer >= HEARTBEAT_INTERVAL {
            self.heartbeat_timer = 0.0;
            self.tx_ctrl.send(KIND_HEARTBEAT).ok();
        }
    }

    pub fn disconnect(&self) {
        self.tx_ctrl.send(KIND_DISCONNECT).ok();
        self.connected.store(false, Ordering::Relaxed);
    }
}

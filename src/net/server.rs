use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::{net::UdpSocket, runtime::Runtime, sync::mpsc};

use super::protocol::*;

/// Events delivered to the game loop from the server.
#[derive(Debug)]
pub enum ServerEvent {
    Connected(ClientId),
    Disconnected(ClientId),
    Message(ClientId, Vec<u8>),
}

impl ServerEvent {
    pub fn into_json<T: serde::de::DeserializeOwned>(&self) -> Option<(ClientId, T)> {
        if let ServerEvent::Message(id, data) = self {
            serde_json::from_slice(data).ok().map(|v| (*id, v))
        } else {
            None
        }
    }
}

struct ClientState {
    time_since_recv: f32,
}

/// UDP server. Manages connected clients and delivers events each frame.
pub struct Server {
    _rt: Runtime,
    rx: mpsc::UnboundedReceiver<ServerEvent>,
    rx_keepalive: mpsc::UnboundedReceiver<ClientId>,
    tx_out: mpsc::UnboundedSender<(ClientId, Vec<u8>)>,
    clients: HashMap<ClientId, ClientState>,
    next_id: u32,
}

impl Server {
    /// Bind a UDP server on `addr` (e.g. `"0.0.0.0:7777"`).
    pub fn bind(addr: &str) -> Result<Self, NetError> {
        let addr: SocketAddr = addr
            .parse()
            .map_err(|_| NetError::Io(std::io::Error::other("invalid address")))?;

        let rt = Runtime::new().map_err(NetError::Io)?;

        let (tx_event, rx_event) = mpsc::unbounded_channel::<ServerEvent>();
        let (tx_keepalive, rx_keepalive) = mpsc::unbounded_channel::<ClientId>();
        let (tx_out, mut rx_out) = mpsc::unbounded_channel::<(ClientId, Vec<u8>)>();

        let addr_to_id: Arc<tokio::sync::Mutex<HashMap<SocketAddr, ClientId>>> =
            Arc::new(tokio::sync::Mutex::new(HashMap::new()));
        let id_to_addr: Arc<tokio::sync::Mutex<HashMap<ClientId, SocketAddr>>> =
            Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let socket = rt
            .block_on(async { UdpSocket::bind(addr).await })
            .map_err(NetError::Io)?;

        let socket = Arc::new(socket);
        let socket_send = Arc::clone(&socket);

        let a2i = Arc::clone(&addr_to_id);
        let i2a_recv = Arc::clone(&id_to_addr);
        let i2a_send = Arc::clone(&id_to_addr);
        let tx = tx_event;
        let tx_ka = tx_keepalive;

        rt.spawn(async move {
            let mut buf = vec![0u8; MAX_PACKET];
            let mut next_id = 0u32;
            loop {
                let (len, from) = match socket.recv_from(&mut buf).await {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if len == 0 {
                    continue;
                }

                let kind = buf[0];
                let payload = buf[1..len].to_vec();

                match kind {
                    KIND_CONNECT => {
                        let mut a2i_guard = a2i.lock().await;
                        if let std::collections::hash_map::Entry::Vacant(e) = a2i_guard.entry(from)
                        {
                            let id = ClientId(next_id);
                            next_id += 1;
                            e.insert(id);
                            i2a_recv.lock().await.insert(id, from);
                            drop(a2i_guard);
                            let _ = socket.send_to(&[KIND_CONNECT_ACK], from).await;
                            if tx.send(ServerEvent::Connected(id)).is_err() {
                                break;
                            }
                        } else {
                            // Already connected — re-ACK, no new ClientId.
                            drop(a2i_guard);
                            let _ = socket.send_to(&[KIND_CONNECT_ACK], from).await;
                        }
                    }
                    KIND_DISCONNECT => {
                        let mut a2i_guard = a2i.lock().await;
                        if let Some(id) = a2i_guard.remove(&from) {
                            i2a_recv.lock().await.remove(&id);
                            drop(a2i_guard);
                            if tx.send(ServerEvent::Disconnected(id)).is_err() {
                                break;
                            }
                        }
                    }
                    KIND_DATA => {
                        if let Some(&id) = a2i.lock().await.get(&from)
                            && tx.send(ServerEvent::Message(id, payload)).is_err()
                        {
                            break;
                        }
                    }
                    KIND_HEARTBEAT => {
                        if let Some(&id) = a2i.lock().await.get(&from) {
                            let _ = tx_ka.send(id);
                        }
                    }
                    _ => {}
                }
            }
        });

        rt.spawn(async move {
            while let Some((id, data)) = rx_out.recv().await {
                if let Some(&addr) = i2a_send.lock().await.get(&id) {
                    let mut pkt = Vec::with_capacity(1 + data.len());
                    pkt.push(KIND_DATA);
                    pkt.extend_from_slice(&data);
                    let _ = socket_send.send_to(&pkt, addr).await;
                }
            }
        });

        Ok(Self {
            _rt: rt,
            rx: rx_event,
            rx_keepalive,
            tx_out,
            clients: HashMap::new(),
            next_id: 0,
        })
    }

    pub fn poll(&mut self) -> Vec<ServerEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            match &ev {
                ServerEvent::Connected(id) => {
                    self.clients.insert(
                        *id,
                        ClientState {
                            time_since_recv: 0.0,
                        },
                    );
                    self.next_id += 1;
                }
                ServerEvent::Disconnected(id) => {
                    self.clients.remove(id);
                }
                ServerEvent::Message(id, _) => {
                    if let Some(c) = self.clients.get_mut(id) {
                        c.time_since_recv = 0.0;
                    }
                }
            }
            events.push(ev);
        }
        events
    }

    pub fn send(&self, id: ClientId, data: &[u8]) -> Result<(), NetError> {
        if !self.clients.contains_key(&id) {
            return Err(NetError::UnknownClient(id));
        }
        self.tx_out
            .send((id, data.to_vec()))
            .map_err(|_| NetError::Io(std::io::Error::other("send task closed")))?;
        Ok(())
    }

    pub fn broadcast(&self, data: &[u8]) -> Result<(), NetError> {
        for &id in self.clients.keys() {
            self.send(id, data)?;
        }
        Ok(())
    }

    pub fn send_json<T: serde::Serialize>(&self, id: ClientId, value: &T) -> Result<(), NetError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        self.send(id, &bytes)
    }

    pub fn broadcast_json<T: serde::Serialize>(&self, value: &T) -> Result<(), NetError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        self.broadcast(&bytes)
    }

    pub fn update(&mut self, dt: f32) {
        while let Ok(id) = self.rx_keepalive.try_recv() {
            if let Some(state) = self.clients.get_mut(&id) {
                state.time_since_recv = 0.0;
            }
        }
        for state in self.clients.values_mut() {
            state.time_since_recv += dt;
        }
        self.clients
            .retain(|_, state| state.time_since_recv < TIMEOUT_SECS);
    }

    pub fn client_ids(&self) -> Vec<ClientId> {
        self.clients.keys().copied().collect()
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

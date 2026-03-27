//! UDP client/server networking.
//!
//! The network runs on a background tokio runtime. The game loop polls
//! incoming events each frame via a non-blocking channel.
//!
//! # Quick start — server
//! ```no_run
//! use nene::net::{Server, ServerEvent};
//!
//! let mut server = Server::bind("0.0.0.0:7777").unwrap();
//!
//! // each frame:
//! for event in server.poll() {
//!     match event {
//!         ServerEvent::Connected(id)        => println!("client {id} connected"),
//!         ServerEvent::Disconnected(id)     => println!("client {id} disconnected"),
//!         ServerEvent::Message(id, bytes)   => println!("got {} bytes from {id}", bytes.len()),
//!     }
//! }
//! server.broadcast(b"hello everyone").unwrap();
//! server.update(delta_time);
//! ```
//!
//! # Quick start — client
//! ```no_run
//! use nene::net::{Client, ClientEvent};
//!
//! let mut client = Client::connect("127.0.0.1:7777").unwrap();
//!
//! // each frame:
//! for event in client.poll() {
//!     match event {
//!         ClientEvent::Connected              => println!("connected!"),
//!         ClientEvent::Disconnected           => println!("disconnected"),
//!         ClientEvent::Message(bytes)         => println!("got {} bytes", bytes.len()),
//!     }
//! }
//! client.send(b"ping").unwrap();
//! client.update(delta_time);
//! ```

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::{net::UdpSocket, runtime::Runtime, sync::mpsc};

// ── Wire protocol ─────────────────────────────────────────────────────────────

const KIND_HEARTBEAT: u8 = 0;
const KIND_CONNECT: u8 = 1;
const KIND_CONNECT_ACK: u8 = 2;
const KIND_DISCONNECT: u8 = 3;
const KIND_DATA: u8 = 4;

const HEARTBEAT_INTERVAL: f32 = 1.0; // seconds
const TIMEOUT_SECS: f32 = 5.0;
const MAX_PACKET: usize = 65507;

// ── ClientId ──────────────────────────────────────────────────────────────────

/// Opaque identifier for a connected client (server-side).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(pub u32);

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Error ─────────────────────────────────────────────────────────────────────

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

// ── Server ────────────────────────────────────────────────────────────────────

/// Events delivered to the game loop from the server.
#[derive(Debug)]
pub enum ServerEvent {
    Connected(ClientId),
    Disconnected(ClientId),
    Message(ClientId, Vec<u8>),
}

impl ServerEvent {
    /// If this is a `Message`, deserialize the payload as `T`. Otherwise returns `None`.
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
    /// Receive events from background task.
    rx: mpsc::UnboundedReceiver<ServerEvent>,
    /// Keep-alive signals from heartbeats (ClientId whose timer should reset).
    rx_keepalive: mpsc::UnboundedReceiver<ClientId>,
    /// Send raw bytes to background task for delivery.
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

        // addr→id map shared between recv task and send task
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

        // ── recv task ─────────────────────────────────────────────────────────
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
                        let id = ClientId(next_id);
                        next_id += 1;
                        a2i.lock().await.insert(from, id);
                        i2a_recv.lock().await.insert(id, from);
                        // send ack
                        let _ = socket.send_to(&[KIND_CONNECT_ACK], from).await;
                        let _ = tx.send(ServerEvent::Connected(id));
                    }
                    KIND_DISCONNECT => {
                        if let Some(id) = a2i.lock().await.remove(&from) {
                            i2a_recv.lock().await.remove(&id);
                            let _ = tx.send(ServerEvent::Disconnected(id));
                        }
                    }
                    KIND_DATA => {
                        if let Some(&id) = a2i.lock().await.get(&from) {
                            let _ = tx.send(ServerEvent::Message(id, payload));
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

        // ── send task ─────────────────────────────────────────────────────────
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

    /// Drain all pending events from the background task.
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

    /// Send raw bytes to a specific client.
    pub fn send(&self, id: ClientId, data: &[u8]) -> Result<(), NetError> {
        if !self.clients.contains_key(&id) {
            return Err(NetError::UnknownClient(id));
        }
        self.tx_out.send((id, data.to_vec())).ok();
        Ok(())
    }

    /// Send raw bytes to all connected clients.
    pub fn broadcast(&self, data: &[u8]) -> Result<(), NetError> {
        for &id in self.clients.keys() {
            self.send(id, data)?;
        }
        Ok(())
    }

    /// Send a JSON-serializable value to a specific client.
    pub fn send_json<T: serde::Serialize>(&self, id: ClientId, value: &T) -> Result<(), NetError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        self.send(id, &bytes)
    }

    /// Broadcast a JSON-serializable value to all clients.
    pub fn broadcast_json<T: serde::Serialize>(&self, value: &T) -> Result<(), NetError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        self.broadcast(&bytes)
    }

    /// Tick timers. Call once per frame with the frame delta time.
    pub fn update(&mut self, dt: f32) {
        // Reset timer for clients that sent a heartbeat this frame.
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

    /// IDs of all currently connected clients.
    pub fn client_ids(&self) -> Vec<ClientId> {
        self.clients.keys().copied().collect()
    }

    /// Number of connected clients.
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

// ── Client ────────────────────────────────────────────────────────────────────

/// Events delivered to the game loop from the client.
#[derive(Debug)]
pub enum ClientEvent {
    Connected,
    Disconnected,
    Message(Vec<u8>),
}

impl ClientEvent {
    /// If this is a `Message`, deserialize the payload as `T`. Otherwise returns `None`.
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
    tx_ctrl: mpsc::UnboundedSender<u8>, // KIND_* control messages
    connected: Arc<AtomicBool>,
    heartbeat_timer: f32,
}

impl Client {
    /// Connect to a UDP server at `addr` (e.g. `"127.0.0.1:7777"`).
    ///
    /// Sends a connect packet immediately; `ClientEvent::Connected` is
    /// delivered once the server acknowledges.
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

        // ── recv task ─────────────────────────────────────────────────────────
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
                        let _ = tx.send(ClientEvent::Connected);
                    }
                    KIND_DISCONNECT => {
                        connected_recv.store(false, Ordering::Relaxed);
                        let _ = tx.send(ClientEvent::Disconnected);
                    }
                    KIND_DATA => {
                        let _ = tx.send(ClientEvent::Message(buf[1..len].to_vec()));
                    }
                    _ => {}
                }
            }
        });

        // ── send task ─────────────────────────────────────────────────────────
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
                }
            }
        });

        // Send connect request
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

    /// Drain all pending events from the background task.
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

    /// Send raw bytes to the server.
    pub fn send(&self, data: &[u8]) -> Result<(), NetError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(NetError::NotConnected);
        }
        self.tx_out.send(data.to_vec()).ok();
        Ok(())
    }

    /// Send a JSON-serializable value to the server.
    pub fn send_json<T: serde::Serialize>(&self, value: &T) -> Result<(), NetError> {
        let bytes = serde_json::to_vec(value)
            .map_err(|e| NetError::Io(std::io::Error::other(e.to_string())))?;
        self.send(&bytes)
    }

    /// `true` if the connection handshake has completed.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Tick timers. Call once per frame with the frame delta time.
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

    /// Gracefully disconnect from the server.
    pub fn disconnect(&self) {
        self.tx_ctrl.send(KIND_DISCONNECT).ok();
        self.connected.store(false, Ordering::Relaxed);
    }
}

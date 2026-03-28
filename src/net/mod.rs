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
//! server.update(1.0 / 60.0);
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
//! client.update(1.0 / 60.0);
//! ```

mod client;
mod protocol;
mod server;

pub use client::{Client, ClientEvent};
pub use protocol::{ClientId, NetError};
pub use server::{Server, ServerEvent};

//! Network demo — server.
//!
//! Run this first, then run `net_client` in another terminal.
//! The server echoes every message back to the sender and
//! broadcasts a timestamp to all clients every second.
//!
//! Usage: cargo run --example net_server

use nene::net::{Server, ServerEvent};
use std::time::{Duration, Instant};

fn main() {
    println!("Starting server on 0.0.0.0:7777 …");
    let mut server = Server::bind("0.0.0.0:7777").expect("bind failed");

    let mut elapsed = 0.0f32;
    let mut broadcast_timer = 0.0f32;
    let start = Instant::now();
    let mut last = start;

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;
        elapsed += dt;
        broadcast_timer += dt;

        // ── poll events ───────────────────────────────────────────────────────
        for ev in server.poll() {
            match ev {
                ServerEvent::Connected(id) => {
                    println!(
                        "[{elapsed:.1}s] client {id} connected  (total: {})",
                        server.client_count()
                    );
                }
                ServerEvent::Disconnected(id) => {
                    println!("[{elapsed:.1}s] client {id} disconnected");
                }
                ServerEvent::Message(id, data) => {
                    let text = String::from_utf8_lossy(&data);
                    println!("[{elapsed:.1}s] from {id}: {text}");
                    // echo back
                    let reply = format!("echo: {text}");
                    let _ = server.send(id, reply.as_bytes());
                }
            }
        }

        // ── broadcast timestamp every second ──────────────────────────────────
        if broadcast_timer >= 1.0 && server.client_count() > 0 {
            broadcast_timer = 0.0;
            let msg = format!("server time: {elapsed:.1}s");
            let _ = server.broadcast(msg.as_bytes());
        }

        server.update(dt);
        std::thread::sleep(Duration::from_millis(16));
    }
}

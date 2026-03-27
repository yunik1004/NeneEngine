//! Network demo — client.
//!
//! Run `net_server` first, then run this.
//! Type messages and press Enter to send. The server echoes them back.
//!
//! Usage: cargo run --example net_client

use nene::net::{Client, ClientEvent};
use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::time::Duration;

fn main() {
    println!("Connecting to 127.0.0.1:7777 …");
    let mut client = Client::connect("127.0.0.1:7777").expect("connect failed");

    // Read stdin on a separate thread so we don't block the network poll.
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if let Ok(l) = line {
                if tx.send(l).is_err() {
                    break;
                }
            }
        }
    });

    let mut elapsed = 0.0f32;
    let mut last = std::time::Instant::now();

    loop {
        let now = std::time::Instant::now();
        let dt = now.duration_since(last).as_secs_f32();
        last = now;
        elapsed += dt;

        // ── poll network events ───────────────────────────────────────────────
        for ev in client.poll() {
            match ev {
                ClientEvent::Connected => {
                    println!("Connected! Type a message and press Enter.");
                    print!("> ");
                    io::stdout().flush().ok();
                }
                ClientEvent::Disconnected => {
                    println!("Disconnected from server.");
                    return;
                }
                ClientEvent::Message(data) => {
                    let text = String::from_utf8_lossy(&data);
                    println!("\n[{elapsed:.1}s] {text}");
                    print!("> ");
                    io::stdout().flush().ok();
                }
            }
        }

        // ── send stdin lines ──────────────────────────────────────────────────
        while let Ok(line) = rx.try_recv() {
            if line == "quit" {
                client.disconnect();
                println!("Disconnected.");
                return;
            }
            if let Err(e) = client.send(line.as_bytes()) {
                eprintln!("send error: {e}");
            }
            print!("> ");
            io::stdout().flush().ok();
        }

        client.update(dt);
        std::thread::sleep(Duration::from_millis(16));
    }
}
